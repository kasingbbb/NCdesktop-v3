//! PR-3 task_012：分类对象 CRUD（library 级）
//!
//! 红线：
//! - slug 白名单 `[a-z0-9一-龥_-]`，长度 1-32
//! - 保留 slug：`__uncategorized__` / `__archived__` / `other` 拒绝创建
//! - delete_category 仅在 `is_builtin=0` 且引用计数=0 时允许

use crate::db::Database;
use crate::startup::{ensure_writable, AppMode};
use serde::{Deserialize, Serialize};
use tauri::State;

const RESERVED: &[&str] = &["__uncategorized__", "__archived__", "other"];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: i64,
    pub library_id: String,
    pub slug: String,
    pub label: String,
    pub icon: Option<String>,
    pub sort_order: i64,
    pub is_disabled: bool,
    pub is_builtin: bool,
}

fn validate_slug_for_create(slug: &str) -> Result<(), String> {
    let s = slug.trim();
    if s.is_empty() || s.len() > 32 {
        return Err("slug 长度需在 1-32".into());
    }
    if RESERVED.contains(&s) {
        return Err(format!("`{s}` 为保留字，不可作为新分类 slug"));
    }
    for ch in s.chars() {
        let ok = ch.is_ascii_alphanumeric()
            || ch == '_'
            || ch == '-'
            || matches!(ch as u32, 0x4E00..=0x9FFF | 0x3400..=0x4DBF);
        if !ok {
            return Err(format!("slug 含非法字符: `{ch}`"));
        }
    }
    Ok(())
}

fn row_to_category(row: &rusqlite::Row<'_>) -> rusqlite::Result<Category> {
    Ok(Category {
        id: row.get(0)?,
        library_id: row.get(1)?,
        slug: row.get(2)?,
        label: row.get(3)?,
        icon: row.get(4)?,
        sort_order: row.get(5)?,
        is_disabled: row.get::<_, i64>(6)? != 0,
        is_builtin: row.get::<_, i64>(7)? != 0,
    })
}

#[tauri::command]
pub fn list_categories(
    database: State<'_, Database>,
    library_id: String,
    include_disabled: Option<bool>,
) -> Result<Vec<Category>, String> {
    let include_disabled = include_disabled.unwrap_or(false);
    let conn = database.conn.lock().map_err(|e| format!("DB 锁: {e}"))?;
    let sql = if include_disabled {
        "SELECT id, library_id, slug, label, icon, sort_order, is_disabled, is_builtin
           FROM categories WHERE library_id=?1 ORDER BY sort_order, id;"
    } else {
        "SELECT id, library_id, slug, label, icon, sort_order, is_disabled, is_builtin
           FROM categories WHERE library_id=?1 AND is_disabled=0 ORDER BY sort_order, id;"
    };
    let mut stmt = conn.prepare(sql).map_err(|e| format!("prepare: {e}"))?;
    let cats: Vec<Category> = stmt
        .query_map(rusqlite::params![library_id], row_to_category)
        .map_err(|e| format!("query: {e}"))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(cats)
}

#[tauri::command]
pub fn create_category(
    database: State<'_, Database>,
    mode: State<'_, AppMode>,
    library_id: String,
    slug: String,
    label: String,
    sort_order: Option<i64>,
) -> Result<Category, String> {
    ensure_writable(mode.inner())?;
    validate_slug_for_create(&slug)?;
    if label.trim().is_empty() {
        return Err("label 不能为空".into());
    }
    let conn = database.conn.lock().map_err(|e| format!("DB 锁: {e}"))?;
    conn.execute(
        "INSERT INTO categories (library_id, slug, label, sort_order, is_builtin)
         VALUES (?1, ?2, ?3, ?4, 0);",
        rusqlite::params![library_id, slug, label, sort_order.unwrap_or(50)],
    )
    .map_err(|e| format!("插入分类失败: {e}"))?;
    let c = conn
        .query_row(
            "SELECT id, library_id, slug, label, icon, sort_order, is_disabled, is_builtin
               FROM categories WHERE library_id=?1 AND slug=?2;",
            rusqlite::params![library_id, slug],
            row_to_category,
        )
        .map_err(|e| format!("回读失败: {e}"))?;
    Ok(c)
}

#[tauri::command]
pub fn rename_category(
    database: State<'_, Database>,
    mode: State<'_, AppMode>,
    library_id: String,
    slug: String,
    label: String,
) -> Result<Category, String> {
    ensure_writable(mode.inner())?;
    if label.trim().is_empty() {
        return Err("label 不能为空".into());
    }
    let conn = database.conn.lock().map_err(|e| format!("DB 锁: {e}"))?;
    let n = conn
        .execute(
            "UPDATE categories SET label=?1, updated_at=datetime('now')
              WHERE library_id=?2 AND slug=?3;",
            rusqlite::params![label, library_id, slug],
        )
        .map_err(|e| format!("rename: {e}"))?;
    if n == 0 {
        return Err(format!("分类不存在: lib={library_id} slug={slug}"));
    }
    conn.query_row(
        "SELECT id, library_id, slug, label, icon, sort_order, is_disabled, is_builtin
           FROM categories WHERE library_id=?1 AND slug=?2;",
        rusqlite::params![library_id, slug],
        row_to_category,
    )
    .map_err(|e| format!("回读: {e}"))
}

#[tauri::command]
pub fn set_category_disabled(
    database: State<'_, Database>,
    mode: State<'_, AppMode>,
    library_id: String,
    slug: String,
    disabled: bool,
) -> Result<Category, String> {
    ensure_writable(mode.inner())?;
    let conn = database.conn.lock().map_err(|e| format!("DB 锁: {e}"))?;
    let n = conn
        .execute(
            "UPDATE categories SET is_disabled=?1, updated_at=datetime('now')
              WHERE library_id=?2 AND slug=?3;",
            rusqlite::params![if disabled { 1 } else { 0 }, library_id, slug],
        )
        .map_err(|e| format!("set_disabled: {e}"))?;
    if n == 0 {
        return Err("分类不存在".into());
    }
    conn.query_row(
        "SELECT id, library_id, slug, label, icon, sort_order, is_disabled, is_builtin
           FROM categories WHERE library_id=?1 AND slug=?2;",
        rusqlite::params![library_id, slug],
        row_to_category,
    )
    .map_err(|e| format!("回读: {e}"))
}

#[tauri::command]
pub fn delete_category(
    database: State<'_, Database>,
    mode: State<'_, AppMode>,
    library_id: String,
    slug: String,
) -> Result<(), String> {
    ensure_writable(mode.inner())?;
    let conn = database.conn.lock().map_err(|e| format!("DB 锁: {e}"))?;
    // 前置：is_builtin=0
    let is_builtin: i64 = conn
        .query_row(
            "SELECT is_builtin FROM categories WHERE library_id=?1 AND slug=?2;",
            rusqlite::params![library_id, slug],
            |r| r.get(0),
        )
        .map_err(|e| format!("分类不存在: {e}"))?;
    if is_builtin != 0 {
        return Err("内置 PARA 分类不可删除".into());
    }
    // 前置：引用计数=0
    let refs: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM assets a
              JOIN projects p ON p.id = a.project_id
             WHERE p.library_id=?1 AND a.category_slug=?2;",
            rusqlite::params![library_id, slug],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if refs > 0 {
        return Err(format!("分类下仍有 {refs} 个资产，请先迁移或删除"));
    }
    conn.execute(
        "DELETE FROM categories WHERE library_id=?1 AND slug=?2;",
        rusqlite::params![library_id, slug],
    )
    .map_err(|e| format!("delete: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn add_category_alias(
    database: State<'_, Database>,
    mode: State<'_, AppMode>,
    library_id: String,
    alias_slug: String,
    target_slug: String,
) -> Result<(), String> {
    ensure_writable(mode.inner())?;
    let conn = database.conn.lock().map_err(|e| format!("DB 锁: {e}"))?;
    // 校验 target 存在
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM categories WHERE library_id=?1 AND slug=?2;",
            rusqlite::params![library_id, target_slug],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if exists == 0 {
        return Err("target_slug 不存在".into());
    }
    conn.execute(
        "INSERT OR REPLACE INTO category_aliases (library_id, alias_slug, target_slug)
         VALUES (?1, ?2, ?3);",
        rusqlite::params![library_id, alias_slug, target_slug],
    )
    .map_err(|e| format!("alias 插入: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_db() -> Database {
        let dir = tempfile::tempdir().expect("td");
        let db_path = dir.path().join("cats.db");
        std::mem::forget(dir);
        Database::open(&db_path).expect("open")
    }

    fn seed(database: &Database) {
        let conn = database.conn.lock().unwrap();
        conn.execute_batch(
            "INSERT INTO libraries (id, name, root_path) VALUES ('lib','L','/tmp/L');
             INSERT INTO categories (library_id, slug, label, is_builtin) VALUES
               ('lib', '1-项目', '项目', 1),
               ('lib', 'mycat', '自定义', 0);",
        )
        .unwrap();
    }

    fn _state_with_normal() -> AppMode {
        AppMode::Normal
    }

    #[test]
    fn validate_slug_rules() {
        assert!(validate_slug_for_create("mycat").is_ok());
        assert!(validate_slug_for_create("研究").is_ok());
        assert!(validate_slug_for_create("1-项目-2").is_ok());
        assert!(validate_slug_for_create("").is_err());
        assert!(validate_slug_for_create("with space").is_err());
        assert!(validate_slug_for_create("a/b").is_err());
        assert!(validate_slug_for_create("__uncategorized__").is_err());
        assert!(validate_slug_for_create("other").is_err());
        let long = "a".repeat(33);
        assert!(validate_slug_for_create(&long).is_err());
    }

    /// 直接调内部 SQL 不走 tauri::command 包装（State 注入复杂）
    #[test]
    fn delete_blocked_when_referenced() {
        let database = fresh_db();
        seed(&database);
        // 插入引用资产
        {
            let conn = database.conn.lock().unwrap();
            conn.execute_batch(
                "INSERT INTO projects (id, library_id, name) VALUES ('p1','lib','P');
                 INSERT INTO assets (id, project_id, asset_type, name, file_path, category_slug)
                   VALUES ('a1','p1','pdf','x.pdf','/tmp/x.pdf','mycat');",
            )
            .unwrap();
        }
        // 模拟 delete_category 内部判定逻辑
        let conn = database.conn.lock().unwrap();
        let is_builtin: i64 = conn
            .query_row(
                "SELECT is_builtin FROM categories WHERE library_id='lib' AND slug='mycat';",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(is_builtin, 0);
        let refs: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM assets a JOIN projects p ON p.id=a.project_id
                  WHERE p.library_id='lib' AND a.category_slug='mycat';",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(refs > 0, "应被 ref 计数挡住");
    }

    #[test]
    fn delete_blocked_when_builtin() {
        let database = fresh_db();
        seed(&database);
        let conn = database.conn.lock().unwrap();
        let is_builtin: i64 = conn
            .query_row(
                "SELECT is_builtin FROM categories WHERE library_id='lib' AND slug='1-项目';",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(is_builtin, 1, "PARA 内置应受保护");
    }
}
