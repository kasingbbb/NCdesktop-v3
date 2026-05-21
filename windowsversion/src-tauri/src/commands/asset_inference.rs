/// 信号推断引擎（Step 3）
///
/// 当素材进入知识库后，自动运行：
///   1. 时间聚类（±2小时窗口），识别采集会话
///   2. 关键词提取 + Jaccard 相似度，计算与现有 KU 的 noveltyScore
///   3. 判断置信度：高→静默归属；低→发出 Toast 事件让用户二选一
///
/// 事件：`notecapt/inference-low-confidence`
///   payload: InferenceLowConfidencePayload

use crate::db::knowledge_units::{
    get_knowledge_units_summary, upsert_asset_inference, AssetInference,
};
use crate::db::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tauri::{Emitter, State};

// ─── 事件 payload ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateKu {
    pub id: String,
    pub title: String,
    pub core_insight: String,
    pub similarity: f64,
}

/// 低置信度时推送给前端，由 InferenceToast 展示
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InferenceLowConfidencePayload {
    pub asset_id: String,
    pub asset_name: String,
    pub candidates: Vec<CandidateKu>, // 最多 2 个
    pub suggested_action: String,     // "link_to_ku" | "create_new" | "choose"
}

// ─── 命令 ─────────────────────────────────────────────────────────────────────

/// 对指定素材执行信号推断，返回 AssetInference
///
/// - 时间聚类：±2h 内同项目其他素材 → session_peer_ids
/// - 关键词相似度：与知识库内 KU 的 title/core_insight/summary 做 Jaccard 匹配
/// - 置信度 ≥ 0.65 → 静默归属（发 `notecapt/inference-done`）
/// - 置信度 < 0.65 → 推送 `notecapt/inference-low-confidence` 让用户确认
#[tauri::command]
pub async fn infer_asset_context(
    db: State<'_, Database>,
    app: tauri::AppHandle,
    asset_id: String,
    library_id: String,
) -> Result<AssetInference, String> {
    let inference = {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;

        // ── 1. 加载素材基础信息 ────────────────────────────────────────────────
        let (asset_name, captured_at, _project_id): (String, String, String) = conn
            .query_row(
                "SELECT name, captured_at, project_id FROM assets WHERE id = ?1",
                params![asset_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(|e| format!("素材不存在 {asset_id}: {e}"))?;

        // ── 2. 时间聚类：±2h 同项目同库素材 ─────────────────────────────────
        // 素材通过 project_id 关联到 library（projects.library_id）
        let window_secs: i64 = 7200; // 2 小时
        let session_peer_ids: Vec<String> = {
            let mut stmt = conn
                .prepare(
                    "SELECT a.id FROM assets a
                     INNER JOIN projects p ON a.project_id = p.id
                     WHERE p.library_id = ?1
                       AND a.id != ?2
                       AND ABS(
                         CAST(strftime('%s', a.captured_at) AS INTEGER)
                         - CAST(strftime('%s', ?3) AS INTEGER)
                       ) <= ?4
                     ORDER BY a.captured_at ASC
                     LIMIT 20",
                )
                .map_err(|e| format!("时间聚类查询失败: {e}"))?;
            let x = stmt.query_map(
                params![library_id, asset_id, captured_at, window_secs],
                |r| r.get::<_, String>(0),
            )
            .map_err(|e| format!("时间聚类遍历失败: {e}"))?
            .filter_map(|r| r.ok())
            .collect();
            x
        };

        // session_id：用当前素材 id 作为会话锚点（取时间窗口最早的）
        let session_id = if session_peer_ids.is_empty() {
            None
        } else {
            Some(asset_id.clone())
        };

        // ── 3. 读取素材提取文本 ───────────────────────────────────────────────
        let asset_text = crate::db::asset::get_preferred_text_content(&conn, &asset_id)?
            .unwrap_or_else(|| asset_name.clone());

        // ── 4. 提取关键词 ─────────────────────────────────────────────────────
        let asset_keywords = extract_keywords(&asset_text, 30);
        let dominant_topics: Vec<String> = asset_keywords.iter().take(10).cloned().collect();

        // ── 5. 加载当前知识库的全部 KU（title + core_insight + summary） ────
        let ku_list = get_knowledge_units_summary(&conn, &library_id)?;

        // ── 6. 计算相似度 ─────────────────────────────────────────────────────
        let mut scored: Vec<(String, f64, String, String)> = Vec::new(); // (id, score, title, insight)

        for ku in &ku_list {
            // 从 knowledge_units 直接读 title/core_insight/summary
            let ku_text: String = conn
                .query_row(
                    "SELECT COALESCE(title, '') || ' ' || COALESCE(core_insight, '') || ' ' || COALESCE(summary, '')
                     FROM knowledge_units WHERE id = ?1",
                    params![ku.id],
                    |r| r.get::<_, String>(0),
                )
                .unwrap_or_default();

            let ku_keywords = extract_keywords(&ku_text, 30);
            let sim = jaccard_similarity(&asset_keywords, &ku_keywords);
            scored.push((ku.id.clone(), sim, ku.title.clone(), ku.core_insight.clone()));
        }

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let top3: Vec<(String, f64, String, String)> = scored.into_iter().take(3).collect();

        let max_sim = top3.first().map(|(_, s, _, _)| *s).unwrap_or(0.0);
        let novelty_score = 1.0 - max_sim;

        let closest_knowledge_ids: Vec<String> = top3.iter().map(|(id, _, _, _)| id.clone()).collect();
        let closest_scores: Vec<f64> = top3.iter().map(|(_, s, _, _)| *s).collect();

        // ── 7. 推断类型 ───────────────────────────────────────────────────────
        let asset_name_lower = asset_name.to_lowercase();
        let inferred_type = if asset_name_lower.contains("复习")
            || asset_name_lower.contains("review")
            || asset_name_lower.contains("recap")
        {
            "review"
        } else if !session_peer_ids.is_empty() {
            // 有会话伙伴 → 可能是课堂内容
            "class_content"
        } else if asset_name_lower.contains("参考")
            || asset_name_lower.contains("ref")
            || asset_name_lower.contains("附录")
        {
            "reference"
        } else {
            "self_study"
        };

        // is_supplementary：相似度很高（>0.75）→ 是已有 KU 的补充
        let is_supplementary = max_sim > 0.75;
        let supplement_target_id = if is_supplementary {
            closest_knowledge_ids.first().cloned()
        } else {
            None
        };

        // ── 8. 置信度 ─────────────────────────────────────────────────────────
        // 高置信：明确是补充（sim>0.75）或明确是新知识（sim<0.2）
        // 低置信：中间地带（0.2~0.75）
        let (confidence, ambiguity_reason) = if max_sim > 0.75 {
            (0.85, None)
        } else if max_sim < 0.20 {
            (0.80, None) // 新知识，明确
        } else {
            let reason = if top3.len() >= 2 {
                let (_, s1, _, _) = &top3[0];
                let (_, s2, _, _) = &top3[1];
                if (s1 - s2).abs() < 0.1 {
                    Some(format!(
                        "与「{}」和「{}」相似度接近，难以自动归属",
                        top3[0].2, top3[1].2
                    ))
                } else {
                    Some(format!("与「{}」有一定相关性，但不确定", top3[0].2))
                }
            } else {
                Some("无法确定是已有知识还是新知识".to_string())
            };
            (0.45, reason)
        };

        // ── 9. 推断所属课程（从 session 内其他素材的 inferred_course 取众数）──
        let inferred_course: Option<String> = if !session_peer_ids.is_empty() {
            let mut courses: HashMap<String, usize> = HashMap::new();
            for peer_id in session_peer_ids.iter().take(5) {
                let course: Option<String> = conn
                    .query_row(
                        "SELECT inferred_course FROM asset_inferences WHERE asset_id = ?1",
                        params![peer_id],
                        |r| r.get(0),
                    )
                    .ok()
                    .flatten();
                if let Some(c) = course {
                    *courses.entry(c).or_default() += 1;
                }
            }
            courses.into_iter().max_by_key(|(_, cnt)| *cnt).map(|(c, _)| c)
        } else {
            None
        };

        // ── 10. 构造并持久化 AssetInference ──────────────────────────────────
        let now = chrono::Utc::now().to_rfc3339();
        let inference = AssetInference {
            id: uuid::Uuid::new_v4().to_string(),
            asset_id: asset_id.clone(),
            session_id,
            session_peer_ids,
            dominant_topics,
            novelty_score,
            closest_knowledge_ids,
            closest_scores,
            inferred_course,
            inferred_type: inferred_type.to_string(),
            is_supplementary,
            supplement_target_id,
            confidence,
            ambiguity_reason,
            created_at: now,
        };

        upsert_asset_inference(&conn, &inference)?;
        inference
    };

    // ── 11. 触发前端事件 ──────────────────────────────────────────────────────
    if inference.confidence < 0.65 {
        // 读取候选 KU 详情用于 Toast
        let candidates: Vec<CandidateKu> = {
            let conn = db.conn.lock().map_err(|e| format!("数据库锁: {e}"))?;
            inference
                .closest_knowledge_ids
                .iter()
                .zip(inference.closest_scores.iter())
                .take(2)
                .filter_map(|(ku_id, &score)| {
                    conn.query_row(
                        "SELECT id, title, core_insight FROM knowledge_units WHERE id = ?1",
                        params![ku_id],
                        |r| {
                            Ok(CandidateKu {
                                id: r.get(0)?,
                                title: r.get(1)?,
                                core_insight: r.get(2)?,
                                similarity: score,
                            })
                        },
                    )
                    .ok()
                })
                .collect()
        };

        let asset_name = {
            let conn = db.conn.lock().map_err(|e| format!("数据库锁: {e}"))?;
            conn.query_row(
                "SELECT name FROM assets WHERE id = ?1",
                params![asset_id],
                |r| r.get::<_, String>(0),
            )
            .unwrap_or_default()
        };

        let suggested_action = if candidates.is_empty() {
            "create_new".to_string()
        } else {
            "choose".to_string()
        };

        let _ = app.emit(
            "notecapt/inference-low-confidence",
            InferenceLowConfidencePayload {
                asset_id,
                asset_name,
                candidates,
                suggested_action,
            },
        );
    } else {
        let _ = app.emit("notecapt/inference-done", &inference.asset_id);
    }

    Ok(inference)
}

/// 批量推断：对一个知识库内所有尚无推断记录的素材执行推断
#[tauri::command]
pub async fn infer_library_assets(
    db: State<'_, Database>,
    app: tauri::AppHandle,
    library_id: String,
) -> Result<u32, String> {
    // 找到库中所有没有推断记录的素材
    let asset_ids: Vec<String> = {
        let conn = db.conn.lock().map_err(|e| format!("数据库锁: {e}"))?;
        let mut stmt = conn
            .prepare(
                "SELECT a.id FROM assets a
                 INNER JOIN projects p ON a.project_id = p.id
                 WHERE p.library_id = ?1
                   AND NOT EXISTS (
                     SELECT 1 FROM asset_inferences ai WHERE ai.asset_id = a.id
                   )
                 ORDER BY a.captured_at DESC
                 LIMIT 100",
            )
            .map_err(|e| format!("批量推断查询失败: {e}"))?;
        let x = stmt.query_map(params![library_id], |r| r.get::<_, String>(0))
            .map_err(|e| format!("批量推断遍历失败: {e}"))?
            .filter_map(|r| r.ok())
            .collect();
        x
    };

    let count = asset_ids.len() as u32;
    for aid in asset_ids {
        // 每个素材独立推断，失败不中断整批
        let _ = infer_asset_context(db.clone(), app.clone(), aid, library_id.clone()).await;
    }
    Ok(count)
}

/// 获取一个素材的推断结果（已存在则直接返回，否则 None）
#[tauri::command]
pub fn get_asset_inference_result(
    db: State<'_, Database>,
    asset_id: String,
) -> Result<Option<AssetInference>, String> {
    let conn = db.conn.lock().map_err(|e| format!("数据库锁: {e}"))?;
    crate::db::knowledge_units::get_asset_inference(&conn, &asset_id)
}

// ─── 关键词提取 ───────────────────────────────────────────────────────────────

/// 从文本中提取 top-N 高频关键词（过滤中英文停用词）
fn extract_keywords(text: &str, top_n: usize) -> Vec<String> {
    // 中文停用词（精简版）
    const STOP_ZH: &[&str] = &[
        "的", "了", "是", "在", "有", "和", "不", "这", "那", "就", "都", "也", "很",
        "一", "我", "你", "他", "她", "它", "我们", "你们", "他们", "什么", "怎么",
        "可以", "如果", "但是", "因为", "所以", "然后", "而且", "但", "或", "并",
        "通过", "对于", "关于", "由于", "已经", "正在", "以及", "可能", "应该",
        "需要", "进行", "使用", "包括", "提供", "具有", "作为", "会", "将", "更",
        "其", "与", "及", "或者", "从", "到", "为", "被", "把", "让", "给",
    ];
    // 英文停用词
    const STOP_EN: &[&str] = &[
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
        "have", "has", "had", "do", "does", "did", "will", "would", "could",
        "should", "may", "might", "shall", "can", "to", "of", "in", "on",
        "at", "by", "for", "with", "from", "as", "and", "or", "but", "not",
        "it", "its", "this", "that", "which", "who", "what", "how", "when",
        "where", "why", "if", "then", "so", "than", "very", "just", "also",
    ];

    let stop_set: HashSet<&str> = STOP_ZH.iter().chain(STOP_EN.iter()).copied().collect();

    // 简单分词：按非字母/非汉字字符分割
    let mut freq: HashMap<String, usize> = HashMap::new();
    for token in text.split(|c: char| !c.is_alphanumeric() && (c as u32) < 0x4E00
        || (c.is_ascii() && !c.is_alphanumeric()))
    {
        let t = token.trim().to_lowercase();
        if t.len() < 2 { continue; }
        if stop_set.contains(t.as_str()) { continue; }
        *freq.entry(t).or_default() += 1;
    }

    let mut pairs: Vec<(String, usize)> = freq.into_iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1));
    pairs.into_iter().take(top_n).map(|(w, _)| w).collect()
}

/// Jaccard 相似度 = |A ∩ B| / |A ∪ B|
fn jaccard_similarity(a: &[String], b: &[String]) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let set_a: HashSet<&String> = a.iter().collect();
    let set_b: HashSet<&String> = b.iter().collect();
    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();
    if union == 0 { 0.0 } else { intersection as f64 / union as f64 }
}
