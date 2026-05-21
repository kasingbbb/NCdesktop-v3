use serde::{Deserialize, Serialize};
use std::path::Path;

/// 同步状态文件（sync_state.json）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SyncState {
    pub synced_sessions: Vec<SyncedSessionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncedSessionRecord {
    pub session_id: String,
    pub device_id: String,
    pub synced_at: String,
    pub project_id: String,
}

/// 读取同步状态
pub fn load_state(state_path: &Path) -> SyncState {
    if state_path.exists() {
        std::fs::read_to_string(state_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        SyncState::default()
    }
}

/// 保存同步状态
pub fn save_state(state_path: &Path, state: &SyncState) -> Result<(), String> {
    if let Some(parent) = state_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建同步状态目录失败: {e}"))?;
    }
    let json = serde_json::to_string_pretty(state)
        .map_err(|e| format!("序列化同步状态失败: {e}"))?;
    std::fs::write(state_path, json)
        .map_err(|e| format!("写入同步状态失败: {e}"))?;
    Ok(())
}

/// 检查会话是否已同步
pub fn is_session_synced(state: &SyncState, session_id: &str, device_id: &str) -> bool {
    state.synced_sessions.iter().any(|r| {
        r.session_id == session_id && r.device_id == device_id
    })
}

/// 记录会话已同步
pub fn mark_synced(
    state: &mut SyncState,
    session_id: &str,
    device_id: &str,
    project_id: &str,
) {
    state.synced_sessions.push(SyncedSessionRecord {
        session_id: session_id.to_string(),
        device_id: device_id.to_string(),
        synced_at: chrono::Utc::now().to_rfc3339(),
        project_id: project_id.to_string(),
    });
}
