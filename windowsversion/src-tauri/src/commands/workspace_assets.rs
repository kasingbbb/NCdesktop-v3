//! PR-3 task_008: list_workspace_assets — DB 权威源 + cursor 分页（ADR-003）
//!
//! 入参 cursor 编码：base64(`<updated_at_unix_secs>:<id>`)；按 `(updated_at DESC, id DESC)` 扫描

use crate::db::repair::parse_topics_or_empty;
use crate::db::Database;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use tauri::State;

const DEFAULT_PAGE: i64 = 200;
const MAX_PAGE: i64 = 500;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetView {
    pub id: String,
    pub name: String,
    pub category_slug: Option<String>,
    pub tags: Vec<String>,
    pub size_bytes: i64,
    pub mime: String,
    pub updated_at: String,
    pub relative_path: String,
    pub icon_hint: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListWorkspaceAssetsResponse {
    pub items: Vec<AssetView>,
    pub next_cursor: Option<String>,
}

fn encode_cursor(updated_at: &str, id: &str) -> String {
    STANDARD.encode(format!("{}:{}", updated_at, id))
}

fn decode_cursor(c: &str) -> Result<(String, String), String> {
    let raw = STANDARD
        .decode(c.as_bytes())
        .map_err(|e| format!("cursor base64 解码失败: {e}"))?;
    let s = String::from_utf8(raw).map_err(|e| format!("cursor utf8 失败: {e}"))?;
    let (a, b) = s.rsplit_once(':').ok_or("cursor 格式错误：缺少 `:`")?;
    Ok((a.to_string(), b.to_string()))
}

fn icon_hint_for(mime: &str, asset_type: &str) -> String {
    if mime.starts_with("image/") {
        "image".into()
    } else if mime.starts_with("video/") {
        "video".into()
    } else if mime.starts_with("audio/") {
        "audio".into()
    } else if mime.contains("pdf") || asset_type == "pdf" {
        "pdf".into()
    } else if mime.contains("word") || mime.contains("document") || mime.contains("officedocument") {
        "office".into()
    } else if mime.starts_with("text/") {
        "text".into()
    } else {
        "unknown".into()
    }
}

#[tauri::command]
pub fn list_workspace_assets(
    database: State<'_, Database>,
    project_id: String,
    category_slug: Option<String>,
    sub_path: Option<String>,
    cursor: Option<String>,
    page_size: Option<i64>,
) -> Result<ListWorkspaceAssetsResponse, String> {
    let page = page_size.unwrap_or(DEFAULT_PAGE).clamp(1, MAX_PAGE);

    let conn = database
        .conn
        .lock()
        .map_err(|e| format!("DB 锁: {e}"))?;

    // 构造 SQL
    let mut sql = String::from(
        "SELECT a.id, a.name, a.category_slug, a.file_size, a.mime_type, a.updated_at, a.file_path, a.asset_type,
                COALESCE(ai.topics, '[]'), COALESCE(ai.suggested_tags, '[]')
           FROM assets a
           LEFT JOIN ai_analyses ai ON ai.asset_id = a.id
          WHERE a.project_id = ?1",
    );
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(project_id.clone())];
    let mut idx = 2;

    if let Some(cs) = category_slug.as_ref() {
        sql.push_str(&format!(" AND a.category_slug = ?{}", idx));
        params.push(Box::new(cs.clone()));
        idx += 1;
    }
    if let Some(_sp) = sub_path.as_ref() {
        // sub_path 的 file_path LIKE 过滤 — 简化：不在 MVP 实现路径过滤，直接忽略
        // 真实子目录支持留 v2（需要 file_path 与 category_slug 解耦）
    }

    if let Some(c) = cursor.as_ref() {
        let (cu_at, cu_id) = decode_cursor(c)?;
        sql.push_str(&format!(
            " AND (a.updated_at < ?{} OR (a.updated_at = ?{} AND a.id < ?{}))",
            idx,
            idx + 1,
            idx + 2
        ));
        params.push(Box::new(cu_at.clone()));
        params.push(Box::new(cu_at));
        params.push(Box::new(cu_id));
        idx += 3;
    }

    sql.push_str(&format!(
        " ORDER BY a.updated_at DESC, a.id DESC LIMIT ?{}",
        idx
    ));
    params.push(Box::new(page + 1)); // 多取一条判断是否有 next

    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|b| b.as_ref()).collect();
    let mut stmt = conn.prepare(&sql).map_err(|e| format!("准备 SQL 失败: {e}"))?;
    let mut rows = stmt
        .query(rusqlite::params_from_iter(param_refs.iter().copied()))
        .map_err(|e| format!("执行查询失败: {e}"))?;

    let mut items: Vec<AssetView> = Vec::new();
    let mut last_at = String::new();
    let mut last_id = String::new();
    while let Some(row) = rows.next().map_err(|e| format!("取行失败: {e}"))? {
        if items.len() >= page as usize {
            break;
        }
        let id: String = row.get(0).map_err(|e| e.to_string())?;
        let name: String = row.get(1).map_err(|e| e.to_string())?;
        let category_slug: Option<String> = row.get(2).map_err(|e| e.to_string())?;
        let size_bytes: i64 = row.get(3).map_err(|e| e.to_string())?;
        let mime: String = row.get(4).map_err(|e| e.to_string())?;
        let updated_at: String = row.get(5).map_err(|e| e.to_string())?;
        let file_path: String = row.get(6).map_err(|e| e.to_string())?;
        let asset_type: String = row.get(7).map_err(|e| e.to_string())?;
        let topics_raw: String = row.get(8).map_err(|e| e.to_string())?;
        let suggested_tags_raw: String = row.get(9).map_err(|e| e.to_string())?;

        // tags 合并 topics + suggested_tags
        let mut tags: Vec<String> = parse_topics_or_empty(&topics_raw);
        if let Ok(serde_json::Value::Array(arr)) =
            serde_json::from_str::<serde_json::Value>(&suggested_tags_raw)
        {
            for v in arr {
                if let serde_json::Value::String(s) = v {
                    if !tags.contains(&s) {
                        tags.push(s);
                    }
                }
            }
        }

        last_at = updated_at.clone();
        last_id = id.clone();

        items.push(AssetView {
            id,
            name,
            category_slug,
            tags,
            size_bytes,
            mime: mime.clone(),
            updated_at,
            relative_path: file_path,
            icon_hint: icon_hint_for(&mime, &asset_type),
        });
    }

    // 是否还有下一页：如果还能从 rows 里 next() 出一条 = next 存在
    let has_next = rows
        .next()
        .map_err(|e| format!("next 检查失败: {e}"))?
        .is_some();
    let next_cursor = if has_next {
        Some(encode_cursor(&last_at, &last_id))
    } else {
        None
    };

    Ok(ListWorkspaceAssetsResponse { items, next_cursor })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_roundtrip() {
        let c = encode_cursor("2026-01-01T10:00:00Z", "asset-123");
        let (a, b) = decode_cursor(&c).unwrap();
        assert_eq!(a, "2026-01-01T10:00:00Z");
        assert_eq!(b, "asset-123");
    }

    #[test]
    fn icon_hint_dispatch() {
        assert_eq!(icon_hint_for("image/png", "image"), "image");
        assert_eq!(icon_hint_for("application/pdf", "pdf"), "pdf");
        assert_eq!(icon_hint_for("video/mp4", "video"), "video");
        assert_eq!(icon_hint_for("audio/mpeg", "audio"), "audio");
        assert_eq!(icon_hint_for("application/vnd.openxmlformats-officedocument.wordprocessingml.document", "doc"), "office");
        assert_eq!(icon_hint_for("text/plain", "text"), "text");
        assert_eq!(icon_hint_for("application/zip", "archive"), "unknown");
    }
}
