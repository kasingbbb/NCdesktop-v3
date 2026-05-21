use crate::db::{self, Database};
use crate::models;
use tauri::State;

#[tauri::command]
pub fn get_notes(
    database: State<'_, Database>,
    project_id: String,
) -> Result<Vec<models::Note>, String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    db::note::get_by_project(&conn, &project_id)
}

#[tauri::command]
pub fn get_note(
    database: State<'_, Database>,
    id: String,
) -> Result<Option<models::Note>, String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    db::note::get_by_id(&conn, &id)
}

#[tauri::command]
pub fn create_note(
    database: State<'_, Database>,
    project_id: String,
    content: String,
    asset_id: Option<String>,
    timeline_time: Option<f64>,
) -> Result<models::Note, String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    let now = chrono::Utc::now().to_rfc3339();
    let note = models::Note {
        id: uuid::Uuid::new_v4().to_string(),
        project_id,
        asset_id,
        timeline_time,
        content,
        created_at: now.clone(),
        updated_at: now,
    };
    db::note::insert(&conn, &note)?;
    Ok(note)
}

#[tauri::command]
pub fn update_note(
    database: State<'_, Database>,
    id: String,
    content: String,
) -> Result<(), String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    if let Some(mut note) = db::note::get_by_id(&conn, &id)? {
        note.content = content;
        note.updated_at = chrono::Utc::now().to_rfc3339();
        db::note::update(&conn, &note)?;
    }
    Ok(())
}

#[tauri::command]
pub fn delete_note(database: State<'_, Database>, id: String) -> Result<(), String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    db::note::delete(&conn, &id)
}
