use std::path::Path;

use crate::extraction::models::{ContentSegment, ExtractionError, ExtractionResult, ExtractOptions};
use crate::extraction::Extractor;

pub struct PdfScanExtractor;

impl Extractor for PdfScanExtractor {
    fn can_handle(&self, mime_type: &str) -> bool {
        mime_type == "application/pdf"
    }

    fn name(&self) -> &'static str {
        "pdf_scan_ocr"
    }

    fn extract(
        &self,
        file_path: &Path,
        _options: &ExtractOptions,
    ) -> Result<ExtractionResult, ExtractionError> {
        #[cfg(target_os = "macos")]
        {
            let page_count = crate::macos::ocr_ffi::pdf_page_count(file_path)
                .map_err(ExtractionError::ParseError)?;

            let mut all_segments = Vec::new();
            let mut all_text = Vec::new();

            for page_idx in 0..page_count {
                let regions = crate::macos::ocr_ffi::ocr_pdf_page(file_path, page_idx)
                    .map_err(ExtractionError::OcrError)?;

                let mut sorted = regions;
                sorted.sort_by(|a, b| {
                    b.bbox[1]
                        .partial_cmp(&a.bbox[1])
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

                let page_text: Vec<String> = sorted.iter().map(|r| r.text.clone()).collect();

                for r in &sorted {
                    all_segments.push(ContentSegment {
                        segment_type: "ocr_region".to_string(),
                        content: r.text.clone(),
                        page: Some((page_idx + 1) as u32),
                        confidence: Some(r.confidence),
                        bbox: Some(r.bbox),
                    });
                }

                if !page_text.is_empty() {
                    all_text.push(page_text.join("\n"));
                }
            }

            let raw_text = all_text.join("\n\n");
            let structured_md = raw_text.clone();

            Ok(ExtractionResult {
                raw_text,
                structured_md,
                quality_level: 1,
                extractor_type: "pdf_scan_ocr".to_string(),
                segments: all_segments,
                needs_ocr_fallback: false,
            })
        }

        #[cfg(target_os = "windows")]
        {
            // Windows 走 cloud_ai 异步分支；extract() trait 为 sync（caller 已在
            // spawn_blocking 内），用 Handle::current().block_on 桥接。
            let handle = tokio::runtime::Handle::current();
            let page_count = handle.block_on(crate::cloud_ai::pdf_page_count(file_path))?;

            let mut all_segments = Vec::new();
            let mut all_text = Vec::new();

            for page_idx in 0..page_count {
                let regions =
                    handle.block_on(crate::cloud_ai::ocr_pdf_page(file_path, page_idx))?;

                let mut sorted = regions;
                sorted.sort_by(|a, b| {
                    b.bbox[1]
                        .partial_cmp(&a.bbox[1])
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

                let page_text: Vec<String> = sorted.iter().map(|r| r.text.clone()).collect();

                for r in &sorted {
                    all_segments.push(ContentSegment {
                        segment_type: "ocr_region".to_string(),
                        content: r.text.clone(),
                        page: Some((page_idx + 1) as u32),
                        confidence: Some(r.confidence),
                        bbox: Some(r.bbox),
                    });
                }

                if !page_text.is_empty() {
                    all_text.push(page_text.join("\n"));
                }
            }

            let raw_text = all_text.join("\n\n");
            let structured_md = raw_text.clone();

            Ok(ExtractionResult {
                raw_text,
                structured_md,
                quality_level: 1,
                extractor_type: "pdf_scan_ocr".to_string(),
                segments: all_segments,
                needs_ocr_fallback: false,
            })
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = file_path;
            Err(ExtractionError::UnsupportedPlatform)
        }
    }
}
