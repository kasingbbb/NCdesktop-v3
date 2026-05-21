use std::io::Read;
use std::path::Path;

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::extraction::{
    models::{ContentSegment, ExtractionError, ExtractionResult, ExtractOptions},
    Extractor,
};

pub struct DocxExtractor;

impl Extractor for DocxExtractor {
    fn can_handle(&self, mime_type: &str) -> bool {
        mime_type == "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    }

    fn name(&self) -> &'static str {
        "docx"
    }

    fn extract(
        &self,
        file_path: &Path,
        _options: &ExtractOptions,
    ) -> Result<ExtractionResult, ExtractionError> {
        let file = std::fs::File::open(file_path)?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| ExtractionError::ParseError(format!("DOCX zip 打开失败: {e}")))?;

        // 读取 word/document.xml
        let xml_content = {
            let mut entry = archive
                .by_name("word/document.xml")
                .map_err(|e| ExtractionError::ParseError(format!("找不到 word/document.xml: {e}")))?;
            let mut buf = String::new();
            entry
                .read_to_string(&mut buf)
                .map_err(|e| ExtractionError::ParseError(format!("读取 document.xml 失败: {e}")))?;
            buf
        };

        let paragraphs = extract_paragraphs_from_docx_xml(&xml_content)?;

        if paragraphs.is_empty() {
            return Ok(ExtractionResult {
                raw_text: String::new(),
                structured_md: String::new(),
                quality_level: 0,
                extractor_type: "docx".to_string(),
                segments: vec![],
                needs_ocr_fallback: false,
            });
        }

        let segments: Vec<ContentSegment> = paragraphs
            .iter()
            .enumerate()
            .map(|(i, p)| ContentSegment {
                segment_type: "paragraph".to_string(),
                content: p.clone(),
                page: Some((i as u32 / 10) + 1),
                confidence: None,
                bbox: None,
            })
            .collect();

        let raw_text = paragraphs.join("\n");
        let structured_md = paragraphs.join("\n\n");

        Ok(ExtractionResult {
            raw_text,
            structured_md,
            quality_level: 1,
            extractor_type: "docx".to_string(),
            segments,
            needs_ocr_fallback: false,
        })
    }
}

/// 从 word/document.xml 的 XML 字符串中提取段落文字
fn extract_paragraphs_from_docx_xml(xml: &str) -> Result<Vec<String>, ExtractionError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut paragraphs: Vec<String> = Vec::new();
    let mut current_para = String::new();
    let mut in_w_t = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                // <w:p> — 新段落开始
                if e.name().as_ref() == b"w:p" {
                    current_para.clear();
                }
                // <w:t> — 文字节点开始
                if e.name().as_ref() == b"w:t" {
                    in_w_t = true;
                }
            }
            Ok(Event::Text(ref e)) if in_w_t => {
                let text = e
                    .unescape()
                    .map_err(|e| ExtractionError::ParseError(format!("XML 解码失败: {e}")))?;
                current_para.push_str(&text);
            }
            Ok(Event::End(ref e)) => {
                if e.name().as_ref() == b"w:t" {
                    in_w_t = false;
                }
                if e.name().as_ref() == b"w:p" {
                    let trimmed = current_para.trim().to_string();
                    if !trimmed.is_empty() {
                        paragraphs.push(trimmed);
                    }
                    current_para.clear();
                }
            }
            Ok(Event::Empty(ref e)) => {
                // 处理自闭合的 <w:t/>（通常是空格）
                if e.name().as_ref() == b"w:t" {
                    // 空文字节点，不处理
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(ExtractionError::ParseError(format!("DOCX XML 解析失败: {e}")));
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(paragraphs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_paragraphs_empty_xml() {
        let xml = r#"<?xml version="1.0"?><w:document><w:body></w:body></w:document>"#;
        let result = extract_paragraphs_from_docx_xml(xml).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_extract_paragraphs_with_text() {
        let xml = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Hello World</w:t></w:r></w:p>
    <w:p><w:r><w:t>Second paragraph</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
        let result = extract_paragraphs_from_docx_xml(xml).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "Hello World");
        assert_eq!(result[1], "Second paragraph");
    }

    #[test]
    fn test_docx_extractor_can_handle() {
        let extractor = DocxExtractor;
        assert!(extractor.can_handle(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        ));
        assert!(!extractor.can_handle("application/pdf"));
        assert!(!extractor.can_handle(""));
    }
}
