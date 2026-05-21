use crate::db::knowledge::get_concept_detail as db_get_concept_detail;
use crate::db::knowledge_understanding::{
    get_explanation, get_relations, get_summary, get_user_note, save_explanation, save_mirror_feedback,
    save_summary, save_user_explanation, ConceptExplanation, ConceptRelation, ConceptSummary,
    ConceptUserNote,
};
use crate::db::Database;
use crate::llm::chat::{chat_completion, ChatMessage};
use crate::llm::client::LLMClient;
use crate::llm::prompts::{
    build_explanation_prompt, build_mirror_prompt, build_summary_prompt, DocumentSection,
    ExcerptItem, KeyPoint,
};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

// ─────────────────────────────────────────────────────────────────────────────
// 错误类型
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "message")]
pub enum KnowledgeError {
    DatabaseError(String),
    LlmError(String),
    InvalidLlmResponse(String),
    NotFound(String),
}

impl std::fmt::Display for KnowledgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KnowledgeError::DatabaseError(m) => write!(f, "DatabaseError: {m}"),
            KnowledgeError::LlmError(m) => write!(f, "LlmError: {m}"),
            KnowledgeError::InvalidLlmResponse(m) => write!(f, "InvalidLlmResponse: {m}"),
            KnowledgeError::NotFound(m) => write!(f, "NotFound: {m}"),
        }
    }
}

impl From<String> for KnowledgeError {
    fn from(s: String) -> Self {
        KnowledgeError::DatabaseError(s)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Event payload
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChunkPayload {
    concept_id: String,
    chunk: String,
    is_final: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Response types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnderstandingData {
    pub summary: Option<ConceptSummary>,
    pub explanation: Option<ConceptExplanation>,
    pub user_note: Option<ConceptUserNote>,
}

// ─────────────────────────────────────────────────────────────────────────────
// LLM Explanation JSON shape (Prompt 2 return schema)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ExplanationLlmResponse {
    mechanism: MechanismField,
    #[serde(default)]
    scenarios: Vec<SourcedText>,
    #[serde(default)]
    misconceptions: Vec<SourcedText>,
    #[serde(default)]
    essence: String,
}

#[derive(Debug, Deserialize)]
struct MechanismField {
    text: String,
    source: String,
}

#[derive(Debug, Deserialize)]
struct SourcedText {
    text: String,
    source: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Command 1: knowledge_get_understanding_data
// ─────────────────────────────────────────────────────────────────────────────

/// 读取概念的理解辅助数据（摘要 + 理解框架 + 用户笔记），纯数据库查询，无 LLM 调用
#[tauri::command]
pub fn knowledge_get_understanding_data(
    db: State<'_, Database>,
    concept_id: String,
) -> Result<UnderstandingData, String> {
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    let summary = get_summary(&conn, &concept_id)?;
    let explanation = get_explanation(&conn, &concept_id)?;
    let user_note = get_user_note(&conn, &concept_id)?;
    Ok(UnderstandingData {
        summary,
        explanation,
        user_note,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Command 2: knowledge_generate_summary
// ─────────────────────────────────────────────────────────────────────────────

/// 生成文档整合摘要。缓存未命中或 force_regenerate=true 时调用 LLM，通过
/// "knowledge:summary:chunk" 事件流式推送。
#[tauri::command]
pub async fn knowledge_generate_summary(
    db: State<'_, Database>,
    app: tauri::AppHandle,
    concept_id: String,
    force_regenerate: bool,
) -> Result<String, String> {
    // 缓存检查
    if !force_regenerate {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        if get_summary(&conn, &concept_id)?.is_some() {
            return Ok("cached".to_string());
        }
    }

    // 读取 LLM 客户端 + 概念信息
    let (client, concept_name, excerpts) = {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        let client = LLMClient::from_db_or_env(&conn)?;
        let detail = db_get_concept_detail(&conn, &concept_id)?
            .ok_or_else(|| format!("概念不存在: {concept_id}"))?;
        let concept_name = detail.concept.name.clone();

        // 从 cases 构建 ExcerptItem 列表
        let excerpts: Vec<ExcerptItem> = detail
            .cases
            .iter()
            .map(|c| ExcerptItem {
                asset_name: c.title.clone(),
                project_name: String::new(),
                text: c.excerpt.clone(),
            })
            .collect();

        (client, concept_name, excerpts)
    };

    let prompt = build_summary_prompt(&concept_name, &excerpts);
    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: "You are a document synthesis engine for a student's knowledge management app. Your task is to integrate information from multiple document excerpts about the same concept into a coherent summary. ONLY use information from the provided excerpts. Do NOT add any external knowledge.".to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: prompt,
        },
    ];

    // 调用 LLM（当前使用非流式，结果一次性推送为 final chunk）
    let result = chat_completion(&client, messages)
        .await
        .map_err(|e| format!("LLM 调用失败: {e}"))?;

    // 推送 chunk event
    let _ = app.emit(
        "knowledge:summary:chunk",
        ChunkPayload {
            concept_id: concept_id.clone(),
            chunk: result.clone(),
            is_final: true,
        },
    );

    // 写入数据库
    let now = chrono::Utc::now().to_rfc3339();
    let summary = ConceptSummary {
        id: uuid::Uuid::new_v4().to_string(),
        concept_id: concept_id.clone(),
        summary: result.clone(),
        source_asset_ids: vec![],
        model: client.model.clone(),
        generated_at: now,
    };
    {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        save_summary(&conn, &summary)?;
    }

    Ok("generated".to_string())
}

// ─────────────────────────────────────────────────────────────────────────────
// Command 3: knowledge_generate_explanation
// ─────────────────────────────────────────────────────────────────────────────

/// 生成理解框架（结构化 JSON）。LLM 返回必须是合法 JSON，且 mechanism.source 非空。
#[tauri::command]
pub async fn knowledge_generate_explanation(
    db: State<'_, Database>,
    app: tauri::AppHandle,
    concept_id: String,
    force_regenerate: bool,
) -> Result<String, String> {
    // 缓存检查
    if !force_regenerate {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        if get_explanation(&conn, &concept_id)?.is_some() {
            return Ok("cached".to_string());
        }
    }

    // 读取 LLM 客户端 + 概念信息
    let (client, concept_name, definition, sections) = {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        let client = LLMClient::from_db_or_env(&conn)?;
        let detail = db_get_concept_detail(&conn, &concept_id)?
            .ok_or_else(|| format!("概念不存在: {concept_id}"))?;
        let concept_name = detail.concept.name.clone();
        let definition = detail
            .concept
            .definition
            .clone()
            .unwrap_or_default();

        // 从 cases 构建 DocumentSection
        let sections: Vec<DocumentSection> = detail
            .cases
            .iter()
            .map(|c| DocumentSection {
                project_name: String::new(),
                asset_name: c.title.clone(),
                content: c.excerpt.clone(),
            })
            .collect();

        (client, concept_name, definition, sections)
    };

    let prompt = build_explanation_prompt(&concept_name, &definition, &sections);
    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: "You are a knowledge explanation engine for a student's learning app. You help students understand concepts they've encountered in their documents. CRITICAL RULES: You MUST ONLY use information from the student's documents provided. Do NOT introduce any information not present in these documents. For EVERY explanatory point, you MUST cite the source document. Do NOT fabricate examples, mechanisms, or explanations.".to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: prompt,
        },
    ];

    let result = chat_completion(&client, messages)
        .await
        .map_err(|e| format!("LLM 调用失败: {e}"))?;

    // 推送 chunk event
    let _ = app.emit(
        "knowledge:explanation:chunk",
        ChunkPayload {
            concept_id: concept_id.clone(),
            chunk: result.clone(),
            is_final: true,
        },
    );

    // 解析 JSON
    let json_start = result.find('{').unwrap_or(0);
    let json_end = result.rfind('}').map(|i| i + 1).unwrap_or(result.len());
    let llm_json: ExplanationLlmResponse =
        serde_json::from_str(&result[json_start..json_end]).map_err(|e| {
            format!(
                "Invalid LLM response: JSON 解析失败 ({e}); 原始内容前200字: {}",
                &result.chars().take(200).collect::<String>()
            )
        })?;

    // 校验 mechanism.source 非空
    if llm_json.mechanism.source.trim().is_empty() {
        return Err("Invalid LLM response: mechanism.source 为空，拒绝写入数据库".to_string());
    }

    // 序列化子字段
    let mechanism_json = serde_json::to_string(&serde_json::json!({
        "text": llm_json.mechanism.text,
        "source": llm_json.mechanism.source,
    }))
    .unwrap_or_default();

    let scenarios_json = serde_json::to_string(
        &llm_json
            .scenarios
            .iter()
            .map(|s| serde_json::json!({"text": s.text, "source": s.source}))
            .collect::<Vec<_>>(),
    )
    .unwrap_or_else(|_| "[]".to_string());

    let misconceptions_json = if llm_json.misconceptions.is_empty() {
        None
    } else {
        Some(
            serde_json::to_string(
                &llm_json
                    .misconceptions
                    .iter()
                    .map(|m| serde_json::json!({"text": m.text, "source": m.source}))
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_else(|_| "[]".to_string()),
        )
    };

    let now = chrono::Utc::now().to_rfc3339();
    let explanation = ConceptExplanation {
        id: uuid::Uuid::new_v4().to_string(),
        concept_id: concept_id.clone(),
        mechanism: mechanism_json,
        typical_scenarios: scenarios_json,
        common_misconceptions: misconceptions_json,
        essence_sentence: llm_json.essence,
        source_asset_ids: vec![],
        model: client.model.clone(),
        generated_at: now,
    };

    {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        save_explanation(&conn, &explanation)?;
    }

    Ok("generated".to_string())
}

// ─────────────────────────────────────────────────────────────────────────────
// Command 4: knowledge_validate_explanation
// ─────────────────────────────────────────────────────────────────────────────

/// AI 镜子反馈：不缓存，每次都调用 LLM；结果存入 concept_user_notes.mirror_feedback
#[tauri::command]
pub async fn knowledge_validate_explanation(
    db: State<'_, Database>,
    app: tauri::AppHandle,
    concept_id: String,
    user_explanation: String,
) -> Result<String, String> {
    // 读取 LLM 客户端 + 概念信息
    let (client, concept_name, key_points) = {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        let client = LLMClient::from_db_or_env(&conn)?;
        let detail = db_get_concept_detail(&conn, &concept_id)?
            .ok_or_else(|| format!("概念不存在: {concept_id}"))?;
        let concept_name = detail.concept.name.clone();

        // 从 cases 构建 KeyPoint
        let key_points: Vec<KeyPoint> = detail
            .cases
            .iter()
            .map(|c| KeyPoint {
                text: c.excerpt.clone(),
                source: c.title.clone(),
            })
            .collect();

        (client, concept_name, key_points)
    };

    let prompt = build_mirror_prompt(&concept_name, &user_explanation, &key_points);
    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: "You are a gentle learning companion helping a student check their understanding of a concept. Your job is to compare their explanation against their own documents — NOT against any external standard. CRITICAL RULES: Compare ONLY against the provided documents. Use encouraging, exploratory language. NEVER use words like 'wrong', 'incorrect', 'incomplete', 'missing', 'failed to'. Acknowledge what the student captured correctly first. Present any uncovered points as additional perspectives, not as mistakes.".to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: prompt,
        },
    ];

    let result = chat_completion(&client, messages)
        .await
        .map_err(|e| format!("LLM 调用失败: {e}"))?;

    // 推送 chunk event
    let _ = app.emit(
        "knowledge:mirror:chunk",
        ChunkPayload {
            concept_id: concept_id.clone(),
            chunk: result.clone(),
            is_final: true,
        },
    );

    // 存入数据库
    let now = chrono::Utc::now().to_rfc3339();
    {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        save_mirror_feedback(&conn, &concept_id, &result, &now)?;
    }

    Ok(result)
}

// ─────────────────────────────────────────────────────────────────────────────
// Command 5: knowledge_save_user_note
// ─────────────────────────────────────────────────────────────────────────────

/// 保存用户个人理解笔记（只更新 user_explanation，不动 mirror_feedback）
#[tauri::command]
pub fn knowledge_save_user_note(
    db: State<'_, Database>,
    concept_id: String,
    user_explanation: String,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    let now = chrono::Utc::now().to_rfc3339();
    save_user_explanation(&conn, &concept_id, &user_explanation, &now)?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Command 6: knowledge_get_relations
// ─────────────────────────────────────────────────────────────────────────────

/// 获取概念关系网络（按共现次数降序，最多 8 条）
#[tauri::command]
pub fn knowledge_get_relations(
    db: State<'_, Database>,
    concept_id: String,
) -> Result<Vec<ConceptRelation>, String> {
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    get_relations(&conn, &concept_id)
}
