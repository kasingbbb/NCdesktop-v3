// Stub by Unit 14：合并 Unit 13 真正实现时会被覆盖。
// 仅签名占位 + 必要类型定义，不实现具体逻辑。
//
// 关键约定（Unit 13 实现需保持兼容）：
//   - `OcrRegion`：与 macOS `crate::macos::ocr_ffi::OcrRegion` 字段一致
//     （text / confidence / bbox），让 extractor 调用点跨平台保持同形。
//   - `AsrResult`：包含 `transcription: String` 字段（audio_asr extractor
//     Windows 分支调用 `.transcription`）。
//   - 四个异步函数签名（ocr_image / ocr_pdf_page / pdf_page_count /
//     transcribe_audio）参数 / 返回类型保持稳定。

use std::path::Path;

use crate::extraction::models::ExtractionError;

#[derive(Debug, Clone)]
pub struct OcrRegion {
    pub text: String,
    pub confidence: f64,
    pub bbox: [f64; 4],
}

#[derive(Debug, Clone)]
pub struct AsrResult {
    pub transcription: String,
}

pub async fn ocr_image(_file_path: &Path) -> Result<Vec<OcrRegion>, ExtractionError> {
    unimplemented!("Stub by Unit 14; real impl in Unit 13")
}

pub async fn ocr_pdf_page(
    _pdf_path: &Path,
    _page_index: i32,
) -> Result<Vec<OcrRegion>, ExtractionError> {
    unimplemented!("Stub by Unit 14; real impl in Unit 13")
}

pub async fn pdf_page_count(_pdf_path: &Path) -> Result<i32, ExtractionError> {
    unimplemented!("Stub by Unit 14; real impl in Unit 13")
}

pub async fn transcribe_audio(_file_path: &Path) -> Result<AsrResult, ExtractionError> {
    unimplemented!("Stub by Unit 14; real impl in Unit 13")
}
