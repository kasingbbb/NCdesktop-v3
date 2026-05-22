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

use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

/// 全局数据库连接包装（线程安全）
pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    /// 在指定路径打开（或创建）数据库，执行初始化迁移
    pub fn open(db_path: &Path) -> Result<Self, String> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建数据库目录失败: {e}"))?;
        }

        let conn = Connection::open(db_path)
            .map_err(|e| format!("打开数据库失败: {e}"))?;

        // PRAGMA 配置：
        // - journal_mode=WAL：允许读写并发；前置依赖（虽然当前是单连接 Mutex 模型暂未受益）。
        // - foreign_keys=ON：保留 schema 级别的引用完整性。
        // - busy_timeout=5000：SQLite 遇到锁冲突时等待最长 5s 而非立即 SQLITE_BUSY。
        //   单 `Mutex<Connection>` 模型下原本不会触发 SQLITE_BUSY，但未来切池后这条
        //   是并发写正确的前提；现在加上零代价。
        // - synchronous=NORMAL：WAL 模式下安全的 fsync 节奏；显著提升写入吞吐。
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;\n             PRAGMA foreign_keys=ON;\n             PRAGMA busy_timeout=5000;\n             PRAGMA synchronous=NORMAL;",
        )
        .map_err(|e| format!("数据库 PRAGMA 设置失败: {e}"))?;

        migration::run_migrations(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
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
            let conn = db.conn.lock().expect("锁");
            let v: i64 = conn
                .pragma_query_value(None, "user_version", |r| r.get(0))
                .expect("user_version");
            assert!(v >= 1, "迁移后 user_version 应 >= 1");
            crate::test_log!("db user_version = {}", v);
        }
    }
}
