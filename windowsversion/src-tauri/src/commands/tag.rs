use crate::db::{self, Database};
use crate::models;
use tauri::State;

#[tauri::command]
pub fn get_tags(database: State<'_, Database>) -> Result<Vec<models::Tag>, String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    db::tag::get_all(&conn)
}

#[tauri::command]
pub fn create_tag(
    database: State<'_, Database>,
    name: String,
    color: String,
    source: String,
) -> Result<models::Tag, String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    let tag = models::Tag {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        color,
        source,
        usage_count: 0,
    };
    db::tag::insert(&conn, &tag)?;
    Ok(tag)
}

#[tauri::command]
pub fn delete_tag(database: State<'_, Database>, id: String) -> Result<(), String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    db::tag::delete(&conn, &id)
}

#[tauri::command]
pub fn link_tag_to_asset(
    database: State<'_, Database>,
    asset_id: String,
    tag_id: String,
) -> Result<(), String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    db::tag::link_to_asset(&conn, &asset_id, &tag_id)
}

#[tauri::command]
pub fn get_asset_tags(
    database: State<'_, Database>,
    asset_id: String,
) -> Result<Vec<models::Tag>, String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    db::tag::get_tags_for_asset(&conn, &asset_id)
}

#[tauri::command]
pub fn unlink_tag_from_asset(
    database: State<'_, Database>,
    asset_id: String,
    tag_id: String,
) -> Result<(), String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    db::tag::unlink_from_asset(&conn, &asset_id, &tag_id)
}

/// 按名称查找或创建标签并关联到素材（避免重复 name 违反唯一约束）
#[tauri::command]
pub fn ensure_asset_tag_by_name(
    database: State<'_, Database>,
    asset_id: String,
    name: String,
) -> Result<models::Tag, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("标签名不能为空".to_string());
    }
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    let tag = db::tag::get_or_create_by_name(&conn, name, "user")?;
    db::tag::link_to_asset(&conn, &asset_id, &tag.id)?;
    Ok(tag)
}
