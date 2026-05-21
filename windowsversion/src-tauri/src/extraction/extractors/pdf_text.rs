use std::path::Path;

use crate::extraction::models::{ContentSegment, ExtractionError, ExtractionResult, ExtractOptions};
use crate::extraction::Extractor;

pub struct PdfTextExtractor;

impl Extractor for PdfTextExtractor {
    fn can_handle(&self, mime_type: &str) -> bool {
        mime_type == "application/pdf"
    }

    fn name(&self) -> &'static str {
        "pdf_text"
    }

    fn extract(
        &self,
        file_path: &Path,
        _options: &ExtractOptions,
    ) -> Result<ExtractionResult, ExtractionError> {
        let bytes = std::fs::read(file_path)?;

        let raw_text = pdf_extract::extract_text_from_mem(&bytes)
            .map_err(|e| ExtractionError::ParseError(format!("PDF 解析失败: {e}")))?;

        let raw_text = raw_text.trim().to_string();

        if raw_text.is_empty() {
            return Ok(ExtractionResult {
                raw_text: String::new(),
                structured_md: String::new(),
                quality_level: 0,
                extractor_type: "pdf_text".to_string(),
                segments: vec![],
                needs_ocr_fallback: true,
            });
        }

        let paragraphs: Vec<&str> = raw_text
            .split("\n\n")
            .filter(|p| !p.trim().is_empty())
            .collect();

        let segments: Vec<ContentSegment> = paragraphs
            .iter()
            .enumerate()
            .map(|(i, p)| ContentSegment {
                segment_type: "text".to_string(),
                content: p.trim().to_string(),
                page: Some((i as u32 / 5) + 1),
                confidence: None,
                bbox: None,
            })
            .collect();

        let structured_md = paragraphs
            .iter()
            .map(|p| p.trim())
            .collect::<Vec<_>>()
            .join("\n\n");

        let quality_level = if structured_md.contains('#') || structured_md.contains("- ") {
            2
        } else {
            1
        };

        Ok(ExtractionResult {
            raw_text,
            structured_md,
            quality_level,
            extractor_type: "pdf_text".to_string(),
            segments,
            needs_ocr_fallback: false,
        })
    }
}
