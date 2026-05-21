use serde::{Deserialize, Serialize};
use std::path::Path;

/// TF 卡清单（manifest.json 完整结构）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TFCardManifest {
    pub device_id: String,
    pub device_name: String,
    pub firmware_version: String,
    pub sessions: Vec<SessionSummary>,
    pub last_sync_at: Option<String>,
}

/// 会话摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub session_id: String,
    pub title: String,
    pub start_time: String,
    pub end_time: String,
    pub audio_duration: f64,
    pub photo_count: i64,
    pub scan_count: i64,
    pub is_synced: bool,
}

/// 解析 .arca/manifest.json
pub fn parse_manifest(arca_path: &Path) -> Result<TFCardManifest, String> {
    let manifest_path = arca_path.join("manifest.json");
    let content = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("读取 manifest.json 失败: {e}"))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("解析 manifest.json 失败: {e}"))
}
