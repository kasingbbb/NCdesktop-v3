//! 数据库启动期自愈
//!
//! 历史代码在 `commands/dropzone.rs` 把 LLM 返回的裸 category 字符串直接写入
//! `ai_analyses.topics`（列契约本应是 JSON 数组字符串）。本模块提供：
//! - `parse_topics_or_empty`：读时解析容错（解析失败回 `[]`）
//! - `run_post_migration_repair`：启动期扫描 + 修正全表
//!
//! 异步 spawn 与 `get_repair_progress` Tauri 命令在 `startup::bootstrap` 中包装（task_004）。

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// 修复模式（由 `startup::bootstrap` 根据 AppMode 选择）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepairMode {
    /// 严格模式：任一行修复失败立即报错
    Strict,
    /// 宽松模式：失败行计数继续；下游应触发 AppMode::Degraded
    Lenient,
    /// 只读模式：仅扫描不写盘
    ReadOnly,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepairReport {
    pub scanned: u64,
    pub repaired: u64,
    pub failed: u64,
    pub dur_ms: u64,
    pub mode: Option<String>,
}

/// 进度快照（task_004 包装为 Tauri State）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepairProgress {
    pub running: bool,
    pub report: RepairReport,
}

/// 容错解析：JSON 数组优先，失败则把整个字符串视为单 topic 包装；空白返回 `[]`
pub fn parse_topics_or_empty(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if let Ok(serde_json::Value::Array(arr)) = serde_json::from_str::<serde_json::Value>(trimmed) {
        return arr
            .into_iter()
            .filter_map(|v| match v {
                serde_json::Value::String(s) => Some(s),
                other => Some(other.to_string()),
            })
            .collect();
    }
    vec![trimmed.to_string()]
}

/// 判断字符串是否已是合法 JSON 数组形态（不必修复）
fn is_json_array(raw: &str) -> bool {
    let t = raw.trim();
    if !t.starts_with('[') {
        return false;
    }
    matches!(
        serde_json::from_str::<serde_json::Value>(t),
        Ok(serde_json::Value::Array(_))
    )
}

/// 启动期自愈扫描：把 `ai_analyses.topics` 中非 JSON 数组形态的值包装为 `["原值"]`
pub fn run_post_migration_repair(
    conn: &Connection,
    mode: RepairMode,
) -> Result<RepairReport, String> {
    let started = Instant::now();
    let mut report = RepairReport {
        mode: Some(format!("{:?}", mode)),
        ..Default::default()
    };

    let rows: Vec<(String, String)> = {
        let mut stmt = conn
            .prepare("SELECT id, topics FROM ai_analyses;")
            .map_err(|e| format!("repair 扫描失败: {e}"))?;
        let collected: Vec<(String, String)> = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(|e| format!("repair 取行失败: {e}"))?
            .filter_map(|r| r.ok())
            .collect();
        collected
    };

    for (id, topics) in rows {
        report.scanned += 1;
        if is_json_array(&topics) {
            continue; // 已是合法 JSON 数组，跳过
        }
        if mode == RepairMode::ReadOnly {
            // 只读模式仅计数，不写
            report.repaired += 1;
            continue;
        }
        let wrapped = serde_json::to_string(&vec![topics.trim().to_string()])
            .unwrap_or_else(|_| "[]".to_string());
        match conn.execute(
            "UPDATE ai_analyses SET topics = ?1 WHERE id = ?2;",
            rusqlite::params![wrapped, id],
        ) {
            Ok(_) => report.repaired += 1,
            Err(e) => {
                report.failed += 1;
                if mode == RepairMode::Strict {
                    return Err(format!("Strict 模式遇到失败行 {id}: {e}"));
                }
                log::warn!("repair 行 {id} 失败（Lenient 跳过）: {e}");
            }
        }
    }

    report.dur_ms = started.elapsed().as_millis() as u64;
    log::info!(
        "post_migration_repair 完成: scanned={} repaired={} failed={} dur_ms={} mode={:?}",
        report.scanned,
        report.repaired,
        report.failed,
        report.dur_ms,
        mode
    );
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn fresh_db_with_v10() -> Connection {
        let conn = Connection::open_in_memory().expect("memdb");
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        crate::db::migration::run_migrations(&conn).expect("V1..V10");
        // 准备依赖：library / project / asset / ai_analyses
        conn.execute_batch(
            "INSERT INTO libraries (id, name, root_path) VALUES ('lib', 'L', '/tmp/L');
             INSERT INTO projects (id, library_id, name) VALUES ('p1', 'lib', 'P');
             INSERT INTO assets (id, project_id, asset_type, name, file_path)
                 VALUES ('a1','p1','pdf','one.pdf','/tmp/one.pdf'),
                        ('a2','p1','pdf','two.pdf','/tmp/two.pdf'),
                        ('a3','p1','pdf','three.pdf','/tmp/three.pdf');
             INSERT INTO ai_analyses (id, asset_id, summary, topics, language, suggested_tags, suggested_name) VALUES
               ('ai1','a1','','1-项目','zh','[]',''),
               ('ai2','a2','','[\"2-领域\",\"Q3\"]','zh','[]',''),
               ('ai3','a3','','','zh','[]','');",
        )
        .unwrap();
        conn
    }

    #[test]
    fn parse_topics_handles_json_bare_and_empty() {
        assert_eq!(parse_topics_or_empty(""), Vec::<String>::new());
        assert_eq!(parse_topics_or_empty("   "), Vec::<String>::new());
        assert_eq!(parse_topics_or_empty(r#"["a","b"]"#), vec!["a", "b"]);
        assert_eq!(parse_topics_or_empty("1-项目"), vec!["1-项目"]);
    }

    #[test]
    fn repair_lenient_wraps_bare_strings() {
        let conn = fresh_db_with_v10();
        let report = run_post_migration_repair(&conn, RepairMode::Lenient).expect("ok");
        assert_eq!(report.scanned, 3);
        // ai1 裸 + ai3 空 都需要包装；ai2 已是 JSON 数组跳过
        assert_eq!(report.repaired, 2);
        assert_eq!(report.failed, 0);

        let ai1_topics: String = conn
            .query_row("SELECT topics FROM ai_analyses WHERE id='ai1';", [], |r| r.get(0))
            .unwrap();
        assert!(ai1_topics.starts_with('['), "ai1 应被包装为 JSON 数组，实际 {}", ai1_topics);
        let parsed = parse_topics_or_empty(&ai1_topics);
        assert_eq!(parsed, vec!["1-项目"]);

        let ai2_topics: String = conn
            .query_row("SELECT topics FROM ai_analyses WHERE id='ai2';", [], |r| r.get(0))
            .unwrap();
        assert_eq!(parse_topics_or_empty(&ai2_topics), vec!["2-领域", "Q3"]);

        let ai3_topics: String = conn
            .query_row("SELECT topics FROM ai_analyses WHERE id='ai3';", [], |r| r.get(0))
            .unwrap();
        assert_eq!(parse_topics_or_empty(&ai3_topics), vec![""]);
    }

    #[test]
    fn repair_readonly_does_not_write() {
        let conn = fresh_db_with_v10();
        let before: String = conn
            .query_row("SELECT topics FROM ai_analyses WHERE id='ai1';", [], |r| r.get(0))
            .unwrap();
        let report = run_post_migration_repair(&conn, RepairMode::ReadOnly).expect("ok");
        let after: String = conn
            .query_row("SELECT topics FROM ai_analyses WHERE id='ai1';", [], |r| r.get(0))
            .unwrap();
        assert_eq!(before, after, "ReadOnly 模式不应写入");
        assert_eq!(report.repaired, 2, "扫描计数仍报告会修复几条");
    }

    #[test]
    fn repair_idempotent() {
        let conn = fresh_db_with_v10();
        run_post_migration_repair(&conn, RepairMode::Lenient).expect("first");
        let r2 = run_post_migration_repair(&conn, RepairMode::Lenient).expect("second");
        assert_eq!(r2.repaired, 0, "二次跑应零修复（已全部 JSON 数组）");
    }
}
