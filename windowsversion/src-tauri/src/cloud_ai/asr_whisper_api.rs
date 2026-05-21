//! ASR via OpenAI Whisper API（`/v1/audio/transcriptions`）
//!
//! 替代 macOS `SFSpeechRecognizer`：把音频整体 multipart upload 到 OpenAI，
//! 模型固定 `whisper-1`，`response_format=verbose_json` 拿到 segment + 时间戳。
//!
//! ## 文件大小限制
//!
//! OpenAI 硬限 **25 MB**。> 24 MB 的文件本模块通过 [`crate::cloud_ai::audio_chunker`]
//! 按时间切成多个 WAV 块，逐块上传后合并 transcription + segments 时间戳
//! （把每段 segment.start/end 加上 chunk.start_seconds 偏移）。
//!
//! ## 关于 multipart
//!
//! 当前 `Cargo.toml` 中的 reqwest 没有 `multipart` feature（避免再拉额外依赖），
//! 本文件手写一个最小 multipart/form-data 编码器（仅需 file + 几个 text 字段）。

use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::cloud_ai::audio_chunker;
use crate::extraction::models::ExtractionError;

use super::{resolve_openai_api_key, resolve_openai_base_url};

const OPENAI_WHISPER_MODEL: &str = "whisper-1";
const OPENAI_WHISPER_PATH: &str = "/v1/audio/transcriptions";
/// 单次上传上限。OpenAI 硬限 25 MB；这里取 24 MB 留余量给 multipart overhead /
/// 网络抖动，超过则走 `audio_chunker` 分块。
const WHISPER_UPLOAD_LIMIT_BYTES: usize = 24 * 1024 * 1024;

// ── 对外类型 ─────────────────────────────────────────────────────────────────

/// ASR 转写结果。
///
/// `transcription` 字段是 Unit 14 stub 锁定的 must-have（Windows 分支
/// `audio_asr` extractor 直接读 `.transcription`）。其余字段是 verbose_json
/// 模式下从 OpenAI 拿到的附加信息：语言识别、整段时长、按 segment 分的时间戳。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsrResult {
    /// 完整转写文本（与 macOS SFSpeechRecognizer 的 transcription 等价）。
    pub transcription: String,
    /// Whisper 探测到的语言（ISO 639-1，如 `zh` / `en`）；缺失时为 None。
    #[serde(default)]
    pub language: Option<String>,
    /// 整段音频时长（秒）。
    #[serde(default)]
    pub duration: Option<f64>,
    /// 按 segment 切分的时间戳列表（与 OpenAI verbose_json 1:1 对应）。
    #[serde(default)]
    pub segments: Vec<AsrSegment>,
}

/// 单个 segment 时间戳条目（对应 OpenAI verbose_json 的 segments[i]）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsrSegment {
    pub id: u32,
    pub start: f64,
    pub end: f64,
    pub text: String,
}

// ── OpenAI 响应 ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct WhisperVerboseResponse {
    text: String,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    segments: Vec<AsrSegment>,
}

// ── HTTP client（带超时）──────────────────────────────────────────────────────

fn cloud_asr_http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            // Whisper 处理时间随音频长度线性增长；25 MB 上限下给 5 分钟超时。
            .timeout(Duration::from_secs(300))
            .build()
            .expect("reqwest cloud_ai whisper client")
    })
}

// ── 对外 API ─────────────────────────────────────────────────────────────────

/// 上传音频到 OpenAI Whisper，返回转写 + segment 时间戳。
///
/// 行为：
/// - 文件 ≤ [`WHISPER_UPLOAD_LIMIT_BYTES`]（24 MB）：直接上传整文件。
/// - 文件 > 24 MB：通过 [`audio_chunker::split_audio_into_chunks`] 切块，
///   逐块走 [`transcribe_single_file`]，最后合并 transcription + segments
///   时间戳（每段 segment.start/end 加上 chunk.start_seconds 偏移）。
pub async fn transcribe_audio(file_path: &Path) -> Result<AsrResult, ExtractionError> {
    let meta = tokio::fs::metadata(file_path).await.map_err(|e| {
        ExtractionError::IoError(std::io::Error::new(
            e.kind(),
            format!("读取音频元数据失败 ({}): {e}", file_path.display()),
        ))
    })?;

    if (meta.len() as usize) <= WHISPER_UPLOAD_LIMIT_BYTES {
        return transcribe_single_file(file_path).await;
    }

    // 大文件：分块上传 + 合并 segments。
    let chunks = audio_chunker::split_audio_into_chunks(file_path, WHISPER_UPLOAD_LIMIT_BYTES).await?;
    let mut merged_segments: Vec<AsrSegment> = Vec::new();
    let mut merged_transcription = String::new();
    let mut last_segment_id: u32 = 0;
    let mut detected_language: Option<String> = None;
    let mut total_duration: f64 = 0.0;

    for (i, chunk) in chunks.iter().enumerate() {
        // 把 chunk.data 写到临时 WAV 文件（audio_chunker 已封装好 WAV header）
        let tmp = std::env::temp_dir().join(format!(
            "ncdesktop-whisper-chunk-{}-{}.wav",
            std::process::id(),
            i,
        ));
        tokio::fs::write(&tmp, &chunk.data).await.map_err(|e| {
            ExtractionError::IoError(std::io::Error::new(
                e.kind(),
                format!("写 chunk 临时文件失败 ({}): {e}", tmp.display()),
            ))
        })?;

        let chunk_result = transcribe_single_file(&tmp).await;
        // 无论结果如何先清理临时文件，避免堆积
        let _ = tokio::fs::remove_file(&tmp).await;
        let r = chunk_result?;

        // 合并 segments：把每段时间戳 += chunk.start_seconds；id 重新连续编号。
        for seg in r.segments {
            last_segment_id = last_segment_id.saturating_add(1);
            merged_segments.push(AsrSegment {
                id: last_segment_id,
                start: seg.start + chunk.start_seconds,
                end: seg.end + chunk.start_seconds,
                text: seg.text,
            });
        }

        // 合并文本：保留 chunk 间空格，避免句末粘连。
        // r.transcription 已在 transcribe_single_file 内 trim 过。
        if !r.transcription.is_empty() {
            if !merged_transcription.is_empty() {
                merged_transcription.push(' ');
            }
            merged_transcription.push_str(&r.transcription);
        }

        // language 取首个非 None；duration 取 chunk 时长之和（与原文件总长对齐）。
        if detected_language.is_none() {
            detected_language = r.language;
        }
        total_duration += chunk.duration_seconds;
    }

    Ok(AsrResult {
        transcription: merged_transcription,
        language: detected_language,
        duration: Some(total_duration),
        segments: merged_segments,
    })
}

/// 单文件上传到 Whisper。调用前由 caller 保证文件 ≤ 24 MB。
async fn transcribe_single_file(file_path: &Path) -> Result<AsrResult, ExtractionError> {
    let bytes = tokio::fs::read(file_path).await.map_err(ExtractionError::IoError)?;
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("audio.bin")
        .to_string();
    let content_type = mime_for_audio(file_path);

    let boundary = format!("----notecaptCloudAsr{}", Uuid::new_v4().simple());
    let body = build_multipart_body(
        &boundary,
        &[
            ("model", OPENAI_WHISPER_MODEL.as_bytes()),
            ("response_format", b"verbose_json"),
        ],
        &file_name,
        content_type,
        &bytes,
    );

    let url = format!("{}{}", resolve_openai_base_url(), OPENAI_WHISPER_PATH);
    let api_key = resolve_openai_api_key()?;

    let res = cloud_asr_http_client()
        .post(&url)
        .bearer_auth(&api_key)
        .header(
            reqwest::header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(body)
        .send()
        .await
        .map_err(|e| ExtractionError::OcrError(format!("OpenAI Whisper 网络请求失败: {e}")))?;

    let status = res.status();
    let text_body = res
        .text()
        .await
        .map_err(|e| ExtractionError::OcrError(format!("读取 Whisper 响应失败: {e}")))?;
    if !status.is_success() {
        return Err(ExtractionError::OcrError(format!(
            "OpenAI Whisper API 错误 ({status}): {text_body}"
        )));
    }

    let parsed: WhisperVerboseResponse = serde_json::from_str(&text_body).map_err(|e| {
        ExtractionError::ParseError(format!("解析 Whisper 响应失败: {e}; body={text_body}"))
    })?;

    Ok(AsrResult {
        transcription: parsed.text.trim().to_string(),
        language: parsed.language,
        duration: parsed.duration,
        segments: parsed.segments,
    })
}

// ── 私有 ──────────────────────────────────────────────────────────────────────

/// 按扩展名给一个 OpenAI Whisper 接受的 Content-Type。
/// 支持列表（OpenAI 文档）：flac / m4a / mp3 / mp4 / mpeg / mpga / oga / ogg / wav / webm
fn mime_for_audio(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
    {
        Some("mp3") | Some("mpga") => "audio/mpeg",
        Some("mp4") | Some("m4a") => "audio/mp4",
        Some("wav") => "audio/wav",
        Some("flac") => "audio/flac",
        Some("oga") | Some("ogg") => "audio/ogg",
        Some("webm") => "audio/webm",
        // 默认按二进制兜底，让 OpenAI 自己拒收（错误信息也比较清晰）
        _ => "application/octet-stream",
    }
}

/// 手写 multipart/form-data：N 个 text 字段 + 1 个 file 字段。
///
/// 不引入 reqwest `multipart` feature，避免额外编译开销。RFC 2388 + RFC 7578 子集，
/// 对 OpenAI 端足够（实测 boundary 用 ASCII 即可，无需 quoted-printable）。
fn build_multipart_body(
    boundary: &str,
    text_fields: &[(&str, &[u8])],
    file_name: &str,
    file_content_type: &'static str,
    file_bytes: &[u8],
) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::with_capacity(file_bytes.len() + 1024);

    for (name, value) in text_fields {
        buf.extend_from_slice(b"--");
        buf.extend_from_slice(boundary.as_bytes());
        buf.extend_from_slice(b"\r\n");
        buf.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
        );
        buf.extend_from_slice(value);
        buf.extend_from_slice(b"\r\n");
    }

    buf.extend_from_slice(b"--");
    buf.extend_from_slice(boundary.as_bytes());
    buf.extend_from_slice(b"\r\n");
    buf.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"file\"; filename=\"{}\"\r\n",
            escape_filename(file_name)
        )
        .as_bytes(),
    );
    buf.extend_from_slice(format!("Content-Type: {file_content_type}\r\n\r\n").as_bytes());
    buf.extend_from_slice(file_bytes);
    buf.extend_from_slice(b"\r\n");

    buf.extend_from_slice(b"--");
    buf.extend_from_slice(boundary.as_bytes());
    buf.extend_from_slice(b"--\r\n");

    buf
}

/// `Content-Disposition` 的 filename 中需要转义 `"` 和 `\`，避免 header 解析失败。
fn escape_filename(name: &str) -> String {
    name.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mime_for_audio_recognizes_common_formats() {
        assert_eq!(mime_for_audio(Path::new("a.mp3")), "audio/mpeg");
        assert_eq!(mime_for_audio(Path::new("a.M4A")), "audio/mp4");
        assert_eq!(mime_for_audio(Path::new("a.wav")), "audio/wav");
        assert_eq!(mime_for_audio(Path::new("a.flac")), "audio/flac");
        assert_eq!(mime_for_audio(Path::new("a.ogg")), "audio/ogg");
        assert_eq!(mime_for_audio(Path::new("a.webm")), "audio/webm");
        // 未知扩展名兜底
        assert_eq!(mime_for_audio(Path::new("a.xyz")), "application/octet-stream");
    }

    #[test]
    fn escape_filename_escapes_quote_and_backslash() {
        assert_eq!(escape_filename("a\"b"), "a\\\"b");
        assert_eq!(escape_filename("a\\b"), "a\\\\b");
        assert_eq!(escape_filename("plain.mp3"), "plain.mp3");
    }

    #[test]
    fn build_multipart_body_contains_boundary_and_payload() {
        let body = build_multipart_body(
            "BOUND",
            &[("model", b"whisper-1")],
            "test.mp3",
            "audio/mpeg",
            b"DATA",
        );
        let s = String::from_utf8(body).expect("body is utf-8 for ascii inputs");
        assert!(s.starts_with("--BOUND\r\n"));
        assert!(s.contains("name=\"model\""));
        assert!(s.contains("whisper-1"));
        assert!(s.contains("name=\"file\"; filename=\"test.mp3\""));
        assert!(s.contains("Content-Type: audio/mpeg"));
        assert!(s.contains("DATA"));
        assert!(s.ends_with("--BOUND--\r\n"));
    }
}
