/// 技能形成系统命令（Step 10）
///
/// 能力域聚合、进度计算、开放式场景验证（宪章 K8：不用选择题）
///
/// 命令列表：
///   skill_get_list          — 获取知识库所有技能
///   skill_get_detail        — 获取单个技能详情
///   skill_create            — 手动创建技能（能力域）
///   skill_auto_aggregate    — LLM 自动聚合：按 inferred_course 把 KU 归入技能
///   skill_compute_progress  — 重算进度（validated/mastered KU 比例）
///   skill_generate_challenge — 生成一道开放式场景题
///   skill_evaluate_answer   — 评判用户作答，更新状态
///   skill_delete            — 删除技能

use crate::db::knowledge_units::get_knowledge_unit;
use crate::db::skills::{
    delete_skill, get_skill, get_skills, insert_skill, update_skill_challenge,
    update_skill_evaluation, update_skill_ku_ids, update_skill_progress, Skill,
};
use crate::db::Database;
use crate::llm::chat::{chat_completion, ChatMessage};
use crate::llm::client::LLMClient;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

// ─── 辅助类型 ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillChallenge {
    pub scenario: String,          // 情景描述
    pub question: String,          // 开放式问题
    pub evaluation_hints: Vec<String>, // 评判要点（LLM 内部参考，不展示给用户）
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillEvaluation {
    pub quality_score: f64,           // 0.0–1.0
    pub covered_points: Vec<String>,  // 答到的要点
    pub missed_points: Vec<String>,   // 遗漏的要点
    pub feedback: String,             // 温和的综合反馈
    pub status_transition: String,    // "learning"|"practicing"|"verified"
    pub evaluated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSkillInput {
    pub library_id: String,
    pub name: String,
    pub description: Option<String>,
    pub ku_ids: Vec<String>,
}

// ─── 基础 CRUD ────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn skill_get_list(
    db: State<'_, Database>,
    library_id: String,
) -> Result<Vec<Skill>, String> {
    let conn = db.conn.lock().map_err(|e| format!("数据库锁: {e}"))?;
    get_skills(&conn, &library_id)
}

#[tauri::command]
pub fn skill_get_detail(
    db: State<'_, Database>,
    skill_id: String,
) -> Result<Option<Skill>, String> {
    let conn = db.conn.lock().map_err(|e| format!("数据库锁: {e}"))?;
    get_skill(&conn, &skill_id)
}

#[tauri::command]
pub fn skill_create(
    db: State<'_, Database>,
    input: CreateSkillInput,
) -> Result<Skill, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let skill = Skill {
        id: uuid::Uuid::new_v4().to_string(),
        library_id: input.library_id,
        name: input.name,
        description: input.description,
        ku_ids: input.ku_ids,
        status: "learning".to_string(),
        progress: 0.0,
        last_challenge: None,
        last_evaluation: None,
        verified_at: None,
        created_at: now.clone(),
        updated_at: now,
    };
    let conn = db.conn.lock().map_err(|e| format!("数据库锁: {e}"))?;
    insert_skill(&conn, &skill)?;
    Ok(skill)
}

#[tauri::command]
pub fn skill_delete(
    db: State<'_, Database>,
    skill_id: String,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("数据库锁: {e}"))?;
    delete_skill(&conn, &skill_id)
}

// ─── 技能绑定 KU ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn skill_update_ku_ids(
    db: State<'_, Database>,
    skill_id: String,
    ku_ids: Vec<String>,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let conn = db.conn.lock().map_err(|e| format!("数据库锁: {e}"))?;
    let json = serde_json::to_string(&ku_ids).map_err(|e| format!("序列化失败: {e}"))?;
    update_skill_ku_ids(&conn, &skill_id, &json, &now)
}

// ─── 进度计算 ─────────────────────────────────────────────────────────────────

/// 重新计算技能进度：validated + mastered KU 数 / 总 KU 数
/// 同时根据进度更新状态：
///   progress < 0.5  → learning
///   0.5 ≤ p < 1.0   → practicing
///   p >= 1.0        → practicing（需要手动触发验证才能变 verified）
#[tauri::command]
pub fn skill_compute_progress(
    db: State<'_, Database>,
    skill_id: String,
) -> Result<f64, String> {
    let conn = db.conn.lock().map_err(|e| format!("数据库锁: {e}"))?;
    let skill = get_skill(&conn, &skill_id)?
        .ok_or_else(|| format!("技能不存在: {skill_id}"))?;

    if skill.ku_ids.is_empty() {
        return Ok(0.0);
    }

    let mut done = 0usize;
    for ku_id in &skill.ku_ids {
        let status: Option<String> = conn
            .query_row(
                "SELECT status FROM knowledge_units WHERE id = ?1",
                params![ku_id],
                |r| r.get(0),
            )
            .ok();
        if matches!(status.as_deref(), Some("validated") | Some("consolidated") | Some("mastered")) {
            done += 1;
        }
    }

    let progress = done as f64 / skill.ku_ids.len() as f64;
    let new_status = if skill.status == "verified" {
        "verified" // 已验证不降级
    } else if progress >= 0.5 {
        "practicing"
    } else {
        "learning"
    };

    let now = chrono::Utc::now().to_rfc3339();
    update_skill_progress(&conn, &skill_id, progress, new_status, &now)?;
    Ok(progress)
}

// ─── LLM 自动聚合 ─────────────────────────────────────────────────────────────

/// 按 inferred_course 自动聚合：为每个课程创建一个技能，把对应 KU 归入
/// 如果该课程已有技能则更新 ku_ids
#[tauri::command]
pub async fn skill_auto_aggregate(
    db: State<'_, Database>,
    library_id: String,
) -> Result<u32, String> {
    // 收集 (inferred_course, ku_id) 对
    let course_ku_pairs: Vec<(String, String)> = {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁: {e}"))?;
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT ai.inferred_course, ku.id
                 FROM knowledge_units ku
                 INNER JOIN assets a ON a.id IN (
                     SELECT value FROM json_each(ku.source_asset_ids)
                 )
                 INNER JOIN asset_inferences ai ON ai.asset_id = a.id
                 WHERE ku.library_id = ?1
                   AND ai.inferred_course IS NOT NULL",
            )
            .map_err(|e| format!("聚合查询失败: {e}"))?;

        let x = stmt
            .query_map(params![library_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(|e| format!("聚合遍历失败: {e}"))?
            .filter_map(|r| r.ok())
            .collect();
        x
    };

    // 按课程分组
    let mut course_to_kus: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (course, ku_id) in course_ku_pairs {
        course_to_kus.entry(course).or_default().push(ku_id);
    }

    let mut created = 0u32;
    for (course, ku_ids) in course_to_kus {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁: {e}"))?;

        // 检查是否已有同名技能
        let existing_id: Option<String> = conn
            .query_row(
                "SELECT id FROM skills WHERE library_id = ?1 AND name = ?2",
                params![library_id, course],
                |r| r.get(0),
            )
            .ok();

        let now = chrono::Utc::now().to_rfc3339();
        if let Some(sid) = existing_id {
            let json = serde_json::to_string(&ku_ids).unwrap_or_default();
            update_skill_ku_ids(&conn, &sid, &json, &now)?;
        } else {
            let skill = Skill {
                id: uuid::Uuid::new_v4().to_string(),
                library_id: library_id.clone(),
                name: course.clone(),
                description: None,
                ku_ids: ku_ids.clone(),
                status: "learning".to_string(),
                progress: 0.0,
                last_challenge: None,
                last_evaluation: None,
                verified_at: None,
                created_at: now.clone(),
                updated_at: now,
            };
            insert_skill(&conn, &skill)?;
            created += 1;
        }
    }

    Ok(created)
}

// ─── 场景题生成 ───────────────────────────────────────────────────────────────

/// 生成一道开放式场景题（宪章 K8：不用选择题）
/// 基于技能中各 KU 的摘要和核心洞见
#[tauri::command]
pub async fn skill_generate_challenge(
    db: State<'_, Database>,
    skill_id: String,
) -> Result<SkillChallenge, String> {
    let (client, skill_name, ku_summaries) = {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁: {e}"))?;
        let client = LLMClient::from_db_or_env(&conn)?;
        let skill = get_skill(&conn, &skill_id)?
            .ok_or_else(|| format!("技能不存在: {skill_id}"))?;

        // 收集关联 KU 的摘要
        let mut summaries: Vec<String> = Vec::new();
        for ku_id in skill.ku_ids.iter().take(6) {
            if let Ok(Some(ku)) = get_knowledge_unit(&conn, ku_id) {
                let entry = format!(
                    "【{}】{}\n{}",
                    ku.title,
                    ku.core_insight,
                    ku.summary.as_deref().unwrap_or("").chars().take(300).collect::<String>()
                );
                summaries.push(entry);
            }
        }
        (client, skill.name.clone(), summaries)
    };

    let ku_context = ku_summaries.join("\n\n---\n\n");

    let prompt = [
        "# 技能验证场景题生成\n\n",
        "## 技能领域：", &skill_name, "\n\n",
        "## 相关知识内容：\n\n", &ku_context, "\n\n",
        "## 任务\n",
        "基于以上知识内容，设计一道真实的开放式场景题。\n",
        "要求：\n",
        "1. 题目是一个真实场景（不是「请解释……」的填空题）\n",
        "2. 需要综合运用以上多个知识点才能回答\n",
        "3. 没有标准答案，考察分析思路\n",
        "4. 不使用选择题形式\n\n",
        "输出合法 JSON：\n",
        "{\n",
        "  \"scenario\": \"情景描述（2-4句话）\",\n",
        "  \"question\": \"开放式问题（1句话）\",\n",
        "  \"evaluation_hints\": [\"评判要点1\", \"评判要点2\", \"要点3\"]\n",
        "}\n\n",
        "只输出 JSON。",
    ].concat();

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: "你是一位考试设计专家，擅长设计考察真实理解的开放式场景题。".to_string(),
        },
        ChatMessage { role: "user".to_string(), content: prompt },
    ];

    let result = chat_completion(&client, messages)
        .await
        .map_err(|e| format!("LLM 调用失败: {e}"))?;

    // 解析 JSON
    let challenge: SkillChallenge = {
        let json_str = extract_json(&result);
        let mut c: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| format!("场景题 JSON 解析失败: {e}\n原始: {result}"))?;
        let now = chrono::Utc::now().to_rfc3339();
        c["generatedAt"] = serde_json::Value::String(now.clone());
        serde_json::from_value(c).map_err(|e| format!("场景题反序列化失败: {e}"))?
    };

    // 持久化
    let challenge_json = serde_json::to_string(&challenge)
        .map_err(|e| format!("序列化失败: {e}"))?;
    let now = chrono::Utc::now().to_rfc3339();
    {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁: {e}"))?;
        update_skill_challenge(&conn, &skill_id, &challenge_json, &now)?;
    }

    Ok(challenge)
}

// ─── 作答评判 ─────────────────────────────────────────────────────────────────

/// 评判用户对场景题的作答
/// 评分基于：用户自己的 KU 知识内容（不依赖外部知识）
#[tauri::command]
pub async fn skill_evaluate_answer(
    db: State<'_, Database>,
    app: tauri::AppHandle,
    skill_id: String,
    user_answer: String,
) -> Result<SkillEvaluation, String> {
    let (client, skill_name, challenge_json, ku_summaries) = {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁: {e}"))?;
        let client = LLMClient::from_db_or_env(&conn)?;
        let skill = get_skill(&conn, &skill_id)?
            .ok_or_else(|| format!("技能不存在: {skill_id}"))?;

        let challenge_json = skill
            .last_challenge
            .ok_or("尚未生成场景题，请先调用 skill_generate_challenge")?;

        let mut summaries: Vec<String> = Vec::new();
        for ku_id in skill.ku_ids.iter().take(6) {
            if let Ok(Some(ku)) = get_knowledge_unit(&conn, ku_id) {
                let entry = format!(
                    "【{}】{}\n{}",
                    ku.title,
                    ku.core_insight,
                    ku.summary.as_deref().unwrap_or("").chars().take(400).collect::<String>()
                );
                summaries.push(entry);
            }
        }
        (client, skill.name.clone(), challenge_json, summaries)
    };

    let challenge: SkillChallenge = serde_json::from_str(&challenge_json)
        .map_err(|e| format!("场景题反序列化失败: {e}"))?;

    let ku_context = ku_summaries.join("\n\n---\n\n");

    let prompt = [
        "# 技能验证作答评判\n\n",
        "## 技能领域：", &skill_name, "\n\n",
        "## 题目情景：\n", &challenge.scenario, "\n\n",
        "## 题目问题：\n", &challenge.question, "\n\n",
        "## 评判要点（供参考）：\n",
        &challenge.evaluation_hints.iter()
            .enumerate()
            .map(|(i, h)| format!("{}. {}", i + 1, h))
            .collect::<Vec<_>>()
            .join("\n"),
        "\n\n",
        "## 知识库参考内容（仅限此范围评判）：\n\n", &ku_context, "\n\n",
        "## 学生作答：\n", &user_answer, "\n\n",
        "## 评判任务\n",
        "基于上述知识内容，温和客观地评判学生的作答质量。\n",
        "规则：\n",
        "- 只参考知识库内容，不添加外部标准\n",
        "- 积极认可答到的要点\n",
        "- 温和指出遗漏的视角\n",
        "- quality_score: 0.0-1.0（>=0.75 视为达到验证水平）\n",
        "- status_transition: \"practicing\"（质量不足）或 \"verified\"（质量>=0.75）\n\n",
        "输出合法 JSON：\n",
        "{\n",
        "  \"quality_score\": 0.82,\n",
        "  \"covered_points\": [\"答到的要点1\", \"要点2\"],\n",
        "  \"missed_points\": [\"遗漏的视角1\"],\n",
        "  \"feedback\": \"整体反馈（2-3句话，温和风格）\",\n",
        "  \"status_transition\": \"verified\"\n",
        "}\n\n",
        "只输出 JSON。",
    ].concat();

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: "你是一位温和的学习评估专家，只基于学生自己的知识库内容进行评判，不引入外部标准。".to_string(),
        },
        ChatMessage { role: "user".to_string(), content: prompt },
    ];

    let result = chat_completion(&client, messages)
        .await
        .map_err(|e| format!("LLM 调用失败: {e}"))?;

    // 解析 JSON
    let evaluation: SkillEvaluation = {
        let json_str = extract_json(&result);
        let mut val: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| format!("评判 JSON 解析失败: {e}\n原始: {result}"))?;
        let now = chrono::Utc::now().to_rfc3339();
        val["evaluatedAt"] = serde_json::Value::String(now);
        serde_json::from_value(val).map_err(|e| format!("评判反序列化失败: {e}"))?
    };

    // 持久化 + 状态转换
    let eval_json = serde_json::to_string(&evaluation)
        .map_err(|e| format!("序列化失败: {e}"))?;
    let now = chrono::Utc::now().to_rfc3339();
    let verified_at = if evaluation.status_transition == "verified" {
        Some(now.as_str())
    } else {
        None
    };

    {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁: {e}"))?;
        update_skill_evaluation(
            &conn,
            &skill_id,
            &eval_json,
            &evaluation.status_transition,
            verified_at,
            &now,
        )?;
    }

    // 通知前端
    let _ = app.emit("notecapt/skill-evaluated", &skill_id);

    Ok(evaluation)
}

// ─── 工具函数 ─────────────────────────────────────────────────────────────────

/// 从 LLM 输出中提取 JSON 字符串（去掉 markdown 代码块包裹）
fn extract_json(s: &str) -> String {
    let trimmed = s.trim();
    // 去掉 ```json ... ``` 包裹
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            return trimmed[start..=end].to_string();
        }
    }
    trimmed.to_string()
}
