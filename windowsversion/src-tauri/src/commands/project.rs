use crate::db::{self, Database};
use crate::models;
use crate::workspace;
use std::fs;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn get_projects(
    database: State<'_, Database>,
    library_id: String,
) -> Result<Vec<models::Project>, String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    db::project::get_by_library(&conn, &library_id)
}

#[tauri::command]
pub fn get_project(
    database: State<'_, Database>,
    id: String,
) -> Result<Option<models::Project>, String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    db::project::get_by_id(&conn, &id)
}

#[tauri::command]
pub fn create_project(
    database: State<'_, Database>,
    library_id: String,
    name: String,
) -> Result<models::Project, String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    let now = chrono::Utc::now().to_rfc3339();
    let project = models::Project {
        id: uuid::Uuid::new_v4().to_string(),
        library_id,
        name,
        description: String::new(),
        cover_asset_id: None,
        source_type: "manual".to_string(),
        source_data: None,
        is_pinned: false,
        is_archived: false,
        created_at: now.clone(),
        updated_at: now,
        total_duration: None,
        asset_count: 0,
        word_count: 0,
        imported_at: None,
    };
    db::project::insert(&conn, &project)?;
    Ok(project)
}

#[tauri::command]
pub fn update_project(
    database: State<'_, Database>,
    project: models::Project,
) -> Result<(), String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    db::project::update(&conn, &project)
}

#[tauri::command]
pub fn delete_project(app: AppHandle, database: State<'_, Database>, id: String) -> Result<(), String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    db::project::delete(&conn, &id)?;
    drop(conn);
    // 旧版：应用数据目录 assets/<projectId>
    if let Ok(dir) = app.path().app_data_dir() {
        let asset_dir = dir.join("assets").join(&id);
        if asset_dir.is_dir() {
            if let Err(e) = fs::remove_dir_all(&asset_dir) {
                log::warn!("删除项目磁盘目录（旧路径）失败 {}: {e}", asset_dir.display());
            }
        }
    }
    // 工作区：~/Downloads/NoteCaptWorkPlace/<projectId>
    if let Ok(ws) = workspace::project_workspace_dir(&id) {
        if ws.is_dir() {
            if let Err(e) = fs::remove_dir_all(&ws) {
                log::warn!("删除项目工作区目录失败 {}: {e}", ws.display());
            }
        }
    }
    Ok(())
}
