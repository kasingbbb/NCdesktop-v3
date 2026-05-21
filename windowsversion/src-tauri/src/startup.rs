//! 应用启动序列：迁移 → 自愈 → 决定 AppMode
//!
//! 三档降级（PRD §6.4 / ADR-006）：
//! - `Normal`：迁移 + 自愈全部成功
//! - `Degraded`：迁移成功但 repair 有失败行 → 横幅提示，写入仍可用
//! - `ReadOnly`：迁移失败或 DB 不可写 → 写命令短路返回错误

use crate::db::repair::{run_post_migration_repair, RepairMode, RepairProgress, RepairReport};
use crate::db::Database;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AppMode {
    Normal,
    Degraded { reason: String, failed_count: u64 },
    ReadOnly { reason: String },
}

impl AppMode {
    pub fn is_writable(&self) -> bool {
        !matches!(self, AppMode::ReadOnly { .. })
    }
}

/// 启动产物
pub struct BootstrapResult {
    pub database: Database,
    pub mode: AppMode,
    pub progress: Arc<Mutex<RepairProgress>>,
}

/// 同步 bootstrap：在 Tauri setup 中阻塞调用一次
pub fn bootstrap(db_path: &Path) -> BootstrapResult {
    let progress = Arc::new(Mutex::new(RepairProgress::default()));

    // Step 1：迁移（含 V10）
    let database = match Database::open(db_path) {
        Ok(db) => db,
        Err(e) => {
            log::error!("启动迁移失败，进入 ReadOnly 安全模式: {e}");
            // 退路：再次以原生模式打开，保证 query-only 命令可用
            let conn = match rusqlite::Connection::open(db_path) {
                Ok(c) => c,
                Err(e2) => {
                    // 最坏情形：连只读 conn 都打不开 → 用 in-memory 占位（命令将拒写）
                    log::error!("DB 文件无法打开: {e2}，使用内存占位");
                    rusqlite::Connection::open_in_memory().expect("内存 conn")
                }
            };
            let database = Database {
                conn: std::sync::Mutex::new(conn),
            };
            return BootstrapResult {
                database,
                mode: AppMode::ReadOnly { reason: e },
                progress,
            };
        }
    };

    // Step 2：repair 选择模式（首启 Lenient，便于把残留写入归类）
    {
        let mut prog_guard = progress.lock().unwrap();
        prog_guard.running = true;
    }
    let report: RepairReport = run_repair_locked(&database).unwrap_or_else(|e| {
        log::error!("启动 repair 异常: {e}");
        RepairReport {
            failed: 1,
            ..Default::default()
        }
    });

    {
        let mut prog_guard = progress.lock().unwrap();
        prog_guard.running = false;
        prog_guard.report = report.clone();
    }

    // Step 3：根据 report.failed 推导 AppMode
    let mode = if report.failed > 0 {
        AppMode::Degraded {
            reason: "部分 ai_analyses.topics 字段无法自愈".into(),
            failed_count: report.failed,
        }
    } else {
        AppMode::Normal
    };

    // task_003 T1：工作区启动 hook（失败仅 log，不阻塞）
    // 1) NFC 自愈：rename NFD 命名目录到 NFC，避免"列表存在但 select 不到"鬼影
    // 2) cleanup_pending 扫描：清理 EXDEV 跨卷迁移中断遗留的 `.cleanup_pending` 标记与孤立 tmp
    workspace_startup_hooks();

    log::info!("启动完成 mode={:?} repair={:?}", mode, report);
    BootstrapResult {
        database,
        mode,
        progress,
    }
}

/// 工作区启动期串行扫描（非阻塞失败：仅 log）。
fn workspace_startup_hooks() {
    use std::panic::{catch_unwind, AssertUnwindSafe};
    // NFC 自愈
    if catch_unwind(AssertUnwindSafe(crate::utils::nfc::nfc_heal_workspace)).is_err() {
        log::warn!("nfc_heal_workspace panic（已捕获，不阻塞启动）");
    }
    // cleanup_pending 扫描
    if let Ok(root) = crate::workspace::workspace_root() {
        if catch_unwind(AssertUnwindSafe(|| {
            crate::utils::safe_rename::cleanup_pending_scan(&root)
        }))
        .is_err()
        {
            log::warn!("cleanup_pending_scan panic（已捕获，不阻塞启动）");
        }
    }
}

fn run_repair_locked(database: &Database) -> Result<RepairReport, String> {
    let conn = database
        .conn
        .lock()
        .map_err(|_| "数据库锁中毒".to_string())?;
    run_post_migration_repair(&conn, RepairMode::Lenient)
}

/// 写命令前置守卫：ReadOnly 模式短路返回中文错误
pub fn ensure_writable(mode: &AppMode) -> Result<(), String> {
    match mode {
        AppMode::ReadOnly { reason } => Err(format!("当前为只读安全模式（{}），无法执行写操作", reason)),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_fresh_db_normal() {
        let dir = tempfile::tempdir().expect("td");
        let db_path = dir.path().join("normal.db");
        let r = bootstrap(&db_path);
        assert!(matches!(r.mode, AppMode::Normal), "新库应 Normal，实际 {:?}", r.mode);
        assert!(r.mode.is_writable());
    }

    #[test]
    fn ensure_writable_blocks_readonly() {
        let m = AppMode::ReadOnly { reason: "test".into() };
        assert!(ensure_writable(&m).is_err());
        assert!(!m.is_writable());

        let n = AppMode::Normal;
        assert!(ensure_writable(&n).is_ok());

        let d = AppMode::Degraded { reason: "x".into(), failed_count: 1 };
        assert!(ensure_writable(&d).is_ok(), "Degraded 仍允许写");
    }

    #[test]
    fn bootstrap_unwritable_dir_falls_to_readonly() {
        // 指向不存在的深路径（且其父目录无法创建）
        let path = Path::new("/dev/null/cannot/create.db");
        let r = bootstrap(path);
        assert!(matches!(r.mode, AppMode::ReadOnly { .. }));
        assert!(!r.mode.is_writable());
    }
}
