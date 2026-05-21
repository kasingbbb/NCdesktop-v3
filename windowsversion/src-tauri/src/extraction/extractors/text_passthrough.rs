//! task_014 Fix-A2：内置文本 / CSV / JSON / XML 直通提取器。
//!
//! 与 [`super::text::TextExtractor`] 的差异：本提取器扩展到 `text/csv` /
//! `application/json` / `application/xml`，并对 text/plain 加 `# {filename}` 包装、
//! text/csv 转 markdown table、json/xml 用 ``` 代码块包裹，**不**走 python subprocess。
//!
//! 注册顺序：在 `extractors/mod.rs::get_extractor_for` 中 **优先于 markitdown**
//! 匹配；text/html 仍交给 markitdown（更结构化）。
//!
//! 质量：纯文本无损 → quality_level=3（high）。

use std::io::Read;
use std::path::Path;

use crate::extraction::{
    models::{ContentSegment, ExtractionError, ExtractionResult, ExtractOptions},
    Extractor,
};

/// CSV 转 markdown table 时最多保留多少数据行（不含表头）。
/// 超过则截断并附"_（截断 N 行）_"。
const CSV_ROW_LIMIT: usize = 100;

pub struct TextPassthroughExtractor;

impl Extractor for TextPassthroughExtractor {
    fn can_handle(&self, mime_type: &str) -> bool {
        // 注意：text/html 不在此列表（保留给 markitdown 处理）。
        matches!(
            mime_type,
            "text/plain"
                | "text/markdown"
                | "text/x-markdown"
                | "text/csv"
                | "application/json"
                | "application/xml"
        )
    }

    fn name(&self) -> &'static str {
        "text_passthrough"
    }

    fn extract(
        &self,
        file_path: &Path,
        _options: &ExtractOptions,
    ) -> Result<ExtractionResult, ExtractionError> {
        // 读文件：尽量 UTF-8，失败回 lossy（确保任何文本输入都不丢提取结果）。
        let mut file = std::fs::File::open(file_path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        let content = match std::str::from_utf8(&bytes) {
            Ok(s) => s.to_string(),
            Err(_) => String::from_utf8_lossy(&bytes).to_string(),
        };

        let file_name = file_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("content");

        // 用扩展名 + 内容启发选择格式化策略；ExtractOptions 当前不带 mime_type。
        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();

        let md = match ext.as_str() {
            "md" | "markdown" => content.clone(),
            "csv" => csv_to_markdown_table(&content, CSV_ROW_LIMIT),
            "json" => format!("# {file_name}\n\n```json\n{}\n```\n", content),
            "xml" => format!("# {file_name}\n\n```xml\n{}\n```\n", content),
            _ => {
                // text/plain：加 `# {filename}` 包装
                format!("# {file_name}\n\n{}\n", content)
            }
        };

        let trimmed_md = md.trim_end().to_string();
        let segments: Vec<ContentSegment> = trimmed_md
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
            raw_text: trimmed_md.clone(),
            structured_md: trimmed_md,
            // 纯文本无损 → high
            quality_level: 3,
            extractor_type: "text_passthrough".to_string(),
            segments,
            needs_ocr_fallback: false,
        })
    }
}

/// 将 CSV 文本转 markdown table。
///
/// 行数 > `row_limit` 时截断，结尾附 "_（截断 N 行）_"。
/// **简化实现**：按逗号 split，不处理引号转义/嵌入换行。csv 大多数场景够用；
/// 含双引号转义的复杂 CSV 应改用 markitdown / 专用库（task_014 不引入新 crate）。
fn csv_to_markdown_table(content: &str, row_limit: usize) -> String {
    let mut lines = content.lines().filter(|l| !l.trim().is_empty());
    let Some(header_line) = lines.next() else {
        return String::new();
    };

    let header_cells: Vec<&str> = header_line.split(',').map(|s| s.trim()).collect();
    if header_cells.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    out.push_str("| ");
    out.push_str(&header_cells.join(" | "));
    out.push_str(" |\n");
    out.push_str("| ");
    out.push_str(
        &header_cells
            .iter()
            .map(|_| "---")
            .collect::<Vec<_>>()
            .join(" | "),
    );
    out.push_str(" |\n");

    let mut count = 0usize;
    let mut truncated = 0usize;
    for line in lines {
        if count >= row_limit {
            truncated += 1;
            continue;
        }
        let cells: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        // 列数对齐：少补空、多截
        let mut row: Vec<String> = Vec::with_capacity(header_cells.len());
        for i in 0..header_cells.len() {
            row.push(cells.get(i).copied().unwrap_or("").to_string());
        }
        out.push_str("| ");
        out.push_str(&row.join(" | "));
        out.push_str(" |\n");
        count += 1;
    }

    if truncated > 0 {
        out.push_str(&format!("\n_（截断 {truncated} 行）_\n"));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;

    fn write_temp(name: &str, content: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ncdesktop_textpass_{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(name);
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        p
    }

    fn opts() -> ExtractOptions {
        ExtractOptions::default()
    }

    /// AC-2：text/plain → 含文件名标题与原文，quality=3，extractor=text_passthrough
    #[test]
    fn ac2_plain_text_wraps_with_filename_heading() {
        let p = write_temp("note.txt", "hello\nworld");
        let r = TextPassthroughExtractor.extract(&p, &opts()).unwrap();
        assert_eq!(r.extractor_type, "text_passthrough");
        assert_eq!(r.quality_level, 3);
        assert!(r.structured_md.contains("# note.txt"));
        assert!(r.structured_md.contains("hello"));
        assert!(r.structured_md.contains("world"));
    }

    /// AC-3：text/csv → markdown table 含表头 separator 与数据行
    #[test]
    fn ac3_csv_to_markdown_table() {
        let p = write_temp("data.csv", "a,b\n1,2\n3,4\n");
        let r = TextPassthroughExtractor.extract(&p, &opts()).unwrap();
        assert!(r.structured_md.contains("| a | b |"));
        assert!(r.structured_md.contains("| --- | --- |"));
        assert!(r.structured_md.contains("| 1 | 2 |"));
        assert!(r.structured_md.contains("| 3 | 4 |"));
    }

    /// AC-3 边界：CSV 行数超过 limit 时截断并附说明
    #[test]
    fn csv_truncates_when_exceeding_limit() {
        let mut body = String::from("col\n");
        for i in 0..120 {
            body.push_str(&format!("{i}\n"));
        }
        let p = write_temp("big.csv", &body);
        let r = TextPassthroughExtractor.extract(&p, &opts()).unwrap();
        assert!(r.structured_md.contains("_（截断 20 行）_"));
    }

    /// AC-4：application/json → 三反引号 json 代码块
    #[test]
    fn ac4_json_wrapped_in_code_block() {
        let p = write_temp("conf.json", "{\"k\":\"v\"}");
        let r = TextPassthroughExtractor.extract(&p, &opts()).unwrap();
        assert!(
            r.structured_md.contains("```json"),
            "应包含 ```json 代码块, got: {}",
            r.structured_md
        );
        assert!(r.structured_md.contains("{\"k\":\"v\"}"));
    }

    /// XML 走 ```xml 包裹
    #[test]
    fn xml_wrapped_in_code_block() {
        let p = write_temp("doc.xml", "<a><b/></a>");
        let r = TextPassthroughExtractor.extract(&p, &opts()).unwrap();
        assert!(r.structured_md.contains("```xml"));
        assert!(r.structured_md.contains("<a><b/></a>"));
    }

    /// text/markdown 不加额外包装
    #[test]
    fn markdown_passthrough_unchanged_body() {
        let p = write_temp("note.md", "# Title\n\nbody");
        let r = TextPassthroughExtractor.extract(&p, &opts()).unwrap();
        // 不应被二次加 # filename 包装
        assert!(r.structured_md.starts_with("# Title"));
    }

    /// can_handle 覆盖与 html 排除
    #[test]
    fn can_handle_excludes_html() {
        let e = TextPassthroughExtractor;
        assert!(e.can_handle("text/plain"));
        assert!(e.can_handle("text/markdown"));
        assert!(e.can_handle("text/x-markdown"));
        assert!(e.can_handle("text/csv"));
        assert!(e.can_handle("application/json"));
        assert!(e.can_handle("application/xml"));
        // html 留给 markitdown
        assert!(!e.can_handle("text/html"));
        assert!(!e.can_handle("application/pdf"));
    }
}
