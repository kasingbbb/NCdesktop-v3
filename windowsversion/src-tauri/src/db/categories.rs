//! 自定义 PARA 类目数据访问层（custom_para_v1）。
//!
//! 与 `commands/categories.rs` 的关系：
//! - `commands/categories.rs` 提供面向前端 IPC 的 CRUD（list / create / rename / disable / delete / add_alias）
//!   ——schema 由 PR-3 task_012 设计、V17 迁移落地。
//! - 本模块（`db/categories.rs`）只提供 **dropzone / LLM 旁路**所需的非 IPC 辅助函数：
//!   - `seed_builtin_categories`：在新建 library 后调用，幂等 seed 4 个 PARA 内置类目
//!   - `resolve_for_slug`：把 LLM 返回的 `category` 字符串解析成 active `Category`，
//!     处理 alias 映射 + 跳过 disabled 行
//!   - `upsert_llm_generated`：当 LLM 输出未知 slug（且未命中 alias）时，
//!     自动创建一条 `source='llm_generated'` 的类目
//!   - `list_active_for_prompt`：用于 `assemble_messages_for_classify` 注入类目清单
//!
//! 设计取舍：
//! - 不重复定义完整 `Category` struct（避免与 `commands::categories::Category` 分裂）；
//!   本模块返回精简 `CategoryRow`，只含落盘所需字段（id / slug / label / is_disabled）。
//! - `resolve_for_slug` 不创建新类目（read-only），upsert 由调用方按需触发。
//! - 所有写操作走 `INSERT OR IGNORE` / `INSERT OR REPLACE`，对并发导入幂等。

use rusqlite::{params, Connection, OptionalExtension};

/// 精简类目行：dropzone 落盘只需要 slug / label / id 三件套。
#[derive(Debug, Clone)]
pub struct CategoryRow {
    pub id: i64,
    pub slug: String,
    pub label: String,
    pub is_builtin: bool,
    pub is_disabled: bool,
}

fn row_to_category_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CategoryRow> {
    Ok(CategoryRow {
        id: row.get(0)?,
        slug: row.get(1)?,
        label: row.get(2)?,
        is_builtin: row.get::<_, i64>(3)? != 0,
        is_disabled: row.get::<_, i64>(4)? != 0,
    })
}

/// 在指定 library 下 seed 4 个 PARA 内置类目（幂等，对已存在的 slug `INSERT OR IGNORE`）。
///
/// 调用时机：
/// - 新建 library（`commands::library::create_library`）后立即调用
/// - `dropzone::ensure_import_project_id` 自动建「默认知识库」后调用
/// - V17 migration 对**已存在的 library** 批量 backfill
pub fn seed_builtin_categories(conn: &Connection, library_id: &str) -> Result<(), String> {
    // 委托给 migration 模块的实现，避免 schema 字面值散落两处。
    super::migration::seed_builtin_categories_impl(conn, library_id)
}

/// 解析 LLM 返回的 `category` slug 到一条 active 类目行。
///
/// 解析顺序：
/// 1. `slug` 在 `categories` 表命中且 `is_disabled=0` → 返回该行
/// 2. `slug` 在 `category_aliases` 表命中 → 用 `target_slug` 再查 categories
/// 3. 都未命中 → 返回 `None`（由调用方决定是否 `upsert_llm_generated` 自动建）
///
/// 备注：对大小写敏感（slug 内中文 + 短横线无大小写概念）；trim 由调用方负责。
pub fn resolve_for_slug(
    conn: &Connection,
    library_id: &str,
    slug: &str,
) -> Result<Option<CategoryRow>, String> {
    // 直接命中
    if let Some(row) = query_active_by_slug(conn, library_id, slug)? {
        return Ok(Some(row));
    }
    // 走 alias
    let target: Option<String> = conn
        .query_row(
            "SELECT target_slug FROM category_aliases WHERE library_id=?1 AND alias_slug=?2",
            params![library_id, slug],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| format!("查询 category_aliases 失败: {e}"))?;
    if let Some(target_slug) = target {
        return query_active_by_slug(conn, library_id, &target_slug);
    }
    Ok(None)
}

fn query_active_by_slug(
    conn: &Connection,
    library_id: &str,
    slug: &str,
) -> Result<Option<CategoryRow>, String> {
    conn.query_row(
        "SELECT id, slug, label, is_builtin, is_disabled
           FROM categories
          WHERE library_id=?1 AND slug=?2 AND is_disabled=0",
        params![library_id, slug],
        row_to_category_row,
    )
    .optional()
    .map_err(|e| format!("查询 categories 失败: {e}"))
}

/// LLM 自动建类目：当 `resolve_for_slug` 返回 None 且调用方判定应当新建时调用。
///
/// `slug` 应已通过 [`sanitize_slug`] 规范化；`label` 是面向用户的展示名（可含原文中文短语）。
/// 返回新创建（或已存在）的 `CategoryRow`。
pub fn upsert_llm_generated(
    conn: &Connection,
    library_id: &str,
    slug: &str,
    label: &str,
) -> Result<CategoryRow, String> {
    conn.execute(
        "INSERT INTO categories
           (library_id, slug, label, sort_order, is_builtin, source)
         VALUES (?1, ?2, ?3, 100, 0, 'llm_generated')
         ON CONFLICT(library_id, slug) DO UPDATE SET updated_at=datetime('now')",
        params![library_id, slug, label],
    )
    .map_err(|e| format!("upsert llm 类目失败（{library_id}/{slug}）: {e}"))?;

    query_active_by_slug(conn, library_id, slug)?
        .ok_or_else(|| format!("upsert 后回读失败（{library_id}/{slug}）"))
}

/// 给 prompt 注入用的 active 类目清单（按 sort_order, id 排序）。
/// 仅返回 `is_disabled=0` 的行；返回顺序：builtin 在前（按 sort_order），user/llm_generated 在后。
pub fn list_active_for_prompt(
    conn: &Connection,
    library_id: &str,
) -> Result<Vec<CategoryRow>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, slug, label, is_builtin, is_disabled
               FROM categories
              WHERE library_id=?1 AND is_disabled=0
              ORDER BY sort_order, id",
        )
        .map_err(|e| format!("准备 list_active 查询失败: {e}"))?;
    let rows = stmt
        .query_map(params![library_id], row_to_category_row)
        .map_err(|e| format!("执行 list_active 查询失败: {e}"))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| format!("遍历 list_active 失败: {e}"))?);
    }
    Ok(out)
}

/// slug 规范化：复用 `commands/categories.rs` 的字符规则（ASCII 字母数字 / `-` / `_` /
/// CJK Unified Ideographs），其余字符替换为 `-`，长度截到 32。空串返回 `"other"`。
///
/// 调用方：`dropzone::resolve_or_create_category` 在 LLM 输出 `new:xxx` 或未知 slug 时调用。
pub fn sanitize_slug(input: &str) -> String {
    let cleaned: String = input
        .trim()
        .chars()
        .map(|ch| {
            let ok = ch.is_ascii_alphanumeric()
                || ch == '_'
                || ch == '-'
                || matches!(ch as u32, 0x4E00..=0x9FFF | 0x3400..=0x4DBF);
            if ok {
                ch
            } else {
                '-'
            }
        })
        .collect();
    // 合并连续的 '-' → 单个；首尾 '-' 去除
    let mut compact = String::with_capacity(cleaned.len());
    let mut prev_dash = false;
    for ch in cleaned.chars() {
        if ch == '-' {
            if !prev_dash {
                compact.push('-');
            }
            prev_dash = true;
        } else {
            compact.push(ch);
            prev_dash = false;
        }
    }
    let trimmed: String = compact
        .trim_matches('-')
        .chars()
        .take(32)
        .collect();
    if trimmed.is_empty() {
        "other".to_string()
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migration::run_migrations;

    fn fresh_conn_with_lib(lib_id: &str) -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn.execute(
            "INSERT INTO libraries (id, name, root_path, created_at) VALUES (?1, 'L', '/tmp/L', datetime('now'))",
            params![lib_id],
        )
        .unwrap();
        // migration 跑完后 lib 为空，没 backfill。这里手动 seed。
        seed_builtin_categories(&conn, lib_id).unwrap();
        conn
    }

    #[test]
    fn seed_creates_four_builtins_and_is_idempotent() {
        let conn = fresh_conn_with_lib("L1");
        seed_builtin_categories(&conn, "L1").unwrap();
        let cnt: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM categories WHERE library_id='L1' AND is_builtin=1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(cnt, 4);
    }

    #[test]
    fn resolve_returns_active_builtin_by_slug() {
        let conn = fresh_conn_with_lib("L1");
        let row = resolve_for_slug(&conn, "L1", "1-项目").unwrap();
        let row = row.expect("应命中 1-项目");
        assert_eq!(row.slug, "1-项目");
        assert!(row.is_builtin);
        assert!(!row.is_disabled);
    }

    #[test]
    fn resolve_skips_disabled() {
        let conn = fresh_conn_with_lib("L1");
        conn.execute(
            "UPDATE categories SET is_disabled=1 WHERE library_id='L1' AND slug='4-存档'",
            [],
        )
        .unwrap();
        assert!(resolve_for_slug(&conn, "L1", "4-存档").unwrap().is_none());
    }

    #[test]
    fn resolve_follows_alias() {
        let conn = fresh_conn_with_lib("L1");
        // 自定义类目 + alias 指向它
        upsert_llm_generated(&conn, "L1", "读书笔记", "读书笔记").unwrap();
        conn.execute(
            "INSERT INTO category_aliases (library_id, alias_slug, target_slug)
             VALUES ('L1', '学习资料', '读书笔记')",
            [],
        )
        .unwrap();
        let row = resolve_for_slug(&conn, "L1", "学习资料").unwrap().unwrap();
        assert_eq!(row.slug, "读书笔记");
    }

    #[test]
    fn resolve_returns_none_for_unknown_slug() {
        let conn = fresh_conn_with_lib("L1");
        assert!(resolve_for_slug(&conn, "L1", "完全没听过").unwrap().is_none());
    }

    #[test]
    fn upsert_llm_generated_creates_and_is_idempotent() {
        let conn = fresh_conn_with_lib("L1");
        let r1 = upsert_llm_generated(&conn, "L1", "读书笔记", "读书笔记").unwrap();
        let r2 = upsert_llm_generated(&conn, "L1", "读书笔记", "读书笔记").unwrap();
        assert_eq!(r1.id, r2.id);
        assert!(!r1.is_builtin);
    }

    #[test]
    fn list_active_excludes_disabled_and_orders_by_sort() {
        let conn = fresh_conn_with_lib("L1");
        upsert_llm_generated(&conn, "L1", "extra1", "额外1").unwrap();
        conn.execute(
            "UPDATE categories SET is_disabled=1 WHERE library_id='L1' AND slug='4-存档'",
            [],
        )
        .unwrap();
        let list = list_active_for_prompt(&conn, "L1").unwrap();
        // 4 个 builtin - 1 disabled + 1 新增 = 4 行
        assert_eq!(list.len(), 4);
        // builtin 在前（sort_order 10/20/30），llm_generated 在后（sort_order 100）
        assert_eq!(list[0].slug, "1-项目");
        assert_eq!(list.last().unwrap().slug, "extra1");
    }

    #[test]
    fn sanitize_slug_keeps_chinese_and_dashes() {
        assert_eq!(sanitize_slug("读书笔记"), "读书笔记");
        assert_eq!(sanitize_slug("1-项目"), "1-项目");
        assert_eq!(sanitize_slug("My_Cat-2"), "My_Cat-2");
    }

    #[test]
    fn sanitize_slug_replaces_illegal_chars() {
        assert_eq!(sanitize_slug("a/b c.d"), "a-b-c-d");
        // 多个非法字符塌缩成单个 dash
        assert_eq!(sanitize_slug("a ///b"), "a-b");
    }

    #[test]
    fn sanitize_slug_truncates_to_32_chars() {
        let long = "中".repeat(40);
        let out = sanitize_slug(&long);
        assert_eq!(out.chars().count(), 32);
    }

    #[test]
    fn sanitize_slug_empty_returns_other() {
        assert_eq!(sanitize_slug(""), "other");
        assert_eq!(sanitize_slug("///"), "other");
        assert_eq!(sanitize_slug("   "), "other");
    }
}
