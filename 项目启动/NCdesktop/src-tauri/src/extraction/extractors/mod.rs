use super::Extractor;
use crate::extraction::models::ExtractOptions;

#[cfg(target_os = "macos")]
pub mod audio_asr;
// image_ocr / pdf_scan 提取器依赖 OCR FFI，但当前 fallback 链路未使用，保持禁用
// pub mod image_ocr;
// pub mod pdf_scan;
pub mod audio_asr_iflytek;
// 图片 OCR：讯飞「通用文档识别(OCR 大模型)」se75ocrbm WebAPI。图片→MD 走此提取器；
// PDF 不走 OCR（仍交给 markitdown）。见 get_extractor_for 路由。
pub mod image_ocr_iflytek;
pub mod docx;
pub mod markitdown;
pub mod pdf_text;
pub mod pptx;
pub mod text;
pub mod text_passthrough;

/// 根据 MIME 类型获取合适的提取器
pub fn get_extractor_for(mime_type: &str, options: &ExtractOptions) -> Option<Box<dyn Extractor>> {
    // task_014 Fix-A2：text/* (plain/markdown/csv/json/xml) 优先走零依赖的
    // text_passthrough，避免 python subprocess 浪费 + 保证 text/markdown 不
    // 被误判 unsupported（V11 之前问题）。text/html 仍交给 markitdown 处理。
    let passthrough = text_passthrough::TextPassthroughExtractor;
    if passthrough.can_handle(mime_type) {
        return Some(Box::new(passthrough));
    }

    // 图片(jpg/png/bmp) → 讯飞 OCR 大模型（se75ocrbm），**先于** markitdown 拦截。
    //   - 用户已购买图片 OCR 能力，图片转 MD 走 OCR 提取文字；
    //   - PDF 不在此（image_ocr_iflytek::can_handle 不含 application/pdf）→ 仍走下方 markitdown；
    //   - OCR 提取器 name != "markitdown"，故不受 runtime_check 短路影响：
    //     python/markitdown 运行时不可用时，图片照样能通过 OCR 转换。
    let ocr = image_ocr_iflytek::IflytekOcrExtractor;
    if ocr.can_handle(mime_type) {
        return Some(Box::new(ocr));
    }

    if options.markitdown_enabled && markitdown::supports_mime(mime_type) {
        return Some(Box::new(markitdown::MarkItDownExtractor::new()));
    }

    get_fallback_extractor_for(mime_type)
}

/// 获取不依赖 MarkItDown 的内置提取器。
/// ASR：默认使用讯飞非实时语音转写云端，对大文件做流式上传 + 30 分钟轮询。
/// 本地 audio_asr（macOS SFSpeechRecognizer）已实现但当前不注册；保留为后续 setting 开关切换。
pub fn get_fallback_extractor_for(mime_type: &str) -> Option<Box<dyn Extractor>> {
    let extractors: Vec<Box<dyn Extractor>> = vec![
        Box::new(pdf_text::PdfTextExtractor),
        Box::new(docx::DocxExtractor),
        Box::new(pptx::PptxExtractor),
        // ASR：统一走讯飞非实时语音转写云端，支持大文件分块上传。
        // 本地 audio_asr（SFSpeechRecognizer）保留代码不注册，便于未来加 setting 开关切换。
        Box::new(audio_asr_iflytek::IflytekAsrExtractor),
        Box::new(text::TextExtractor),
    ];

    extractors.into_iter().find(|e| e.can_handle(mime_type))
}

// pdf_scan 依赖 macos OCR FFI，暂不激活
// pub fn get_pdf_scan_extractor() -> Box<dyn Extractor> {
//     Box::new(pdf_scan::PdfScanExtractor)
// }

/// 获取**排除指定名称**的 fallback 提取器（task_008 ADR-003）。
///
/// 用于 primary 失败后选择 fallback：必须排除 primary 自身（例如 markitdown
/// 失败时不能再选 markitdown），否则形成死循环。
///
/// 行为：遍历 `get_fallback_extractor_for` 内部的同一序列，跳过 `name()` 等于
/// `excluded_name` 的项，返回第一个 `can_handle(mime_type)` 的实例。
pub fn get_fallback_extractor_for_excluding(
    mime_type: &str,
    excluded_name: &str,
) -> Option<Box<dyn Extractor>> {
    let extractors: Vec<Box<dyn Extractor>> = vec![
        Box::new(pdf_text::PdfTextExtractor),
        Box::new(docx::DocxExtractor),
        Box::new(pptx::PptxExtractor),
        // ASR：统一走讯飞非实时语音转写云端，支持大文件分块上传。
        // 本地 audio_asr（SFSpeechRecognizer）保留代码不注册，便于未来加 setting 开关切换。
        Box::new(audio_asr_iflytek::IflytekAsrExtractor),
        Box::new(text::TextExtractor),
    ];

    extractors
        .into_iter()
        .find(|e| e.name() != excluded_name && e.can_handle(mime_type))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AC-4 (task_008)：mime 无任何 fallback 候选 → 返回 None。
    #[test]
    fn excluding_returns_none_when_no_candidate() {
        // 不存在的 MIME，无任何 extractor 可处理
        let r = get_fallback_extractor_for_excluding("application/x-nonexistent", "markitdown");
        assert!(r.is_none());
    }

    /// AC-4 (task_008)：PDF 仅 pdf_text 可处理；excluded != pdf_text 时仍返回 pdf_text。
    #[test]
    fn excluding_returns_pdf_text_when_excluding_markitdown() {
        let r = get_fallback_extractor_for_excluding("application/pdf", "markitdown");
        let r = r.expect("应返回 pdf_text");
        assert_eq!(r.name(), "pdf_text");
    }

    /// AC-4 (task_008)：excluded 即为唯一候选 pdf_text 自身 → None。
    /// 这保证调用方传 primary_name = pdf_text 时不会陷入自循环。
    #[test]
    fn excluding_returns_none_when_only_candidate_is_excluded() {
        let r = get_fallback_extractor_for_excluding("application/pdf", "pdf_text");
        assert!(r.is_none(), "排除唯一候选后应返回 None");
    }
}
