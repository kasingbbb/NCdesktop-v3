use crate::db::{self, Database};
use crate::models;
use tauri::State;

#[tauri::command]
pub fn get_libraries(database: State<'_, Database>) -> Result<Vec<models::Library>, String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    db::library::get_all(&conn)
}

#[tauri::command]
pub fn create_library(
    database: State<'_, Database>,
    name: String,
    root_path: String,
) -> Result<models::Library, String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    let lib = models::Library {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        root_path,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    db::library::insert(&conn, &lib)?;
    // custom_para_v1：新建 library 立刻 seed 4 个 PARA 内置类目（与 V17 backfill 对称）。
    // 失败仅 warn 不阻断主流程：库已建好，类目可以稍后通过 UI 手动补全。
    if let Err(e) = db::categories::seed_builtin_categories(&conn, &lib.id) {
        log::warn!("新建 library {} 后 seed 内置类目失败: {}", lib.id, e);
    }
    Ok(lib)
}

#[tauri::command]
pub fn update_library(
    database: State<'_, Database>,
    library: models::Library,
) -> Result<(), String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    db::library::update(&conn, &library)
}

#[tauri::command]
pub fn delete_library(database: State<'_, Database>, id: String) -> Result<(), String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    db::library::delete(&conn, &id)
}
