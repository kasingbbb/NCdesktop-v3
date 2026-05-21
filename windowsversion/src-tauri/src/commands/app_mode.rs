//! AppMode + 启动期 repair 进度查询
use crate::db::repair::RepairProgress;
use crate::startup::AppMode;
use std::sync::{Arc, Mutex};

#[tauri::command]
pub fn get_app_mode(state: tauri::State<'_, AppMode>) -> Result<AppMode, String> {
    Ok(state.inner().clone())
}

#[tauri::command]
pub fn get_repair_progress(
    state: tauri::State<'_, Arc<Mutex<RepairProgress>>>,
) -> Result<RepairProgress, String> {
    let g = state
        .inner()
        .lock()
        .map_err(|_| "进度锁中毒".to_string())?;
    Ok(g.clone())
}
