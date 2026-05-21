use crate::db::calendar::get_event_by_id;
use crate::db::course_preview::{
    get_by_event, insert_or_replace, update_user_notes, CoursePreview,
};
use crate::db::Database;
use crate::llm::chat::{chat_completion, ChatMessage};
use crate::llm::client::LLMClient;
use rusqlite::params;
use tauri::State;

// ─────────────────────────────────────────────────────────────────────────────
// Tauri Commands
// ─────────────────────────────────────────────────────────────────────────────

/// 生成（或缓存读取）AI 课程预习内容
///
/// 流程：
///   1. 读取课程事件信息
///   2. 若已有缓存且 force_regenerate=false，直接返回缓存
///   3. 查找该课程相关的历史笔记/素材摘要（基于 courseCode/projectId）
///   4. 组装 PRD §3.4 的 System + User Prompt
///   5. 调用 LLM（复用 llm/client.rs，宪章 B3）
///   6. 存入 course_previews 表（覆盖旧版）
///   7. 返回给前端渲染
#[tauri::command]
pub async fn generate_course_preview(
    db: State<'_, Database>,
    course_event_id: String,
    force_regenerate: bool,
) -> Result<CoursePreview, String> {
    // ── 1. 读取课程事件 ──────────────────────────────────────────────────────
    let (event, client, related_context) = {
        let conn = db
            .conn
            .lock()
            .map_err(|e| format!("数据库锁获取失败: {e}"))?;

        let event = get_event_by_id(&conn, &course_event_id)?
            .ok_or_else(|| format!("找不到课程事件: {course_event_id}"))?;

        // ── 2. 缓存检查 ──────────────────────────────────────────────────────
        if !force_regenerate {
            if let Some(cached) = get_by_event(&conn, &course_event_id)? {
                return Ok(cached);
            }
        }

        // ── 3. 读取 LLM 配置 ─────────────────────────────────────────────────
        let client = LLMClient::from_db_or_env(&conn)?;

        // ── 4. 检索相关历史内容 ──────────────────────────────────────────────
        let context = collect_related_context(&conn, &event);

        (event, client, context)
    };

    // ── 5. 组装 Prompt（宪章 B3：复用 llm 基础设施）─────────────────────────
    let (system_prompt, user_prompt) = build_preview_prompt(&event, &related_context);

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: system_prompt,
        },
        ChatMessage {
            role: "user".to_string(),
            content: user_prompt,
        },
    ];

    // ── 6. 调用 LLM ─────────────────────────────────────────────────────────
    let content = chat_completion(&client, messages).await?;

    // ── 7. 持久化 ────────────────────────────────────────────────────────────
    let now = chrono::Utc::now().to_rfc3339();
    let preview = CoursePreview {
        id: uuid::Uuid::new_v4().to_string(),
        course_event_id: course_event_id.clone(),
        content,
        user_notes: None,
        model: Some(client.model.clone()),
        prompt_hash: None,
        generated_at: now.clone(),
        created_at: now,
    };

    {
        let conn = db
            .conn
            .lock()
            .map_err(|e| format!("数据库锁获取失败: {e}"))?;
        insert_or_replace(&conn, &preview)?;
    }

    Ok(preview)
}

/// 查询已生成的预习内容（不触发 LLM）
#[tauri::command]
pub fn get_course_preview(
    db: State<'_, Database>,
    course_event_id: String,
) -> Result<Option<CoursePreview>, String> {
    let conn = db
        .conn
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    get_by_event(&conn, &course_event_id)
}

/// 保存用户在预习空间输入的笔记（debounce 1s 后由前端调用）
#[tauri::command]
pub fn save_preview_notes(
    db: State<'_, Database>,
    course_event_id: String,
    notes: String,
) -> Result<(), String> {
    let conn = db
        .conn
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    update_user_notes(&conn, &course_event_id, &notes)
}

// ─────────────────────────────────────────────────────────────────────────────
// Prompt 组装（PRD §3.4）
// ─────────────────────────────────────────────────────────────────────────────

fn build_preview_prompt(
    event: &crate::db::calendar::CourseEvent,
    related_context: &RelatedContext,
) -> (String, String) {
    let system = r#"You are a world-class academic tutor specializing in helping American college
students prepare for their upcoming classes. Your goal is to create a concise,
actionable preview guide that helps the student walk into class feeling confident
and ready to engage.

Rules:
- Write at a level appropriate for undergraduate/graduate college students
- Be concise — students are time-constrained; aim for 5-minute read max
- Use both English section headers and bilingual hints where helpful
- Ground your suggestions in the course context provided
- If the student has prior notes/documents, reference them specifically to show
  continuity
- Do NOT fabricate specific page numbers or reading assignments — use general
  chapter/topic references
- Structure output in Markdown"#
        .to_string();

    // 计算大约是学期第几周（从 1 月初算起）
    let start = chrono::DateTime::parse_from_rfc3339(&event.start_time)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());
    let week_of_year = start.format("%W").to_string().parse::<u32>().unwrap_or(1);
    // 学期通常 1 月或 8 月开始，粗略估算
    let week_number = week_of_year % 20 + 1;
    let session_number = week_number * 2; // 每周 2 节粗估

    let mut user = format!(
        "# Course Preview Request\n\n\
        ## Course Info\n\
        - Course: {title}\n\
        - Course Code: {code}\n\
        - Instructor: {instructor}\n\
        - Session Time: {start_time} — {end_time}\n\
        - Location: {location}\n\n\
        ## Context\n\
        - This is approximately session #{session_number} of the semester (approx. week {week_number})\n\
        - Course description: {description}\n",
        title = event.title,
        code = event.course_code.as_deref().unwrap_or("N/A"),
        instructor = event.instructor.as_deref().unwrap_or("Not specified"),
        start_time = event.start_time,
        end_time = event.end_time,
        location = event.location.as_deref().unwrap_or("TBD"),
        session_number = session_number,
        week_number = week_number,
        description = event
            .description
            .as_deref()
            .unwrap_or("No course description available"),
    );

    // 注入历史笔记/素材摘要
    if !related_context.previous_topics.is_empty() {
        user.push_str("- Topics covered in recent sessions (from student's notes):\n");
        for topic in &related_context.previous_topics {
            user.push_str(&format!("  * {topic}\n"));
        }
    }

    if !related_context.related_assets.is_empty() {
        user.push_str("\n## Student's Related Materials\nThe student has these relevant materials in their library:\n");
        for asset in &related_context.related_assets {
            user.push_str(&format!("- {asset}\n"));
        }
    }

    user.push_str(
        r#"
## Task
Generate a structured preview guide with these sections:

### 1. 本次课程主题预测 (Predicted Topic)
Based on the course progression and week number, predict what this session
will likely cover. Be specific but note this is a prediction.

### 2. 核心概念 (Key Concepts to Preview)
List 3-5 key concepts the student should familiarize themselves with before
class. For each concept, give a 1-2 sentence plain-language explanation.

### 3. 课前思考问题 (Pre-Class Thinking Questions)
Pose 2-3 thought-provoking questions that will prime the student's thinking.
These should be questions that the lecture will help answer.

### 4. 与已有知识的联系 (Connections to Prior Knowledge)
If the student has prior notes or materials, draw explicit connections.
If not, connect to general prerequisite knowledge.

### 5. 推荐预读 (Suggested Pre-Reading)
Suggest 1-2 accessible resources (textbook chapter topics, short videos,
articles) that would help. Be general (e.g., "Chapter on X" rather than
"page 142")."#,
    );

    (system, user)
}

// ─────────────────────────────────────────────────────────────────────────────
// 历史上下文收集（基于 courseCode 在项目中检索）
// ─────────────────────────────────────────────────────────────────────────────

struct RelatedContext {
    previous_topics: Vec<String>,
    related_assets: Vec<String>,
}

fn collect_related_context(
    conn: &rusqlite::Connection,
    event: &crate::db::calendar::CourseEvent,
) -> RelatedContext {
    let mut ctx = RelatedContext {
        previous_topics: Vec::new(),
        related_assets: Vec::new(),
    };

    // 基于 course_code 模糊匹配项目名，取最近的笔记摘要
    let course_code = match &event.course_code {
        Some(c) if !c.is_empty() => c.clone(),
        _ => return ctx,
    };

    // 检索笔记内容片段（最多 5 条）
    let note_sql = "SELECT n.content FROM notes n
         INNER JOIN projects p ON p.id = n.project_id
         WHERE p.name LIKE ?1 OR p.description LIKE ?1
         ORDER BY n.updated_at DESC LIMIT 5";

    let pattern = format!("%{course_code}%");
    if let Ok(mut stmt) = conn.prepare(note_sql) {
        if let Ok(rows) = stmt.query_map(params![pattern], |row| {
            row.get::<_, String>(0)
        }) {
            for row in rows.flatten() {
                // 只取前 100 字符作为主题提示
                let snippet = row.chars().take(100).collect::<String>();
                if !snippet.trim().is_empty() {
                    ctx.previous_topics.push(snippet);
                }
            }
        }
    }

    // 检索相关素材名（最多 5 个）
    let asset_sql = "SELECT a.name FROM assets a
         INNER JOIN projects p ON p.id = a.project_id
         WHERE p.name LIKE ?1 OR p.description LIKE ?1
         ORDER BY a.imported_at DESC LIMIT 5";

    if let Ok(mut stmt) = conn.prepare(asset_sql) {
        if let Ok(rows) = stmt.query_map(params![pattern], |row| {
            row.get::<_, String>(0)
        }) {
            for row in rows.flatten() {
                ctx.related_assets.push(row);
            }
        }
    }

    ctx
}
