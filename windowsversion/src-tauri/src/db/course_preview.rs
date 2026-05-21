use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

/// 已持久化的 AI 预习内容
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoursePreview {
    pub id: String,
    pub course_event_id: String,
    pub content: String,         // Markdown
    pub user_notes: Option<String>,
    pub model: Option<String>,
    pub prompt_hash: Option<String>,
    pub generated_at: String,
    pub created_at: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// 写操作
// ─────────────────────────────────────────────────────────────────────────────

/// 插入或覆盖预习内容（每个课程事件仅保留最新一份）
pub fn insert_or_replace(conn: &Connection, preview: &CoursePreview) -> Result<(), String> {
    conn.execute(
        "INSERT INTO course_previews
             (id, course_event_id, content, user_notes, model, prompt_hash, generated_at, created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
         ON CONFLICT(course_event_id) DO UPDATE SET
             content      = excluded.content,
             model        = excluded.model,
             prompt_hash  = excluded.prompt_hash,
             generated_at = excluded.generated_at",
        params![
            preview.id,
            preview.course_event_id,
            preview.content,
            preview.user_notes,
            preview.model,
            preview.prompt_hash,
            preview.generated_at,
            preview.created_at,
        ],
    )
    .map_err(|e| format!("写入预习内容失败: {e}"))?;
    Ok(())
}

/// 更新用户笔记（不覆盖 AI 生成内容）
pub fn update_user_notes(
    conn: &Connection,
    course_event_id: &str,
    notes: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE course_previews SET user_notes = ?1 WHERE course_event_id = ?2",
        params![notes, course_event_id],
    )
    .map_err(|e| format!("更新预习笔记失败: {e}"))?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// 读操作
// ─────────────────────────────────────────────────────────────────────────────

/// 按课程事件 ID 查询预习内容
pub fn get_by_event(
    conn: &Connection,
    course_event_id: &str,
) -> Result<Option<CoursePreview>, String> {
    conn.query_row(
        "SELECT id, course_event_id, content, user_notes, model, prompt_hash, generated_at, created_at
         FROM course_previews WHERE course_event_id = ?1",
        params![course_event_id],
        row_to_preview,
    )
    .optional()
    .map_err(|e| format!("查询预习内容失败: {e}"))
}

// ─────────────────────────────────────────────────────────────────────────────
// 行映射
// ─────────────────────────────────────────────────────────────────────────────

fn row_to_preview(row: &rusqlite::Row) -> rusqlite::Result<CoursePreview> {
    Ok(CoursePreview {
        id: row.get(0)?,
        course_event_id: row.get(1)?,
        content: row.get(2)?,
        user_notes: row.get(3)?,
        model: row.get(4)?,
        prompt_hash: row.get(5)?,
        generated_at: row.get(6)?,
        created_at: row.get(7)?,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// 单元测试
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn open_db() -> Database {
        let dir = tempfile::tempdir().expect("tempdir");
        Database::open(&dir.path().join("test.db")).expect("open db")
    }

    fn seed_event(conn: &Connection) -> String {
        let event_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO course_events
             (id, library_id, title, start_time, end_time, created_at)
             VALUES (?1, 'lib-1', 'ECON 101', '2099-10-07T09:00:00Z', '2099-10-07T10:15:00Z', '2099-01-01T00:00:00Z')",
            params![event_id],
        )
        .expect("seed event");
        event_id
    }

    fn make_preview(course_event_id: &str) -> CoursePreview {
        let now = chrono::Utc::now().to_rfc3339();
        CoursePreview {
            id: uuid::Uuid::new_v4().to_string(),
            course_event_id: course_event_id.to_string(),
            content: "## 预习指南\n价格弹性...".to_string(),
            user_notes: None,
            model: Some("gpt-4o".to_string()),
            prompt_hash: Some("abc123".to_string()),
            generated_at: now.clone(),
            created_at: now,
        }
    }

    #[test]
    fn insert_and_get_preview() {
        let db = open_db();
        let conn = db.conn.lock().unwrap();
        let event_id = seed_event(&conn);

        let preview = make_preview(&event_id);
        insert_or_replace(&conn, &preview).expect("insert");

        let got = get_by_event(&conn, &event_id).expect("get").expect("should exist");
        assert_eq!(got.course_event_id, event_id);
        assert!(got.content.contains("价格弹性"));
        assert_eq!(got.model.as_deref(), Some("gpt-4o"));
    }

    #[test]
    fn insert_replaces_existing_preview() {
        let db = open_db();
        let conn = db.conn.lock().unwrap();
        let event_id = seed_event(&conn);

        // 第一次插入
        let p1 = make_preview(&event_id);
        insert_or_replace(&conn, &p1).expect("insert 1");

        // 第二次插入（同一 course_event_id，内容不同）
        let mut p2 = make_preview(&event_id);
        p2.content = "## 新版预习\n供需均衡...".to_string();
        insert_or_replace(&conn, &p2).expect("insert 2");

        let got = get_by_event(&conn, &event_id).expect("get").expect("exists");
        assert!(got.content.contains("供需均衡"), "应以新版内容覆盖旧版");
    }

    #[test]
    fn update_user_notes_preserves_ai_content() {
        let db = open_db();
        let conn = db.conn.lock().unwrap();
        let event_id = seed_event(&conn);

        let preview = make_preview(&event_id);
        insert_or_replace(&conn, &preview).expect("insert");

        update_user_notes(&conn, &event_id, "我的笔记：价格弹性影响消费决策").expect("update notes");

        let got = get_by_event(&conn, &event_id).expect("get").expect("exists");
        assert_eq!(got.user_notes.as_deref(), Some("我的笔记：价格弹性影响消费决策"));
        assert!(got.content.contains("价格弹性"), "AI 内容应保持不变");
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let db = open_db();
        let conn = db.conn.lock().unwrap();
        let got = get_by_event(&conn, "no-such-event").expect("query ok");
        assert!(got.is_none());
    }
}
