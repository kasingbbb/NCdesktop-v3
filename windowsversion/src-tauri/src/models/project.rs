use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub library_id: String,
    pub name: String,
    pub description: String,
    pub cover_asset_id: Option<String>,
    pub source_type: String,
    pub source_data: Option<String>,
    pub is_pinned: bool,
    pub is_archived: bool,
    pub created_at: String,
    pub updated_at: String,
    pub total_duration: Option<f64>,
    pub asset_count: i64,
    pub word_count: i64,
    pub imported_at: Option<String>,
}
