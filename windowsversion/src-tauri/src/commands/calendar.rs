use crate::db::calendar::{
    delete_by_source, get_events, insert_events, touch_synced, CourseEvent,
};
use crate::db::Database;
use crate::ics_parser::{parse_ics, ParsedEvent};
use serde::Serialize;
use tauri::State;

/// 解析结果：供前端预览（未写入 DB）
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportIcsResult {
    pub events: Vec<ParsedEvent>,
    pub total_parsed: usize,
    pub duplicates_skipped: usize,
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri Commands
// ─────────────────────────────────────────────────────────────────────────────

/// 解析本地 .ics 文件，返回事件列表供用户预览（不写 DB）
#[tauri::command]
pub fn import_ics_file(
    library_id: String,
    file_path: String,
) -> Result<ImportIcsResult, String> {
    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("读取 .ics 文件失败: {e}"))?;

    let events = parse_ics(&content)?;
    let total_parsed = events.len();

    Ok(ImportIcsResult {
        events,
        total_parsed,
        duplicates_skipped: 0, // 去重在 confirm_import_events 阶段发生
    })
}

/// 从 URL 拉取 .ics 内容并解析，返回事件列表供用户预览（不写 DB）
#[tauri::command]
pub async fn import_ics_url(
    library_id: String,
    url: String,
) -> Result<ImportIcsResult, String> {
    // 使用 reqwest 拉取（复用项目已有的 reqwest 依赖）
    let content = reqwest::get(&url)
        .await
        .map_err(|e| format!("拉取日历 URL 失败: {e}"))?
        .text()
        .await
        .map_err(|e| format!("读取日历内容失败: {e}"))?;

    let events = parse_ics(&content)?;
    let total_parsed = events.len();

    Ok(ImportIcsResult {
        events,
        total_parsed,
        duplicates_skipped: 0,
    })
}

/// 用户确认后，将选中的事件（通过 temp_id 筛选）写入数据库
///
/// - `library_id`：所属知识库
/// - `events`：前端传回的完整事件列表（来自 import_ics_file / import_ics_url）
/// - `selected_temp_ids`：用户勾选的 temp_id 列表；空则全量导入
/// - `calendar_source`：`"ics_file"` 或 `"ics_url"`
/// - `source_url`：订阅 URL（ics_url 模式时非空）
#[tauri::command]
pub fn confirm_import_events(
    db: State<'_, Database>,
    library_id: String,
    events: Vec<ParsedEvent>,
    selected_temp_ids: Vec<String>,
    calendar_source: String,
    source_url: Option<String>,
) -> Result<usize, String> {
    let to_insert: Vec<ParsedEvent> = if selected_temp_ids.is_empty() {
        events
    } else {
        let id_set: std::collections::HashSet<&str> =
            selected_temp_ids.iter().map(|s| s.as_str()).collect();
        events
            .into_iter()
            .filter(|e| id_set.contains(e.temp_id.as_str()))
            .collect()
    };

    let conn = db
        .conn
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;

    insert_events(
        &conn,
        &library_id,
        &to_insert,
        &calendar_source,
        source_url.as_deref(),
    )
}

/// 按时间范围查询课程事件
#[tauri::command]
pub fn get_course_events(
    db: State<'_, Database>,
    library_id: String,
    start_after: Option<String>,
    end_before: Option<String>,
) -> Result<Vec<CourseEvent>, String> {
    let conn = db
        .conn
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    get_events(
        &conn,
        &library_id,
        start_after.as_deref(),
        end_before.as_deref(),
    )
}

/// 删除某个日历来源的所有事件
#[tauri::command]
pub fn delete_calendar_source(
    db: State<'_, Database>,
    library_id: String,
    calendar_source: String,
    source_url: Option<String>,
) -> Result<usize, String> {
    let conn = db
        .conn
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    delete_by_source(&conn, &library_id, &calendar_source, source_url.as_deref())
}

/// 刷新订阅日历：拉取最新 .ics → 删除旧事件 → 插入新事件
#[tauri::command]
pub async fn refresh_ics_subscription(
    db: State<'_, Database>,
    library_id: String,
    source_url: String,
) -> Result<ImportIcsResult, String> {
    // 1. 拉取最新内容
    let content = reqwest::get(&source_url)
        .await
        .map_err(|e| format!("刷新日历失败: {e}"))?
        .text()
        .await
        .map_err(|e| format!("读取刷新内容失败: {e}"))?;

    let events = parse_ics(&content)?;
    let total_parsed = events.len();

    let conn = db
        .conn
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;

    // 2. 删除该 URL 的旧事件
    delete_by_source(&conn, &library_id, "ics_url", Some(&source_url))?;

    // 3. 插入新事件
    let inserted = insert_events(
        &conn,
        &library_id,
        &events,
        "ics_url",
        Some(&source_url),
    )?;

    // 4. 更新同步时间
    touch_synced(&conn, &library_id, &source_url)?;

    let duplicates_skipped = total_parsed.saturating_sub(inserted);

    Ok(ImportIcsResult {
        events,
        total_parsed,
        duplicates_skipped,
    })
}
