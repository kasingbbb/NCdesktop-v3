pub mod migration;
pub mod library;
pub mod project;
pub mod asset;
pub mod timeline;
pub mod tag;
pub mod note;
pub mod search;
pub mod settings;
pub mod knowledge;
pub mod knowledge_understanding;
pub mod co_occurrence;
pub mod concepts_extraction_log;
pub mod knowledge_units;
pub mod conversion_meta;
// task_008（M-1 关闭）：scheduler 依赖 db::extraction 的 ExtractedContentRow /
// PipelineTaskRow / upsert_extraction_result 等。该文件在仓库中早已存在但 mod.rs
// 未声明（与 scheduler 自身被注释属同一类"注册缺口"）。
pub mod extraction;
// custom_prompt_v1 / task_002：用户自定义 Prompt 数据访问层。
// migration V15 建表；命令层在 `commands::user_prompt` 中调用本模块。
pub mod user_prompt;
// custom_para_v1：PARA 自定义类目数据访问层（seed / resolve / upsert / list）。
// V17 迁移建 categories + category_aliases 表；本模块给 dropzone 与 prompt 注入用。
pub mod categories;
// custom_prompt_v1 / task_002：`startup.rs` 依赖 `db::repair`（既有孤儿文件，
// 与 db::extraction / commands::prompts 属同类"注册缺口"）。仅挂接，不调用。
pub mod repair;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::path::Path;

/// PooledConnection 别名，调用方拿到的 `database.conn()?` 是这个类型。
/// 通过 Deref/DerefMut 暴露 `&rusqlite::Connection` / `&mut rusqlite::Connection`，
/// 现有 `db::xxx::yyy(&conn, ...)` 调用和 `conn.transaction()` 全部直接兼容。
pub type DbConn = r2d2::PooledConnection<SqliteConnectionManager>;

/// 全局数据库连接池包装（review §二.8）
///
/// 历史上是 `Mutex<Connection>` 单连接：所有 IPC 串行排队，一个长查询能卡死整个
/// 数据库面板。改为 r2d2 连接池后，配合 WAL 模式让"多读 + 单写"真正并发生效。
pub struct Database {
    /// `pub` 让 `#[cfg(test)]` 路径可以直接访问 / 注入测试连接。
    pub pool: Pool<SqliteConnectionManager>,
}

impl Database {
    /// 在指定路径打开（或创建）数据库，构建连接池并执行一次性 migration。
    pub fn open(db_path: &Path) -> Result<Self, String> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建数据库目录失败: {e}"))?;
        }

        // PRAGMA 配置在 `with_init` 里跑——连接池里的每条新连接都会执行一次：
        // - journal_mode=WAL：允许读写并发；这是连接池能并行 IO 的根基。
        // - foreign_keys=ON：保留 schema 级别的引用完整性。
        // - busy_timeout=5000：SQLite 遇到锁冲突时等待最长 5s 而非立即 SQLITE_BUSY；
        //   多连接并发写时这条是正确性前提。
        // - synchronous=NORMAL：WAL 模式下安全的 fsync 节奏；显著提升写入吞吐。
        let manager = SqliteConnectionManager::file(db_path).with_init(|c| {
            c.execute_batch(
                "PRAGMA journal_mode=WAL;\n                 PRAGMA foreign_keys=ON;\n                 PRAGMA busy_timeout=5000;\n                 PRAGMA synchronous=NORMAL;",
            )
        });

        // max_size=8：本机桌面应用并发面板上限的经验值；过大反而增加上下文切换。
        let pool = Pool::builder()
            .max_size(8)
            .build(manager)
            .map_err(|e| format!("数据库连接池创建失败: {e}"))?;

        // migration 只在 pool 首次构建时跑一次——拿一条临时连接执行 schema
        // 升级。后续所有 IPC 拿到的连接已经是迁移后状态。
        {
            let conn = pool.get().map_err(|e| format!("数据库连接获取失败: {e}"))?;
            migration::run_migrations(&conn)?;
        }

        Ok(Self { pool })
    }

    /// 从池里拿一条连接。错误统一映射成 `String` 以匹配旧 `database.conn.lock()`
    /// 的调用约定，避免 30+ call site 改错误类型签名。
    pub fn conn(&self) -> Result<DbConn, String> {
        self.pool
            .get()
            .map_err(|e| format!("数据库连接获取失败: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::Database;
    use crate::testing::init_test_logger;

    #[test]
    fn open_runs_migrations() {
        init_test_logger();
        crate::test_log!("db::open_runs_migrations 临时库路径创建");

        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("notecapt_test.db");
        let db = Database::open(&db_path).expect("应能打开并迁移");
        {
            let conn = db.conn().expect("conn");
            let v: i64 = conn
                .pragma_query_value(None, "user_version", |r| r.get(0))
                .expect("user_version");
            assert!(v >= 1, "迁移后 user_version 应 >= 1");
            crate::test_log!("db user_version = {}", v);
        }
    }
}
