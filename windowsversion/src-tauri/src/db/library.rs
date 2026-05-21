use crate::models::Library;
use rusqlite::{params, Connection};

pub fn insert(conn: &Connection, lib: &Library) -> Result<(), String> {
    conn.execute(
        "INSERT INTO libraries (id, name, root_path, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![lib.id, lib.name, lib.root_path, lib.created_at],
    )
    .map_err(|e| format!("插入知识库失败: {e}"))?;
    Ok(())
}

pub fn get_all(conn: &Connection) -> Result<Vec<Library>, String> {
    let mut stmt = conn
        .prepare("SELECT id, name, root_path, created_at FROM libraries ORDER BY created_at DESC")
        .map_err(|e| format!("查询知识库失败: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(Library {
                id: row.get(0)?,
                name: row.get(1)?,
                root_path: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .map_err(|e| format!("遍历知识库失败: {e}"))?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| format!("读取行失败: {e}"))?);
    }
    Ok(result)
}

pub fn get_by_id(conn: &Connection, id: &str) -> Result<Option<Library>, String> {
    conn.query_row(
        "SELECT id, name, root_path, created_at FROM libraries WHERE id = ?1",
        params![id],
        |row| {
            Ok(Library {
                id: row.get(0)?,
                name: row.get(1)?,
                root_path: row.get(2)?,
                created_at: row.get(3)?,
            })
        },
    )
    .optional()
    .map_err(|e| format!("查询知识库失败: {e}"))
}

pub fn update(conn: &Connection, lib: &Library) -> Result<(), String> {
    conn.execute(
        "UPDATE libraries SET name = ?2, root_path = ?3 WHERE id = ?1",
        params![lib.id, lib.name, lib.root_path],
    )
    .map_err(|e| format!("更新知识库失败: {e}"))?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM libraries WHERE id = ?1", params![id])
        .map_err(|e| format!("删除知识库失败: {e}"))?;
    Ok(())
}

use rusqlite::OptionalExtension;
