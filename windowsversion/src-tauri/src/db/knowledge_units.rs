use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

// ─── 知识单元 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeUnit {
    pub id: String,
    pub library_id: String,
    pub title: String,
    pub core_insight: String,
    pub summary: Option<String>,
    pub explanation: Option<String>,          // JSON: KnowledgeExplanation
    pub constituent_concept_ids: Vec<String>,
    pub source_asset_ids: Vec<String>,
    pub status: String,                       // raw/synthesized/understood/articulated/validated/consolidated/mastered
    pub user_note: Option<String>,
    pub last_mirror_feedback: Option<String>, // JSON: MirrorFeedbackResult
    pub depth_level: i64,
    pub legacy_concept_ids: Vec<String>,
    pub first_captured_at: String,
    pub last_reviewed_at: Option<String>,
    pub next_review_due: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeUnitSummary {
    pub id: String,
    pub library_id: String,
    pub title: String,
    pub core_insight: String,
    pub status: String,
    pub depth_level: i64,
    pub source_asset_count: i64,
    pub snapshot_count: i64,
    pub next_review_due: Option<String>,
    pub last_reviewed_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateKnowledgeUnit {
    pub id: String,
    pub library_id: String,
    pub title: String,
    pub core_insight: String,
    pub constituent_concept_ids: Vec<String>,
    pub source_asset_ids: Vec<String>,
    pub legacy_concept_ids: Vec<String>,
    pub first_captured_at: String,
    pub created_at: String,
    pub updated_at: String,
}

pub fn insert_knowledge_unit(conn: &Connection, unit: &CreateKnowledgeUnit) -> Result<(), String> {
    conn.execute(
        "INSERT INTO knowledge_units (
            id, library_id, title, core_insight,
            constituent_concept_ids, source_asset_ids,
            status, depth_level, legacy_concept_ids,
            first_captured_at, created_at, updated_at
        ) VALUES (?1,?2,?3,?4,?5,?6,'raw',1,?7,?8,?9,?10)",
        params![
            unit.id,
            unit.library_id,
            unit.title,
            unit.core_insight,
            serde_json::to_string(&unit.constituent_concept_ids).unwrap_or_default(),
            serde_json::to_string(&unit.source_asset_ids).unwrap_or_default(),
            serde_json::to_string(&unit.legacy_concept_ids).unwrap_or_default(),
            unit.first_captured_at,
            unit.created_at,
            unit.updated_at,
        ],
    )
    .map_err(|e| format!("insert_knowledge_unit 失败: {e}"))?;
    Ok(())
}

pub fn get_knowledge_units_summary(
    conn: &Connection,
    library_id: &str,
) -> Result<Vec<KnowledgeUnitSummary>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT
                ku.id, ku.library_id, ku.title, ku.core_insight,
                ku.status, ku.depth_level,
                ku.source_asset_ids,
                ku.next_review_due, ku.last_reviewed_at, ku.updated_at,
                COUNT(DISTINCT s.id) as snapshot_count
             FROM knowledge_units ku
             LEFT JOIN understanding_snapshots s ON s.knowledge_unit_id = ku.id
             WHERE ku.library_id = ?1
             GROUP BY ku.id
             ORDER BY
               CASE ku.status
                 WHEN 'raw' THEN 1
                 WHEN 'synthesized' THEN 2
                 WHEN 'understood' THEN 3
                 WHEN 'articulated' THEN 4
                 WHEN 'validated' THEN 5
                 WHEN 'consolidated' THEN 6
                 WHEN 'mastered' THEN 7
                 ELSE 8
               END,
               ku.next_review_due ASC NULLS LAST,
               ku.updated_at DESC",
        )
        .map_err(|e| format!("prepare get_knowledge_units_summary 失败: {e}"))?;

    let rows = stmt
        .query_map(params![library_id], |row| {
            let source_ids_json: String = row.get(6)?;
            let source_count = serde_json::from_str::<Vec<serde_json::Value>>(&source_ids_json)
                .map(|v| v.len() as i64)
                .unwrap_or(0);
            Ok(KnowledgeUnitSummary {
                id: row.get(0)?,
                library_id: row.get(1)?,
                title: row.get(2)?,
                core_insight: row.get(3)?,
                status: row.get(4)?,
                depth_level: row.get(5)?,
                source_asset_count: source_count,
                next_review_due: row.get(7)?,
                last_reviewed_at: row.get(8)?,
                updated_at: row.get(9)?,
                snapshot_count: row.get(10)?,
            })
        })
        .map_err(|e| format!("query get_knowledge_units_summary 失败: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect get_knowledge_units_summary 失败: {e}"))
}

pub fn get_knowledge_unit(conn: &Connection, id: &str) -> Result<Option<KnowledgeUnit>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, library_id, title, core_insight, summary, explanation,
                    constituent_concept_ids, source_asset_ids, status,
                    user_note, last_mirror_feedback, depth_level, legacy_concept_ids,
                    first_captured_at, last_reviewed_at, next_review_due,
                    created_at, updated_at
             FROM knowledge_units WHERE id = ?1",
        )
        .map_err(|e| format!("prepare get_knowledge_unit 失败: {e}"))?;

    let result = stmt
        .query_row(params![id], row_to_knowledge_unit)
        .optional()
        .map_err(|e| format!("get_knowledge_unit 失败: {e}"))?;
    Ok(result)
}

pub fn update_knowledge_unit_status(
    conn: &Connection,
    id: &str,
    status: &str,
    updated_at: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE knowledge_units SET status = ?1, updated_at = ?2 WHERE id = ?3",
        params![status, updated_at, id],
    )
    .map_err(|e| format!("update_knowledge_unit_status 失败: {e}"))?;
    Ok(())
}

pub fn update_knowledge_unit_summary(
    conn: &Connection,
    id: &str,
    summary: &str,
    updated_at: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE knowledge_units SET summary = ?1, status = CASE WHEN status = 'raw' THEN 'synthesized' ELSE status END, updated_at = ?2 WHERE id = ?3",
        params![summary, updated_at, id],
    )
    .map_err(|e| format!("update_knowledge_unit_summary 失败: {e}"))?;
    Ok(())
}

pub fn update_knowledge_unit_explanation(
    conn: &Connection,
    id: &str,
    explanation_json: &str,
    updated_at: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE knowledge_units SET explanation = ?1, updated_at = ?2 WHERE id = ?3",
        params![explanation_json, updated_at, id],
    )
    .map_err(|e| format!("update_knowledge_unit_explanation 失败: {e}"))?;
    Ok(())
}

pub fn update_knowledge_unit_note(
    conn: &Connection,
    id: &str,
    user_note: &str,
    updated_at: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE knowledge_units SET user_note = ?1,
            status = CASE WHEN status IN ('raw','synthesized','understood') THEN 'articulated' ELSE status END,
            updated_at = ?2
         WHERE id = ?3",
        params![user_note, updated_at, id],
    )
    .map_err(|e| format!("update_knowledge_unit_note 失败: {e}"))?;
    Ok(())
}

pub fn update_knowledge_unit_mirror_feedback(
    conn: &Connection,
    id: &str,
    feedback_json: &str,
    updated_at: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE knowledge_units SET last_mirror_feedback = ?1,
            status = CASE WHEN status IN ('raw','synthesized','understood','articulated') THEN 'validated' ELSE status END,
            last_reviewed_at = ?2, updated_at = ?2
         WHERE id = ?3",
        params![feedback_json, updated_at, id],
    )
    .map_err(|e| format!("update_knowledge_unit_mirror_feedback 失败: {e}"))?;
    Ok(())
}

pub fn update_knowledge_unit_review_schedule(
    conn: &Connection,
    id: &str,
    next_review_due: Option<&str>,
    depth_level: i64,
    updated_at: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE knowledge_units SET next_review_due = ?1, depth_level = ?2, last_reviewed_at = ?3, updated_at = ?3 WHERE id = ?4",
        params![next_review_due, depth_level, updated_at, id],
    )
    .map_err(|e| format!("update_knowledge_unit_review_schedule 失败: {e}"))?;
    Ok(())
}

pub fn delete_knowledge_unit(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM knowledge_units WHERE id = ?1", params![id])
        .map_err(|e| format!("delete_knowledge_unit 失败: {e}"))?;
    Ok(())
}

pub fn get_knowledge_units_due_for_review(
    conn: &Connection,
    library_id: &str,
    today: &str,
    limit: i64,
) -> Result<Vec<KnowledgeUnitSummary>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT
                ku.id, ku.library_id, ku.title, ku.core_insight,
                ku.status, ku.depth_level,
                ku.source_asset_ids,
                ku.next_review_due, ku.last_reviewed_at, ku.updated_at,
                COUNT(DISTINCT s.id) as snapshot_count
             FROM knowledge_units ku
             LEFT JOIN understanding_snapshots s ON s.knowledge_unit_id = ku.id
             WHERE ku.library_id = ?1
               AND ku.next_review_due <= ?2
             GROUP BY ku.id
             ORDER BY ku.next_review_due ASC
             LIMIT ?3",
        )
        .map_err(|e| format!("prepare get_knowledge_units_due 失败: {e}"))?;

    let rows = stmt
        .query_map(params![library_id, today, limit], |row| {
            let source_ids_json: String = row.get(6)?;
            let source_count = serde_json::from_str::<Vec<serde_json::Value>>(&source_ids_json)
                .map(|v| v.len() as i64)
                .unwrap_or(0);
            Ok(KnowledgeUnitSummary {
                id: row.get(0)?,
                library_id: row.get(1)?,
                title: row.get(2)?,
                core_insight: row.get(3)?,
                status: row.get(4)?,
                depth_level: row.get(5)?,
                source_asset_count: source_count,
                next_review_due: row.get(7)?,
                last_reviewed_at: row.get(8)?,
                updated_at: row.get(9)?,
                snapshot_count: row.get(10)?,
            })
        })
        .map_err(|e| format!("query get_knowledge_units_due 失败: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect get_knowledge_units_due 失败: {e}"))
}

fn row_to_knowledge_unit(row: &rusqlite::Row) -> rusqlite::Result<KnowledgeUnit> {
    let parse_json_vec = |s: String| -> Vec<String> {
        serde_json::from_str(&s).unwrap_or_default()
    };
    Ok(KnowledgeUnit {
        id: row.get(0)?,
        library_id: row.get(1)?,
        title: row.get(2)?,
        core_insight: row.get(3)?,
        summary: row.get(4)?,
        explanation: row.get(5)?,
        constituent_concept_ids: parse_json_vec(row.get::<_, String>(6).unwrap_or_default()),
        source_asset_ids: parse_json_vec(row.get::<_, String>(7).unwrap_or_default()),
        status: row.get(8)?,
        user_note: row.get(9)?,
        last_mirror_feedback: row.get(10)?,
        depth_level: row.get(11)?,
        legacy_concept_ids: parse_json_vec(row.get::<_, String>(12).unwrap_or_default()),
        first_captured_at: row.get(13)?,
        last_reviewed_at: row.get(14)?,
        next_review_due: row.get(15)?,
        created_at: row.get(16)?,
        updated_at: row.get(17)?,
    })
}

// ─── 理解快照 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnderstandingSnapshot {
    pub id: String,
    pub knowledge_unit_id: String,
    pub user_explanation: String,
    pub mirror_covered_count: i64,
    pub mirror_covered_points: Vec<String>,
    pub mirror_missed_areas: Vec<String>,
    pub depth_level_at_time: i64,
    pub source_asset_count_at_time: i64,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSnapshot {
    pub id: String,
    pub knowledge_unit_id: String,
    pub user_explanation: String,
    pub mirror_covered_count: i64,
    pub mirror_covered_points: Vec<String>,
    pub mirror_missed_areas: Vec<String>,
    pub depth_level_at_time: i64,
    pub source_asset_count_at_time: i64,
    pub timestamp: String,
}

pub fn insert_snapshot(conn: &Connection, snap: &CreateSnapshot) -> Result<(), String> {
    conn.execute(
        "INSERT INTO understanding_snapshots (
            id, knowledge_unit_id, user_explanation,
            mirror_covered_count, mirror_covered_points, mirror_missed_areas,
            depth_level_at_time, source_asset_count_at_time, timestamp
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![
            snap.id,
            snap.knowledge_unit_id,
            snap.user_explanation,
            snap.mirror_covered_count,
            serde_json::to_string(&snap.mirror_covered_points).unwrap_or_default(),
            serde_json::to_string(&snap.mirror_missed_areas).unwrap_or_default(),
            snap.depth_level_at_time,
            snap.source_asset_count_at_time,
            snap.timestamp,
        ],
    )
    .map_err(|e| format!("insert_snapshot 失败: {e}"))?;
    Ok(())
}

pub fn get_snapshots(
    conn: &Connection,
    knowledge_unit_id: &str,
) -> Result<Vec<UnderstandingSnapshot>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, knowledge_unit_id, user_explanation,
                    mirror_covered_count, mirror_covered_points, mirror_missed_areas,
                    depth_level_at_time, source_asset_count_at_time, timestamp
             FROM understanding_snapshots
             WHERE knowledge_unit_id = ?1
             ORDER BY timestamp ASC",
        )
        .map_err(|e| format!("prepare get_snapshots 失败: {e}"))?;

    let rows = stmt
        .query_map(params![knowledge_unit_id], |row| {
            let parse = |s: String| -> Vec<String> {
                serde_json::from_str(&s).unwrap_or_default()
            };
            Ok(UnderstandingSnapshot {
                id: row.get(0)?,
                knowledge_unit_id: row.get(1)?,
                user_explanation: row.get(2)?,
                mirror_covered_count: row.get(3)?,
                mirror_covered_points: parse(row.get::<_, String>(4).unwrap_or_default()),
                mirror_missed_areas: parse(row.get::<_, String>(5).unwrap_or_default()),
                depth_level_at_time: row.get(6)?,
                source_asset_count_at_time: row.get(7)?,
                timestamp: row.get(8)?,
            })
        })
        .map_err(|e| format!("query get_snapshots 失败: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect get_snapshots 失败: {e}"))
}

// ─── 素材推断 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetInference {
    pub id: String,
    pub asset_id: String,
    pub session_id: Option<String>,
    pub session_peer_ids: Vec<String>,
    pub dominant_topics: Vec<String>,
    pub novelty_score: f64,
    pub closest_knowledge_ids: Vec<String>,
    pub closest_scores: Vec<f64>,
    pub inferred_course: Option<String>,
    pub inferred_type: String,
    pub is_supplementary: bool,
    pub supplement_target_id: Option<String>,
    pub confidence: f64,
    pub ambiguity_reason: Option<String>,
    pub created_at: String,
}

pub fn upsert_asset_inference(conn: &Connection, inf: &AssetInference) -> Result<(), String> {
    conn.execute(
        "INSERT INTO asset_inferences (
            id, asset_id, session_id, session_peer_ids, dominant_topics,
            novelty_score, closest_knowledge_ids, closest_scores,
            inferred_course, inferred_type, is_supplementary, supplement_target_id,
            confidence, ambiguity_reason, created_at
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)
         ON CONFLICT(asset_id) DO UPDATE SET
            session_id = excluded.session_id,
            session_peer_ids = excluded.session_peer_ids,
            dominant_topics = excluded.dominant_topics,
            novelty_score = excluded.novelty_score,
            closest_knowledge_ids = excluded.closest_knowledge_ids,
            closest_scores = excluded.closest_scores,
            inferred_course = excluded.inferred_course,
            inferred_type = excluded.inferred_type,
            is_supplementary = excluded.is_supplementary,
            supplement_target_id = excluded.supplement_target_id,
            confidence = excluded.confidence,
            ambiguity_reason = excluded.ambiguity_reason",
        params![
            inf.id,
            inf.asset_id,
            inf.session_id,
            serde_json::to_string(&inf.session_peer_ids).unwrap_or_default(),
            serde_json::to_string(&inf.dominant_topics).unwrap_or_default(),
            inf.novelty_score,
            serde_json::to_string(&inf.closest_knowledge_ids).unwrap_or_default(),
            serde_json::to_string(&inf.closest_scores).unwrap_or_default(),
            inf.inferred_course,
            inf.inferred_type,
            if inf.is_supplementary { 1i64 } else { 0i64 },
            inf.supplement_target_id,
            inf.confidence,
            inf.ambiguity_reason,
            inf.created_at,
        ],
    )
    .map_err(|e| format!("upsert_asset_inference 失败: {e}"))?;
    Ok(())
}

pub fn get_asset_inference(
    conn: &Connection,
    asset_id: &str,
) -> Result<Option<AssetInference>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, asset_id, session_id, session_peer_ids, dominant_topics,
                    novelty_score, closest_knowledge_ids, closest_scores,
                    inferred_course, inferred_type, is_supplementary, supplement_target_id,
                    confidence, ambiguity_reason, created_at
             FROM asset_inferences WHERE asset_id = ?1",
        )
        .map_err(|e| format!("prepare get_asset_inference 失败: {e}"))?;

    let result = stmt
        .query_row(params![asset_id], |row| {
            let parse_vec_str = |s: String| -> Vec<String> {
                serde_json::from_str(&s).unwrap_or_default()
            };
            let parse_vec_f64 = |s: String| -> Vec<f64> {
                serde_json::from_str(&s).unwrap_or_default()
            };
            Ok(AssetInference {
                id: row.get(0)?,
                asset_id: row.get(1)?,
                session_id: row.get(2)?,
                session_peer_ids: parse_vec_str(row.get::<_, String>(3).unwrap_or_default()),
                dominant_topics: parse_vec_str(row.get::<_, String>(4).unwrap_or_default()),
                novelty_score: row.get(5)?,
                closest_knowledge_ids: parse_vec_str(row.get::<_, String>(6).unwrap_or_default()),
                closest_scores: parse_vec_f64(row.get::<_, String>(7).unwrap_or_default()),
                inferred_course: row.get(8)?,
                inferred_type: row.get(9)?,
                is_supplementary: row.get::<_, i64>(10).unwrap_or(0) != 0,
                supplement_target_id: row.get(11)?,
                confidence: row.get(12)?,
                ambiguity_reason: row.get(13)?,
                created_at: row.get(14)?,
            })
        })
        .optional()
        .map_err(|e| format!("get_asset_inference 失败: {e}"))?;
    Ok(result)
}

// ─── 语音备注 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceMemo {
    pub id: String,
    pub asset_id: Option<String>,
    pub audio_path: String,
    pub transcript: String,
    pub memo_type: String, // supplementary/standalone/question/connection
    pub link_target_id: Option<String>,
    pub link_reason: Option<String>,
    pub captured_at: String,
    pub created_at: String,
}

pub fn insert_voice_memo(conn: &Connection, memo: &VoiceMemo) -> Result<(), String> {
    conn.execute(
        "INSERT INTO voice_memos (id, asset_id, audio_path, transcript, memo_type, link_target_id, link_reason, captured_at, created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![
            memo.id,
            memo.asset_id,
            memo.audio_path,
            memo.transcript,
            memo.memo_type,
            memo.link_target_id,
            memo.link_reason,
            memo.captured_at,
            memo.created_at,
        ],
    )
    .map_err(|e| format!("insert_voice_memo 失败: {e}"))?;
    Ok(())
}

pub fn update_voice_memo_classification(
    conn: &Connection,
    id: &str,
    memo_type: &str,
    link_target_id: Option<&str>,
    link_reason: Option<&str>,
) -> Result<(), String> {
    conn.execute(
        "UPDATE voice_memos SET memo_type = ?1, link_target_id = ?2, link_reason = ?3 WHERE id = ?4",
        params![memo_type, link_target_id, link_reason, id],
    )
    .map_err(|e| format!("update_voice_memo_classification 失败: {e}"))?;
    Ok(())
}

pub fn get_voice_memos_unarchived(conn: &Connection) -> Result<Vec<VoiceMemo>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, asset_id, audio_path, transcript, memo_type, link_target_id, link_reason, captured_at, created_at
             FROM voice_memos
             WHERE memo_type = 'standalone' AND link_target_id IS NULL
             ORDER BY captured_at DESC",
        )
        .map_err(|e| format!("prepare get_voice_memos_unarchived 失败: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(VoiceMemo {
                id: row.get(0)?,
                asset_id: row.get(1)?,
                audio_path: row.get(2)?,
                transcript: row.get(3)?,
                memo_type: row.get(4)?,
                link_target_id: row.get(5)?,
                link_reason: row.get(6)?,
                captured_at: row.get(7)?,
                created_at: row.get(8)?,
            })
        })
        .map_err(|e| format!("query get_voice_memos_unarchived 失败: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect get_voice_memos_unarchived 失败: {e}"))
}

pub fn get_voice_memos_for_unit(
    conn: &Connection,
    knowledge_unit_id: &str,
) -> Result<Vec<VoiceMemo>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, asset_id, audio_path, transcript, memo_type, link_target_id, link_reason, captured_at, created_at
             FROM voice_memos
             WHERE link_target_id = ?1
             ORDER BY captured_at DESC",
        )
        .map_err(|e| format!("prepare get_voice_memos_for_unit 失败: {e}"))?;

    let rows = stmt
        .query_map(params![knowledge_unit_id], |row| {
            Ok(VoiceMemo {
                id: row.get(0)?,
                asset_id: row.get(1)?,
                audio_path: row.get(2)?,
                transcript: row.get(3)?,
                memo_type: row.get(4)?,
                link_target_id: row.get(5)?,
                link_reason: row.get(6)?,
                captured_at: row.get(7)?,
                created_at: row.get(8)?,
            })
        })
        .map_err(|e| format!("query get_voice_memos_for_unit 失败: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect get_voice_memos_for_unit 失败: {e}"))
}
