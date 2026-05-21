use rusqlite::{params, Connection};
use serde::Serialize;

/// 搜索结果条目
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub id: String,
    pub hit_type: String,
    pub title: String,
    pub snippet: String,
    pub project_id: String,
    pub asset_id: Option<String>,
    pub score: f64,
}

/// FTS5 全文搜索：搜索素材名称/路径
pub fn search_assets(conn: &Connection, query: &str, limit: i64) -> Result<Vec<SearchHit>, String> {
    let fts_query = format!("{}*", query.replace('"', ""));
    let mut stmt = conn
        .prepare(
            "SELECT a.id, a.name, a.file_path, a.project_id, rank
             FROM fts_assets f
             JOIN assets a ON a.rowid = f.rowid
             WHERE fts_assets MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )
        .map_err(|e| format!("搜索素材失败: {e}"))?;

    let rows = stmt
        .query_map(params![fts_query, limit], |row| {
            let rank: f64 = row.get(4)?;
            Ok(SearchHit {
                id: row.get(0)?,
                hit_type: "asset".to_string(),
                title: row.get(1)?,
                snippet: row.get(2)?,
                project_id: row.get(3)?,
                asset_id: Some(row.get::<_, String>(0)?),
                score: -rank,
            })
        })
        .map_err(|e| format!("遍历搜索结果失败: {e}"))?;

    let mut result = Vec::new();
    for r in rows {
        result.push(r.map_err(|e| format!("读取行失败: {e}"))?);
    }
    Ok(result)
}

/// FTS5 全文搜索：搜索笔记内容
pub fn search_notes(conn: &Connection, query: &str, limit: i64) -> Result<Vec<SearchHit>, String> {
    let fts_query = format!("{}*", query.replace('"', ""));
    let mut stmt = conn
        .prepare(
            "SELECT n.id, n.content, n.project_id, n.asset_id, rank
             FROM fts_notes f
             JOIN notes n ON n.rowid = f.rowid
             WHERE fts_notes MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )
        .map_err(|e| format!("搜索笔记失败: {e}"))?;

    let rows = stmt
        .query_map(params![fts_query, limit], |row| {
            let content: String = row.get(1)?;
            let rank: f64 = row.get(4)?;
            let snippet = if content.len() > 120 {
                format!("{}...", &content[..120])
            } else {
                content
            };
            Ok(SearchHit {
                id: row.get(0)?,
                hit_type: "note".to_string(),
                title: "笔记".to_string(),
                snippet,
                project_id: row.get(2)?,
                asset_id: row.get(3)?,
                score: -rank,
            })
        })
        .map_err(|e| format!("遍历搜索结果失败: {e}"))?;

    let mut result = Vec::new();
    for r in rows {
        result.push(r.map_err(|e| format!("读取行失败: {e}"))?);
    }
    Ok(result)
}

/// 聚合搜索：同时搜索素材 + 笔记
pub fn search_all(conn: &Connection, query: &str, limit: i64) -> Result<Vec<SearchHit>, String> {
    let per_type_limit = limit;
    let mut results = search_assets(conn, query, per_type_limit)?;
    results.extend(search_notes(conn, query, per_type_limit)?);
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(limit as usize);
    Ok(results)
}
