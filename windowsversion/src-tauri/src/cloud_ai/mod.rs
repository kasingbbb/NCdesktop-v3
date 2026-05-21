//! Windows 版云端 AI 推理（OCR / ASR）替代层
//!
//! macOS 版用 Vision Framework（OCR）+ SFSpeechRecognizer（ASR）的原生 FFI
//! 实现端上推理。Windows 没有等价的开箱即用系统服务，本模块用云端 API 顶替：
//!
//! - **OCR**：OpenAI Chat Completions `gpt-4o-mini` 的 vision 能力（图片以
//!   base64 嵌入消息），返回结构化的 `Vec<OcrRegion>`。
//! - **ASR**：OpenAI `/v1/audio/transcriptions` 端点（`whisper-1` 模型，
//!   `verbose_json` 格式带 segment + 时间戳），返回 `AsrResult`。
//! - **PDF 页数**：`lopdf` 纯 Rust 实现，**不走云端**。
//! - **PDF 单页 OCR**：用 `pdf-extract` 抽取该页文本并合成单一 `OcrRegion`
//!   返回（接受 fallback；扫描型 PDF 走空向量 + 上层 `needs_ocr_fallback`）。
//!
//! ### 接口契约（与 Unit 14 stub 严格保持一致）
//!
//! ```ignore
//! pub async fn ocr_image(file_path: &Path) -> Result<Vec<OcrRegion>, ExtractionError>;
//! pub async fn ocr_pdf_page(pdf_path: &Path, page_index: i32) -> Result<Vec<OcrRegion>, ExtractionError>;
//! pub async fn pdf_page_count(pdf_path: &Path) -> Result<i32, ExtractionError>;
//! pub async fn transcribe_audio(file_path: &Path) -> Result<AsrResult, ExtractionError>;
//! ```
//!
//! `OcrRegion { text, confidence, bbox }` 字段与 `macos::ocr_ffi::OcrRegion` 一致。
//! `AsrResult { transcription, .. }` 至少含 `transcription: String`（Windows 分支
//! 调用 `.transcription`），并按 OpenAI verbose_json 暴露 segment + 时间戳。
//!
//! ### API Key
//!
//! 与 `llm/client.rs` 一致，从 `OPENAI_API_KEY` 环境变量读取；缺失时返回
//! `ExtractionError::OcrError`。Base URL 可通过 `OPENAI_BASE_URL` 覆盖，
//! 默认 `https://api.openai.com`。

pub mod asr_whisper_api;
pub mod ocr_openai_vision;

pub use asr_whisper_api::{transcribe_audio, AsrResult, AsrSegment};
pub use ocr_openai_vision::{ocr_image, ocr_pdf_page, pdf_page_count, OcrRegion};

use crate::extraction::models::ExtractionError;
use std::env;

/// 解析 OpenAI API key（仅读环境变量；未来可扩展到 Settings 表）。
pub(crate) fn resolve_openai_api_key() -> Result<String, ExtractionError> {
    env::var("OPENAI_API_KEY")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| {
            ExtractionError::OcrError(
                "未检测到 OPENAI_API_KEY 环境变量（Windows 版云端 OCR/ASR 必须配置）".to_string(),
            )
        })
}

/// 解析 OpenAI Base URL；默认 `https://api.openai.com`，可通过 `OPENAI_BASE_URL` 覆盖。
pub(crate) fn resolve_openai_base_url() -> String {
    env::var("OPENAI_BASE_URL")
        .ok()
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "https://api.openai.com".to_string())
}
