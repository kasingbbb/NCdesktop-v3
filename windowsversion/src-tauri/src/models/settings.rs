use serde::{Deserialize, Serialize};

/// 应用设置以 key-value 存储在 SQLite 中
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingRow {
    pub key: String,
    pub value: String,
}
