use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

// ---- extracted_content ----

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedContentRow {
    pub id: String,
    pub asset_id: String,
    pub status: String,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub raw_text: Option<String>,
    pub structured_md: Option<String>,
    pub quality_level: i32,
    pub extractor_type: String,
    pub segments_json: Option<String>,
    /// task_026 AC-3：KC 增强标志（V18 列），暴露给前端 Inspector 做"重新增强"
    /// 按钮的显隐判断。NULL = 未走过 KC、"true" = enrich 成功、"false" = enrich
    /// 失败、"partial" = LLM 不可用规则兜底（task_011 PartialLlmUnavailable）。
    pub kc_enriched: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub fn insert_extracted_content(conn: &Connection, row: &ExtractedContentRow) -> Result<(), String> {
    conn.execute(
        "INSERT INTO extracted_content
            (id, asset_id, status, error_message, retry_count, raw_text, structured_md,
             quality_level, extractor_type, segments_json, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![
            row.id,
            row.asset_id,
            row.status,
            row.error_message,
            row.retry_count,
            row.raw_text,
            row.structured_md,
            row.quality_level,
            row.extractor_type,
            row.segments_json,
            row.created_at,
            row.updated_at,
        ],
    )
    .map_err(|e| format!("插入提取内容失败: {e}"))?;
    Ok(())
}

pub fn get_extracted_content(
    conn: &Connection,
    asset_id: &str,
) -> Result<Option<ExtractedContentRow>, String> {
    conn.query_row(
        "SELECT id, asset_id, status, error_message, retry_count, raw_text, structured_md,
                quality_level, extractor_type, segments_json, kc_enriched, created_at, updated_at
         FROM extracted_content WHERE asset_id = ?1",
        params![asset_id],
        |row| {
            Ok(ExtractedContentRow {
                id: row.get(0)?,
                asset_id: row.get(1)?,
                status: row.get(2)?,
                error_message: row.get(3)?,
                retry_count: row.get(4)?,
                raw_text: row.get(5)?,
                structured_md: row.get(6)?,
                quality_level: row.get(7)?,
                extractor_type: row.get(8)?,
                segments_json: row.get(9)?,
                kc_enriched: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        },
    )
    .optional()
    .map_err(|e| format!("查询提取内容失败: {e}"))
}

pub fn update_extraction_status(
    conn: &Connection,
    asset_id: &str,
    status: &str,
    error_message: Option<&str>,
) -> Result<(), String> {
    conn.execute(
        "UPDATE extracted_content
         SET status = ?2, error_message = ?3, updated_at = datetime('now')
         WHERE asset_id = ?1",
        params![asset_id, status, error_message],
    )
    .map_err(|e| format!("更新提取状态失败: {e}"))?;
    Ok(())
}

pub fn update_extraction_result(
    conn: &Connection,
    asset_id: &str,
    raw_text: &str,
    structured_md: &str,
    quality_level: i32,
    extractor_type: &str,
    segments_json: Option<&str>,
) -> Result<(), String> {
    conn.execute(
        "UPDATE extracted_content
         SET status = 'extracted', raw_text = ?2, structured_md = ?3,
             quality_level = ?4, extractor_type = ?5, segments_json = ?6,
             error_message = NULL, updated_at = datetime('now')
         WHERE asset_id = ?1",
        params![asset_id, raw_text, structured_md, quality_level, extractor_type, segments_json],
    )
    .map_err(|e| format!("更新提取结果失败: {e}"))?;
    Ok(())
}

// ===== task_015：KC enrichment 字段写入/读取（v18 schema 3 列） =====

/// task_015 AC-3：KC 三态查询的轻量结果行。
///
/// 与 `ExtractedContentRow` 区分：只暴露给前端 Tauri command 用于"KC 增强状态"展示，
/// 避免读路径拉出整张大 row（含 raw_text / structured_md 这种大字段）。
///
/// 字段语义：
/// - `kc_enriched`：NULL → "未增强"（历史行 / KC 集成前）；"true" / "false" / "partial"
/// - `kc_version`：KC compiler 版本字符串；NULL 表示尚未走过 KC
/// - `kc_tags_source`：标签来源，"ai+rule" / "rule_only" / NULL
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KcStatusRow {
    pub kc_enriched: Option<String>,
    pub kc_version: Option<String>,
    pub kc_tags_source: Option<String>,
}

/// task_015 AC-1：把 KC enrichment 结果写到 `extracted_content` 表的 3 个 KC 列。
///
/// 调用语义：
/// - `kc_enriched` 是必填字符串字面（"true" / "false" / "partial"），由调用方判断；
/// - `kc_version` / `kc_tags_source` 可选——KC 失败兜底场景下可只写 `kc_enriched="false"`，
///   两个版本/来源字段保持 NULL。
///
/// **不会** 因为 `asset_id` 不存在而报错（rows_affected=0 视为成功，与现状
/// `update_extraction_status` / `update_failure_code` 容忍语义一致）；
/// 调用方负责确保 extracted_content 行已建（通常 extract 完成后才调本函数）。
///
/// 幂等：重复调用同一个 `asset_id` + 同样的入参不会报错（UPDATE 是 idempotent SQL，
/// SQLite 不会因"列值未变"额外计数）。
pub fn db_update_kc_fields(
    conn: &Connection,
    asset_id: &str,
    kc_enriched: &str,
    kc_version: Option<&str>,
    kc_tags_source: Option<&str>,
) -> Result<(), String> {
    conn.execute(
        "UPDATE extracted_content
         SET kc_enriched = ?2, kc_version = ?3, kc_tags_source = ?4,
             updated_at = datetime('now')
         WHERE asset_id = ?1",
        params![asset_id, kc_enriched, kc_version, kc_tags_source],
    )
    .map_err(|e| format!("更新 extracted_content KC 字段失败: {e}"))?;
    Ok(())
}

/// task_015 AC-3：读取某 asset 的 KC 三态状态。
///
/// 返回值语义：
/// - `Ok(None)`：该 asset 不存在 extracted_content 行（即未走过抽取链路）；
/// - `Ok(Some(KcStatusRow))`：行存在；KcStatusRow 内部字段可能全 NULL（historic 行 /
///   KC 失败 / 未触发 KC 的合法状态，由前端按"未增强"展示）。
///
/// 不抛错的边界：仅当 SQLite 系统级错误才返回 `Err`。
pub fn db_read_kc_status(
    conn: &Connection,
    asset_id: &str,
) -> Result<Option<KcStatusRow>, String> {
    conn.query_row(
        "SELECT kc_enriched, kc_version, kc_tags_source
         FROM extracted_content WHERE asset_id = ?1",
        params![asset_id],
        |row| {
            Ok(KcStatusRow {
                kc_enriched: row.get(0)?,
                kc_version: row.get(1)?,
                kc_tags_source: row.get(2)?,
            })
        },
    )
    .optional()
    .map_err(|e| format!("查询 KC 状态失败: {e}"))
}

/// 写入内容指纹（SHA-256 of structured_md），供增量抽取判重。
pub fn set_content_hash(
    conn: &Connection,
    asset_id: &str,
    content_hash: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE extracted_content SET content_hash = ?2 WHERE asset_id = ?1",
        params![asset_id, content_hash],
    )
    .map_err(|e| format!("更新 content_hash 失败: {e}"))?;
    Ok(())
}

pub fn upsert_extraction_result(
    conn: &Connection,
    asset_id: &str,
    raw_text: &str,
    structured_md: &str,
    quality_level: i32,
    extractor_type: &str,
    segments_json: Option<&str>,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO extracted_content
            (id, asset_id, status, error_message, retry_count, raw_text, structured_md,
             quality_level, extractor_type, segments_json, created_at, updated_at)
         VALUES (?1, ?2, 'extracted', NULL, 0, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
         ON CONFLICT(asset_id) DO UPDATE SET
            status='extracted',
            error_message=NULL,
            raw_text=excluded.raw_text,
            structured_md=excluded.structured_md,
            quality_level=excluded.quality_level,
            extractor_type=excluded.extractor_type,
            segments_json=excluded.segments_json,
            updated_at=excluded.updated_at",
        params![
            uuid::Uuid::new_v4().to_string(),
            asset_id,
            raw_text,
            structured_md,
            quality_level,
            extractor_type,
            segments_json,
            now,
        ],
    )
    .map_err(|e| format!("写入提取结果失败: {e}"))?;
    Ok(())
}

// ---- pipeline_tasks ----

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineTaskRow {
    pub id: String,
    pub asset_id: String,
    pub task_type: String,
    pub status: String,
    pub retry_count: i32,
    pub max_retries: i32,
    pub error_message: Option<String>,
    pub priority: i32,
    pub batch_id: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

pub fn insert_pipeline_task(conn: &Connection, task: &PipelineTaskRow) -> Result<(), String> {
    conn.execute(
        "INSERT INTO pipeline_tasks
            (id, asset_id, task_type, status, retry_count, max_retries,
             error_message, priority, batch_id, created_at, started_at, completed_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![
            task.id,
            task.asset_id,
            task.task_type,
            task.status,
            task.retry_count,
            task.max_retries,
            task.error_message,
            task.priority,
            task.batch_id,
            task.created_at,
            task.started_at,
            task.completed_at,
        ],
    )
    .map_err(|e| format!("插入管线任务失败: {e}"))?;
    Ok(())
}

pub fn get_queued_tasks(conn: &Connection, limit: i64) -> Result<Vec<PipelineTaskRow>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, asset_id, task_type, status, retry_count, max_retries,
                    error_message, priority, batch_id, created_at, started_at, completed_at
             FROM pipeline_tasks
             WHERE status = 'queued'
             ORDER BY priority ASC, created_at ASC
             LIMIT ?1",
        )
        .map_err(|e| format!("查询排队任务失败: {e}"))?;

    let rows = stmt
        .query_map(params![limit], |row| row_to_pipeline_task(row))
        .map_err(|e| format!("遍历排队任务失败: {e}"))?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| format!("读取行失败: {e}"))?);
    }
    Ok(result)
}

pub fn update_task_status(
    conn: &Connection,
    task_id: &str,
    status: &str,
    error_message: Option<&str>,
) -> Result<(), String> {
    let now = if status == "running" {
        "started_at = datetime('now'),"
    } else if status == "completed" || status == "failed" {
        "completed_at = datetime('now'),"
    } else {
        ""
    };

    let sql = format!(
        "UPDATE pipeline_tasks SET status = ?2, error_message = ?3, {now} retry_count = CASE WHEN ?2 = 'failed' THEN retry_count + 1 ELSE retry_count END WHERE id = ?1"
    );

    conn.execute(&sql, params![task_id, status, error_message])
        .map_err(|e| format!("更新任务状态失败: {e}"))?;
    Ok(())
}

/// 启动恢复：将所有 running 状态的任务重置为 queued
pub fn reset_running_tasks(conn: &Connection) -> Result<u64, String> {
    let changed = conn
        .execute(
            "UPDATE pipeline_tasks SET status = 'queued', started_at = NULL WHERE status = 'running'",
            [],
        )
        .map_err(|e| format!("重置运行中任务失败: {e}"))?;
    Ok(changed as u64)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStats {
    pub queued: i64,
    pub running: i64,
    pub completed: i64,
    pub failed: i64,
    pub cancelled: i64,
}

pub fn get_pipeline_stats(conn: &Connection) -> Result<PipelineStats, String> {
    let mut stats = PipelineStats {
        queued: 0,
        running: 0,
        completed: 0,
        failed: 0,
        cancelled: 0,
    };

    let mut stmt = conn
        .prepare("SELECT status, COUNT(*) FROM pipeline_tasks GROUP BY status")
        .map_err(|e| format!("查询管线统计失败: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|e| format!("遍历管线统计失败: {e}"))?;

    for row in rows {
        let (status, count) = row.map_err(|e| format!("读取行失败: {e}"))?;
        match status.as_str() {
            "queued" => stats.queued = count,
            "running" => stats.running = count,
            "completed" => stats.completed = count,
            "failed" => stats.failed = count,
            "cancelled" => stats.cancelled = count,
            _ => {}
        }
    }

    Ok(stats)
}

fn row_to_pipeline_task(row: &rusqlite::Row) -> rusqlite::Result<PipelineTaskRow> {
    Ok(PipelineTaskRow {
        id: row.get(0)?,
        asset_id: row.get(1)?,
        task_type: row.get(2)?,
        status: row.get(3)?,
        retry_count: row.get(4)?,
        max_retries: row.get(5)?,
        error_message: row.get(6)?,
        priority: row.get(7)?,
        batch_id: row.get(8)?,
        created_at: row.get(9)?,
        started_at: row.get(10)?,
        completed_at: row.get(11)?,
    })
}

// ============================================================================
// task_015：KC enrichment 字段读写测试（v18 schema 集成）
// ============================================================================

#[cfg(test)]
mod kc_tests {
    use super::*;
    use crate::db::migration::run_migrations;

    /// 建一个跑完 V18 迁移的内存库 + 一条 extracted_content 行用于 UPDATE 锚定。
    fn setup_db_with_extraction(asset_id: &str) -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .expect("enable FK");
        run_migrations(&conn).expect("run migrations (incl. v18)");

        // 建外键链路：library + project + asset
        conn.execute(
            "INSERT INTO libraries (id, name, root_path) VALUES (?1, 'lib', '/tmp/lib')",
            params!["lib1"],
        )
        .expect("insert library");
        conn.execute(
            "INSERT INTO projects (id, library_id, name) VALUES (?1, ?2, 'p')",
            params!["proj1", "lib1"],
        )
        .expect("insert project");
        conn.execute(
            "INSERT INTO assets (id, project_id, asset_type, name, file_path)
             VALUES (?1, ?2, 'document', 'a.pdf', '/tmp/a.pdf')",
            params![asset_id, "proj1"],
        )
        .expect("insert asset");

        // 插一条 extracted_content（用 upsert）
        upsert_extraction_result(
            &conn,
            asset_id,
            "原文 raw",
            "# 结构化 MD",
            2,
            "markitdown",
            None,
        )
        .expect("upsert extraction");

        conn
    }

    /// 不预建 extracted_content 行的 setup（用于测试 db_read_kc_status 的 None 路径）。
    fn setup_db_without_extraction(asset_id: &str) -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .expect("enable FK");
        run_migrations(&conn).expect("run migrations");
        conn.execute(
            "INSERT INTO libraries (id, name, root_path) VALUES (?1, 'lib', '/tmp/lib')",
            params!["lib1"],
        )
        .expect("insert library");
        conn.execute(
            "INSERT INTO projects (id, library_id, name) VALUES (?1, ?2, 'p')",
            params!["proj1", "lib1"],
        )
        .expect("insert project");
        conn.execute(
            "INSERT INTO assets (id, project_id, asset_type, name, file_path)
             VALUES (?1, ?2, 'document', 'a.pdf', '/tmp/a.pdf')",
            params![asset_id, "proj1"],
        )
        .expect("insert asset");
        conn
    }

    /// AC-1：`db_update_kc_fields` 应同时更新 3 列。
    #[test]
    fn db_update_kc_fields_sets_three_columns() {
        let conn = setup_db_with_extraction("asset-a");

        db_update_kc_fields(
            &conn,
            "asset-a",
            "true",
            Some("0.9"),
            Some("ai+rule"),
        )
        .expect("update kc fields");

        let (enriched, version, tags_source): (Option<String>, Option<String>, Option<String>) =
            conn.query_row(
                "SELECT kc_enriched, kc_version, kc_tags_source
                 FROM extracted_content WHERE asset_id = ?1",
                params!["asset-a"],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .expect("read kc fields");

        assert_eq!(enriched.as_deref(), Some("true"));
        assert_eq!(version.as_deref(), Some("0.9"));
        assert_eq!(tags_source.as_deref(), Some("ai+rule"));
    }

    /// AC-1：`db_update_kc_fields` 第二次调用覆盖第一次的值，且不报错（idempotent）。
    /// 也覆盖"version / tags_source 可清回 None"的边界。
    #[test]
    fn db_update_kc_fields_idempotent() {
        let conn = setup_db_with_extraction("asset-b");

        // 第一轮：partial + 有 version + 有 tags_source
        db_update_kc_fields(&conn, "asset-b", "partial", Some("0.8"), Some("rule_only"))
            .expect("update 1");

        // 第二轮：true + 不同 version + 清掉 tags_source
        db_update_kc_fields(&conn, "asset-b", "true", Some("0.9"), None)
            .expect("update 2 (idempotent retry)");

        let (enriched, version, tags_source): (Option<String>, Option<String>, Option<String>) =
            conn.query_row(
                "SELECT kc_enriched, kc_version, kc_tags_source
                 FROM extracted_content WHERE asset_id = ?1",
                params!["asset-b"],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .expect("read");
        assert_eq!(enriched.as_deref(), Some("true"));
        assert_eq!(version.as_deref(), Some("0.9"));
        assert!(tags_source.is_none(), "第二轮把 tags_source 清回 None");

        // 第三轮：完全重复第二轮入参，不报错
        db_update_kc_fields(&conn, "asset-b", "true", Some("0.9"), None)
            .expect("idempotent same-values");

        // AC-1 容忍语义：UPDATE 不存在的 asset_id 不报错（rows_affected=0 视为成功）
        db_update_kc_fields(&conn, "nonexistent-asset", "true", None, None)
            .expect("missing asset_id should be Ok (rows_affected=0 tolerated)");
    }

    /// AC-3：无 extracted_content 行时 `db_read_kc_status` 返回 `Ok(None)`。
    #[test]
    fn db_read_kc_status_returns_none_when_no_row() {
        let conn = setup_db_without_extraction("asset-c");

        let st = db_read_kc_status(&conn, "asset-c").expect("query");
        assert!(st.is_none(), "无 extracted_content 行应返回 None");

        // 同样地：完全不存在的 asset_id 也是 None
        let st2 = db_read_kc_status(&conn, "totally-nonexistent").expect("query");
        assert!(st2.is_none());
    }

    /// AC-3：调用 `db_update_kc_fields` 后 `db_read_kc_status` 应读到对应值。
    #[test]
    fn db_read_kc_status_returns_values_after_update() {
        let conn = setup_db_with_extraction("asset-d");

        // UPDATE 前：行存在，3 个 KC 列全 NULL
        let st_before = db_read_kc_status(&conn, "asset-d").expect("read before");
        let row_before = st_before.expect("行存在");
        assert!(row_before.kc_enriched.is_none());
        assert!(row_before.kc_version.is_none());
        assert!(row_before.kc_tags_source.is_none());

        // UPDATE：写入 KC 字段
        db_update_kc_fields(&conn, "asset-d", "true", Some("0.9"), Some("ai+rule"))
            .expect("update");

        // UPDATE 后：3 列均有值
        let st_after = db_read_kc_status(&conn, "asset-d").expect("read after");
        let row_after = st_after.expect("行存在");
        assert_eq!(row_after.kc_enriched.as_deref(), Some("true"));
        assert_eq!(row_after.kc_version.as_deref(), Some("0.9"));
        assert_eq!(row_after.kc_tags_source.as_deref(), Some("ai+rule"));
    }

    /// 边界：`KcStatusRow` JSON 序列化为 camelCase。
    /// 前端 Tauri 命令层将通过 serde 直出该结构，camelCase 关乎前端契约稳定性。
    #[test]
    fn kc_status_row_serializes_as_camel_case() {
        let row = KcStatusRow {
            kc_enriched: Some("true".to_string()),
            kc_version: Some("0.9".to_string()),
            kc_tags_source: None,
        };
        let json = serde_json::to_string(&row).expect("ser");
        assert!(json.contains("\"kcEnriched\":\"true\""));
        assert!(json.contains("\"kcVersion\":\"0.9\""));
        assert!(json.contains("\"kcTagsSource\":null"));
        // 反序回来验证 round-trip
        let back: KcStatusRow = serde_json::from_str(&json).expect("de");
        assert_eq!(back.kc_enriched.as_deref(), Some("true"));
        assert_eq!(back.kc_version.as_deref(), Some("0.9"));
        assert!(back.kc_tags_source.is_none());
    }
}
