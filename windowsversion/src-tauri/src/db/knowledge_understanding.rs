use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// 数据结构
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptSummary {
    pub id: String,
    pub concept_id: String,
    pub summary: String,
    pub source_asset_ids: Vec<String>,
    pub model: String,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptExplanation {
    pub id: String,
    pub concept_id: String,
    /// JSON: {"text":"...","source":"..."}
    pub mechanism: String,
    /// JSON array: [{"text":"...","source":"..."}]
    pub typical_scenarios: String,
    /// JSON array or null
    pub common_misconceptions: Option<String>,
    pub essence_sentence: String,
    pub source_asset_ids: Vec<String>,
    pub model: String,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptUserNote {
    pub id: String,
    pub concept_id: String,
    pub user_explanation: String,
    /// JSON string (mirror feedback from LLM)
    pub mirror_feedback: Option<String>,
    pub last_validated_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptRelation {
    pub id: String,
    pub concept_a_id: String,
    pub concept_b_id: String,
    pub relation_type: String,
    pub source_asset_ids: Vec<String>,
    pub co_occurrence_count: i64,
    pub created_at: String,
    /// Name of the "other" concept (populated at query time)
    pub other_concept_id: String,
    pub other_concept_name: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// concept_summaries CRUD
// ─────────────────────────────────────────────────────────────────────────────

pub fn get_summary(conn: &Connection, concept_id: &str) -> Result<Option<ConceptSummary>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, concept_id, summary, source_asset_ids, model, generated_at
             FROM concept_summaries WHERE concept_id = ?1 LIMIT 1",
        )
        .map_err(|e| format!("准备查询 concept_summaries 失败: {e}"))?;

    let result = stmt
        .query_row(params![concept_id], |row| {
            let source_ids_json: String = row.get(3)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                source_ids_json,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .optional()
        .map_err(|e| format!("查询 concept_summaries 失败: {e}"))?;

    Ok(result.map(|(id, cid, summary, ids_json, model, generated_at)| {
        let source_asset_ids: Vec<String> =
            serde_json::from_str(&ids_json).unwrap_or_default();
        ConceptSummary {
            id,
            concept_id: cid,
            summary,
            source_asset_ids,
            model,
            generated_at,
        }
    }))
}

pub fn save_summary(conn: &Connection, s: &ConceptSummary) -> Result<(), String> {
    let ids_json = serde_json::to_string(&s.source_asset_ids).unwrap_or_default();
    conn.execute(
        "INSERT OR REPLACE INTO concept_summaries
         (id, concept_id, summary, source_asset_ids, model, generated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![s.id, s.concept_id, s.summary, ids_json, s.model, s.generated_at],
    )
    .map_err(|e| format!("写入 concept_summaries 失败: {e}"))?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// concept_explanations CRUD
// ─────────────────────────────────────────────────────────────────────────────

pub fn get_explanation(
    conn: &Connection,
    concept_id: &str,
) -> Result<Option<ConceptExplanation>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, concept_id, mechanism, typical_scenarios, common_misconceptions,
                    essence_sentence, source_asset_ids, model, generated_at
             FROM concept_explanations WHERE concept_id = ?1 LIMIT 1",
        )
        .map_err(|e| format!("准备查询 concept_explanations 失败: {e}"))?;

    let result = stmt
        .query_row(params![concept_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
            ))
        })
        .optional()
        .map_err(|e| format!("查询 concept_explanations 失败: {e}"))?;

    Ok(result.map(
        |(id, cid, mechanism, scenarios, misconceptions, essence, ids_json, model, generated_at)| {
            let source_asset_ids: Vec<String> =
                serde_json::from_str(&ids_json).unwrap_or_default();
            ConceptExplanation {
                id,
                concept_id: cid,
                mechanism,
                typical_scenarios: scenarios,
                common_misconceptions: misconceptions,
                essence_sentence: essence,
                source_asset_ids,
                model,
                generated_at,
            }
        },
    ))
}

pub fn save_explanation(conn: &Connection, e: &ConceptExplanation) -> Result<(), String> {
    let ids_json = serde_json::to_string(&e.source_asset_ids).unwrap_or_default();
    conn.execute(
        "INSERT OR REPLACE INTO concept_explanations
         (id, concept_id, mechanism, typical_scenarios, common_misconceptions,
          essence_sentence, source_asset_ids, model, generated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            e.id,
            e.concept_id,
            e.mechanism,
            e.typical_scenarios,
            e.common_misconceptions,
            e.essence_sentence,
            ids_json,
            e.model,
            e.generated_at,
        ],
    )
    .map_err(|e| format!("写入 concept_explanations 失败: {e}"))?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// concept_user_notes CRUD
// ─────────────────────────────────────────────────────────────────────────────

pub fn get_user_note(
    conn: &Connection,
    concept_id: &str,
) -> Result<Option<ConceptUserNote>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, concept_id, user_explanation, mirror_feedback,
                    last_validated_at, created_at, updated_at
             FROM concept_user_notes WHERE concept_id = ?1 LIMIT 1",
        )
        .map_err(|e| format!("准备查询 concept_user_notes 失败: {e}"))?;

    let result = stmt
        .query_row(params![concept_id], |row| {
            Ok(ConceptUserNote {
                id: row.get(0)?,
                concept_id: row.get(1)?,
                user_explanation: row.get(2)?,
                mirror_feedback: row.get(3)?,
                last_validated_at: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })
        .optional()
        .map_err(|e| format!("查询 concept_user_notes 失败: {e}"))?;

    Ok(result)
}

/// Upsert user_explanation only — never touches mirror_feedback
pub fn save_user_explanation(
    conn: &Connection,
    concept_id: &str,
    user_explanation: &str,
    now: &str,
) -> Result<(), String> {
    let existing = get_user_note(conn, concept_id)?;
    if existing.is_some() {
        conn.execute(
            "UPDATE concept_user_notes
             SET user_explanation = ?2, updated_at = ?3
             WHERE concept_id = ?1",
            params![concept_id, user_explanation, now],
        )
        .map_err(|e| format!("更新 concept_user_notes.user_explanation 失败: {e}"))?;
    } else {
        let new_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO concept_user_notes
             (id, concept_id, user_explanation, mirror_feedback, last_validated_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, NULL, ?4, ?4)",
            params![new_id, concept_id, user_explanation, now],
        )
        .map_err(|e| format!("插入 concept_user_notes 失败: {e}"))?;
    }
    Ok(())
}

/// Update mirror_feedback and last_validated_at only
pub fn save_mirror_feedback(
    conn: &Connection,
    concept_id: &str,
    mirror_feedback_json: &str,
    now: &str,
) -> Result<(), String> {
    let existing = get_user_note(conn, concept_id)?;
    if existing.is_some() {
        conn.execute(
            "UPDATE concept_user_notes
             SET mirror_feedback = ?2, last_validated_at = ?3, updated_at = ?3
             WHERE concept_id = ?1",
            params![concept_id, mirror_feedback_json, now],
        )
        .map_err(|e| format!("更新 mirror_feedback 失败: {e}"))?;
    } else {
        let new_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO concept_user_notes
             (id, concept_id, user_explanation, mirror_feedback, last_validated_at, created_at, updated_at)
             VALUES (?1, ?2, '', ?3, ?4, ?4, ?4)",
            params![new_id, concept_id, mirror_feedback_json, now],
        )
        .map_err(|e| format!("插入 concept_user_notes (mirror_feedback) 失败: {e}"))?;
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// concept_relations 查询
// ─────────────────────────────────────────────────────────────────────────────

pub fn get_relations(
    conn: &Connection,
    concept_id: &str,
) -> Result<Vec<ConceptRelation>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT cr.id, cr.concept_a_id, cr.concept_b_id, cr.relation_type,
                    cr.source_asset_ids, cr.co_occurrence_count, cr.created_at,
                    CASE WHEN cr.concept_a_id = ?1 THEN cr.concept_b_id ELSE cr.concept_a_id END AS other_id,
                    COALESCE(c.name, '') AS other_name
             FROM concept_relations cr
             LEFT JOIN concepts c ON c.id = (
                 CASE WHEN cr.concept_a_id = ?1 THEN cr.concept_b_id ELSE cr.concept_a_id END
             )
             WHERE cr.concept_a_id = ?1 OR cr.concept_b_id = ?1
             ORDER BY cr.co_occurrence_count DESC
             LIMIT 8",
        )
        .map_err(|e| format!("准备查询 concept_relations 失败: {e}"))?;

    let rows: Result<Vec<ConceptRelation>, _> = stmt
        .query_map(params![concept_id], |row| {
            let ids_json: String = row.get(4)?;
            let source_asset_ids: Vec<String> =
                serde_json::from_str(&ids_json).unwrap_or_default();
            Ok(ConceptRelation {
                id: row.get(0)?,
                concept_a_id: row.get(1)?,
                concept_b_id: row.get(2)?,
                relation_type: row.get(3)?,
                source_asset_ids,
                co_occurrence_count: row.get(5)?,
                created_at: row.get(6)?,
                other_concept_id: row.get(7)?,
                other_concept_name: row.get(8)?,
            })
        })
        .map_err(|e| format!("查询 concept_relations 失败: {e}"))?
        .collect();

    rows.map_err(|e| format!("读取 concept_relations 行失败: {e}"))
}
