use serde::{Deserialize, Serialize};
use std::path::Path;

/// AI 元数据（.meta.json 结构）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetMetadata {
    pub summary: Option<String>,
    pub topics: Option<Vec<String>>,
    pub ocr_text: Option<String>,
    pub language: Option<String>,
    pub suggested_tags: Option<Vec<String>>,
    pub suggested_name: Option<String>,
    pub captured_at: Option<String>,
    pub offset_in_audio: Option<f64>,
}

/// 解析 .meta.json 文件
pub fn parse_meta(meta_path: &Path) -> Result<AssetMetadata, String> {
    let content = std::fs::read_to_string(meta_path)
        .map_err(|e| format!("读取 meta.json 失败: {e}"))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("解析 meta.json 失败: {e}"))
}

/// 尝试解析元数据，找不到文件时返回 None
pub fn try_parse_meta(meta_path_str: &Option<String>) -> Option<AssetMetadata> {
    let path_str = meta_path_str.as_ref()?;
    let path = Path::new(path_str);
    if path.exists() {
        parse_meta(path).ok()
    } else {
        None
    }
}
