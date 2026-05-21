use crate::ics_parser::ParsedEvent;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

/// 已持久化的课程事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseEvent {
    pub id: String,
    pub library_id: String,
    pub project_id: Option<String>,
    pub title: String,
    pub course_code: Option<String>,
    pub instructor: Option<String>,
    pub location: Option<String>,
    pub start_time: String,
    pub end_time: String,
    pub recurrence_rule: Option<String>,
    pub day_of_week: Vec<i64>,
    pub description: Option<String>,
    pub calendar_source: String, // "ics_file" | "ics_url"
    pub source_url: Option<String>,
    pub source_uid: Option<String>,
    pub last_synced: Option<String>,
    pub created_at: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// 写操作
// ─────────────────────────────────────────────────────────────────────────────

/// 批量插入课程事件（已由调用方去重，基于 source_uid+start_time）
pub fn insert_events(
    conn: &Connection,
    library_id: &str,
    events: &[ParsedEvent],
    calendar_source: &str,
    source_url: Option<&str>,
) -> Result<usize, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut inserted = 0usize;

    for ev in events {
        // 先检查 source_uid + start_time 是否已存在（去重）
        let uid_key = ev.source_uid.as_deref().unwrap_or(&ev.title);
        let exists: i64 = conn
            .query_row(
                "SELECT count(*) FROM course_events
                 WHERE library_id=?1 AND source_uid=?2 AND start_time=?3",
                params![library_id, uid_key, ev.start_time],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if exists > 0 {
            continue;
        }

        let id = uuid::Uuid::new_v4().to_string();
        let dow_json = serde_json::to_string(&ev.day_of_week)
            .unwrap_or_else(|_| "[]".to_string());

        conn.execute(
            "INSERT INTO course_events
             (id, library_id, project_id, title, course_code, instructor, location,
              start_time, end_time, recurrence_rule, day_of_week, description,
              calendar_source, source_url, source_uid, last_synced, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",
            params![
                id,
                library_id,
                Option::<String>::None,
                ev.title,
                ev.course_code,
                ev.instructor,
                ev.location,
                ev.start_time,
                ev.end_time,
                ev.recurrence_rule,
                dow_json,
                ev.description,
                calendar_source,
                source_url,
                ev.source_uid,
                Option::<String>::None,
                now,
            ],
        )
        .map_err(|e| format!("插入课程事件失败: {e}"))?;
        inserted += 1;
    }
    Ok(inserted)
}

/// 删除指定来源（ics_file 或某个 url）的全部事件
pub fn delete_by_source(
    conn: &Connection,
    library_id: &str,
    calendar_source: &str,
    source_url: Option<&str>,
) -> Result<usize, String> {
    let n = if let Some(url) = source_url {
        conn.execute(
            "DELETE FROM course_events WHERE library_id=?1 AND calendar_source=?2 AND source_url=?3",
            params![library_id, calendar_source, url],
        )
    } else {
        conn.execute(
            "DELETE FROM course_events WHERE library_id=?1 AND calendar_source=?2",
            params![library_id, calendar_source],
        )
    }
    .map_err(|e| format!("删除课程事件失败: {e}"))?;
    Ok(n)
}

/// 更新指定 URL 来源的 last_synced 时间
pub fn touch_synced(conn: &Connection, library_id: &str, source_url: &str) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE course_events SET last_synced=?1 WHERE library_id=?2 AND source_url=?3",
        params![now, library_id, source_url],
    )
    .map_err(|e| format!("更新同步时间失败: {e}"))?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// 读操作
// ─────────────────────────────────────────────────────────────────────────────

/// 按时间范围查询课程事件（两端均可选）
pub fn get_events(
    conn: &Connection,
    library_id: &str,
    start_after: Option<&str>,
    end_before: Option<&str>,
) -> Result<Vec<CourseEvent>, String> {
    let mut sql = String::from(
        "SELECT id, library_id, project_id, title, course_code, instructor, location,
                start_time, end_time, recurrence_rule, day_of_week, description,
                calendar_source, source_url, source_uid, last_synced, created_at
         FROM course_events WHERE library_id = ?1",
    );
    let mut bind_idx = 2usize;

    if start_after.is_some() {
        sql.push_str(&format!(" AND start_time >= ?{bind_idx}"));
        bind_idx += 1;
    }
    if end_before.is_some() {
        sql.push_str(&format!(" AND end_time <= ?{bind_idx}"));
    }
    sql.push_str(" ORDER BY start_time ASC");

    let mut stmt = conn.prepare(&sql).map_err(|e| format!("查询课程事件失败: {e}"))?;

    let rows: Vec<CourseEvent> = match (start_after, end_before) {
        (Some(sa), Some(eb)) => stmt
            .query_map(params![library_id, sa, eb], row_to_event)
            .map_err(|e| format!("遍历课程事件失败: {e}"))?,
        (Some(sa), None) => stmt
            .query_map(params![library_id, sa], row_to_event)
            .map_err(|e| format!("遍历课程事件失败: {e}"))?,
        (None, Some(eb)) => stmt
            .query_map(params![library_id, eb], row_to_event)
            .map_err(|e| format!("遍历课程事件失败: {e}"))?,
        (None, None) => stmt
            .query_map(params![library_id], row_to_event)
            .map_err(|e| format!("遍历课程事件失败: {e}"))?,
    }
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| format!("读取课程事件失败: {e}"))?;

    Ok(rows)
}

/// 按 ID 查单条事件
pub fn get_event_by_id(conn: &Connection, id: &str) -> Result<Option<CourseEvent>, String> {
    conn.query_row(
        "SELECT id, library_id, project_id, title, course_code, instructor, location,
                start_time, end_time, recurrence_rule, day_of_week, description,
                calendar_source, source_url, source_uid, last_synced, created_at
         FROM course_events WHERE id = ?1",
        params![id],
        row_to_event,
    )
    .optional()
    .map_err(|e| format!("查询课程事件失败: {e}"))
}

// ─────────────────────────────────────────────────────────────────────────────
// 行映射
// ─────────────────────────────────────────────────────────────────────────────

fn row_to_event(row: &rusqlite::Row) -> rusqlite::Result<CourseEvent> {
    let dow_str: Option<String> = row.get(10)?;
    let day_of_week: Vec<i64> = dow_str
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    Ok(CourseEvent {
        id: row.get(0)?,
        library_id: row.get(1)?,
        project_id: row.get(2)?,
        title: row.get(3)?,
        course_code: row.get(4)?,
        instructor: row.get(5)?,
        location: row.get(6)?,
        start_time: row.get(7)?,
        end_time: row.get(8)?,
        recurrence_rule: row.get(9)?,
        day_of_week,
        description: row.get(11)?,
        calendar_source: row.get(12)?,
        source_url: row.get(13)?,
        source_uid: row.get(14)?,
        last_synced: row.get(15)?,
        created_at: row.get(16)?,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// 单元测试
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::ics_parser::ParsedEvent;

    fn open_test_db() -> Database {
        let dir = tempfile::tempdir().expect("tempdir");
        Database::open(&dir.path().join("test.db")).expect("open db")
    }

    fn make_event(uid: &str, title: &str, start: &str) -> ParsedEvent {
        ParsedEvent {
            temp_id: uid.to_string(),
            title: title.to_string(),
            course_code: Some("ECON 101".to_string()),
            instructor: None,
            location: None,
            start_time: start.to_string(),
            end_time: start.replace("T09", "T10"),
            recurrence_rule: None,
            day_of_week: vec![1],
            description: None,
            source_uid: Some(uid.to_string()),
        }
    }

    #[test]
    fn insert_and_query_events() {
        let db = open_test_db();
        let conn = db.conn.lock().unwrap();
        let lib = "lib-test";

        let evs = vec![
            make_event("uid-1", "ECON 101", "2099-10-07T09:00:00+00:00"),
            make_event("uid-2", "CS 231", "2099-10-08T10:00:00+00:00"),
        ];
        let inserted = insert_events(&conn, lib, &evs, "ics_file", None).unwrap();
        assert_eq!(inserted, 2);

        let result = get_events(&conn, lib, None, None).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].title, "ECON 101");
        assert_eq!(result[0].day_of_week, vec![1]);
    }

    #[test]
    fn insert_deduplicates_same_uid_starttime() {
        let db = open_test_db();
        let conn = db.conn.lock().unwrap();
        let lib = "lib-dedup";

        let evs = vec![make_event("uid-dup", "HIST 150", "2099-10-07T09:00:00+00:00")];
        insert_events(&conn, lib, &evs, "ics_file", None).unwrap();
        // 再次插入相同 source_uid + start_time
        let second = insert_events(&conn, lib, &evs, "ics_file", None).unwrap();
        assert_eq!(second, 0, "重复事件不应重复插入");

        let result = get_events(&conn, lib, None, None).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn time_range_filter_works() {
        let db = open_test_db();
        let conn = db.conn.lock().unwrap();
        let lib = "lib-range";

        let evs = vec![
            make_event("uid-a", "ECON 101", "2099-10-07T09:00:00+00:00"),
            make_event("uid-b", "MATH 201", "2099-11-07T09:00:00+00:00"),
        ];
        insert_events(&conn, lib, &evs, "ics_file", None).unwrap();

        // 只查 10 月份
        let oct = get_events(
            &conn,
            lib,
            Some("2099-10-01T00:00:00+00:00"),
            Some("2099-10-31T23:59:59+00:00"),
        )
        .unwrap();
        assert_eq!(oct.len(), 1);
        assert_eq!(oct[0].title, "ECON 101");
    }

    #[test]
    fn delete_by_source_removes_correct_events() {
        let db = open_test_db();
        let conn = db.conn.lock().unwrap();
        let lib = "lib-del";

        let file_evs = vec![make_event("uid-f1", "ECON 101", "2099-10-07T09:00:00+00:00")];
        let url_evs = vec![make_event("uid-u1", "CS 231", "2099-10-08T10:00:00+00:00")];

        insert_events(&conn, lib, &file_evs, "ics_file", None).unwrap();
        insert_events(&conn, lib, &url_evs, "ics_url", Some("https://example.com/cal.ics")).unwrap();

        // 删除 ics_file 来源
        let deleted = delete_by_source(&conn, lib, "ics_file", None).unwrap();
        assert_eq!(deleted, 1);

        let remaining = get_events(&conn, lib, None, None).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].calendar_source, "ics_url");
    }

    #[test]
    fn get_event_by_id_works() {
        let db = open_test_db();
        let conn = db.conn.lock().unwrap();
        let lib = "lib-byid";

        let evs = vec![make_event("uid-x", "PHIL 220", "2099-10-09T11:00:00+00:00")];
        insert_events(&conn, lib, &evs, "ics_file", None).unwrap();

        let all = get_events(&conn, lib, None, None).unwrap();
        let id = &all[0].id;

        let found = get_event_by_id(&conn, id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().title, "PHIL 220");

        let not_found = get_event_by_id(&conn, "no-such-id").unwrap();
        assert!(not_found.is_none());
    }
}
