/// 科大讯飞非实时语音转写 WebAPI 提取器
///
/// 文档：https://office-api-ist-dx.iflyaisol.com
/// 鉴权：HMAC-SHA1，参数按 key 自然排序后 URL 编码，signature 放请求头
/// 音频：raw binary 直接上传（Content-Type: application/octet-stream）

use std::collections::BTreeMap;
use std::path::Path;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use hmac::{Hmac, Mac};
use sha1::Sha1;
use serde::Deserialize;

use crate::extraction::{
    models::{ContentSegment, ExtractionError, ExtractionResult, ExtractOptions},
    Extractor,
};

// ── 凭据（v1.3 迁移至设置页）────────────────────────────────────────────
const APPID: &str = "6b22481d";
const ACCESS_KEY_ID: &str = "05c5027bf1c45c067a7c78d7f3c11243";
const ACCESS_KEY_SECRET: &str = "OTNjODViOTczODdiOWYwYmZkZTRkMzVk";

// ── API 端点 ─────────────────────────────────────────────────────────────
const IFLYTEK_BASE_URL: &str = "https://office-api-ist-dx.iflyaisol.com";
const UPLOAD_PATH: &str = "/v2/upload";
const QUERY_PATH: &str = "/v2/getResult";

// ── 轮询参数 ─────────────────────────────────────────────────────────────
const POLL_INTERVAL_SECS: u64 = 10;
const POLL_MAX_RETRIES: u32 = 180; // 180 × 10s = 30 分钟

/// language 默认值。历史 task_014 Fix-A3 写死为 "cn"，但实测部分账号 SKU 返回
/// `code=100020 language[cn] does not support`，需要回退到其他候选值。
/// 通过 `ExtractOptions.iflytek_language` 可由 setting `iflytekLanguage` 显式覆盖。
const DEFAULT_IFLYTEK_LANGUAGE: &str = "cn";

/// 当账号当前 language 值被讯飞拒收（code=100020）时，按顺序自动重试的候选值。
/// 顺序覆盖：通用普通话 → 中文混合 → 自动方言 → 英文 → 空（让服务端用默认值）。
/// 列表里的值会跳过等于"当前已尝试值"的项，避免无效重试。
const IFLYTEK_LANGUAGE_FALLBACKS: &[&str] = &["cn", "mandarin", "cn_lay", "autodialect", "en", ""];

/// 业务错误码：language 不支持。`fn submit_task` 命中后会换下一个候选语言重试。
const IFLYTEK_CODE_LANGUAGE_UNSUPPORTED: &str = "100020";

/// 选择 language 参数：option 非空 → option；否则 DEFAULT_IFLYTEK_LANGUAGE。
fn resolve_language(opts: &ExtractOptions) -> String {
    opts.iflytek_language
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_IFLYTEK_LANGUAGE)
        .to_string()
}

/// 给定"当前已被拒"的语言，返回应该尝试的下一个候选。返回 None 表示候选用尽。
fn next_language_candidate(current: &str) -> Option<&'static str> {
    let mut started_after_current = false;
    for &cand in IFLYTEK_LANGUAGE_FALLBACKS {
        if started_after_current && cand != current {
            return Some(cand);
        }
        if cand == current {
            started_after_current = true;
        }
    }
    // 当前值不在候选表里（用户显式设置过特殊值）→ 从表头试
    if !started_after_current {
        return IFLYTEK_LANGUAGE_FALLBACKS
            .iter()
            .find(|c| **c != current)
            .copied();
    }
    None
}

type HmacSha1 = Hmac<Sha1>;

// ── 工具函数 ─────────────────────────────────────────────────────────────

/// 与 Java URLEncoder.encode 行为完全一致的编码函数
/// 规则：A-Z a-z 0-9 - _ . * 不编码；空格编为 `+`；其余全部 %XX（大写十六进制）
/// 注意：Java URLEncoder 不编码 `*`，但编码 `~`；与标准 percent-encoding 有差异
fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'*' => {
                out.push(byte as char);
            }
            b' ' => {
                out.push('+');
            }
            _ => {
                out.push('%');
                out.push(char::from_digit((byte >> 4) as u32, 16).unwrap().to_ascii_uppercase());
                out.push(char::from_digit((byte & 0xf) as u32, 16).unwrap().to_ascii_uppercase());
            }
        }
    }
    out
}

/// 生成讯飞签名
/// 规则：params 按 key 自然升序，URL 编码 value，&连接，HMAC-SHA1(secret, baseString)，Base64
fn generate_signature(
    params: &BTreeMap<&str, String>,
    secret: &str,
) -> Result<String, ExtractionError> {
    let base_string: String = params
        .iter()
        .filter(|(k, v)| **k != "signature" && !v.is_empty())
        .map(|(k, v)| format!("{k}={}", url_encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    let mut mac = HmacSha1::new_from_slice(secret.as_bytes())
        .map_err(|e| ExtractionError::OcrError(format!("讯飞鉴权初始化失败: {e}")))?;
    mac.update(base_string.as_bytes());
    Ok(BASE64.encode(mac.finalize().into_bytes()))
}

/// 构建 URL 查询串（每个 value 都做 percent 编码）
fn build_query(params: &BTreeMap<&str, String>) -> String {
    params
        .iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(k, v)| format!("{k}={}", url_encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

/// 当前时间，格式 yyyy-MM-dd'T'HH:mm:ss+HHmm（东八区）
fn datetime_now() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%z").to_string()
}

/// 生成 16 位随机字母数字串
fn signature_random() -> String {
    uuid::Uuid::new_v4()
        .to_string()
        .replace('-', "")
        .chars()
        .take(16)
        .collect()
}

// ── 响应结构体 ────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
struct UploadResponse {
    code: String,
    #[serde(default, rename = "descInfo")]
    desc_info: String,
    #[serde(default)]
    content: Option<UploadContent>,
}

#[derive(Deserialize, Debug)]
struct UploadContent {
    #[serde(rename = "orderId")]
    order_id: String,
}

#[derive(Deserialize, Debug)]
struct QueryResponse {
    code: String,
    #[serde(default, rename = "descInfo")]
    desc_info: String,
    #[serde(default)]
    content: Option<QueryContent>,
}

#[derive(Deserialize, Debug)]
struct QueryContent {
    #[serde(default, rename = "orderResult")]
    order_result: String,
    #[serde(rename = "orderInfo")]
    order_info: OrderInfo,
}

#[derive(Deserialize, Debug)]
struct OrderInfo {
    status: i32,
    #[serde(default, rename = "failType")]
    fail_type: i32,
}

// ── 提取器 ────────────────────────────────────────────────────────────────

pub struct IflytekAsrExtractor;

impl Extractor for IflytekAsrExtractor {
    fn can_handle(&self, mime_type: &str) -> bool {
        matches!(
            mime_type,
            "audio/mpeg" | "audio/mp4" | "audio/wav" | "audio/flac" | "audio/x-wav"
        )
    }

    fn name(&self) -> &'static str {
        "audio_asr_iflytek"
    }

    fn extract(
        &self,
        file_path: &Path,
        options: &ExtractOptions,
    ) -> Result<ExtractionResult, ExtractionError> {
        let language = resolve_language(options);
        tokio::runtime::Handle::current().block_on(transcribe(file_path, &language))
    }
}

// ── 核心异步逻辑 ──────────────────────────────────────────────────────────

async fn transcribe(file_path: &Path, language: &str) -> Result<ExtractionResult, ExtractionError> {
    // 自动语言回退：账号 SKU 不同时 100020 (language does not support) 反复出现，
    // 写死单值无法覆盖所有情况；这里在收到 100020 时按 IFLYTEK_LANGUAGE_FALLBACKS
    // 顺序换下一个候选重试 submit_task，直到提交成功或候选用尽。
    let mut current_lang = language.to_string();
    let order_id = loop {
        match submit_task(file_path, &current_lang).await {
            Ok(id) => break id,
            Err(ExtractionError::OcrError(msg))
                if msg.contains(&format!("code={IFLYTEK_CODE_LANGUAGE_UNSUPPORTED}")) =>
            {
                match next_language_candidate(&current_lang) {
                    Some(next) => {
                        log::warn!(
                            "[讯飞 ASR] language=\"{current_lang}\" 被服务端拒收（{IFLYTEK_CODE_LANGUAGE_UNSUPPORTED}），回退尝试 language=\"{next}\""
                        );
                        current_lang = next.to_string();
                    }
                    None => {
                        return Err(ExtractionError::OcrError(format!(
                            "讯飞 ASR：所有候选 language 均被账号拒收（最后一次 code={IFLYTEK_CODE_LANGUAGE_UNSUPPORTED}）。\
                             请去讯飞控制台确认该 appId 实际开通的语言版本，并在 App 设置中配置 iflytekLanguage。原始错误: {msg}"
                        )));
                    }
                }
            }
            Err(e) => return Err(e),
        }
    };
    log::info!("[讯飞 ASR] 任务提交成功，orderId={} language={current_lang}", order_id);

    let text = poll_result(&order_id).await?;

    if text.trim().is_empty() {
        return Ok(ExtractionResult {
            raw_text: String::new(),
            structured_md: String::new(),
            quality_level: 0,
            extractor_type: "audio_asr_iflytek".to_string(),
            segments: vec![],
            needs_ocr_fallback: false,
        });
    }

    let segments = vec![ContentSegment {
        segment_type: "asr_transcription".to_string(),
        content: text.clone(),
        page: None,
        confidence: None,
        bbox: None,
    }];

    Ok(ExtractionResult {
        raw_text: text.clone(),
        structured_md: text,
        quality_level: 1,
        extractor_type: "audio_asr_iflytek".to_string(),
        segments,
        needs_ocr_fallback: false,
    })
}

async fn submit_task(file_path: &Path, language: &str) -> Result<String, ExtractionError> {
    // 流式上传：避免把整个音频文件一次性读入 RAM（大文件可能 OOM）。
    // 通过 tokio::fs::File 元数据拿大小用于签名，再用 ReaderStream 边读边发。
    let meta = tokio::fs::metadata(file_path).await.map_err(|e| {
        ExtractionError::OcrError(format!("读取音频元数据失败: {e}"))
    })?;
    let file_size_bytes = meta.len();
    let file_size = file_size_bytes.to_string();

    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("audio.mp3")
        .to_string();

    log::info!(
        "[讯飞 ASR] 准备上传：file={} size={:.2} MB",
        file_name,
        file_size_bytes as f64 / 1024.0 / 1024.0
    );
    // 讯飞非实时转写单文件上限按账号档位不同（通常 ≥500MB）；超大文件给出警示
    if file_size_bytes > 500 * 1024 * 1024 {
        log::warn!(
            "[讯飞 ASR] 文件 >500MB（{:.0} MB），上传/转写耗时可能超出 30 分钟轮询窗口",
            file_size_bytes as f64 / 1024.0 / 1024.0
        );
    }

    let date_time = datetime_now();
    let random = signature_random();

    // BTreeMap 保证 key 自然排序，满足签名要求
    let mut params: BTreeMap<&str, String> = BTreeMap::new();
    params.insert("accessKeyId", ACCESS_KEY_ID.to_string());
    params.insert("appId", APPID.to_string());
    params.insert("dateTime", date_time.clone());
    params.insert("durationCheckDisable", "true".to_string());
    params.insert("fileName", file_name.clone());
    params.insert("fileSize", file_size.clone());
    params.insert("language", language.to_string());
    params.insert("signatureRandom", random.clone());

    let sig = generate_signature(&params, ACCESS_KEY_SECRET)?;
    let query = build_query(&params);
    let url = format!("{IFLYTEK_BASE_URL}{UPLOAD_PATH}?{query}");

    // 上传超时：大文件传输时间正比于带宽，1GB / 5MB/s ≈ 3.5 分钟；放宽到 30 分钟以
    // 覆盖慢网络场景。连接握手仍保持 30s 短超时及早暴露离线问题。
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(30))
        .timeout(std::time::Duration::from_secs(30 * 60))
        .build()
        .map_err(|e| ExtractionError::OcrError(format!("HTTP 客户端创建失败: {e}")))?;

    let file = tokio::fs::File::open(file_path).await.map_err(|e| {
        ExtractionError::OcrError(format!("打开音频文件失败: {e}"))
    })?;
    let stream = tokio_util::io::ReaderStream::new(file);
    let body = reqwest::Body::wrap_stream(stream);

    log::info!(
        "[讯飞 ASR] 发送上传请求：lang={} sigLen={} url={}{}?{}…",
        language,
        sig.len(),
        IFLYTEK_BASE_URL,
        UPLOAD_PATH,
        &query.chars().take(80).collect::<String>()
    );

    let send_started = std::time::Instant::now();
    let resp = client
        .post(&url)
        .header("Content-Type", "application/octet-stream")
        .header("Content-Length", file_size_bytes)
        .header("signature", &sig)
        .body(body)
        .send()
        .await
        .map_err(|e| {
            let elapsed = send_started.elapsed();
            log::error!(
                "[讯飞 ASR] 上传请求失败（{:.1}s 后）: {e}",
                elapsed.as_secs_f32()
            );
            ExtractionError::OcrError(format!("讯飞上传请求失败: {e}"))
        })?;

    let http_status = resp.status();
    log::info!(
        "[讯飞 ASR] 上传响应 HTTP {} 耗时 {:.1}s",
        http_status,
        send_started.elapsed().as_secs_f32()
    );
    let raw = resp
        .text()
        .await
        .map_err(|e| ExtractionError::OcrError(format!("讯飞上传响应读取失败: {e}")))?;

    if !http_status.is_success() {
        log::error!(
            "[讯飞 ASR] 上传返回非 2xx HTTP {http_status}，响应体（前 500 字符）: {}",
            raw.chars().take(500).collect::<String>()
        );
        return Err(ExtractionError::OcrError(format!(
            "讯飞上传失败 HTTP {http_status}，响应: {raw}"
        )));
    }

    let parsed: UploadResponse = serde_json::from_str(&raw).map_err(|e| {
        log::error!(
            "[讯飞 ASR] 上传响应 JSON 解析失败: {e}，原始内容（前 500 字符）: {}",
            raw.chars().take(500).collect::<String>()
        );
        ExtractionError::OcrError(format!(
            "讯飞上传响应解析失败: {e}，原始内容: {raw}"
        ))
    })?;

    if parsed.code != "000000" {
        log::error!(
            "[讯飞 ASR] 业务错误 code={} desc={} raw={}",
            parsed.code,
            parsed.desc_info,
            raw.chars().take(500).collect::<String>()
        );
        return Err(ExtractionError::OcrError(format!(
            "讯飞上传错误 code={}: {}，原始响应: {raw}",
            parsed.code, parsed.desc_info
        )));
    }

    parsed
        .content
        .map(|c| c.order_id)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| {
            ExtractionError::OcrError(format!(
                "讯飞上传成功但未返回 orderId，原始响应: {raw}"
            ))
        })
}

async fn poll_result(order_id: &str) -> Result<String, ExtractionError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| ExtractionError::OcrError(format!("HTTP 客户端创建失败: {e}")))?;

    for attempt in 1..=POLL_MAX_RETRIES {
        tokio::time::sleep(tokio::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;

        let date_time = datetime_now();
        let random = signature_random();

        let mut params: BTreeMap<&str, String> = BTreeMap::new();
        params.insert("accessKeyId", ACCESS_KEY_ID.to_string());
        params.insert("dateTime", date_time.clone());
        params.insert("orderId", order_id.to_string());
        params.insert("resultType", "transfer".to_string());
        params.insert("signatureRandom", random.clone());

        let sig = generate_signature(&params, ACCESS_KEY_SECRET)?;
        let query = build_query(&params);
        let url = format!("{IFLYTEK_BASE_URL}{QUERY_PATH}?{query}");

        let resp = match client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("signature", &sig)
            .body("{}")
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                log::warn!("[讯飞 ASR] 第{attempt}次查询网络错误: {e}，继续重试");
                continue;
            }
        };

        let raw = resp.text().await.map_err(|e| {
            ExtractionError::OcrError(format!("讯飞查询响应读取失败: {e}"))
        })?;

        let parsed: QueryResponse = serde_json::from_str(&raw).map_err(|e| {
            ExtractionError::OcrError(format!(
                "讯飞查询响应解析失败: {e}，原始内容: {raw}"
            ))
        })?;

        if parsed.code != "000000" {
            return Err(ExtractionError::OcrError(format!(
                "讯飞查询错误 code={}: {}",
                parsed.code, parsed.desc_info
            )));
        }

        let Some(content) = parsed.content else {
            log::debug!("[讯飞 ASR] 第{attempt}次查询：无 content，继续等待");
            continue;
        };

        match content.order_info.status {
            -1 => {
                return Err(ExtractionError::OcrError(format!(
                    "讯飞转录任务失败（failType={}）",
                    content.order_info.fail_type
                )));
            }
            4 => {
                log::info!("[讯飞 ASR] orderId={order_id} 转录完成");
                return parse_order_result(&content.order_result);
            }
            s => {
                log::debug!("[讯飞 ASR] 第{attempt}次查询：状态={s}，继续等待");
            }
        }
    }

    Err(ExtractionError::OcrError(format!(
        "讯飞转录超时（orderId={order_id}）"
    )))
}

/// 解析 orderResult JSON 字符串，提取纯文本
/// orderResult 是一个 JSON 字符串，内含 lattice 数组，每项有 json_1best 字段
fn parse_order_result(order_result: &str) -> Result<String, ExtractionError> {
    if order_result.trim().is_empty() {
        return Ok(String::new());
    }

    #[derive(Deserialize)]
    struct OrderResult {
        #[serde(default)]
        lattice: Vec<LatticeItem>,
    }
    #[derive(Deserialize)]
    struct LatticeItem {
        #[serde(default)]
        json_1best: String,
    }

    let parsed: OrderResult = serde_json::from_str(order_result).map_err(|e| {
        ExtractionError::OcrError(format!(
            "讯飞结果解析失败: {e}，原始片段: {}",
            &order_result[..order_result.len().min(200)]
        ))
    })?;

    let mut text = String::new();
    for item in parsed.lattice {
        if let Some(t) = extract_json_1best(&item.json_1best) {
            text.push_str(&t);
        }
    }

    Ok(text)
}

/// 从 json_1best 字符串提取文字，跳过分段标记（wp="g"）和空词
fn extract_json_1best(json_1best: &str) -> Option<String> {
    if json_1best.is_empty() {
        return None;
    }

    #[derive(Deserialize)]
    struct Root {
        st: St,
    }
    #[derive(Deserialize)]
    struct St {
        rt: Vec<Rt>,
    }
    #[derive(Deserialize)]
    struct Rt {
        ws: Vec<Ws>,
    }
    #[derive(Deserialize)]
    struct Ws {
        cw: Vec<Cw>,
    }
    #[derive(Deserialize)]
    struct Cw {
        w: String,
        #[serde(default)]
        wp: String,
    }

    let root: Root = serde_json::from_str(json_1best).ok()?;
    let mut seg_text = String::new();
    for rt in root.st.rt {
        for ws in rt.ws {
            for cw in ws.cw {
                // wp="g" 是分段标记，w 为空；跳过空词
                if cw.wp != "g" && !cw.w.trim().is_empty() {
                    seg_text.push_str(&cw.w);
                }
            }
        }
    }

    if seg_text.is_empty() {
        None
    } else {
        Some(seg_text)
    }
}

// ── 测试 ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_handle_audio_types() {
        let e = IflytekAsrExtractor;
        assert!(e.can_handle("audio/mpeg"));
        assert!(e.can_handle("audio/mp4"));
        assert!(e.can_handle("audio/wav"));
        assert!(e.can_handle("audio/flac"));
        assert!(e.can_handle("audio/x-wav"));
        assert!(!e.can_handle("application/pdf"));
        assert!(!e.can_handle(""));
    }

    #[test]
    fn test_url_encode() {
        // 普通字母数字不编码
        assert_eq!(url_encode("abc123"), "abc123");
        // 冒号和加号编码（dateTime 中的 : 和时区 + 号）
        assert_eq!(url_encode("2025-09-08T22:58:29+0800"), "2025-09-08T22%3A58%3A29%2B0800");
        // 空格编为 + （Java URLEncoder 行为，不是 %20）
        assert_eq!(url_encode("hello world"), "hello+world");
        // 中文编为 %XX
        assert_eq!(url_encode("麦风"), "%E9%BA%A6%E9%A3%8E");
        // * 不编码（Java URLEncoder 行为）
        assert_eq!(url_encode("a*b"), "a*b");
        // ~ 编码（Java URLEncoder 行为，与标准 percent-encoding 不同）
        assert_eq!(url_encode("a~b"), "a%7Eb");
        // 文件名含空格和中文
        assert_eq!(url_encode("03-20 麦风科技.mp3"), "03-20+%E9%BA%A6%E9%A3%8E%E7%A7%91%E6%8A%80.mp3");
    }

    #[test]
    fn test_generate_signature_deterministic() {
        // 相同输入应产生相同签名
        let mut params: BTreeMap<&str, String> = BTreeMap::new();
        params.insert("accessKeyId", "testKeyId".to_string());
        params.insert("appId", "testAppId".to_string());
        params.insert("dateTime", "2025-01-01T00:00:00+0800".to_string());
        params.insert("signatureRandom", "abcdefgh12345678".to_string());

        let sig1 = generate_signature(&params, "testSecret").unwrap();
        let sig2 = generate_signature(&params, "testSecret").unwrap();
        assert_eq!(sig1, sig2);
        assert!(!sig1.is_empty());
    }

    #[test]
    fn test_generate_signature_key_order_independent() {
        // 插入顺序不影响签名（BTreeMap 保证排序）
        let mut params_a: BTreeMap<&str, String> = BTreeMap::new();
        params_a.insert("accessKeyId", "kid".to_string());
        params_a.insert("appId", "aid".to_string());
        params_a.insert("signatureRandom", "rand".to_string());

        let mut params_b: BTreeMap<&str, String> = BTreeMap::new();
        params_b.insert("signatureRandom", "rand".to_string());
        params_b.insert("appId", "aid".to_string());
        params_b.insert("accessKeyId", "kid".to_string());

        let sig_a = generate_signature(&params_a, "secret").unwrap();
        let sig_b = generate_signature(&params_b, "secret").unwrap();
        assert_eq!(sig_a, sig_b);
    }

    #[test]
    fn test_parse_order_result_json_1best() {
        let order_result = r#"{"lattice":[{"json_1best":"{\"st\":{\"rt\":[{\"ws\":[{\"cw\":[{\"w\":\"你好\",\"wp\":\"n\"}]}]}]}}"},{"json_1best":"{\"st\":{\"rt\":[{\"ws\":[{\"cw\":[{\"w\":\"世界\",\"wp\":\"n\"}]}]}]}}"}]}"#;
        let text = parse_order_result(order_result).unwrap();
        assert_eq!(text, "你好世界");
    }

    #[test]
    fn test_parse_order_result_skips_segment_marker() {
        // wp="g" 的词（分段标记）不应出现在输出中
        let order_result = r#"{"lattice":[{"json_1best":"{\"st\":{\"rt\":[{\"ws\":[{\"cw\":[{\"w\":\"你好\",\"wp\":\"n\"},{\"w\":\"\",\"wp\":\"g\"}]}]}]}}"}]}"#;
        let text = parse_order_result(order_result).unwrap();
        assert_eq!(text, "你好");
    }

    #[test]
    fn test_parse_order_result_empty() {
        let text = parse_order_result("").unwrap();
        assert!(text.is_empty());
    }

    #[test]
    fn test_signature_random_length() {
        let r = signature_random();
        assert_eq!(r.len(), 16);
    }

    /// 候选语言回退：当前值是表中某项 → 返回表中下一项
    #[test]
    fn next_language_candidate_returns_next_in_table() {
        assert_eq!(next_language_candidate("cn"), Some("mandarin"));
        assert_eq!(next_language_candidate("mandarin"), Some("cn_lay"));
        assert_eq!(next_language_candidate("cn_lay"), Some("autodialect"));
        assert_eq!(next_language_candidate("autodialect"), Some("en"));
        assert_eq!(next_language_candidate("en"), Some(""));
    }

    /// 候选用尽：当前是表中最后一项 → None
    #[test]
    fn next_language_candidate_exhausted_returns_none() {
        // "" 是 IFLYTEK_LANGUAGE_FALLBACKS 中最后一项
        assert_eq!(next_language_candidate(""), None);
    }

    /// 用户传了表外的自定义值（首次尝试）→ 从表头第一个不同的值开始
    #[test]
    fn next_language_candidate_unknown_value_starts_from_head() {
        let next = next_language_candidate("zh_custom").unwrap();
        assert_eq!(next, "cn");
    }

    /// task_014 Fix-A3 AC-5：默认 language = "cn"（修复 code=100020 autodialect 不支持）
    #[test]
    fn resolve_language_defaults_to_cn() {
        let opts = ExtractOptions::default();
        assert_eq!(resolve_language(&opts), "cn");
    }

    /// task_014 Fix-A3：空字符串视为 None → 默认 "cn"
    #[test]
    fn resolve_language_empty_string_falls_back_to_cn() {
        let opts = ExtractOptions {
            iflytek_language: Some("   ".to_string()),
            ..ExtractOptions::default()
        };
        assert_eq!(resolve_language(&opts), "cn");
    }

    /// task_014 Fix-A3：setting 覆盖时取覆盖值
    #[test]
    fn resolve_language_uses_option_when_present() {
        let opts = ExtractOptions {
            iflytek_language: Some("autodialect".to_string()),
            ..ExtractOptions::default()
        };
        assert_eq!(resolve_language(&opts), "autodialect");

        let opts2 = ExtractOptions {
            iflytek_language: Some("en".to_string()),
            ..ExtractOptions::default()
        };
        assert_eq!(resolve_language(&opts2), "en");
    }
}
