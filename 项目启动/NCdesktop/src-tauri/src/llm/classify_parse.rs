//! 模型返回的分类结果常为「带解释 / markdown 代码块」的文本，这里做健壮解析。

use serde::{Deserialize, Serialize};

/// LLM 分类结果（字段尽量宽松，避免模型少字段时整段失败）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassifyResult {
    /// PARA 主类别：
    /// - V17 之前：仅允许 `1-项目` / `2-领域` / `3-资源` / `4-存档` / `other`
    /// - V17 起（custom_para_v1）：允许任意 active 类目 slug、`new:<新类目名>` 请求
    ///   新建，或 `other` 兜底；最终解析/落盘由
    ///   `commands::dropzone::resolve_or_create_category` 处理（含 `new:` 前缀剥离、
    ///   alias 跳转、`sanitize_slug` 规范化、upsert llm_generated 行）。
    ///
    /// 本字段保持 `String` 原样不在解析层做枚举校验：normalize 只 trim + 空字符兜底。
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    #[serde(default)]
    pub language: String,
    /// 建议主文件名（不含扩展名），用于导入后重命名（JSON：`suggestedFileName`）
    #[serde(default)]
    pub suggested_file_name: String,
}

fn default_confidence() -> f64 {
    0.6
}

fn extract_brace_object(text: &str) -> Option<&str> {
    let t = text.trim();
    let start = t.find('{')?;
    let end = t.rfind('}')?;
    if start >= end {
        return None;
    }
    Some(&t[start..=end])
}

fn strip_markdown_json_fence(text: &str) -> String {
    let t = text.trim();
    let after_open = if let Some(i) = t.find("```") {
        let rest = &t[i + 3..];
        let rest = rest.trim_start();
        if rest.to_lowercase().starts_with("json") {
            rest[4..].trim_start()
        } else {
            rest
        }
    } else {
        return t.to_string();
    };

    if let Some(j) = after_open.find("```") {
        return after_open[..j].trim().to_string();
    }
    after_open.trim().to_string()
}

/// 从模型原始输出解析 `ClassifyResult`。
pub fn parse_classify_response(raw: &str) -> Result<ClassifyResult, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("模型返回内容为空".to_string());
    }

    let candidates: Vec<String> = vec![
        trimmed.to_string(),
        strip_markdown_json_fence(trimmed),
    ];

    for c in &candidates {
        if c.is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<ClassifyResult>(c) {
            return Ok(normalize(v));
        }
        if let Some(obj) = extract_brace_object(c) {
            if let Ok(v) = serde_json::from_str::<ClassifyResult>(obj) {
                return Ok(normalize(v));
            }
        }
    }

    let sample: String = trimmed.chars().take(500).collect();
    Err(format!("解析分类 JSON 失败（请确认 LLM 返回了合法 JSON）。片段：{sample}"))
}

fn normalize(mut r: ClassifyResult) -> ClassifyResult {
    r.category = r.category.trim().to_string();
    if r.category.is_empty() {
        r.category = "other".to_string();
    }
    r.tags = r
        .tags
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    r.language = r.language.trim().to_string();
    if r.language.is_empty() {
        r.language = "zh".to_string();
    }
    if !r.confidence.is_finite() || r.confidence < 0.0 {
        r.confidence = 0.5;
    }
    if r.confidence > 1.0 {
        r.confidence = 1.0;
    }
    r.suggested_file_name = r.suggested_file_name.trim().chars().take(120).collect();
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::init_test_logger;

    #[test]
    fn parse_plain_json() {
        init_test_logger();
        crate::test_log!("classify_parse::parse_plain_json 开始");
        let raw = r#"{"category":"note","tags":["课堂","要点"],"confidence":0.85,"language":"zh"}"#;
        let r = parse_classify_response(raw).expect("应解析成功");
        assert_eq!(r.category, "note");
        assert_eq!(r.tags, vec!["课堂", "要点"]);
        assert!((r.confidence - 0.85).abs() < f64::EPSILON);
        assert_eq!(r.language, "zh");
    }

    #[test]
    fn parse_markdown_fence() {
        init_test_logger();
        let raw = r##"```json
{"category":"lecture","tags":["数学"],"confidence":0.7,"language":"zh"}
```"##;
        let r = parse_classify_response(raw).unwrap();
        assert_eq!(r.category, "lecture");
        assert_eq!(r.tags, vec!["数学"]);
    }

    #[test]
    fn parse_extracts_from_prefix_text() {
        init_test_logger();
        let raw = r#"好的，结果如下：{"category":"other","tags":[],"confidence":0.5,"language":"en"} 完毕"#;
        let r = parse_classify_response(raw).unwrap();
        assert_eq!(r.category, "other");
        assert!(r.tags.is_empty());
        assert_eq!(r.language, "en");
    }

    #[test]
    fn parse_defaults_missing_fields() {
        init_test_logger();
        let raw = r#"{"tags":["仅标签"]}"#;
        let r = parse_classify_response(raw).unwrap();
        assert_eq!(r.category, "other");
        assert_eq!(r.tags, vec!["仅标签"]);
        assert_eq!(r.language, "zh");
        assert!(r.suggested_file_name.is_empty());
    }

    #[test]
    fn parse_suggested_file_name() {
        init_test_logger();
        let raw = r#"{"category":"note","tags":["测试"],"confidence":0.9,"language":"zh","suggestedFileName":"连通性"}"#;
        let r = parse_classify_response(raw).unwrap();
        assert_eq!(r.suggested_file_name, "连通性");
    }
}
