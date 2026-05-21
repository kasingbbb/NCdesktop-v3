use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub color: String,
    pub source: String,
    pub usage_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub project_id: String,
    pub asset_id: Option<String>,
    pub timeline_time: Option<f64>,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

/// 资产—标签关联
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetTag {
    pub asset_id: String,
    pub tag_id: String,
}

/// 项目—标签关联
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTag {
    pub project_id: String,
    pub tag_id: String,
}
