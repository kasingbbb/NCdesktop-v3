//! 概念抽取日志：记录"此 library + asset + content_hash 已跑过 LLM 概念抽取"。
//! 配合 `extracted_content.content_hash` 实现增量抽取（F-8）。

use rusqlite::{params, Connection};
use std::collections::HashSet;

/// 读取某 library 已抽取过的 (asset_id, content_hash) 集合。
pub fn fetch_logged_pairs(
    conn: &Connection,
    library_id: &str,
) -> Result<HashSet<(String, String)>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT asset_id, content_hash FROM concepts_extraction_log WHERE library_id = ?1",
        )
        .map_err(|e| format!("准备查询抽取日志失败: {e}"))?;
    let rows = stmt
        .query_map(params![library_id], |row| {
            let aid: String = row.get(0)?;
            let h: String = row.get(1)?;
            Ok((aid, h))
        })
        .map_err(|e| format!("查询抽取日志失败: {e}"))?;

    let mut set = HashSet::new();
    for row in rows {
        set.insert(row.map_err(|e| format!("读取抽取日志行失败: {e}"))?);
    }
    Ok(set)
}

/// 记录一条抽取日志（幂等；UNIQUE 约束冲突时忽略）。
pub fn insert(
    conn: &Connection,
    library_id: &str,
    asset_id: &str,
    content_hash: &str,
) -> Result<(), String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT OR IGNORE INTO concepts_extraction_log
            (id, library_id, asset_id, content_hash, extracted_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, library_id, asset_id, content_hash, now],
    )
    .map_err(|e| format!("写入抽取日志失败: {e}"))?;
    Ok(())
}
