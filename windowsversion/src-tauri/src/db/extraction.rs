use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

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
                quality_level, extractor_type, segments_json, created_at, updated_at
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
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
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
