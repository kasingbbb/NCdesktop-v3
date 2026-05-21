use crate::models::Note;
use rusqlite::{params, Connection, OptionalExtension};

pub fn insert(conn: &Connection, n: &Note) -> Result<(), String> {
    conn.execute(
        "INSERT INTO notes (id, project_id, asset_id, timeline_time, content, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            n.id, n.project_id, n.asset_id, n.timeline_time,
            n.content, n.created_at, n.updated_at,
        ],
    )
    .map_err(|e| format!("插入笔记失败: {e}"))?;
    Ok(())
}

pub fn get_by_project(conn: &Connection, project_id: &str) -> Result<Vec<Note>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, project_id, asset_id, timeline_time, content, created_at, updated_at
             FROM notes WHERE project_id = ?1 ORDER BY created_at DESC",
        )
        .map_err(|e| format!("查询笔记失败: {e}"))?;

    let rows = stmt
        .query_map(params![project_id], |row| row_to_note(row))
        .map_err(|e| format!("遍历笔记失败: {e}"))?;

    let mut result = Vec::new();
    for r in rows {
        result.push(r.map_err(|e| format!("读取行失败: {e}"))?);
    }
    Ok(result)
}

pub fn get_by_id(conn: &Connection, id: &str) -> Result<Option<Note>, String> {
    conn.query_row(
        "SELECT id, project_id, asset_id, timeline_time, content, created_at, updated_at
         FROM notes WHERE id = ?1",
        params![id],
        |row| row_to_note(row),
    )
    .optional()
    .map_err(|e| format!("查询笔记失败: {e}"))
}

pub fn update(conn: &Connection, n: &Note) -> Result<(), String> {
    conn.execute(
        "UPDATE notes SET content=?2, updated_at=?3 WHERE id=?1",
        params![n.id, n.content, n.updated_at],
    )
    .map_err(|e| format!("更新笔记失败: {e}"))?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM notes WHERE id = ?1", params![id])
        .map_err(|e| format!("删除笔记失败: {e}"))?;
    Ok(())
}

fn row_to_note(row: &rusqlite::Row) -> rusqlite::Result<Note> {
    Ok(Note {
        id: row.get(0)?,
        project_id: row.get(1)?,
        asset_id: row.get(2)?,
        timeline_time: row.get(3)?,
        content: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}
