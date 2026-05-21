use std::io::Read;
use std::path::Path;

use crate::extraction::{
    models::{ContentSegment, ExtractionError, ExtractionResult, ExtractOptions},
    Extractor,
};

pub struct TextExtractor;

impl Extractor for TextExtractor {
    fn can_handle(&self, mime_type: &str) -> bool {
        matches!(mime_type, "text/markdown" | "text/plain")
    }

    fn name(&self) -> &'static str {
        "text"
    }

    fn extract(
        &self,
        file_path: &Path,
        _options: &ExtractOptions,
    ) -> Result<ExtractionResult, ExtractionError> {
        let mut file = std::fs::File::open(file_path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        let content = content.trim().to_string();
        if content.is_empty() {
            return Ok(ExtractionResult {
                raw_text: String::new(),
                structured_md: String::new(),
                quality_level: 0,
                extractor_type: "text".to_string(),
                segments: vec![],
                needs_ocr_fallback: false,
            });
        }

        let segments: Vec<ContentSegment> = content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .enumerate()
            .map(|(i, line)| ContentSegment {
                segment_type: "paragraph".to_string(),
                content: line.to_string(),
                page: Some((i as u32 / 30) + 1),
                confidence: None,
                bbox: None,
            })
            .collect();

        Ok(ExtractionResult {
            raw_text: content.clone(),
            structured_md: content,
            quality_level: 2,
            extractor_type: "text".to_string(),
            segments,
            needs_ocr_fallback: false,
        })
    }
}
