use std::io::Read;
use std::path::Path;

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::extraction::{
    models::{ContentSegment, ExtractionError, ExtractionResult, ExtractOptions},
    Extractor,
};

pub struct PptxExtractor;

impl Extractor for PptxExtractor {
    fn can_handle(&self, mime_type: &str) -> bool {
        mime_type
            == "application/vnd.openxmlformats-officedocument.presentationml.presentation"
    }

    fn name(&self) -> &'static str {
        "pptx"
    }

    fn extract(
        &self,
        file_path: &Path,
        _options: &ExtractOptions,
    ) -> Result<ExtractionResult, ExtractionError> {
        let file = std::fs::File::open(file_path)?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| ExtractionError::ParseError(format!("PPTX zip 打开失败: {e}")))?;

        // 枚举所有 ppt/slides/slide*.xml 文件名，按自然数排序
        let mut slide_names: Vec<String> = (0..archive.len())
            .filter_map(|i| {
                let entry = archive.by_index(i).ok()?;
                let name = entry.name().to_string();
                if name.starts_with("ppt/slides/slide")
                    && name.ends_with(".xml")
                    && !name.contains("_rels")
                    && !name.contains("slideLayout")
                    && !name.contains("slideMaster")
                {
                    Some(name)
                } else {
                    None
                }
            })
            .collect();

        // 自然数排序：slide1.xml < slide2.xml < ... < slide10.xml
        slide_names.sort_by(|a, b| {
            let num_a = extract_slide_number(a);
            let num_b = extract_slide_number(b);
            num_a.cmp(&num_b)
        });

        if slide_names.is_empty() {
            return Ok(ExtractionResult {
                raw_text: String::new(),
                structured_md: String::new(),
                quality_level: 0,
                extractor_type: "pptx".to_string(),
                segments: vec![],
                needs_ocr_fallback: false,
            });
        }

        let mut all_segments: Vec<ContentSegment> = Vec::new();
        let mut md_sections: Vec<String> = Vec::new();

        for (slide_idx, slide_name) in slide_names.iter().enumerate() {
            let slide_num = slide_idx + 1;

            let xml_content = {
                let mut entry = archive
                    .by_name(slide_name)
                    .map_err(|e| ExtractionError::ParseError(format!("读取幻灯片 {slide_name} 失败: {e}")))?;
                let mut buf = String::new();
                entry
                    .read_to_string(&mut buf)
                    .map_err(|e| ExtractionError::ParseError(format!("读取幻灯片 XML 失败: {e}")))?;
                buf
            };

            let texts = extract_texts_from_pptx_slide_xml(&xml_content)?;

            if texts.is_empty() {
                continue;
            }

            // 每张幻灯片生成一个 ## 二级标题段
            let mut section = format!("## Slide {slide_num}");
            for text in &texts {
                section.push('\n');
                section.push_str(text);
            }
            md_sections.push(section);

            for text in texts {
                all_segments.push(ContentSegment {
                    segment_type: "slide_text".to_string(),
                    content: text,
                    page: Some(slide_num as u32),
                    confidence: None,
                    bbox: None,
                });
            }
        }

        if md_sections.is_empty() {
            return Ok(ExtractionResult {
                raw_text: String::new(),
                structured_md: String::new(),
                quality_level: 0,
                extractor_type: "pptx".to_string(),
                segments: vec![],
                needs_ocr_fallback: false,
            });
        }

        let raw_text = all_segments
            .iter()
            .map(|s| s.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let structured_md = md_sections.join("\n\n");

        Ok(ExtractionResult {
            raw_text,
            structured_md,
            quality_level: 2, // 有结构（## 标题）
            extractor_type: "pptx".to_string(),
            segments: all_segments,
            needs_ocr_fallback: false,
        })
    }
}

/// 从 slide*.xml 中提取所有 <a:t> 文字节点（非空）
fn extract_texts_from_pptx_slide_xml(xml: &str) -> Result<Vec<String>, ExtractionError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut texts: Vec<String> = Vec::new();
    let mut in_a_t = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                if e.name().as_ref() == b"a:t" {
                    in_a_t = true;
                }
            }
            Ok(Event::Text(ref e)) if in_a_t => {
                let text = e
                    .unescape()
                    .map_err(|e| ExtractionError::ParseError(format!("XML 解码失败: {e}")))?;
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty() {
                    texts.push(trimmed);
                }
            }
            Ok(Event::End(ref e)) => {
                if e.name().as_ref() == b"a:t" {
                    in_a_t = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(ExtractionError::ParseError(format!("PPTX XML 解析失败: {e}")));
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(texts)
}

/// 从路径中提取幻灯片编号（ppt/slides/slide5.xml → 5）
fn extract_slide_number(path: &str) -> u32 {
    // 找到最后一个 / 后的部分，再去掉 "slide" 前缀和 ".xml" 后缀
    let file_name = path.rsplit('/').next().unwrap_or(path);
    let without_ext = file_name.trim_end_matches(".xml");
    let without_prefix = without_ext.trim_start_matches("slide");
    without_prefix.parse::<u32>().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_slide_number() {
        assert_eq!(extract_slide_number("ppt/slides/slide1.xml"), 1);
        assert_eq!(extract_slide_number("ppt/slides/slide10.xml"), 10);
        assert_eq!(extract_slide_number("ppt/slides/slide2.xml"), 2);
    }

    #[test]
    fn test_extract_texts_empty() {
        let xml = r#"<?xml version="1.0"?><p:sld><p:cSld><p:spTree></p:spTree></p:cSld></p:sld>"#;
        let result = extract_texts_from_pptx_slide_xml(xml).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_extract_texts_with_content() {
        let xml = r#"<?xml version="1.0"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:sp>
        <p:txBody>
          <a:p><a:r><a:t>标题文字</a:t></a:r></a:p>
          <a:p><a:r><a:t>正文内容</a:t></a:r></a:p>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;
        let result = extract_texts_from_pptx_slide_xml(xml).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.contains(&"标题文字".to_string()));
        assert!(result.contains(&"正文内容".to_string()));
    }

    #[test]
    fn test_pptx_extractor_can_handle() {
        let extractor = PptxExtractor;
        assert!(extractor.can_handle(
            "application/vnd.openxmlformats-officedocument.presentationml.presentation"
        ));
        assert!(!extractor.can_handle("application/pdf"));
    }
}
