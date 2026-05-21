/// 知识图谱数据命令（Step 9）
///
/// 为前端力导向图提供节点（Node）和边（Edge）数据。
///
/// 边的来源：
///   1. 共享 constituent_concept_ids → 同域边（solid）
///   2. asset_inferences.closest_knowledge_ids → 相似度边（dashed，跨域）
///   3. asset_inferences.supplement_target_id → 补充关系边（dotted）

use crate::db::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tauri::State;

// ─── 数据结构 ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub id: String,
    pub title: String,
    pub core_insight: String,
    pub status: String,
    pub depth_level: i64,
    pub source_asset_count: i64,
    /// 用于分组/颜色：inferred_course（来自 asset_inferences）
    pub inferred_course: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    /// "concept" | "similarity" | "supplement"
    pub edge_type: String,
    /// 0.0–1.0，影响边的粗细
    pub weight: f64,
    /// true = 来自不同 inferred_course → 虚线渲染
    pub is_cross_domain: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeGraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

// ─── 命令 ─────────────────────────────────────────────────────────────────────

/// 返回指定知识库的图谱数据（节点 + 边）
#[tauri::command]
pub fn get_knowledge_graph(
    db: State<'_, Database>,
    library_id: String,
) -> Result<KnowledgeGraphData, String> {
    let conn = db.conn.lock().map_err(|e| format!("数据库锁: {e}"))?;

    // ── 1. 加载知识单元节点 ────────────────────────────────────────────────────
    let mut stmt = conn
        .prepare(
            "SELECT
                ku.id,
                ku.title,
                ku.core_insight,
                ku.status,
                ku.depth_level,
                length(ku.source_asset_ids) - length(replace(ku.source_asset_ids, ',', '')) + 1 AS asset_count,
                ku.constituent_concept_ids,
                ku.source_asset_ids,
                ai.inferred_course
             FROM knowledge_units ku
             LEFT JOIN (
               SELECT ai2.asset_id, ai2.inferred_course
               FROM asset_inferences ai2
               WHERE ai2.inferred_course IS NOT NULL
               GROUP BY ai2.inferred_course
             ) ai ON ai.asset_id IN (
               SELECT a.id FROM assets a
               INNER JOIN projects p ON a.project_id = p.id
               WHERE p.library_id = ?1
               LIMIT 1
             )
             WHERE ku.library_id = ?1
             ORDER BY ku.depth_level DESC, ku.updated_at DESC",
        )
        .map_err(|e| format!("图谱节点查询失败: {e}"))?;

    // node_id → (inferred_course, constituent_concept_ids JSON, source_asset_ids JSON)
    let mut raw_nodes: Vec<(GraphNode, String, String)> = Vec::new();

    {
        let rows = stmt
            .query_map(params![library_id], |r| {
                let concept_ids_json: String = r.get(6).unwrap_or_else(|_| "[]".to_string());
                let asset_ids_json: String = r.get(7).unwrap_or_else(|_| "[]".to_string());
                let course: Option<String> = r.get(8).unwrap_or(None);
                // asset_count via JSON array length approximation
                let asset_count: i64 = {
                    let s: String = r.get(7).unwrap_or_else(|_| "[]".to_string());
                    if s == "[]" || s.is_empty() { 0 }
                    else {
                        // count commas + 1 for quoted items
                        (s.matches(',').count() as i64) + 1
                    }
                };
                Ok((
                    GraphNode {
                        id: r.get(0)?,
                        title: r.get(1)?,
                        core_insight: r.get(2)?,
                        status: r.get(3)?,
                        depth_level: r.get(4)?,
                        source_asset_count: asset_count,
                        inferred_course: course,
                    },
                    concept_ids_json,
                    asset_ids_json,
                ))
            })
            .map_err(|e| format!("图谱节点遍历失败: {e}"))?;

        for row in rows {
            raw_nodes.push(row.map_err(|e| format!("图谱节点读取失败: {e}"))?);
        }
    }

    // ── 2. 从 asset_inferences 补全 inferred_course ──────────────────────────
    // 为没有课程的 KU，尝试从其 source_asset_ids 对应的 inference 查找
    let ku_course: HashMap<String, Option<String>> = {
        let mut map: HashMap<String, Option<String>> = HashMap::new();
        for (node, _, asset_ids_json) in &raw_nodes {
            if node.inferred_course.is_some() {
                map.insert(node.id.clone(), node.inferred_course.clone());
                continue;
            }
            // 解析 asset_ids
            let asset_ids: Vec<String> = serde_json::from_str(asset_ids_json).unwrap_or_default();
            let mut course: Option<String> = None;
            for aid in asset_ids.iter().take(3) {
                let c: Option<String> = conn
                    .query_row(
                        "SELECT inferred_course FROM asset_inferences WHERE asset_id = ?1",
                        params![aid],
                        |r| r.get(0),
                    )
                    .ok()
                    .flatten();
                if c.is_some() {
                    course = c;
                    break;
                }
            }
            map.insert(node.id.clone(), course);
        }
        map
    };

    // ── 3. 构建节点列表（附 inferred_course）────────────────────────────────
    let nodes: Vec<GraphNode> = raw_nodes
        .iter()
        .map(|(n, _, _)| GraphNode {
            inferred_course: ku_course.get(&n.id).and_then(|c| c.clone()),
            ..n.clone()
        })
        .collect();

    // ── 4. 构建边：共享 concept_id ───────────────────────────────────────────
    // concept_id → [ku_id]
    let mut concept_to_kus: HashMap<String, Vec<String>> = HashMap::new();
    for (node, concept_ids_json, _) in &raw_nodes {
        let concept_ids: Vec<String> =
            serde_json::from_str(concept_ids_json).unwrap_or_default();
        for cid in concept_ids {
            concept_to_kus.entry(cid).or_default().push(node.id.clone());
        }
    }

    let mut edges: Vec<GraphEdge> = Vec::new();
    let mut seen_pairs: HashSet<(String, String)> = HashSet::new();

    for (_cid, ku_ids) in &concept_to_kus {
        for i in 0..ku_ids.len() {
            for j in (i + 1)..ku_ids.len() {
                let (src, tgt) = (&ku_ids[i], &ku_ids[j]);
                let pair = if src < tgt {
                    (src.clone(), tgt.clone())
                } else {
                    (tgt.clone(), src.clone())
                };
                if seen_pairs.insert(pair.clone()) {
                    let cross = ku_course.get(src).and_then(|c| c.clone())
                        != ku_course.get(tgt).and_then(|c| c.clone());
                    edges.push(GraphEdge {
                        source: pair.0,
                        target: pair.1,
                        edge_type: "concept".to_string(),
                        weight: 0.8,
                        is_cross_domain: cross,
                    });
                }
            }
        }
    }

    // ── 5. 构建边：asset_inferences.closest_knowledge_ids ───────────────────
    {
        let mut stmt2 = conn
            .prepare(
                "SELECT ai.asset_id, ai.closest_knowledge_ids, ai.closest_scores, ai.supplement_target_id, ai.is_supplementary
                 FROM asset_inferences ai
                 INNER JOIN assets a ON ai.asset_id = a.id
                 INNER JOIN projects p ON a.project_id = p.id
                 WHERE p.library_id = ?1
                   AND ai.closest_knowledge_ids != '[]'",
            )
            .map_err(|e| format!("相似度边查询失败: {e}"))?;

        let node_id_set: HashSet<String> = nodes.iter().map(|n| n.id.clone()).collect();

        let rows = stmt2
            .query_map(params![library_id], |r| {
                let closest_json: String = r.get(1).unwrap_or_else(|_| "[]".to_string());
                let scores_json: String = r.get(2).unwrap_or_else(|_| "[]".to_string());
                let supp_target: Option<String> = r.get(3).unwrap_or(None);
                let is_supp: bool = r.get::<_, i64>(4).unwrap_or(0) != 0;
                Ok((closest_json, scores_json, supp_target, is_supp))
            })
            .map_err(|e| format!("相似度边遍历失败: {e}"))?;

        // asset_inferences links to KU via closest_knowledge_ids
        // We need to get the KU that owns this asset, then link to its closest KUs
        // But we don't have asset→KU mapping here. Instead, build edges between closest KUs.
        for row in rows {
            let (closest_json, scores_json, supp_target, is_supp) =
                row.map_err(|e| format!("相似度边行读取失败: {e}"))?;

            let closest: Vec<String> =
                serde_json::from_str(&closest_json).unwrap_or_default();
            let scores: Vec<f64> =
                serde_json::from_str(&scores_json).unwrap_or_default();

            // Supplement edge
            if is_supp {
                if let (Some(src_id), Some(tgt_id)) = (closest.first(), &supp_target) {
                    if node_id_set.contains(src_id) && node_id_set.contains(tgt_id) {
                        let pair = if src_id < tgt_id {
                            (src_id.clone(), tgt_id.clone())
                        } else {
                            (tgt_id.clone(), src_id.clone())
                        };
                        if seen_pairs.insert(pair.clone()) {
                            let cross = ku_course.get(&pair.0).and_then(|c| c.clone())
                                != ku_course.get(&pair.1).and_then(|c| c.clone());
                            edges.push(GraphEdge {
                                source: pair.0,
                                target: pair.1,
                                edge_type: "supplement".to_string(),
                                weight: 0.9,
                                is_cross_domain: cross,
                            });
                        }
                    }
                }
            }

            // Similarity edges between the top-2 closest KUs (if similarity is high enough)
            if closest.len() >= 2 {
                let score = scores.first().copied().unwrap_or(0.0);
                if score > 0.3 {
                    let (src, tgt) = (&closest[0], &closest[1]);
                    if node_id_set.contains(src) && node_id_set.contains(tgt) {
                        let pair = if src < tgt {
                            (src.clone(), tgt.clone())
                        } else {
                            (tgt.clone(), src.clone())
                        };
                        if seen_pairs.insert(pair.clone()) {
                            let cross = ku_course.get(&pair.0).and_then(|c| c.clone())
                                != ku_course.get(&pair.1).and_then(|c| c.clone());
                            edges.push(GraphEdge {
                                source: pair.0,
                                target: pair.1,
                                edge_type: "similarity".to_string(),
                                weight: score,
                                is_cross_domain: cross,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(KnowledgeGraphData { nodes, edges })
}
