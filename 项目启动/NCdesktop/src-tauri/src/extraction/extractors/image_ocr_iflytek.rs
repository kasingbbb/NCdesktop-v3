//! 科大讯飞「通用文档识别(OCR 大模型)」WebAPI 提取器 —— 图片转 Markdown。
//!
//! 服务：se75ocrbm（<https://www.xfyun.cn/doc/words/OCRforLLM/API.html>）
//! 端点：`https://cbm01.cn-huabei-1.xf-yun.com/v1/private/se75ocrbm`
//! 鉴权：HMAC-SHA256，签名串 = `host`+`date`+`request-line` 三行，authorization 走 query 参数
//!       （与 audio_asr_iflytek 的 iflyaisol.com HMAC-SHA1 方案**不同**，那是另一套服务域）。
//!
//! 用途：图片(jpg/png/bmp)→MD 走本提取器；**PDF 不走 OCR**（仍交给 markitdown）。
//! 路由见 `extractors::get_extractor_for`：图片在 markitdown 之前被本提取器拦截。

use std::path::Path;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::Sha256;

use crate::extraction::{
    models::{ContentSegment, ExtractionError, ExtractionResult, ExtractOptions},
    Extractor,
};

// ── 凭据（与 audio_asr_iflytek 同一讯飞账号；后续可迁移至设置页）──────────────
const APPID: &str = "6b22481d";
const API_KEY: &str = "05c5027bf1c45c067a7c78d7f3c11243";
const API_SECRET: &str = "OTNjODViOTczODdiOWYwYmZkZTRkMzVk";

// ── API 端点 ─────────────────────────────────────────────────────────────
const OCR_HOST: &str = "cbm01.cn-huabei-1.xf-yun.com";
const OCR_PATH: &str = "/v1/private/se75ocrbm";
const OCR_URL: &str = "https://cbm01.cn-huabei-1.xf-yun.com/v1/private/se75ocrbm";

/// 讯飞 OCR 大模型单图 base64 体积上限（文档：image 字段 ≤ 10MB；保守按 8MB 预警）。
const OCR_BASE64_WARN_BYTES: usize = 8 * 1024 * 1024;

type HmacSha256 = Hmac<Sha256>;

// ── 工具：RFC1123 GMT 时间 + RFC3986 百分号编码 ────────────────────────────

/// RFC1123 格式 GMT 时间，例 `Wed, 11 Aug 2021 06:55:18 GMT`。
/// chrono 的 `%a`/`%b` 恒为英文缩写（不随 locale 变化），符合讯飞要求。
fn datetime_rfc1123() -> String {
    chrono::Utc::now()
        .format("%a, %d %b %Y %H:%M:%S GMT")
        .to_string()
}

/// RFC3986 组件编码：仅 `A-Za-z0-9-_.~` 不编码，其余一律 %XX（大写）。
/// 用于把 authorization / host / date 安全拼进 URL query。空格→`%20`。
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push('%');
                out.push(char::from_digit((b >> 4) as u32, 16).unwrap().to_ascii_uppercase());
                out.push(char::from_digit((b & 0xf) as u32, 16).unwrap().to_ascii_uppercase());
            }
        }
    }
    out
}

/// 生成带鉴权 query 的最终请求 URL。
///
/// 步骤（讯飞通用鉴权 v2）：
///   1. signature_origin = "host: {host}\ndate: {date}\nPOST {path} HTTP/1.1"
///   2. signature = base64(HMAC-SHA256(api_secret, signature_origin))
///   3. authorization_origin = `api_key="..", algorithm="hmac-sha256", headers="host date request-line", signature=".."`
///   4. authorization = base64(authorization_origin)
///   5. URL = {OCR_URL}?authorization=&host=&date=（三者 percent-encode）
fn build_signed_url(date: &str) -> Result<String, ExtractionError> {
    let signature_origin = format!(
        "host: {OCR_HOST}\ndate: {date}\nPOST {OCR_PATH} HTTP/1.1"
    );

    let mut mac = HmacSha256::new_from_slice(API_SECRET.as_bytes())
        .map_err(|e| ExtractionError::OcrError(format!("讯飞 OCR 鉴权初始化失败: {e}")))?;
    mac.update(signature_origin.as_bytes());
    let signature = BASE64.encode(mac.finalize().into_bytes());

    let authorization_origin = format!(
        "api_key=\"{API_KEY}\", algorithm=\"hmac-sha256\", headers=\"host date request-line\", signature=\"{signature}\""
    );
    let authorization = BASE64.encode(authorization_origin.as_bytes());

    Ok(format!(
        "{OCR_URL}?authorization={}&host={}&date={}",
        percent_encode(&authorization),
        percent_encode(OCR_HOST),
        percent_encode(date),
    ))
}

/// 由 MIME / 扩展名推断讯飞 `image.encoding` 取值（仅 jpg/png/bmp 受支持）。
fn image_encoding_for(mime_type: &str, file_path: &Path) -> &'static str {
    match mime_type {
        "image/png" => "png",
        "image/bmp" => "bmp",
        "image/jpeg" | "image/jpg" => "jpg",
        _ => {
            // 兜底看扩展名
            match file_path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_ascii_lowercase())
                .as_deref()
            {
                Some("png") => "png",
                Some("bmp") => "bmp",
                _ => "jpg",
            }
        }
    }
}

// ── 响应结构体 ────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
struct OcrResponse {
    header: OcrHeader,
    #[serde(default)]
    payload: Option<OcrPayload>,
}

#[derive(Deserialize, Debug)]
struct OcrHeader {
    code: i64,
    #[serde(default)]
    message: String,
    #[serde(default)]
    sid: String,
}

#[derive(Deserialize, Debug)]
struct OcrPayload {
    #[serde(default)]
    result: Option<OcrResult>,
}

#[derive(Deserialize, Debug)]
struct OcrResult {
    #[serde(default)]
    text: String,
}

// ── 提取器 ────────────────────────────────────────────────────────────────

pub struct IflytekOcrExtractor;

impl Extractor for IflytekOcrExtractor {
    fn can_handle(&self, mime_type: &str) -> bool {
        // 仅 se75ocrbm 受支持的位图格式；heic/webp/gif/tiff/svg 不在此（交回 markitdown）。
        matches!(
            mime_type,
            "image/jpeg" | "image/jpg" | "image/png" | "image/bmp"
        )
    }

    fn name(&self) -> &'static str {
        "image_ocr_iflytek"
    }

    fn extract(
        &self,
        file_path: &Path,
        _options: &ExtractOptions,
    ) -> Result<ExtractionResult, ExtractionError> {
        let mime_hint = self_mime_hint(file_path);
        tokio::runtime::Handle::current().block_on(recognize(file_path, &mime_hint))
    }
}

/// extract() 不带 mime（trait 限制），这里据扩展名给个提示供 encoding 推断。
fn self_mime_hint(file_path: &Path) -> String {
    match file_path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("bmp") => "image/bmp",
        _ => "image/jpeg",
    }
    .to_string()
}

// ── 核心异步逻辑 ──────────────────────────────────────────────────────────

async fn recognize(
    file_path: &Path,
    mime_type: &str,
) -> Result<ExtractionResult, ExtractionError> {
    let bytes = tokio::fs::read(file_path)
        .await
        .map_err(|e| ExtractionError::OcrError(format!("读取图片失败: {e}")))?;
    let image_b64 = BASE64.encode(&bytes);

    if image_b64.len() > OCR_BASE64_WARN_BYTES {
        log::warn!(
            "[讯飞 OCR] 图片较大（base64 {:.1} MB），可能超出 se75ocrbm 单图上限",
            image_b64.len() as f64 / 1024.0 / 1024.0
        );
    }

    let encoding = image_encoding_for(mime_type, file_path);
    let body = json!({
        "header": { "app_id": APPID, "status": 0 },
        "parameter": {
            "ocr": {
                "result_option": "normal",
                "result_format": "json",
                "output_type": "one_shot",
                "result": { "encoding": "utf8", "compress": "raw", "format": "plain" }
            }
        },
        "payload": {
            "image": { "encoding": encoding, "image": image_b64, "status": 0 }
        }
    });

    let date = datetime_rfc1123();
    let url = build_signed_url(&date)?;

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(30))
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| ExtractionError::OcrError(format!("HTTP 客户端创建失败: {e}")))?;

    log::info!(
        "[讯飞 OCR] 提交识别：file={} encoding={} bytes={}",
        file_path.display(),
        encoding,
        bytes.len()
    );

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Host", OCR_HOST)
        .json(&body)
        .send()
        .await
        .map_err(|e| ExtractionError::OcrError(format!("讯飞 OCR 请求失败: {e}")))?;

    let http_status = resp.status();
    let raw = resp
        .text()
        .await
        .map_err(|e| ExtractionError::OcrError(format!("讯飞 OCR 响应读取失败: {e}")))?;

    if !http_status.is_success() {
        log::error!(
            "[讯飞 OCR] HTTP {http_status} 响应（前 500）: {}",
            raw.chars().take(500).collect::<String>()
        );
        return Err(ExtractionError::OcrError(format!(
            "讯飞 OCR 失败 HTTP {http_status}: {raw}"
        )));
    }

    let parsed: OcrResponse = serde_json::from_str(&raw).map_err(|e| {
        log::error!(
            "[讯飞 OCR] 响应 JSON 解析失败: {e}，原始（前 500）: {}",
            raw.chars().take(500).collect::<String>()
        );
        ExtractionError::OcrError(format!("讯飞 OCR 响应解析失败: {e}"))
    })?;

    if parsed.header.code != 0 {
        log::error!(
            "[讯飞 OCR] 业务错误 code={} message={} sid={}",
            parsed.header.code,
            parsed.header.message,
            parsed.header.sid
        );
        return Err(ExtractionError::OcrError(format!(
            "讯飞 OCR 错误 code={}: {}",
            parsed.header.code, parsed.header.message
        )));
    }

    let text_b64 = parsed
        .payload
        .and_then(|p| p.result)
        .map(|r| r.text)
        .unwrap_or_default();

    if text_b64.trim().is_empty() {
        log::info!("[讯飞 OCR] 识别完成但结果为空（图片可能无文字）");
        return Ok(empty_result());
    }

    let decoded_bytes = BASE64.decode(text_b64.trim()).map_err(|e| {
        ExtractionError::OcrError(format!("讯飞 OCR 结果 base64 解码失败: {e}"))
    })?;
    let decoded = String::from_utf8_lossy(&decoded_bytes).to_string();
    log::debug!(
        "[讯飞 OCR] 结果原文（前 800）: {}",
        decoded.chars().take(800).collect::<String>()
    );

    let text = extract_text_from_ocr_result(&decoded);

    if text.trim().is_empty() {
        return Ok(empty_result());
    }

    let segments = vec![ContentSegment {
        segment_type: "ocr_text".to_string(),
        content: text.clone(),
        page: None,
        confidence: None,
        bbox: None,
    }];

    Ok(ExtractionResult {
        raw_text: text.clone(),
        structured_md: text,
        quality_level: 1,
        extractor_type: "image_ocr_iflytek".to_string(),
        segments,
        needs_ocr_fallback: false,
    })
}

fn empty_result() -> ExtractionResult {
    ExtractionResult {
        raw_text: String::new(),
        structured_md: String::new(),
        quality_level: 0,
        extractor_type: "image_ocr_iflytek".to_string(),
        segments: vec![],
        needs_ocr_fallback: false,
    }
}

/// 解析 se75ocrbm 解码后的结果文本 → 纯文本/Markdown。
///
/// 真机验证（2026-05-30，live se75ocrbm）：`result_format=json` 下，`text`（base64 解码后）
/// 是一棵深层嵌套 JSON，识别文本落在 `type=="text_block"` 节点的 `text` 字段
/// （形如 `["整行文本"]` 数组 或 `"整行文本"` 字符串）。**没有** pages/lines 结构。
///
/// 防御式解析（按优先级）：
///   1. 若是 JSON：递归收集所有 `type=="text_block"` 的 `text` 作行文本（命中后不深入，
///      避免 `text_unit` 子片段重复）—— 这是 se75ocrbm 的真实结构；
///   2. 兜底：老式 `pages[].lines[].content`（防未来 SKU 形态差异）；
///   3. 兜底：递归收集所有 `content` 字符串叶子；
///   4. JSON 解析成功但三法都取不到 → 返回空串（绝不把整坨 JSON 当 MD 落库）。
///   5. 非 JSON：当作纯文本 / Markdown 原样返回。
///
/// 注：首次真机如遇异常，看 `[讯飞 OCR] 结果原文` debug 日志核对结构。
fn extract_text_from_ocr_result(decoded: &str) -> String {
    let trimmed = decoded.trim();
    let looks_like_json = trimmed.starts_with('{') || trimmed.starts_with('[');
    if looks_like_json {
        if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
            // (1) se75ocrbm 真实结构：type=="text_block" 节点
            let mut lines: Vec<String> = Vec::new();
            collect_text_blocks(&v, &mut lines);
            if !lines.is_empty() {
                return lines.join("\n").trim().to_string();
            }
            // (2) 老式 pages/lines 兜底
            let structured = extract_pages_lines(&v);
            if !structured.trim().is_empty() {
                return structured.trim().to_string();
            }
            // (3) 递归 content 叶子兜底
            let mut recursive = String::new();
            collect_content_recursive(&v, &mut recursive);
            if !recursive.trim().is_empty() {
                return recursive.trim().to_string();
            }
            // (4) 结构化全部落空 → 返回空，不回吐 JSON
            return String::new();
        }
    }
    // (5) 非 JSON：原样（纯文本 / markdown）
    trimmed.to_string()
}

/// 递归收集 se75ocrbm 的 `type=="text_block"` 节点文本。
///
/// `text_block.text` 形如 `["整行文本"]`（数组）或 `"整行文本"`（字符串）。
/// 命中 text_block 后**不再深入**其子节点，避免把内部 `text_unit` 子片段重复计入。
fn collect_text_blocks(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(map) => {
            if map.get("type").and_then(|t| t.as_str()) == Some("text_block") {
                if let Some(t) = map.get("text") {
                    let line = match t {
                        Value::String(s) => s.clone(),
                        Value::Array(arr) => arr
                            .iter()
                            .filter_map(|x| x.as_str())
                            .collect::<Vec<_>>()
                            .join(""),
                        _ => String::new(),
                    };
                    let line = line.trim();
                    if !line.is_empty() {
                        out.push(line.to_string());
                    }
                }
                return; // 不深入 text_block 内部
            }
            for val in map.values() {
                collect_text_blocks(val, out);
            }
        }
        Value::Array(arr) => {
            for item in arr {
                collect_text_blocks(item, out);
            }
        }
        _ => {}
    }
}

/// 按 `pages[].lines[]` 结构抽取文本：行级 `content` 优先，缺失则拼 `words[].content`。
fn extract_pages_lines(v: &Value) -> String {
    let Some(pages) = v.get("pages").and_then(|p| p.as_array()) else {
        return String::new();
    };
    let mut out = String::new();
    for page in pages {
        let Some(lines) = page.get("lines").and_then(|l| l.as_array()) else {
            continue;
        };
        for line in lines {
            if let Some(c) = line.get("content").and_then(|c| c.as_str()) {
                if !c.trim().is_empty() {
                    out.push_str(c.trim());
                    out.push('\n');
                }
            } else if let Some(words) = line.get("words").and_then(|w| w.as_array()) {
                let mut line_text = String::new();
                for w in words {
                    if let Some(s) = w.get("content").and_then(|s| s.as_str()) {
                        line_text.push_str(s);
                    }
                }
                if !line_text.trim().is_empty() {
                    out.push_str(line_text.trim());
                    out.push('\n');
                }
            }
        }
        out.push('\n'); // 页分隔
    }
    out
}

/// 兜底：递归收集 JSON 中所有名为 `content` 的字符串叶子（按出现顺序、换行连接）。
fn collect_content_recursive(v: &Value, out: &mut String) {
    match v {
        Value::Object(map) => {
            for (k, val) in map {
                if k == "content" {
                    if let Some(s) = val.as_str() {
                        if !s.trim().is_empty() {
                            out.push_str(s.trim());
                            out.push('\n');
                        }
                        continue;
                    }
                }
                collect_content_recursive(val, out);
            }
        }
        Value::Array(arr) => {
            for item in arr {
                collect_content_recursive(item, out);
            }
        }
        _ => {}
    }
}

// ── 测试 ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_handle_only_bitmap_images() {
        let e = IflytekOcrExtractor;
        assert!(e.can_handle("image/jpeg"));
        assert!(e.can_handle("image/jpg"));
        assert!(e.can_handle("image/png"));
        assert!(e.can_handle("image/bmp"));
        // 这些不归 OCR（交回 markitdown），且 PDF 绝不走 OCR
        assert!(!e.can_handle("image/heic"));
        assert!(!e.can_handle("image/webp"));
        assert!(!e.can_handle("image/gif"));
        assert!(!e.can_handle("application/pdf"));
    }

    #[test]
    fn percent_encode_matches_rfc3986() {
        assert_eq!(percent_encode("abc-_.~"), "abc-_.~");
        assert_eq!(percent_encode("a b"), "a%20b");
        // base64 里的 + / = 必须编码，否则 URL 解码错乱
        assert_eq!(percent_encode("a+b/c=d"), "a%2Bb%2Fc%3Dd");
        // RFC1123 日期里的逗号和冒号
        assert_eq!(
            percent_encode("Wed, 11 Aug 2021 06:55:18 GMT"),
            "Wed%2C%2011%20Aug%202021%2006%3A55%3A18%20GMT"
        );
    }

    #[test]
    fn build_signed_url_is_deterministic_for_fixed_date() {
        let date = "Wed, 11 Aug 2021 06:55:18 GMT";
        let u1 = build_signed_url(date).unwrap();
        let u2 = build_signed_url(date).unwrap();
        assert_eq!(u1, u2);
        assert!(u1.starts_with(OCR_URL));
        assert!(u1.contains("authorization="));
        assert!(u1.contains("host="));
        assert!(u1.contains("date="));
    }

    #[test]
    fn image_encoding_inference() {
        let p = Path::new("/tmp/x.PNG");
        assert_eq!(image_encoding_for("image/png", p), "png");
        assert_eq!(image_encoding_for("image/jpeg", Path::new("/tmp/a.jpg")), "jpg");
        assert_eq!(image_encoding_for("image/bmp", Path::new("/tmp/a.bmp")), "bmp");
        // mime 缺失 → 看扩展名
        assert_eq!(image_encoding_for("", Path::new("/tmp/a.png")), "png");
        assert_eq!(image_encoding_for("", Path::new("/tmp/a.unknown")), "jpg");
    }

    #[test]
    fn extract_pages_lines_joins_line_content() {
        let v: Value = serde_json::json!({
            "pages": [
                { "lines": [ { "content": "第一行" }, { "content": "第二行" } ] },
                { "lines": [ { "content": "第二页" } ] }
            ]
        });
        let out = extract_pages_lines(&v);
        assert!(out.contains("第一行"));
        assert!(out.contains("第二行"));
        assert!(out.contains("第二页"));
    }

    #[test]
    fn extract_pages_lines_falls_back_to_words() {
        let v: Value = serde_json::json!({
            "pages": [ { "lines": [ { "words": [ {"content": "你"}, {"content": "好"} ] } ] } ]
        });
        let out = extract_pages_lines(&v);
        assert_eq!(out.trim(), "你好");
    }

    #[test]
    fn extract_text_handles_plain_text_passthrough() {
        // 非 JSON → 原样返回
        let t = extract_text_from_ocr_result("这是一段纯文本结果");
        assert_eq!(t, "这是一段纯文本结果");
    }

    #[test]
    fn extract_text_recursive_fallback() {
        // 没有 pages/lines，但深层有 content 叶子
        let decoded = r#"{"document":{"blocks":[{"content":"标题"},{"sub":{"content":"正文"}}]}}"#;
        let t = extract_text_from_ocr_result(decoded);
        assert!(t.contains("标题"));
        assert!(t.contains("正文"));
    }

    /// 真机 se75ocrbm 结构（2026-05-30 验证）：识别文本在 type=="text_block" 的 text 字段。
    /// 这是去掉 coord/contour 等噪声后的代表性裁剪。
    #[test]
    fn extract_text_from_real_se75ocrbm_text_block_structure() {
        let decoded = r#"{
          "document": [],
          "image": [{
            "content": [[{
              "category": "text",
              "content": [[
                {"type":"text_block","id":"7","text":["NoteCapt OCR"],
                 "content":[[{"type":"text_unit","id":"8","text":"NoteCapt OCR"}]]},
                {"type":"text_block","id":"12","text":["Hello World 12345"],
                 "content":[[{"type":"text_unit","id":"13","text":"Hello World 12345"}]]}
              ]]
            }]]
          }]
        }"#;
        let t = extract_text_from_ocr_result(decoded);
        // 应得两行干净文本，且不因 text_unit 子节点重复
        assert_eq!(t, "NoteCapt OCR\nHello World 12345");
        assert_eq!(t.matches("Hello World 12345").count(), 1);
    }

    /// text_block.text 为字符串（非数组）形态也应支持。
    #[test]
    fn collect_text_blocks_handles_string_text() {
        let v: Value = serde_json::json!({
            "x": [{"type":"text_block","text":"单行字符串"}]
        });
        let mut lines = Vec::new();
        collect_text_blocks(&v, &mut lines);
        assert_eq!(lines, vec!["单行字符串".to_string()]);
    }

    /// JSON 解析成功但没有任何可识别文本 → 返回空串（绝不回吐整坨 JSON）。
    #[test]
    fn extract_text_json_without_text_returns_empty_not_raw_json() {
        let decoded = r#"{"document":[],"image":[],"engine_version":"1.1.2"}"#;
        let t = extract_text_from_ocr_result(decoded);
        assert!(t.is_empty(), "无文本时必须返回空，不能把 JSON 当 MD，实际: {t}");
    }

    /// 网络集成冒烟：用**实际 Rust 代码路径**命中真实 se75ocrbm，验证鉴权/请求/解析端到端。
    /// 默认 ignore（需要网络 + 消耗配额）。手动运行：
    ///   cargo test -p notecapt --lib image_ocr_iflytek::tests::live_ocr_smoke -- --ignored --nocapture
    /// 前置：先用脚本生成 /tmp/nc_ocr_test.png（含可识别文字）。
    #[test]
    #[ignore = "network: hits live 讯飞 se75ocrbm; run with --ignored"]
    fn live_ocr_smoke() {
        let path = std::path::Path::new("/tmp/nc_ocr_test.png");
        if !path.exists() {
            eprintln!("跳过 live_ocr_smoke：/tmp/nc_ocr_test.png 不存在");
            return;
        }
        let rt = tokio::runtime::Runtime::new().unwrap();
        let r = rt
            .block_on(recognize(path, "image/png"))
            .expect("OCR 应成功返回");
        eprintln!("==== live OCR structured_md ====\n{}", r.structured_md);
        assert_eq!(r.extractor_type, "image_ocr_iflytek");
        assert!(
            r.structured_md.contains("NoteCapt") || r.structured_md.contains("Hello"),
            "应识别出测试图片中的英文文本，实际: {}",
            r.structured_md
        );
    }
}
