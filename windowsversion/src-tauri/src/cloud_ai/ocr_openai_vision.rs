//! OCR via OpenAI Vision (gpt-4o-mini)
//!
//! 替代 macOS Vision Framework：图片 → base64 → Chat Completions（含 vision
//! content block）→ 结构化 `Vec<OcrRegion>`。
//!
//! PDF 走两条路径：
//! 1. `pdf_page_count`：纯 `lopdf`，零网络调用；
//! 2. `ocr_pdf_page`：优先尝试用 `pdf-extract` 抽取该页文本（lopdf 本身不能
//!    渲染像素，云端渲染又会产生额外依赖与延迟），命中即合成单一 `OcrRegion`
//!    返回；失败再 raise `ExtractionError::OcrError`。
//!
//! 这条降级路径有意"宽松对待精度"——Windows 版云端 OCR 的目标是让流水线跑通，
//! 后续若用户对扫描型 PDF 精度有要求，可以接入 Poppler/Ghostscript 渲染再走
//! OpenAI Vision，但这超出本 Unit 范围。

use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};

use crate::extraction::models::ExtractionError;

use super::{resolve_openai_api_key, resolve_openai_base_url};

const OPENAI_VISION_MODEL: &str = "gpt-4o-mini";
const OPENAI_VISION_PATH: &str = "/v1/chat/completions";

/// OCR 区域；字段与 macOS `OcrRegion`（`macos::ocr_ffi`）保持一致，
/// 让 Unit 14 在两边切换时无需做字段映射。
///
/// 注意：云端 LLM 的 `bbox` 是估算值（按 "top/middle/bottom" 三档纵向位置粗略给出），
/// `confidence` 取自模型对该行的自评（0.0~1.0），与 Vision 的几何精度不对等，
/// 仅供"是否可疑"提示用。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrRegion {
    pub text: String,
    pub confidence: f64,
    pub bbox: [f64; 4],
}

// ── OpenAI Vision 请求/响应模型 ─────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: &'static str,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
    /// 强制 JSON 响应；gpt-4o-mini 支持 `{"type":"json_object"}`。
    response_format: ResponseFormat,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    fmt_type: &'static str,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: &'static str,
    content: Vec<ContentPart>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum ContentPart {
    Text {
        #[serde(rename = "type")]
        part_type: &'static str,
        text: String,
    },
    ImageUrl {
        #[serde(rename = "type")]
        part_type: &'static str,
        image_url: ImageUrl,
    },
}

#[derive(Debug, Serialize)]
struct ImageUrl {
    url: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct ChatChoiceMessage {
    content: Option<String>,
}

/// 模型按本 schema 返回。`position` 取值 `top` / `middle` / `bottom`，用于近似
/// 还原 bbox 的纵向位置；`confidence` 由模型自评（0~1）。
#[derive(Debug, Deserialize)]
struct ParsedOcrPayload {
    #[serde(default)]
    lines: Vec<ParsedOcrLine>,
}

#[derive(Debug, Deserialize)]
struct ParsedOcrLine {
    text: String,
    #[serde(default)]
    position: Option<String>,
    #[serde(default)]
    confidence: Option<f64>,
}

// ── HTTP client（单例，带超时）──────────────────────────────────────────────

fn cloud_http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(120))
            .build()
            .expect("reqwest cloud_ai vision client")
    })
}

// ── 对外 API ─────────────────────────────────────────────────────────────────

/// 用 OpenAI Vision OCR 一张图。
pub async fn ocr_image(file_path: &Path) -> Result<Vec<OcrRegion>, ExtractionError> {
    let bytes = tokio::fs::read(file_path)
        .await
        .map_err(ExtractionError::IoError)?;
    let mime = mime_for_image(file_path);
    let data_url = format!("data:{};base64,{}", mime, BASE64.encode(&bytes));
    ocr_data_url(&data_url).await
}

/// PDF 总页数：纯 `lopdf`，零网络调用。
///
/// 返回 `i32` 而非 `usize` 是为了对齐 Unit 14 stub 给定的接口契约（与
/// macOS `macos::ocr_ffi::pdf_page_count` 的 `Result<i32, String>` 同形）。
pub async fn pdf_page_count(pdf_path: &Path) -> Result<i32, ExtractionError> {
    let doc = lopdf::Document::load(pdf_path).map_err(|e| {
        ExtractionError::ParseError(format!("lopdf 打开 PDF 失败 ({}): {e}", pdf_path.display()))
    })?;
    let n = doc.get_pages().len();
    i32::try_from(n).map_err(|_| {
        ExtractionError::ParseError(format!("PDF 页数超出 i32 范围: {n}"))
    })
}

/// PDF 单页 OCR。
///
/// 当前实现走文本抽取降级（见模块顶层文档）：用 `pdf-extract` 抽取全文，
/// 按 form feed `\x0C` 切页（pdf-extract 没有可靠的逐页接口），命中目标页
/// 则把整页文本作为单个 `OcrRegion` 返回。对原生文本 PDF 完全可用；对扫描
/// 型 PDF（全是图）会返回空向量，由上层走 `needs_ocr_fallback`。
///
/// `page_index` 0-based，与 macOS `ocr_ffi::ocr_pdf_page` 一致。
pub async fn ocr_pdf_page(
    pdf_path: &Path,
    page_index: i32,
) -> Result<Vec<OcrRegion>, ExtractionError> {
    if page_index < 0 {
        return Err(ExtractionError::ParseError(format!(
            "page_index 不能为负: {page_index}"
        )));
    }
    let total = pdf_page_count(pdf_path).await?;
    if page_index >= total {
        return Err(ExtractionError::ParseError(format!(
            "页码越界: page_index={page_index} >= total_pages={total}"
        )));
    }

    // pdf-extract 是同步 CPU 工作，放到 blocking pool 避免阻塞 runtime
    let pdf_path_buf = pdf_path.to_path_buf();
    let all_text = tokio::task::spawn_blocking(move || {
        pdf_extract::extract_text(&pdf_path_buf)
            .map_err(|e| ExtractionError::ParseError(format!("pdf-extract 抽取失败: {e}")))
    })
    .await
    .map_err(|e| ExtractionError::OcrError(format!("blocking task join 失败: {e}")))??;

    // pdf-extract 按 \x0C 分页；若分页失败则把全文当作单页
    let page_text = all_text
        .split('\u{000C}')
        .nth(page_index as usize)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("")
        .to_string();

    if page_text.is_empty() {
        // 扫描型 PDF 或抽取失败：返回空向量；上层走 `needs_ocr_fallback` 流程
        return Ok(vec![]);
    }

    Ok(vec![OcrRegion {
        text: page_text,
        confidence: 0.5, // 文本抽取走出来的内容，置信度按"中等"标注
        bbox: [0.0, 0.0, 1.0, 1.0],
    }])
}

// ── 私有：调用 OpenAI Vision ────────────────────────────────────────────────

/// 给定 `data:image/...;base64,...` URL，调用 OpenAI Vision，返回结构化结果。
async fn ocr_data_url(data_url: &str) -> Result<Vec<OcrRegion>, ExtractionError> {
    let api_key = resolve_openai_api_key()?;
    let url = format!("{}{}", resolve_openai_base_url(), OPENAI_VISION_PATH);

    let prompt = concat!(
        "你是一名专业的 OCR 引擎。请提取图片中的所有可见文字，按视觉顺序（从上到下、",
        "从左到右）逐行返回。严格按以下 JSON schema 输出，不要任何解释性文字：\n",
        "{\n",
        "  \"lines\": [\n",
        "    { \"text\": \"<这一行的文字内容>\", \"position\": \"top|middle|bottom\", \"confidence\": 0.0~1.0 }\n",
        "  ]\n",
        "}\n",
        "若图中没有任何文字，返回 {\"lines\": []}。",
    )
    .to_string();

    let req = ChatRequest {
        model: OPENAI_VISION_MODEL,
        messages: vec![ChatMessage {
            role: "user",
            content: vec![
                ContentPart::Text {
                    part_type: "text",
                    text: prompt,
                },
                ContentPart::ImageUrl {
                    part_type: "image_url",
                    image_url: ImageUrl {
                        url: data_url.to_string(),
                    },
                },
            ],
        }],
        max_tokens: 4096,
        temperature: 0.0,
        response_format: ResponseFormat {
            fmt_type: "json_object",
        },
    };

    let res = cloud_http_client()
        .post(&url)
        .bearer_auth(&api_key)
        .json(&req)
        .send()
        .await
        .map_err(|e| ExtractionError::OcrError(format!("OpenAI Vision 网络请求失败: {e}")))?;

    let status = res.status();
    let body = res
        .text()
        .await
        .map_err(|e| ExtractionError::OcrError(format!("读取 Vision 响应失败: {e}")))?;
    if !status.is_success() {
        return Err(ExtractionError::OcrError(format!(
            "OpenAI Vision API 错误 ({status}): {body}"
        )));
    }

    let parsed: ChatResponse = serde_json::from_str(&body)
        .map_err(|e| ExtractionError::ParseError(format!("解析 Vision 响应失败: {e}; body={body}")))?;

    let content = parsed
        .choices
        .into_iter()
        .next()
        .and_then(|c| c.message.content)
        .ok_or_else(|| ExtractionError::ParseError("Vision 响应中没有 choices[0].message.content".to_string()))?;

    let payload: ParsedOcrPayload = serde_json::from_str(content.trim()).map_err(|e| {
        ExtractionError::ParseError(format!("Vision 内层 JSON 解析失败: {e}; content={content}"))
    })?;

    Ok(payload
        .lines
        .into_iter()
        .map(|line| OcrRegion {
            text: line.text,
            confidence: line.confidence.unwrap_or(0.8).clamp(0.0, 1.0),
            bbox: bbox_for_position(line.position.as_deref()),
        })
        .collect())
}

/// 按 LLM 返回的 `top|middle|bottom` 给出近似 bbox。
/// 注意：bbox 仅用于上层"是否可疑/排序"参考，云端 OCR 不承诺像素级精度。
fn bbox_for_position(position: Option<&str>) -> [f64; 4] {
    match position.map(str::to_ascii_lowercase).as_deref() {
        Some("top") => [0.0, 0.66, 1.0, 1.0],
        Some("bottom") => [0.0, 0.0, 1.0, 0.33],
        // middle / 未知 / None 都归到中间
        _ => [0.0, 0.33, 1.0, 0.66],
    }
}

/// 用扩展名粗略推断 image MIME；OpenAI 接受 jpeg/png/gif/webp。
fn mime_for_image(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        // HEIC 不被 OpenAI Vision 原生支持，这里仍标注 jpeg 让请求至少能到端；
        // 真要稳妥处理 HEIC，需先转 JPEG，TODO 后续 Unit。
        Some("heic") | Some("heif") => "image/jpeg",
        // jpg / jpeg / 其他默认 jpeg（与 OpenAI 接受度最宽的格式对齐）
        _ => "image/jpeg",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mime_for_image_recognizes_common_formats() {
        assert_eq!(mime_for_image(Path::new("a.png")), "image/png");
        assert_eq!(mime_for_image(Path::new("a.PNG")), "image/png");
        assert_eq!(mime_for_image(Path::new("a.gif")), "image/gif");
        assert_eq!(mime_for_image(Path::new("a.webp")), "image/webp");
        assert_eq!(mime_for_image(Path::new("a.jpg")), "image/jpeg");
        assert_eq!(mime_for_image(Path::new("a.jpeg")), "image/jpeg");
        assert_eq!(mime_for_image(Path::new("a.heic")), "image/jpeg");
        assert_eq!(mime_for_image(Path::new("noext")), "image/jpeg");
    }

    #[test]
    fn bbox_for_position_maps_known_values() {
        let top = bbox_for_position(Some("top"));
        let bottom = bbox_for_position(Some("bottom"));
        let middle = bbox_for_position(Some("middle"));
        let none = bbox_for_position(None);
        let unknown = bbox_for_position(Some("weird"));

        // top 的 y 范围位于上方（数值大）
        assert!(top[1] > middle[1]);
        // bottom 的 y 范围位于下方（数值小）
        assert!(bottom[3] < middle[3]);
        // 未知/缺失值落到 middle
        assert_eq!(none, middle);
        assert_eq!(unknown, middle);
    }

    #[test]
    fn bbox_for_position_handles_case_variants() {
        assert_eq!(bbox_for_position(Some("TOP")), bbox_for_position(Some("top")));
        assert_eq!(
            bbox_for_position(Some("Bottom")),
            bbox_for_position(Some("bottom"))
        );
    }
}
