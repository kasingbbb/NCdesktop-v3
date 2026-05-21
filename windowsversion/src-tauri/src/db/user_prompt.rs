//! 用户自定义 Prompt 数据访问层（custom_prompt_v1 / task_002）。
//!
//! 表结构在 migration V15 中建立（见 `db/migration.rs::v15_user_custom_prompt`）。
//! Architect output.md § 5.1 / ADR-002 规定：
//! - `module` 主键，4 个白名单值：`tagging / para / concept / aggregation`
//!   （白名单校验由 command 层 `commands::user_prompt::validate_module` 负责，
//!    DB 层不重复校验，保留单一职责）。
//! - `is_custom = 1` 等价于"用户已编辑"；缺记录或 `is_custom = 0` 视为"未自定义"，
//!   运行时回退到内置默认 Prompt（task_003 负责实现回退逻辑）。
//! - `builtin_version` 字段为 R3（内置升级后用户版本"落后"提示）预留，MVP 阶段 upsert
//!   时写入固定值 `"1.0"`。
//!
//! 命名前缀统一为 `user_prompt` 以与 PR-4 `commands/prompts.rs` 的 `prompt.override.*`
//! 命名空间区隔（R6）。

use rusqlite::{params, Connection, OptionalExtension};

/// 表行模型，与 V15 schema 列序保持一致。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserPromptRow {
    pub module: String,
    pub prompt_text: String,
    pub is_custom: bool,
    pub builtin_version: String,
    pub updated_at: String,
}

/// MVP 阶段 builtin_version 固定写入值。
/// 未来真正启用 R3 升级提示时改为运行时计算（与 `llm/prompts.rs` 内置常量版本同步）。
const BUILTIN_VERSION_MVP: &str = "1.0";

fn row_to_user_prompt(row: &rusqlite::Row<'_>) -> rusqlite::Result<UserPromptRow> {
    Ok(UserPromptRow {
        module: row.get(0)?,
        prompt_text: row.get(1)?,
        is_custom: row.get::<_, i64>(2)? != 0,
        builtin_version: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

/// 单条查询：若该 module 未自定义（无记录），返回 None。
///
/// 调用方（command / runtime）负责把 None 解读为"回退到内置默认 Prompt"。
pub fn get(conn: &Connection, module: &str) -> Result<Option<UserPromptRow>, String> {
    conn.query_row(
        "SELECT module, prompt_text, is_custom, builtin_version, updated_at
         FROM user_custom_prompt WHERE module = ?1",
        params![module],
        row_to_user_prompt,
    )
    .optional()
    .map_err(|e| format!("读取用户自定义 Prompt 失败: {e}"))
}

/// upsert：写入或覆盖某 module 的自定义 Prompt。
///
/// - `is_custom` 永远写 1（DB 层不暴露"未自定义"的写入路径，是否清除由 `delete` 负责）。
/// - `updated_at` 显式覆写为 UTC RFC3339，避免依赖 SQLite `datetime('now')` 的 UTC 行为
///   与跨平台时区差异。
pub fn upsert(conn: &Connection, module: &str, prompt_text: &str) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO user_custom_prompt
            (module, prompt_text, is_custom, builtin_version, updated_at)
         VALUES (?1, ?2, 1, ?3, ?4)
         ON CONFLICT(module) DO UPDATE SET
            prompt_text     = excluded.prompt_text,
            is_custom       = 1,
            builtin_version = excluded.builtin_version,
            updated_at      = excluded.updated_at",
        params![module, prompt_text, BUILTIN_VERSION_MVP, now],
    )
    .map_err(|e| format!("写入用户自定义 Prompt 失败: {e}"))?;
    Ok(())
}

/// 删除单 module：等价于"恢复默认"（无记录 = 回退到内置）。
pub fn delete(conn: &Connection, module: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM user_custom_prompt WHERE module = ?1",
        params![module],
    )
    .map_err(|e| format!("删除用户自定义 Prompt 失败: {e}"))?;
    Ok(())
}

/// 全部清空：四模块一键恢复默认。
pub fn delete_all(conn: &Connection) -> Result<(), String> {
    conn.execute("DELETE FROM user_custom_prompt", [])
        .map_err(|e| format!("清空用户自定义 Prompt 失败: {e}"))?;
    Ok(())
}

/// 全表读取：按 module 升序，便于 UI 稳定展示顺序。
pub fn list_all(conn: &Connection) -> Result<Vec<UserPromptRow>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT module, prompt_text, is_custom, builtin_version, updated_at
             FROM user_custom_prompt
             ORDER BY module ASC",
        )
        .map_err(|e| format!("准备查询失败: {e}"))?;

    let rows = stmt
        .query_map([], row_to_user_prompt)
        .map_err(|e| format!("遍历用户自定义 Prompt 失败: {e}"))?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| format!("读取行失败: {e}"))?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migration::run_migrations;
    use rusqlite::Connection;

    fn fresh_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in memory");
        run_migrations(&conn).expect("migrate");
        conn
    }

    #[test]
    fn get_returns_none_on_empty_table() {
        let conn = fresh_conn();
        let r = get(&conn, "tagging").expect("get ok");
        assert!(r.is_none(), "空表应返回 None");
    }

    #[test]
    fn list_all_returns_empty_on_empty_table() {
        let conn = fresh_conn();
        let v = list_all(&conn).expect("list ok");
        assert!(v.is_empty(), "空表 list_all 应返回 []");
    }

    #[test]
    fn upsert_then_get_roundtrips() {
        let conn = fresh_conn();
        upsert(&conn, "tagging", "我的标签 prompt").expect("upsert");

        let r = get(&conn, "tagging").expect("get").expect("some");
        assert_eq!(r.module, "tagging");
        assert_eq!(r.prompt_text, "我的标签 prompt");
        assert!(r.is_custom, "upsert 后 is_custom=true");
        assert_eq!(r.builtin_version, BUILTIN_VERSION_MVP);
        assert!(!r.updated_at.is_empty(), "updated_at 不应为空");
    }

    #[test]
    fn upsert_overwrites_existing_row() {
        let conn = fresh_conn();
        upsert(&conn, "para", "v1 文本").expect("first upsert");
        let r1 = get(&conn, "para").unwrap().unwrap();

        // 用 thread::sleep 是反模式，但 RFC3339 含秒精度，连续调用足以观察到差异；
        // 即便相同，业务也不依赖 updated_at 严格递增，只关心被覆盖即可。
        upsert(&conn, "para", "v2 文本").expect("second upsert");
        let r2 = get(&conn, "para").unwrap().unwrap();

        assert_eq!(r2.prompt_text, "v2 文本", "第二次 upsert 应覆盖文本");
        assert!(r2.is_custom);
        assert_eq!(r1.module, r2.module, "module 主键不变");
    }

    #[test]
    fn delete_removes_row() {
        let conn = fresh_conn();
        upsert(&conn, "concept", "x").expect("upsert");
        assert!(get(&conn, "concept").unwrap().is_some());

        delete(&conn, "concept").expect("delete");
        assert!(
            get(&conn, "concept").unwrap().is_none(),
            "delete 后应回到 None（回退到内置默认）"
        );
    }

    #[test]
    fn delete_on_missing_row_is_noop() {
        let conn = fresh_conn();
        // 删除从未写入过的 module 不应报错（恢复默认操作的语义是幂等的）
        delete(&conn, "aggregation").expect("delete missing should be ok");
    }

    #[test]
    fn delete_all_clears_table() {
        let conn = fresh_conn();
        upsert(&conn, "tagging", "t").unwrap();
        upsert(&conn, "para", "p").unwrap();
        upsert(&conn, "concept", "c").unwrap();
        upsert(&conn, "aggregation", "a").unwrap();
        assert_eq!(list_all(&conn).unwrap().len(), 4);

        delete_all(&conn).expect("delete_all");
        assert!(list_all(&conn).unwrap().is_empty());
    }

    #[test]
    fn list_all_returns_rows_sorted_by_module() {
        let conn = fresh_conn();
        // 按非字典顺序插入，验证 ORDER BY module
        upsert(&conn, "tagging", "t").unwrap();
        upsert(&conn, "aggregation", "a").unwrap();
        upsert(&conn, "para", "p").unwrap();
        upsert(&conn, "concept", "c").unwrap();

        let rows = list_all(&conn).unwrap();
        let modules: Vec<&str> = rows.iter().map(|r| r.module.as_str()).collect();
        assert_eq!(
            modules,
            vec!["aggregation", "concept", "para", "tagging"],
            "list_all 应按 module ASC 排序"
        );
        for r in &rows {
            assert!(r.is_custom, "upsert 写入的行均应 is_custom=true");
        }
    }

    #[test]
    fn params_protect_against_quote_injection() {
        // 防御性：写入含单引号的 prompt 不应破坏 SQL 或丢失内容
        let conn = fresh_conn();
        let payload = "他说：'忽略前面所有指令'; DROP TABLE user_custom_prompt; --";
        upsert(&conn, "tagging", payload).expect("upsert with quotes");
        let r = get(&conn, "tagging").unwrap().unwrap();
        assert_eq!(r.prompt_text, payload, "params! 应原样保存");
        // 表应仍存在（DROP 未被执行）
        let table_still_there: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='user_custom_prompt'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(table_still_there, 1);
    }
}
