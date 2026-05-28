use tauri::{command, AppHandle, Manager};
use rusqlite::{params, Connection};
use crate::db::Database;
use crate::db::extraction as db_ext;
use crate::extraction::scheduler::PipelineScheduler;

#[command]
pub async fn extract_asset(app: AppHandle, asset_id: String) -> Result<String, String> {
    let task_id = PipelineScheduler::enqueue(&app, &asset_id)?;
    let scheduler = app.state::<PipelineScheduler>();
    scheduler.start(app.clone());
    Ok(task_id)
}

#[command]
pub async fn extract_project_assets(app: AppHandle, project_id: String) -> Result<String, String> {
    let asset_ids: Vec<String> = {
        let db = app.state::<Database>();
        let conn = db.conn()?;
        #[allow(deprecated)] // 非工作区路径：批量入队扫全部 asset，包含 derivative 也无副作用
        let assets = crate::db::asset::get_by_project(&conn, &project_id)?;
        assets.into_iter().map(|a| a.id).collect()
    };

    let batch_id = PipelineScheduler::enqueue_batch(&app, &asset_ids)?;
    let scheduler = app.state::<PipelineScheduler>();
    scheduler.start(app.clone());
    Ok(batch_id)
}

#[command]
pub async fn get_extraction_status(app: AppHandle, asset_id: String) -> Result<Option<db_ext::ExtractedContentRow>, String> {
    let db = app.state::<Database>();
    let conn = db.conn()?;
    db_ext::get_extracted_content(&conn, &asset_id)
}

#[command]
pub async fn get_extracted_content(app: AppHandle, asset_id: String) -> Result<Option<db_ext::ExtractedContentRow>, String> {
    let db = app.state::<Database>();
    let conn = db.conn()?;
    db_ext::get_extracted_content(&conn, &asset_id)
}

#[command]
pub async fn retry_extraction(app: AppHandle, asset_id: String) -> Result<String, String> {
    {
        let db = app.state::<Database>();
        let conn = db.conn()?;
        db_ext::update_extraction_status(&conn, &asset_id, "pending", None)?;
    }
    extract_asset(app, asset_id).await
}

/// task_006 AC-1（M5）：对外的"失败重试"命令薄包装。
///
/// 工作区右键 / 失败态视图的"重试转换"按钮唯一入口。内部直接转发
/// [`retrigger_extraction`]，避免重复实现 reset + enqueue 逻辑；存在的意义
/// 是把命令名与"asset 视角的重试"语义对齐（`retrigger_extraction` 偏抽取流水线
/// 视角），并让 UI 在调用前不需要知道底层管线分层。
///
/// 幂等性来自三道护栏：
/// 1. `retrigger_extraction` 内部"already running/queued → noop"检查；
/// 2. `PipelineScheduler::enqueue` 对 `UNIQUE constraint` 冲突静默返回；
/// 3. V7 部分唯一索引 `idx_pipeline_tasks_active_unique`（asset_id + task_type
///    在 queued/running 态唯一）作为最终兜底。
#[command]
pub async fn retry_asset_conversion(app: AppHandle, asset_id: String) -> Result<(), String> {
    // task_026：retry_asset_conversion 不暴露 force_kc_refresh —— 这是"失败重试"
    // 语义（重跑 markitdown），与"强制重 enrich KC"语义不同；保持 None
    // 即默认 false，行为与 task_011/006 完全一致。
    retrigger_extraction(app, asset_id, None).await
}

#[command]
pub async fn get_pipeline_progress(app: AppHandle) -> Result<db_ext::PipelineStats, String> {
    let db = app.state::<Database>();
    let conn = db.conn()?;
    db_ext::get_pipeline_stats(&conn)
}

/// task_011 AC-1 / task_026 AC-1：重新触发指定素材的抽取（统一的"重试"入口）。
///
/// 流程：
/// 1. 校验 asset 存在；
/// 2. 查 `extracted_content.status`，若已是 `queued` / `extracting` 则视为正在运行，
///    安全返回 `Ok(())`（幂等，避免重复入队 —— AC-4 第三场景）；
/// 3. 在同一把锁内调用纯函数 `reset_extraction_state` 重置 `extracted_content`
///    与 `pipeline_tasks` 至 `queued`；
/// 4. （task_026 AC-2）若 `force_kc_refresh=true`，在同一把锁内调用纯函数
///    `clear_kc_enriched` 把 `extracted_content.kc_enriched` 置 NULL，让
///    task_012 注入的 enrichment 在重新走到 `save_and_materialize` 时
///    重新跑 KC（NULL → re-enrich，"true" → skip）；
/// 5. 释放锁后通过 `PipelineScheduler::enqueue` 入队（其内部对 `UNIQUE constraint`
///    冲突返回 `already_queued`，再次防重复）；
/// 6. 唤醒调度循环（`scheduler.start(app)` 幂等）。
///
/// **硬约束**：
/// - 绝不直接把 `status` 置为 `extracted` 跳过 pipeline，必须走完整
///   `queued → extracting → extracted` 路径。
/// - `force_kc_refresh` 默认 `false`（即旧调用方零改动 / Tauri 反序列化缺失字段
///   填 None → unwrap_or(false)），向后兼容 task_011 的全部既有调用方
///   （`retry_asset_conversion` / `stores/extractionStore.ts::retryExtraction`）。
/// - `force_kc_refresh=true` **只清 kc_enriched 一个字段**，不动 raw_text /
///   structured_md / quality_level —— 即不重跑 markitdown，只强制 KC 重 enrich。
#[command]
pub async fn retrigger_extraction(
    app: AppHandle,
    asset_id: String,
    force_kc_refresh: Option<bool>,
) -> Result<(), String> {
    let force = force_kc_refresh.unwrap_or(false);

    // ── 1 + 2 + 3 + 4：在一把锁内完成校验 + 幂等检查 + 重置 + 可选 force kc clear
    let proceed = {
        let db = app.state::<Database>();
        let conn = db.conn()?;

        // 1. 校验 asset 存在
        let _asset = crate::db::asset::get_by_id(&conn, &asset_id)?
            .ok_or_else(|| format!("素材不存在: {asset_id}"))?;

        // 2. 幂等检查：already running/queued → noop
        let current_status = db_ext::get_extracted_content(&conn, &asset_id)?
            .map(|r| r.status);
        if matches!(current_status.as_deref(), Some("queued") | Some("extracting")) {
            log::info!(
                "retrigger_extraction: {} 已处于 {} 状态，跳过重复入队",
                asset_id,
                current_status.as_deref().unwrap_or("?")
            );
            false
        } else {
            // 3. 重置 extracted_content + pipeline_tasks
            reset_extraction_state(&conn, &asset_id)?;

            // 4. task_026 AC-2：force_kc_refresh=true 时清 kc_enriched=NULL
            //    让 task_012 enrichment 在 save_and_materialize 时重新跑 KC。
            //    幂等：即便当前已为 NULL，UPDATE 0/1 行都 Ok（与 reset 同性质）。
            if force {
                clear_kc_enriched(&conn, &asset_id)?;
                log::info!(
                    "retrigger_extraction: {} force_kc_refresh=true，已清 kc_enriched=NULL",
                    asset_id
                );
            }

            true
        }
    };

    if !proceed {
        return Ok(());
    }

    // ── 5. 入队（enqueue 内部已防 UNIQUE 冲突）
    PipelineScheduler::enqueue(&app, &asset_id)?;

    // ── 6. 唤醒调度循环
    let scheduler = app.state::<PipelineScheduler>();
    scheduler.start(app.clone());

    Ok(())
}

/// task_011 AC-1：纯函数 —— 在已持锁的 `Connection` 上重置某 asset 的抽取状态。
///
/// 把 `extracted_content.status` 重置为 `queued`、清空 `error_message`；
/// 把 `pipeline_tasks` 中该 asset 的所有记录 `status` 置为 `queued`、`retry_count`
/// 置零、清空 `error_message`、清空 `completed_at` 与 `started_at`。
///
/// 调用方负责：
/// - 已经核对 asset 存在；
/// - 已经做过 already-running 幂等检查（本函数无条件重置）。
///
/// 单测覆盖该函数，避免在生产命令路径上构造 `AppHandle` / `State<Database>`。
/// task_026 AC-2：纯函数 —— 在已持锁的 `Connection` 上把
/// `extracted_content.kc_enriched` 置为 NULL。
///
/// 设计意图：与 `reset_extraction_state` 解耦 —— 后者重置整个抽取流水线
/// （status / pipeline_tasks），本函数**只**清 KC 增强标志。这样
/// `retrigger_extraction(force=false)` 的旧路径完全不被影响。
///
/// 幂等：当前已为 NULL 时 UPDATE 0 行也返回 Ok（与 reset 同性质）。
///
/// 调用方负责：
/// - 已经核对 asset 存在；
/// - 已经在 `force_kc_refresh=true` 的分支下；
/// - 已经做完 reset_extraction_state（顺序不强制，但语义上 reset → clear 更直白）。
pub fn clear_kc_enriched(conn: &Connection, asset_id: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE extracted_content
         SET kc_enriched = NULL, updated_at = datetime('now')
         WHERE asset_id = ?1",
        params![asset_id],
    )
    .map_err(|e| format!("清 kc_enriched 失败: {e}"))?;
    Ok(())
}

pub fn reset_extraction_state(conn: &Connection, asset_id: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE extracted_content
         SET status = 'queued', error_message = NULL, updated_at = datetime('now')
         WHERE asset_id = ?1",
        params![asset_id],
    )
    .map_err(|e| format!("重置 extracted_content 失败: {e}"))?;

    conn.execute(
        "UPDATE pipeline_tasks
         SET status = 'queued',
             retry_count = 0,
             error_message = NULL,
             started_at = NULL,
             completed_at = NULL
         WHERE asset_id = ?1",
        params![asset_id],
    )
    .map_err(|e| format!("重置 pipeline_tasks 失败: {e}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// 构造内存 DB + 两张测试表
    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().expect("打开内存数据库");
        conn.execute_batch(
            "CREATE TABLE extracted_content (
                id TEXT PRIMARY KEY,
                asset_id TEXT NOT NULL,
                status TEXT NOT NULL,
                error_message TEXT,
                retry_count INTEGER DEFAULT 0,
                raw_text TEXT,
                structured_md TEXT,
                quality_level INTEGER DEFAULT 0,
                extractor_type TEXT,
                segments_json TEXT,
                content_hash TEXT,
                kc_enriched TEXT,
                created_at TEXT,
                updated_at TEXT
             );
             CREATE TABLE pipeline_tasks (
                id TEXT PRIMARY KEY,
                asset_id TEXT NOT NULL,
                task_type TEXT NOT NULL,
                status TEXT NOT NULL,
                retry_count INTEGER DEFAULT 0,
                max_retries INTEGER DEFAULT 3,
                error_message TEXT,
                priority INTEGER DEFAULT 100,
                batch_id TEXT,
                created_at TEXT,
                started_at TEXT,
                completed_at TEXT
             );",
        )
        .unwrap();
        conn
    }

    fn insert_ec(conn: &Connection, asset_id: &str, status: &str, err: Option<&str>) {
        conn.execute(
            "INSERT INTO extracted_content (id, asset_id, status, error_message, retry_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 0, '2026-01-01', '2026-01-01')",
            params![format!("ec-{asset_id}"), asset_id, status, err],
        )
        .unwrap();
    }

    fn insert_pt(conn: &Connection, asset_id: &str, status: &str, retry: i32, err: Option<&str>) {
        conn.execute(
            "INSERT INTO pipeline_tasks (id, asset_id, task_type, status, retry_count, max_retries, error_message, started_at, completed_at, created_at)
             VALUES (?1, ?2, 'extract', ?3, ?4, 3, ?5, '2026-01-01', '2026-01-01', '2026-01-01')",
            params![format!("pt-{asset_id}"), asset_id, status, retry, err],
        )
        .unwrap();
    }

    fn ec_status(conn: &Connection, asset_id: &str) -> (String, Option<String>) {
        conn.query_row(
            "SELECT status, error_message FROM extracted_content WHERE asset_id = ?1",
            params![asset_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .unwrap()
    }

    fn pt_state(
        conn: &Connection,
        asset_id: &str,
    ) -> (String, i32, Option<String>, Option<String>, Option<String>) {
        conn.query_row(
            "SELECT status, retry_count, error_message, started_at, completed_at
             FROM pipeline_tasks WHERE asset_id = ?1",
            params![asset_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i32>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            },
        )
        .unwrap()
    }

    /// AC-1 a: failed → queued，error 清空，pipeline_tasks 重置
    #[test]
    fn reset_from_failed_clears_error_and_requeues() {
        let conn = setup_db();
        insert_ec(&conn, "a1", "failed", Some("boom"));
        insert_pt(&conn, "a1", "failed", 2, Some("boom"));

        reset_extraction_state(&conn, "a1").expect("reset 应成功");

        let (status, err) = ec_status(&conn, "a1");
        assert_eq!(status, "queued");
        assert_eq!(err, None);

        let (pt_status, retry, pt_err, started, completed) = pt_state(&conn, "a1");
        assert_eq!(pt_status, "queued");
        assert_eq!(retry, 0);
        assert_eq!(pt_err, None);
        assert_eq!(started, None);
        assert_eq!(completed, None);
    }

    /// AC-1 b: extracted → queued（重跑场景）
    #[test]
    fn reset_from_extracted_requeues_for_rerun() {
        let conn = setup_db();
        insert_ec(&conn, "a2", "extracted", None);
        insert_pt(&conn, "a2", "completed", 0, None);

        reset_extraction_state(&conn, "a2").expect("reset 应成功");

        let (status, _) = ec_status(&conn, "a2");
        assert_eq!(status, "queued");
        let (pt_status, retry, _, _, _) = pt_state(&conn, "a2");
        assert_eq!(pt_status, "queued");
        assert_eq!(retry, 0);
    }

    /// AC-1 c: asset 没有 extracted_content 行也不应崩溃（UPDATE 0 行也 Ok）
    /// —— 由 retrigger_extraction 在 enqueue 里插入新行
    #[test]
    fn reset_when_no_row_is_noop() {
        let conn = setup_db();
        // 不插入任何行
        let result = reset_extraction_state(&conn, "ghost");
        assert!(result.is_ok(), "无行时 UPDATE 0 行应仍返回 Ok");
    }

    /// task_026 AC-2 / AC-4 第 1 项：`clear_kc_enriched` 把
    /// `extracted_content.kc_enriched` 从 "true" 清成 NULL；其他字段不动。
    ///
    /// 同时验证：
    /// - clear 不依赖 `reset_extraction_state`（两函数解耦）；
    /// - status / raw_text / structured_md / extractor_type 完全保留；
    /// - 幂等：第 2 次 clear 不报错（NULL → NULL）。
    #[test]
    fn retrigger_extraction_with_force_kc_clears_kc_enriched_field() {
        let conn = setup_db();

        // 准备一个"已 enrich 过"的 extracted_content 行
        conn.execute(
            "INSERT INTO extracted_content
                (id, asset_id, status, error_message, retry_count, raw_text,
                 structured_md, quality_level, extractor_type, segments_json,
                 content_hash, kc_enriched, created_at, updated_at)
             VALUES ('ec-a3', 'a3', 'extracted', NULL, 0,
                     'raw', 'md', 3, 'markitdown+kc', NULL,
                     'h', 'true', '2026-01-01', '2026-01-01')",
            [],
        )
        .unwrap();

        // 调用 clear_kc_enriched
        clear_kc_enriched(&conn, "a3").expect("clear 应成功");

        // 断言 kc_enriched 已被置 NULL
        let kc: Option<String> = conn
            .query_row(
                "SELECT kc_enriched FROM extracted_content WHERE asset_id = ?1",
                params!["a3"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(kc, None, "kc_enriched 应被清为 NULL");

        // 断言其他字段未被动：force_kc_refresh 只清 KC 标志，不重跑 markitdown
        let (status, raw_text, structured_md, extractor_type): (
            String,
            Option<String>,
            Option<String>,
            String,
        ) = conn
            .query_row(
                "SELECT status, raw_text, structured_md, extractor_type
                 FROM extracted_content WHERE asset_id = ?1",
                params!["a3"],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(status, "extracted", "status 不应被 clear_kc_enriched 改动");
        assert_eq!(raw_text.as_deref(), Some("raw"));
        assert_eq!(structured_md.as_deref(), Some("md"));
        assert_eq!(extractor_type, "markitdown+kc");

        // 幂等：第 2 次清 NULL → NULL 仍 Ok
        clear_kc_enriched(&conn, "a3").expect("第 2 次 clear（已为 NULL）也应 Ok");

        // 边界：不存在的 asset_id 也不应崩溃（UPDATE 0 行）
        clear_kc_enriched(&conn, "ghost").expect("无行时 UPDATE 0 行应仍返回 Ok");
    }

    /// task_026 AC-1 配套：`clear_kc_enriched` 对 `kc_enriched='partial'`
    /// 的 row 也能干净清成 NULL（覆盖 task_011 PartialLlmUnavailable 路径）。
    #[test]
    fn clear_kc_enriched_handles_partial_state() {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO extracted_content
                (id, asset_id, status, retry_count, kc_enriched, created_at, updated_at)
             VALUES ('ec-a4', 'a4', 'extracted', 0, 'partial',
                     '2026-01-01', '2026-01-01')",
            [],
        )
        .unwrap();

        clear_kc_enriched(&conn, "a4").expect("partial → NULL 应成功");

        let kc: Option<String> = conn
            .query_row(
                "SELECT kc_enriched FROM extracted_content WHERE asset_id = ?1",
                params!["a4"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(kc, None);
    }

    /// task_006 AC-2（M5 幂等护栏）：模拟 retry 连击 5 次，断言活动态
    /// （queued + running）行数 ≤ 1。
    ///
    /// 真正的 `retry_asset_conversion` 命令依赖 `tauri::AppHandle` / `State`，
    /// 在 unit test 里无法构造；但它的幂等保证最终落在 V7 部分唯一索引
    /// `idx_pipeline_tasks_active_unique` 上。本测试用 `run_migrations` 建出
    /// 与生产一致的 schema（包含该索引），再连击 5 次"按 enqueue 同款 SQL
    /// 插入 queued 行"，断言：
    ///   - 第 1 次成功；
    ///   - 第 2…5 次返回 UNIQUE constraint 错误（被命令层 enqueue 当作
    ///     `already_queued` 静默吞掉）；
    ///   - 终局 `pipeline_tasks` 在该 asset 下活动态行数为 1。
    ///
    /// 这相当于把"上层 retry 命令的所有护栏失效，只剩索引"的最坏情况
    /// 暴露在测试里 —— 若哪天有人改这个索引，本测试会率先红。
    #[test]
    fn retry_asset_conversion_active_unique_guard_caps_at_one() {
        use crate::db::migration::run_migrations;
        let conn = Connection::open_in_memory().expect("内存库");
        run_migrations(&conn).expect("migrate");

        // 准备 asset 行（满足 V1 schema 的 NOT NULL 列；library/project 先注入）
        conn.execute(
            "INSERT INTO libraries (id, name, root_path) VALUES ('lib','l','/tmp')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO projects (id, library_id, name) VALUES ('p','lib','p')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO assets (id, project_id, asset_type, name, original_name,
                file_path, file_size, mime_type, captured_at, imported_at, source_type)
             VALUES ('a','p','pdf','a.pdf','a.pdf','/tmp/a.pdf',1,'application/pdf',
                     '2025-01-01','2025-01-01','import')",
            [],
        )
        .unwrap();

        // 模拟 retry 连击 5 次：每次 INSERT 一行 queued 任务
        let insert_queued = |seq: usize| -> rusqlite::Result<usize> {
            conn.execute(
                "INSERT INTO pipeline_tasks (id, asset_id, task_type, status,
                    retry_count, max_retries, error_message, priority, batch_id,
                    created_at, started_at, completed_at)
                 VALUES (?1, 'a', 'extract', 'queued', 0, 3, NULL, 100, NULL,
                         ?2, NULL, NULL)",
                params![format!("pt-{seq}"), format!("2025-01-01T00:00:0{seq}Z")],
            )
        };

        // 第 1 次：成功落盘
        insert_queued(1).expect("首次入队应成功");
        // 第 2…5 次：被 idx_pipeline_tasks_active_unique 拦截
        for i in 2..=5 {
            let r = insert_queued(i);
            assert!(
                r.is_err(),
                "第 {i} 次插入应被 V7 部分唯一索引拦截，实际: {r:?}"
            );
        }

        // 终局：活动态行数 ≤ 1
        let active: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pipeline_tasks
                 WHERE asset_id = 'a' AND status IN ('queued','running')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(active, 1, "活动态行数应被索引兜底为 1");
    }
}
