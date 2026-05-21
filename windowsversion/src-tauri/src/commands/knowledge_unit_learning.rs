/// 知识单元学习命令（Step 6）
///
/// 为 KnowledgeUnit 提供流式摘要/理解框架/镜子反馈生成，
/// 复用现有 LLM 基础设施（llm::chat / llm::client / llm::prompts）
///
/// 事件名（与前端 KnowledgeUnitPage 对应）：
///   notecapt/ku-summary-chunk
///   notecapt/ku-explanation-chunk
///   notecapt/ku-mirror-chunk

use crate::db::knowledge_units::{
    get_knowledge_unit, update_knowledge_unit_explanation,
    update_knowledge_unit_mirror_feedback, update_knowledge_unit_summary,
};
use crate::db::Database;
use crate::llm::chat::{chat_completion, ChatMessage};
use crate::llm::client::LLMClient;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

// ─── Event payload ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KuChunkPayload {
    knowledge_unit_id: String,
    chunk: String,
    is_final: bool,
}

// ─── 摘要生成 ─────────────────────────────────────────────────────────────────

/// 为指定知识单元生成摘要（基于关联素材的已提取文本）
///
/// 流式事件：`notecapt/ku-summary-chunk`
#[tauri::command]
pub async fn ku_generate_summary(
    db: State<'_, Database>,
    app: tauri::AppHandle,
    knowledge_unit_id: String,
    force_regenerate: bool,
) -> Result<String, String> {
    // 缓存检查
    if !force_regenerate {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        if let Some(unit) = get_knowledge_unit(&conn, &knowledge_unit_id)? {
            if unit.summary.is_some() {
                return Ok("cached".to_string());
            }
        }
    }

    // 读取 KU + LLM 客户端 + 来源文本
    let (client, title, core_insight, source_texts) = {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        let client = LLMClient::from_db_or_env(&conn)?;
        let unit = get_knowledge_unit(&conn, &knowledge_unit_id)?
            .ok_or_else(|| format!("知识单元不存在: {knowledge_unit_id}"))?;

        let source_texts = fetch_source_texts(&conn, &unit.source_asset_ids)?;
        (client, unit.title.clone(), unit.core_insight.clone(), source_texts)
    };

    let prompt = build_ku_summary_prompt(&title, &core_insight, &source_texts);
    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: "你是一个知识整合助手，专门帮助学生理解自己的学习材料。\
            只使用用户提供的文档内容生成摘要，不添加外部知识。".to_string(),
        },
        ChatMessage { role: "user".to_string(), content: prompt },
    ];

    let result = chat_completion(&client, messages)
        .await
        .map_err(|e| format!("LLM 调用失败: {e}"))?;

    // 发送流式事件
    let _ = app.emit(
        "notecapt/ku-summary-chunk",
        KuChunkPayload {
            knowledge_unit_id: knowledge_unit_id.clone(),
            chunk: result.clone(),
            is_final: true,
        },
    );

    // 持久化
    let now = chrono::Utc::now().to_rfc3339();
    {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        update_knowledge_unit_summary(&conn, &knowledge_unit_id, &result, &now)?;
    }

    Ok("generated".to_string())
}

// ─── 理解框架生成 ─────────────────────────────────────────────────────────────

/// 为指定知识单元生成理解框架（JSON 格式：机制/场景/误区/精华）
///
/// 流式事件：`notecapt/ku-explanation-chunk`
#[tauri::command]
pub async fn ku_generate_explanation(
    db: State<'_, Database>,
    app: tauri::AppHandle,
    knowledge_unit_id: String,
) -> Result<String, String> {
    let (client, title, core_insight, source_texts) = {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        let client = LLMClient::from_db_or_env(&conn)?;
        let unit = get_knowledge_unit(&conn, &knowledge_unit_id)?
            .ok_or_else(|| format!("知识单元不存在: {knowledge_unit_id}"))?;
        let source_texts = fetch_source_texts(&conn, &unit.source_asset_ids)?;
        (client, unit.title.clone(), unit.core_insight.clone(), source_texts)
    };

    let prompt = build_ku_explanation_prompt(&title, &core_insight, &source_texts);
    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: "你是一个学习框架构建助手。基于学生的学习材料，生成结构化的理解框架。\
            只引用用户文档中的内容，严格输出合法 JSON，不添加外部知识。".to_string(),
        },
        ChatMessage { role: "user".to_string(), content: prompt },
    ];

    let result = chat_completion(&client, messages)
        .await
        .map_err(|e| format!("LLM 调用失败: {e}"))?;

    // 发送事件
    let _ = app.emit(
        "notecapt/ku-explanation-chunk",
        KuChunkPayload {
            knowledge_unit_id: knowledge_unit_id.clone(),
            chunk: result.clone(),
            is_final: true,
        },
    );

    // 持久化（存储原始 JSON 字符串）
    let now = chrono::Utc::now().to_rfc3339();
    {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        update_knowledge_unit_explanation(&conn, &knowledge_unit_id, &result, &now)?;
    }

    Ok("generated".to_string())
}

// ─── 镜子反馈生成 ─────────────────────────────────────────────────────────────

/// 对比用户笔记与知识单元内容，生成镜子反馈
///
/// 流式事件：`notecapt/ku-mirror-chunk`
#[tauri::command]
pub async fn ku_validate_explanation(
    db: State<'_, Database>,
    app: tauri::AppHandle,
    knowledge_unit_id: String,
    user_explanation: String,
) -> Result<String, String> {
    let (client, title, summary) = {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        let client = LLMClient::from_db_or_env(&conn)?;
        let unit = get_knowledge_unit(&conn, &knowledge_unit_id)?
            .ok_or_else(|| format!("知识单元不存在: {knowledge_unit_id}"))?;
        (client, unit.title.clone(), unit.summary.unwrap_or_default())
    };

    let prompt = build_ku_mirror_prompt(&title, &summary, &user_explanation);
    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: "你是一个温和的学习镜子，不评分、不评判，只客观对比用户理解与文档内容。\
            严格输出合法 JSON，不添加主观评价。".to_string(),
        },
        ChatMessage { role: "user".to_string(), content: prompt },
    ];

    let result = chat_completion(&client, messages)
        .await
        .map_err(|e| format!("LLM 调用失败: {e}"))?;

    // 发送事件
    let _ = app.emit(
        "notecapt/ku-mirror-chunk",
        KuChunkPayload {
            knowledge_unit_id: knowledge_unit_id.clone(),
            chunk: result.clone(),
            is_final: true,
        },
    );

    // 持久化
    let now = chrono::Utc::now().to_rfc3339();
    {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        update_knowledge_unit_mirror_feedback(&conn, &knowledge_unit_id, &result, &now)?;
    }

    Ok("generated".to_string())
}

// ─── staleness 检查 ────────────────────────────────────────────────────────────

/// 检查知识单元的摘要是否因新增素材而过期
/// 返回 true = 已有新素材未纳入摘要
#[tauri::command]
pub fn ku_check_staleness(
    db: State<'_, Database>,
    knowledge_unit_id: String,
) -> Result<bool, String> {
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    let unit = get_knowledge_unit(&conn, &knowledge_unit_id)?;
    let unit = match unit {
        Some(u) => u,
        None => return Ok(false),
    };

    // 检查是否有 constituent concepts 在摘要生成后新增了 source_asset
    // 简化实现：比对 source_asset_ids 数量是否超过摘要时记录的数量
    // 真正的 staleness tracking 在 Step 8 中通过 UnderstandingSnapshot 完成
    if unit.summary.is_none() {
        return Ok(false);
    }

    let latest_snapshot_asset_count: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(source_asset_count_at_time), 0)
             FROM understanding_snapshots
             WHERE knowledge_unit_id = ?1",
            params![knowledge_unit_id],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let current_count = unit.source_asset_ids.len() as i64;
    Ok(current_count > latest_snapshot_asset_count && latest_snapshot_asset_count > 0)
}

// ─── Prompt 构建 ──────────────────────────────────────────────────────────────

fn build_ku_summary_prompt(title: &str, core_insight: &str, source_texts: &[(String, String)]) -> String {
    let mut parts = vec![
        format!("# 知识单元摘要生成请求\n\n知识单元：{}\n核心洞见：{}\n\n", title, core_insight),
        "## 来源文档内容：\n\n".to_string(),
    ];
    for (asset_name, text) in source_texts.iter().take(5) {
        parts.push(format!("### 素材：{}\n{}\n\n", asset_name, text.chars().take(1500).collect::<String>()));
    }
    parts.push(
        "## 任务\n\
        综合以上文档内容，用中文写一段 200-300 字的整合摘要，说明这个知识单元的核心内容。\n\
        要求：\n\
        1. 只使用上述文档中的信息\n\
        2. 每个关键点标注来自哪个素材\n\
        3. 语言简洁清晰，面向学生\n\
        直接输出摘要文本，不需要其他格式。".to_string(),
    );
    parts.concat()
}

fn build_ku_explanation_prompt(title: &str, core_insight: &str, source_texts: &[(String, String)]) -> String {
    let mut parts = vec![
        format!("# 知识单元理解框架生成\n\n知识单元：{}\n核心洞见：{}\n\n", title, core_insight),
        "## 来源文档内容：\n\n".to_string(),
    ];
    for (asset_name, text) in source_texts.iter().take(4) {
        parts.push(format!("### 素材：{}\n{}\n\n", asset_name, text.chars().take(1000).collect::<String>()));
    }
    parts.push(
        "## 任务\n\
        基于以上文档，生成理解框架，输出合法 JSON：\n\
        {\n\
          \"mechanism\": {\"text\": \"核心机制描述\", \"source\": \"来源素材名\"},\n\
          \"typicalScenarios\": [{\"text\": \"场景描述\", \"source\": \"来源素材名\"}],\n\
          \"commonMisconceptions\": [{\"text\": \"误区描述\", \"source\": \"来源素材名\"}],\n\
          \"essenceSentence\": \"一句话精华（不超过30字）\",\n\
          \"sourceAssetIds\": [],\n\
          \"model\": \"gpt-4\",\n\
          \"generatedAt\": \"2026-01-01T00:00:00Z\"\n\
        }\n\n\
        注意：\n\
        - typicalScenarios 提供 2-3 个\n\
        - commonMisconceptions 如没有可返回空数组 []\n\
        - source 字段填写来源素材名称\n\
        - 只使用文档中的内容，不添加外部知识\n\
        只输出 JSON，不要其他文字。".to_string(),
    );
    parts.concat()
}

fn build_ku_mirror_prompt(title: &str, summary: &str, user_explanation: &str) -> String {
    [
        "# 镜子反馈请求\n\n",
        "## 知识单元：", title, "\n\n",
        "## 文档内容摘要：\n", summary, "\n\n",
        "## 学生写的理解：\n", user_explanation, "\n\n",
        "## 任务\n",
        "对比学生的理解与文档内容，生成温和的镜子反馈。\n",
        "输出合法 JSON：\n",
        "{\n",
        "  \"coveredCount\": 3,\n",
        "  \"coveredPoints\": [\"学生提到的要点1\", \"要点2\"],\n",
        "  \"additionalPerspectives\": [{\"text\": \"文档中还有这个角度\", \"source\": \"来源素材\"}],\n",
        "  \"differenceNote\": \"温和说明，如有必要（否则为 null）\"\n",
        "}\n\n",
        "规则：\n",
        "- 不评分、不说「错了」\n",
        "- coveredPoints 积极列举学生说到的\n",
        "- additionalPerspectives 是「补充」而非「纠错」\n",
        "- differenceNote 用温和措辞，或 null\n",
        "只输出 JSON。",
    ].concat()
}

// ─── 内部工具 ─────────────────────────────────────────────────────────────────

/// 从 extracted_content 表中读取多个素材的文本内容
fn fetch_source_texts(
    conn: &rusqlite::Connection,
    asset_ids: &[String],
) -> Result<Vec<(String, String)>, String> {
    let mut result = Vec::new();
    for asset_id in asset_ids {
        let text = crate::db::asset::get_preferred_text_content(conn, asset_id)?;

        let asset_name: String = conn
            .query_row("SELECT name FROM assets WHERE id = ?1", params![asset_id], |r| r.get(0))
            .unwrap_or_else(|_| asset_id.clone());

        if let Some(text) = text {
            if !text.trim().is_empty() {
                result.push((asset_name, text));
            }
        }
    }
    Ok(result)
}
