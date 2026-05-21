/// 知识合成管道 (Step 2)
///
/// 将一个知识库中已提取的原始概念聚类成 10-15 个"知识单元"（洞见级，非词条级）。
/// 流程：
///   Step 2-1  拉取所有概念 + 其来源素材文本片段
///   Step 2-2  调用 LLM 做主题群归纳（优先 LLM 直接归纳，embedding 聚类作 fallback）
///   Step 2-3  为每个主题群生成 KnowledgeUnit（洞见句 title + coreInsight）
///   Step 2-4  写入 knowledge_units 表（幂等：相同 library 先清空，再重写）
///   Step 2-5  emit 进度事件

use crate::db::knowledge::get_concepts_with_stats;
use crate::db::knowledge_units::{
    delete_knowledge_unit, get_knowledge_units_summary, insert_knowledge_unit, CreateKnowledgeUnit,
    KnowledgeUnitSummary,
};
use crate::db::Database;
use crate::llm::chat::{chat_completion, ChatMessage};
use crate::llm::client::LLMClient;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

// ─── 进度事件 ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SynthesisProgress {
    pub library_id: String,
    pub stage: String,   // "clustering" | "naming" | "writing" | "completed" | "error"
    pub groups_found: usize,
    pub units_written: usize,
    pub error: Option<String>,
}

// ─── LLM 解析结构体 ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ConceptGroup {
    group_name: String,
    concept_ids: Vec<String>,
    #[allow(dead_code)]
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MergedGroup {
    group_name: String,
    merged_indices: Vec<usize>,
    #[allow(dead_code)]
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KnowledgeUnitDraft {
    title: String,
    core_insight: String,
}

// ─── 主命令 ──────────────────────────────────────────────────────────────────

/// 触发知识合成管道：从原始概念 → 知识单元
///
/// - `force=true`：删除已有知识单元重新生成
/// - 进度通过 `notecapt/knowledge-synthesis-progress` 事件推送
#[tauri::command]
pub async fn synthesize_knowledge_units(
    db: State<'_, Database>,
    app: tauri::AppHandle,
    library_id: String,
    force: bool,
) -> Result<Vec<KnowledgeUnitSummary>, String> {
    emit_synthesis(&app, &library_id, "clustering", 0, 0, None);

    // 1. 拉取 LLM client + 所有概念
    let (client, concepts) = {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        let client = LLMClient::from_db_or_env(&conn)?;
        let concepts = get_concepts_with_stats(&conn, &library_id)?;
        (client, concepts)
    };

    if concepts.is_empty() {
        emit_synthesis(&app, &library_id, "completed", 0, 0, None);
        return Ok(vec![]);
    }

    // 2. 如果 force，先删除旧知识单元
    if force {
        let existing = {
            let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
            get_knowledge_units_summary(&conn, &library_id)?
        };
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
        for unit in &existing {
            let _ = delete_knowledge_unit(&conn, &unit.id);
        }
    }

    // 3. 分批聚类：单次 prompt 过大时方舟网关会断连，按 CHUNK_SIZE 切块
    const MAX_CONCEPTS_PER_SYNTHESIS: usize = 300;
    const CHUNK_SIZE: usize = 40;
    const DEF_MAX_CHARS: usize = 100;
    let concepts_to_use: Vec<_> = concepts.iter().take(MAX_CONCEPTS_PER_SYNTHESIS).collect();
    let target_count = usize::max(5, usize::min(15, concepts_to_use.len() / 5 + 3));

    fn fmt_concept(c: &crate::db::knowledge::ConceptWithStats, def_max: usize) -> String {
        let def = c.definition.as_deref().unwrap_or("(无定义)");
        let def_trim: String = if def.chars().count() > def_max {
            def.chars().take(def_max).collect::<String>() + "…"
        } else {
            def.to_string()
        };
        format!("- id:{} name:「{}」 def:{}", c.id, c.name, def_trim)
    }

    let chunks: Vec<Vec<&crate::db::knowledge::ConceptWithStats>> = concepts_to_use
        .chunks(CHUNK_SIZE)
        .map(|c| c.to_vec())
        .collect();

    // 3a. 每批局部聚类
    let mut local_groups: Vec<ConceptGroup> = Vec::new();
    for (idx, chunk) in chunks.iter().enumerate() {
        let per_chunk_target = usize::max(3, usize::min(8, chunk.len() / 6 + 2));
        let list = chunk.iter().map(|c| fmt_concept(c, DEF_MAX_CHARS)).collect::<Vec<_>>().join("\n");
        let prompt = format!(
            "以下是 {} 个概念（第 {}/{} 批），请归为约 {} 个主题子群。\n概念：\n{}\n\n\
             输出 JSON 数组：[{{\"group_name\":\"...\",\"concept_ids\":[...],\"reason\":\"...\"}}]，\n\
             concept_ids 必须来自上述概念的 id。只输出 JSON 数组，不要任何其他文字。",
            chunk.len(), idx + 1, chunks.len(), per_chunk_target, list
        );
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "你是知识提炼专家。将概念归纳为主题子群。只输出合法 JSON。".to_string(),
            },
            ChatMessage { role: "user".to_string(), content: prompt },
        ];
        let resp = chat_completion(&client, messages).await
            .map_err(|e| format!("概念聚类 LLM 调用失败（第 {}/{} 批）: {e}", idx + 1, chunks.len()))?;
        let batch = parse_concept_groups(&resp)?;
        local_groups.extend(batch);
        emit_synthesis(&app, &library_id, "clustering", local_groups.len(), 0, None);
    }

    // 3b. 合并阶段：若局部 group 数 > target，让 LLM 按索引合并（prompt 仅含 group 名与计数，远小于概念全量）
    let groups: Vec<ConceptGroup> = if chunks.len() <= 1 || local_groups.len() <= target_count {
        local_groups
    } else {
        let list = local_groups
            .iter()
            .enumerate()
            .map(|(i, g)| {
                format!(
                    "{}. 「{}」 — {} (含 {} 个概念)",
                    i,
                    g.group_name,
                    g.reason.as_deref().unwrap_or(""),
                    g.concept_ids.len()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let merge_prompt = format!(
            "以下是分批聚类得到的 {} 个子群。请将语义相近的子群合并，输出 {} 个最终主题群。\n\
             子群列表（前导数字为索引）：\n{}\n\n\
             输出 JSON 数组：[{{\"group_name\":\"...\",\"merged_indices\":[索引...],\"reason\":\"...\"}}]。\n\
             merged_indices 来自上述 0-based 索引；每个子群必须恰好出现在一个最终群中。只输出 JSON。",
            local_groups.len(), target_count, list
        );
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "你是知识提炼专家。合并语义相近的主题子群。只输出合法 JSON。".to_string(),
            },
            ChatMessage { role: "user".to_string(), content: merge_prompt },
        ];
        let resp = chat_completion(&client, messages).await
            .map_err(|e| format!("概念聚类合并阶段 LLM 调用失败: {e}"))?;
        let text = extract_json_array(&resp);
        let merges: Vec<MergedGroup> = serde_json::from_str(&text).map_err(|e| {
            format!(
                "解析合并结果失败（{e}）\n原始响应：{}",
                &resp[..resp.len().min(300)]
            )
        })?;

        expand_merges(&local_groups, merges)
    };
    emit_synthesis(&app, &library_id, "naming", groups.len(), 0, None);

    // 5. 为每个主题群生成 KnowledgeUnit 命名
    let mut units_written = 0usize;
    let now = chrono::Utc::now().to_rfc3339();

    for group in &groups {
        // 找到 group 中所有概念的定义文本
        let concept_names: Vec<String> = group
            .concept_ids
            .iter()
            .filter_map(|cid| concepts.iter().find(|c| &c.id == cid))
            .map(|c| format!("「{}」: {}", c.name, c.definition.as_deref().unwrap_or("")))
            .collect();

        let naming_prompt = [
            "你是一个知识提炼专家。以下是从用户文档中提取的一组概念，它们来自同一主题群。\n",
            "主题群名称：", &group.group_name, "\n",
            "概念列表：\n", &concept_names.join("\n"), "\n\n",
            "请生成一个知识单元：\n",
            "1. title：用一句话说明这件事的本质/规律/机制，格式为[X]如何/为什么/是什么[Y]，不能是词条名\n",
            "2. core_insight：一句话，这件事最核心的洞见是什么\n\n",
            "示例好的 title：泰勒规则如何描述央行对通胀的反应函数\n",
            "示例不好的 title：泰勒规则（这是词条，不是洞见）\n\n",
            "输出 JSON：{\"title\":\"...\",\"core_insight\":\"...\"}\n",
            "只输出 JSON，不要其他文字。",
        ].concat();

        let naming_messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "你是知识提炼专家。将概念群命名为洞见句。只输出合法 JSON。".to_string(),
            },
            ChatMessage { role: "user".to_string(), content: naming_prompt },
        ];

        let naming_resp = chat_completion(&client, naming_messages).await;

        let draft = match naming_resp {
            Ok(resp) => parse_unit_draft(&resp).unwrap_or_else(|_| KnowledgeUnitDraft {
                title: group.group_name.clone(),
                core_insight: format!("关于{}的核心知识", group.group_name),
            }),
            Err(_) => KnowledgeUnitDraft {
                title: group.group_name.clone(),
                core_insight: format!("关于{}的核心知识", group.group_name),
            },
        };

        // 收集来源素材 IDs（从 group 中所有概念的 source_asset_ids 合并）
        let source_asset_ids: Vec<String> = {
            let source_raw: Vec<String> = {
                let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
                group
                    .concept_ids
                    .iter()
                    .filter_map(|cid| {
                        conn.query_row(
                            "SELECT source_asset_ids FROM concepts WHERE id = ?1",
                            params![cid],
                            |r| r.get::<_, String>(0),
                        )
                        .ok()
                    })
                    .flat_map(|j| {
                        serde_json::from_str::<Vec<String>>(&j).unwrap_or_default()
                    })
                    .collect()
            };
            let mut deduped = source_raw.clone();
            deduped.sort();
            deduped.dedup();
            deduped
        };

        let unit = CreateKnowledgeUnit {
            id: uuid::Uuid::new_v4().to_string(),
            library_id: library_id.clone(),
            title: draft.title,
            core_insight: draft.core_insight,
            constituent_concept_ids: group.concept_ids.clone(),
            source_asset_ids,
            legacy_concept_ids: group.concept_ids.clone(),
            first_captured_at: now.clone(),
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        {
            let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
            insert_knowledge_unit(&conn, &unit)?;
        }

        units_written += 1;
        emit_synthesis(&app, &library_id, "naming", groups.len(), units_written, None);
    }

    emit_synthesis(&app, &library_id, "completed", groups.len(), units_written, None);

    // 返回最新的知识单元列表
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    get_knowledge_units_summary(&conn, &library_id)
}

// ─── 内部工具 ────────────────────────────────────────────────────────────────

fn emit_synthesis(
    app: &tauri::AppHandle,
    library_id: &str,
    stage: &str,
    groups_found: usize,
    units_written: usize,
    error: Option<&str>,
) {
    let _ = app.emit(
        "notecapt/knowledge-synthesis-progress",
        SynthesisProgress {
            library_id: library_id.to_string(),
            stage: stage.to_string(),
            groups_found,
            units_written,
            error: error.map(String::from),
        },
    );
}

fn parse_concept_groups(response: &str) -> Result<Vec<ConceptGroup>, String> {
    let text = extract_json_array(response);
    serde_json::from_str::<Vec<ConceptGroup>>(&text)
        .map_err(|e| format!("解析聚类结果失败（{e}）\n原始响应：{}", &response[..response.len().min(300)]))
}

fn parse_unit_draft(response: &str) -> Result<KnowledgeUnitDraft, String> {
    let text = extract_json_obj(response);
    serde_json::from_str::<KnowledgeUnitDraft>(&text)
        .map_err(|e| format!("解析知识单元命名失败（{e}）\n原始响应：{}", &response[..response.len().min(300)]))
}

/// 从响应文本中提取 JSON 数组（兼容 LLM 在 JSON 前后加额外文字的情况）
fn extract_json_array(text: &str) -> String {
    if let Some(start) = text.find('[') {
        if let Some(end) = text.rfind(']') {
            if end > start {
                return text[start..=end].to_string();
            }
        }
    }
    text.trim().to_string()
}

fn extract_json_obj(text: &str) -> String {
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            if end > start {
                return text[start..=end].to_string();
            }
        }
    }
    text.trim().to_string()
}

/// 将合并阶段的 LLM 输出（按索引）展开回含具体 concept_ids 的 ConceptGroup。
/// - 索引越界自动跳过
/// - 同一局部 group 被多次引用时只算一次（以首次出现的合并为准）
/// - 未被引用的局部 group 作为独立最终群兜底追加，防止概念丢失
fn expand_merges(local_groups: &[ConceptGroup], merges: Vec<MergedGroup>) -> Vec<ConceptGroup> {
    let mut assigned = vec![false; local_groups.len()];
    let mut merged: Vec<ConceptGroup> = merges
        .into_iter()
        .map(|m| {
            let mut ids: Vec<String> = Vec::new();
            for i in &m.merged_indices {
                if let Some(g) = local_groups.get(*i) {
                    if !assigned[*i] {
                        ids.extend(g.concept_ids.clone());
                        assigned[*i] = true;
                    }
                }
            }
            ids.sort();
            ids.dedup();
            ConceptGroup {
                group_name: m.group_name,
                concept_ids: ids,
                reason: m.reason,
            }
        })
        .collect();

    for (i, g) in local_groups.iter().enumerate() {
        if !assigned[i] {
            merged.push(ConceptGroup {
                group_name: g.group_name.clone(),
                concept_ids: g.concept_ids.clone(),
                reason: g.reason.clone(),
            });
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cg(name: &str, ids: &[&str]) -> ConceptGroup {
        ConceptGroup {
            group_name: name.to_string(),
            concept_ids: ids.iter().map(|s| s.to_string()).collect(),
            reason: None,
        }
    }

    #[test]
    fn parse_concept_groups_plain_json() {
        let resp = r#"[{"group_name":"A","concept_ids":["c1","c2"],"reason":"x"}]"#;
        let groups = parse_concept_groups(resp).expect("parse");
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].group_name, "A");
        assert_eq!(groups[0].concept_ids, vec!["c1", "c2"]);
    }

    #[test]
    fn parse_concept_groups_handles_markdown_fence() {
        let resp = "```json\n[{\"group_name\":\"A\",\"concept_ids\":[\"c1\"]}]\n```";
        let groups = parse_concept_groups(resp).expect("parse");
        assert_eq!(groups[0].concept_ids, vec!["c1"]);
    }

    #[test]
    fn parse_concept_groups_handles_prose_preamble() {
        let resp = "这是聚类结果：\n[{\"group_name\":\"A\",\"concept_ids\":[\"c1\"]}]\n希望对你有帮助。";
        let groups = parse_concept_groups(resp).expect("parse");
        assert_eq!(groups.len(), 1);
    }

    #[test]
    fn parse_concept_groups_errors_on_malformed() {
        let resp = "完全不是 JSON";
        assert!(parse_concept_groups(resp).is_err());
    }

    #[test]
    fn expand_merges_unions_and_dedups() {
        let locals = vec![
            cg("g0", &["c1", "c2"]),
            cg("g1", &["c2", "c3"]),
            cg("g2", &["c4"]),
        ];
        let merges = vec![MergedGroup {
            group_name: "Merged".into(),
            merged_indices: vec![0, 1],
            reason: None,
        }];
        let out = expand_merges(&locals, merges);
        // 第一群合并 0+1 → c1,c2,c3；索引 2 未被引用 → 独立保留
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].group_name, "Merged");
        assert_eq!(out[0].concept_ids, vec!["c1", "c2", "c3"]);
        assert_eq!(out[1].group_name, "g2");
        assert_eq!(out[1].concept_ids, vec!["c4"]);
    }

    #[test]
    fn expand_merges_handles_out_of_range_index() {
        let locals = vec![cg("g0", &["c1"])];
        let merges = vec![MergedGroup {
            group_name: "M".into(),
            merged_indices: vec![0, 99], // 99 越界
            reason: None,
        }];
        let out = expand_merges(&locals, merges);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].concept_ids, vec!["c1"]);
    }

    #[test]
    fn expand_merges_skips_duplicate_reference() {
        let locals = vec![cg("g0", &["c1"])];
        // 同一 local 被两个合并群引用 → 只归入第一个，避免 concept 重复
        let merges = vec![
            MergedGroup { group_name: "A".into(), merged_indices: vec![0], reason: None },
            MergedGroup { group_name: "B".into(), merged_indices: vec![0], reason: None },
        ];
        let out = expand_merges(&locals, merges);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].concept_ids, vec!["c1"]);
        assert!(out[1].concept_ids.is_empty());
    }

    #[test]
    fn expand_merges_preserves_all_when_no_merges() {
        let locals = vec![cg("g0", &["c1"]), cg("g1", &["c2"])];
        let out = expand_merges(&locals, vec![]);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].group_name, "g0");
        assert_eq!(out[1].group_name, "g1");
    }
}
