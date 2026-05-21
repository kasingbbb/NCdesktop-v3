use crate::models::Project;
use rusqlite::{params, Connection, OptionalExtension};

pub fn insert(conn: &Connection, p: &Project) -> Result<(), String> {
    conn.execute(
        "INSERT INTO projects (id, library_id, name, description, cover_asset_id,
         source_type, source_data, is_pinned, is_archived, created_at, updated_at,
         total_duration, asset_count, word_count, imported_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
        params![
            p.id, p.library_id, p.name, p.description, p.cover_asset_id,
            p.source_type, p.source_data,
            p.is_pinned as i32, p.is_archived as i32,
            p.created_at, p.updated_at,
            p.total_duration, p.asset_count, p.word_count, p.imported_at,
        ],
    )
    .map_err(|e| format!("插入项目失败: {e}"))?;
    Ok(())
}

pub fn get_by_library(conn: &Connection, library_id: &str) -> Result<Vec<Project>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, library_id, name, description, cover_asset_id,
             source_type, source_data, is_pinned, is_archived, created_at, updated_at,
             total_duration, asset_count, word_count, imported_at
             FROM projects WHERE library_id = ?1 ORDER BY updated_at DESC",
        )
        .map_err(|e| format!("查询项目失败: {e}"))?;

    let rows = stmt
        .query_map(params![library_id], |row| row_to_project(row))
        .map_err(|e| format!("遍历项目失败: {e}"))?;

    collect_rows(rows)
}

pub fn get_by_id(conn: &Connection, id: &str) -> Result<Option<Project>, String> {
    conn.query_row(
        "SELECT id, library_id, name, description, cover_asset_id,
         source_type, source_data, is_pinned, is_archived, created_at, updated_at,
         total_duration, asset_count, word_count, imported_at
         FROM projects WHERE id = ?1",
        params![id],
        |row| row_to_project(row),
    )
    .optional()
    .map_err(|e| format!("查询项目失败: {e}"))
}

pub fn update(conn: &Connection, p: &Project) -> Result<(), String> {
    conn.execute(
        "UPDATE projects SET name=?2, description=?3, cover_asset_id=?4,
         is_pinned=?5, is_archived=?6, updated_at=?7,
         total_duration=?8, asset_count=?9, word_count=?10
         WHERE id=?1",
        params![
            p.id, p.name, p.description, p.cover_asset_id,
            p.is_pinned as i32, p.is_archived as i32,
            p.updated_at, p.total_duration, p.asset_count, p.word_count,
        ],
    )
    .map_err(|e| format!("更新项目失败: {e}"))?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM projects WHERE id = ?1", params![id])
        .map_err(|e| format!("删除项目失败: {e}"))?;
    Ok(())
}

fn row_to_project(row: &rusqlite::Row) -> rusqlite::Result<Project> {
    let is_pinned: i32 = row.get(7)?;
    let is_archived: i32 = row.get(8)?;
    Ok(Project {
        id: row.get(0)?,
        library_id: row.get(1)?,
        name: row.get(2)?,
        description: row.get(3)?,
        cover_asset_id: row.get(4)?,
        source_type: row.get(5)?,
        source_data: row.get(6)?,
        is_pinned: is_pinned != 0,
        is_archived: is_archived != 0,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        total_duration: row.get(11)?,
        asset_count: row.get(12)?,
        word_count: row.get(13)?,
        imported_at: row.get(14)?,
    })
}

fn collect_rows(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row) -> rusqlite::Result<Project>>,
) -> Result<Vec<Project>, String> {
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| format!("读取行失败: {e}"))?);
    }
    Ok(result)
}
