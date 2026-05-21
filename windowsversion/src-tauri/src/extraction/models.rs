use serde::{Deserialize, Serialize};
use std::fmt;

/// 提取结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionResult {
    pub raw_text: String,
    pub structured_md: String,
    pub quality_level: i32,
    pub extractor_type: String,
    pub segments: Vec<ContentSegment>,
    pub needs_ocr_fallback: bool,
}

/// 内容段（P0 最简版）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSegment {
    #[serde(rename = "type")]
    pub segment_type: String,
    pub content: String,
    pub page: Option<u32>,
    pub confidence: Option<f64>,
    pub bbox: Option<[f64; 4]>,
}

/// 提取选项
#[derive(Debug, Clone, Default)]
pub struct ExtractOptions {
    pub language_hint: Option<String>,
    pub max_pages: Option<u32>,
    pub markitdown_enabled: bool,
    pub markitdown_python_cmd: Option<String>,
    /// 嵌入式 venv 内的 python 解释器绝对路径（task_008 scheduler 通过 AppHandle 注入）
    pub markitdown_embedded_python: Option<String>,
    /// task_014 Fix-A3：讯飞非实时转写 language 参数（默认 "cn"，可被 setting `iflytekLanguage` 覆盖）。
    /// None / 空字符串视为使用默认 "cn"。
    pub iflytek_language: Option<String>,
    /// task_007 FIX：runtime 自检失败时的 FailureCode 快照（scheduler 路由前注入）。
    /// `Some(code)` → markitdown extract 入口立即短路返回（不进 python 子进程）；
    /// `None` → 自检通过（或调用方未注入），走常规路径。
    pub runtime_check_failed: Option<crate::extraction::failure_code::FailureCode>,
}

pub fn markdown_to_segments(markdown: &str) -> Vec<ContentSegment> {
    markdown
        .lines()
        .filter(|line| !line.trim().is_empty())
        .enumerate()
        .map(|(i, line)| {
            let trimmed = line.trim();
            let segment_type = if trimmed.starts_with('#') {
                "heading"
            } else if trimmed.starts_with("- ")
                || trimmed.starts_with("* ")
                || trimmed
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_digit() && trimmed.contains(". "))
            {
                "list_item"
            } else if trimmed.contains('|') {
                "table_row"
            } else {
                "paragraph"
            };

            ContentSegment {
                segment_type: segment_type.to_string(),
                content: trimmed.to_string(),
                page: Some((i as u32 / 30) + 1),
                confidence: None,
                bbox: None,
            }
        })
        .collect()
}

pub fn evaluate_markdown_quality(markdown: &str) -> i32 {
    let trimmed = markdown.trim();
    if trimmed.is_empty() {
        return 0;
    }

    let mut score = 1;
    if trimmed.lines().any(|line| line.trim_start().starts_with('#')) {
        score += 1;
    }
    if trimmed.lines().any(|line| {
        let line = line.trim_start();
        line.starts_with("- ")
            || line.starts_with("* ")
            || line
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit() && line.contains(". "))
    }) {
        score += 1;
    }
    if trimmed.contains('|') && trimmed.contains("---") {
        score += 1;
    }
    if trimmed.chars().count() > 1500 {
        score += 1;
    }

    score.min(4)
}

/// 提取错误
#[derive(Debug)]
pub enum ExtractionError {
    UnsupportedFormat(String),
    IoError(std::io::Error),
    ParseError(String),
    OcrError(String),
    UnsupportedPlatform,
}

impl fmt::Display for ExtractionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedFormat(s) => write!(f, "不支持的文件格式: {s}"),
            Self::IoError(e) => write!(f, "IO 错误: {e}"),
            Self::ParseError(s) => write!(f, "解析错误: {s}"),
            Self::OcrError(s) => write!(f, "OCR 错误: {s}"),
            Self::UnsupportedPlatform => write!(f, "不支持的平台"),
        }
    }
}

impl std::error::Error for ExtractionError {}

impl From<std::io::Error> for ExtractionError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}
