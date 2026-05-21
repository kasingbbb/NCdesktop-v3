//! 启动期 source 扫描（task_007 / ADR-004 §六 内存态模型）。
//!
//! 应用启动时一次性遍历所有 root assets，stat 其 `file_path`，
//! 不存在的加入内存态 [`SourceMissingSet`]，仅用于 UI 红点 / "源文件缺失"提示；
//! **不改变四态本身**（参见 `db::asset::compute_asset_state` 中 `source_missing_known`
//! 显式被忽略）。
//!
//! 设计要点：
//! - 不引入 fsnotify（PRD 硬约束）；
//! - 不阻塞 setup hook（调用方用 `tauri::async_runtime::spawn` 包裹）；
//! - 不在 commands/ 拼 SQL：扫描走 `db::asset::list_root_assets`；
//! - 读多写少 → 用 `RwLock<HashSet>` 而非 `Mutex`；
//! - 启动期任何失败仅 warn（不让应用崩溃）。

use crate::db::{self, Database};
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;
use std::sync::RwLock;
use tauri::{AppHandle, Emitter, Manager};

/// 进程内的"源文件缺失"集合。
///
/// 在 setup 阶段经 `app.manage()` 注册一次，命令层通过 `State<SourceMissingSet>`
/// 读取。该类型原由 task_003 临时放在 `commands::asset`；task_007 起归位到本
/// 模块，并通过 `pub use crate::source_scan::SourceMissingSet;` 维持兼容。
#[derive(Debug, Default)]
pub struct SourceMissingSet {
    inner: RwLock<HashSet<String>>,
}

impl SourceMissingSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn contains(&self, asset_id: &str) -> bool {
        self.inner
            .read()
            .map(|g| g.contains(asset_id))
            .unwrap_or(false)
    }

    pub fn insert(&self, asset_id: String) {
        if let Ok(mut g) = self.inner.write() {
            g.insert(asset_id);
        }
    }

    pub fn remove(&self, asset_id: &str) {
        if let Ok(mut g) = self.inner.write() {
            g.remove(asset_id);
        }
    }

    /// 快照当前缺失 id 列表（顺序不稳定）。供调试命令与集成测试使用。
    pub fn snapshot(&self) -> Vec<String> {
        self.inner
            .read()
            .map(|g| g.iter().cloned().collect())
            .unwrap_or_default()
    }
}

/// emit 事件 payload（前端 task_008 据此 invalidate workspace list）。
#[derive(Debug, Clone, Serialize)]
pub struct SourceScanFinishedPayload {
    pub scanned: usize,
    pub missing: usize,
}

/// 纯函数版扫描：单个 project 的 root assets 走 stat，缺失的写入 missing_set。
///
/// 抽出独立函数便于单测（避免依赖 Tauri AppHandle 与真实 Database 锁）。
///
/// 返回 `(scanned, missing)`：本次扫描的 root 数 / 新增缺失数。
pub fn scan_with_conn(
    conn: &rusqlite::Connection,
    missing_set: &SourceMissingSet,
    project_id: &str,
) -> Result<(usize, usize), String> {
    let rows = db::asset::list_root_assets(conn, project_id)?;
    let scanned = rows.len();
    let mut missing = 0usize;
    for (asset, _join) in rows {
        if !Path::new(&asset.file_path).exists() {
            missing += 1;
            missing_set.insert(asset.id);
        }
    }
    Ok((scanned, missing))
}

/// 全库扫描入口：遍历 library → project → root assets，调 [`scan_with_conn`]。
///
/// 完成后 emit `notecapt/source-scan-finished` 携带 `{ scanned, missing }`。
/// 失败仅 warn，不让 setup 崩溃。返回总 missing 数（启动期日志用）。
pub fn scan_all_projects(app: &AppHandle) -> Result<usize, String> {
    let database = app.state::<Database>();
    let conn = database
        .conn
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;

    let missing_set = app.state::<SourceMissingSet>();

    let mut total_scanned = 0usize;
    let mut total_missing = 0usize;

    let libraries = match db::library::get_all(&conn) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("[source_scan] 列举 libraries 失败: {e}");
            return Err(e);
        }
    };

    for lib in libraries {
        let projects = match db::project::get_by_library(&conn, &lib.id) {
            Ok(v) => v,
            Err(e) => {
                log::warn!("[source_scan] library={} 列举 projects 失败: {e}", lib.id);
                continue;
            }
        };
        for proj in projects {
            match scan_with_conn(&conn, &missing_set, &proj.id) {
                Ok((scanned, missing)) => {
                    total_scanned += scanned;
                    total_missing += missing;
                }
                Err(e) => {
                    log::warn!(
                        "[source_scan] project={} 扫描失败（跳过）: {e}",
                        proj.id
                    );
                }
            }
        }
    }

    // 释放 DB 锁后再 emit，避免阻塞其他命令。
    drop(conn);

    let payload = SourceScanFinishedPayload {
        scanned: total_scanned,
        missing: total_missing,
    };
    if let Err(e) = app.emit("notecapt/source-scan-finished", payload) {
        log::warn!("[source_scan] emit 失败: {e}");
    }

    log::info!(
        "[source_scan] 扫描完成 scanned={} missing={}",
        total_scanned,
        total_missing
    );
    Ok(total_missing)
}

/// Debug 专用命令：返回当前 missing set 的快照，便于 task_009 集成测试断言。
#[cfg(debug_assertions)]
#[tauri::command]
pub fn source_scan_get_missing(
    missing: tauri::State<'_, SourceMissingSet>,
) -> Vec<String> {
    missing.snapshot()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migration::run_migrations;
    use crate::models::Asset;
    use rusqlite::{params, Connection};
    use std::fs;

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("打开内存库失败");
        run_migrations(&conn).expect("迁移失败");
        conn
    }

    fn insert_project(conn: &Connection, id: &str) {
        conn.execute(
            "INSERT OR IGNORE INTO libraries (id, name, root_path) VALUES (?1, ?2, ?3)",
            params!["lib_test", "test_lib", "/tmp/test_lib"],
        )
        .expect("插入 library 失败");
        conn.execute(
            "INSERT INTO projects (id, library_id, name) VALUES (?1, ?2, ?3)",
            params![id, "lib_test", "test_proj"],
        )
        .expect("插入 project 失败");
    }

    fn mk_root(id: &str, project_id: &str, file_path: &str) -> Asset {
        Asset {
            id: id.to_string(),
            project_id: project_id.to_string(),
            asset_type: "pdf".to_string(),
            name: format!("{id}.pdf"),
            original_name: format!("{id}.pdf"),
            file_path: file_path.to_string(),
            file_size: 100,
            mime_type: "application/pdf".to_string(),
            captured_at: "2025-01-01T00:00:00Z".to_string(),
            imported_at: "2025-01-01T00:00:00Z".to_string(),
            source_type: "import".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn detects_missing_file() {
        let conn = setup_conn();
        insert_project(&conn, "p1");

        let dir = tempfile::tempdir().expect("tempdir");
        let present_path = dir.path().join("present.pdf");
        let missing_path = dir.path().join("missing.pdf");
        fs::write(&present_path, b"hello").expect("写入存在文件");
        // missing_path 不创建（或写后立即删除均可）；这里直接不创建。

        let present = mk_root("a_present", "p1", present_path.to_str().unwrap());
        let missing = mk_root("a_missing", "p1", missing_path.to_str().unwrap());
        db::asset::insert(&conn, &present).unwrap();
        db::asset::insert(&conn, &missing).unwrap();

        let set = SourceMissingSet::new();
        let (scanned, missing_count) =
            scan_with_conn(&conn, &set, "p1").expect("scan 失败");

        assert_eq!(scanned, 2, "应扫描到 2 个 root");
        assert_eq!(missing_count, 1, "只有 1 个 root 文件缺失");
        assert!(set.contains("a_missing"), "缺失 root 应被记录");
        assert!(!set.contains("a_present"), "存在 root 不应被标记");
        assert_eq!(set.snapshot().len(), 1);
    }

    #[test]
    fn empty_project_is_noop() {
        let conn = setup_conn();
        insert_project(&conn, "p_empty");
        let set = SourceMissingSet::new();
        let (scanned, missing) =
            scan_with_conn(&conn, &set, "p_empty").expect("scan 失败");
        assert_eq!(scanned, 0);
        assert_eq!(missing, 0);
        assert!(set.snapshot().is_empty());
    }

    #[test]
    fn set_basic_ops() {
        let s = SourceMissingSet::new();
        assert!(!s.contains("x"));
        s.insert("x".to_string());
        assert!(s.contains("x"));
        s.remove("x");
        assert!(!s.contains("x"));
    }
}
