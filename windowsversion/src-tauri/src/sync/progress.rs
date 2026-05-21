use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncProgress {
    pub session_id: String,
    pub phase: String,
    pub current: u32,
    pub total: u32,
    pub message: String,
}

pub const SYNC_PROGRESS_EVENT: &str = "sync-progress";

/// 发送同步进度事件到前端
pub fn emit_progress(
    app: &AppHandle,
    session_id: &str,
    phase: &str,
    current: u32,
    total: u32,
    message: &str,
) {
    let progress = SyncProgress {
        session_id: session_id.to_string(),
        phase: phase.to_string(),
        current,
        total,
        message: message.to_string(),
    };
    if let Err(e) = app.emit(SYNC_PROGRESS_EVENT, &progress) {
        log::warn!("发送同步进度事件失败: {e}");
    }
}
