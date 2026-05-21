use crate::db::{self, search::SearchHit, Database};
use tauri::State;

#[tauri::command]
pub fn search(
    database: State<'_, Database>,
    query: String,
    limit: Option<i64>,
) -> Result<Vec<SearchHit>, String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    let limit = limit.unwrap_or(20);
    db::search::search_all(&conn, &query, limit)
}
