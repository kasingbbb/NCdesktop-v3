use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

// ─── 数据结构 ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    pub id: String,
    pub library_id: String,
    pub name: String,
    pub description: Option<String>,
    pub ku_ids: Vec<String>,
    /// "learning" | "practicing" | "verified"
    pub status: String,
    /// 0.0–1.0，validated/mastered KU 的比例
    pub progress: f64,
    pub last_challenge: Option<String>,   // JSON: SkillChallenge
    pub last_evaluation: Option<String>,  // JSON: SkillEvaluation
    pub verified_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ─── CRUD ─────────────────────────────────────────────────────────────────────

pub fn insert_skill(conn: &Connection, skill: &Skill) -> Result<(), String> {
    conn.execute(
        "INSERT INTO skills (id, library_id, name, description, ku_ids, status, progress,
                             last_challenge, last_evaluation, verified_at, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![
            skill.id,
            skill.library_id,
            skill.name,
            skill.description,
            serde_json::to_string(&skill.ku_ids).unwrap_or_default(),
            skill.status,
            skill.progress,
            skill.last_challenge,
            skill.last_evaluation,
            skill.verified_at,
            skill.created_at,
            skill.updated_at,
        ],
    )
    .map_err(|e| format!("insert_skill 失败: {e}"))?;
    Ok(())
}

pub fn get_skills(conn: &Connection, library_id: &str) -> Result<Vec<Skill>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, library_id, name, description, ku_ids, status, progress,
                    last_challenge, last_evaluation, verified_at, created_at, updated_at
             FROM skills WHERE library_id = ?1 ORDER BY progress DESC, updated_at DESC",
        )
        .map_err(|e| format!("prepare get_skills 失败: {e}"))?;

    let rows = stmt
        .query_map(params![library_id], row_to_skill)
        .map_err(|e| format!("query get_skills 失败: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect get_skills 失败: {e}"))
}

pub fn get_skill(conn: &Connection, id: &str) -> Result<Option<Skill>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, library_id, name, description, ku_ids, status, progress,
                    last_challenge, last_evaluation, verified_at, created_at, updated_at
             FROM skills WHERE id = ?1",
        )
        .map_err(|e| format!("prepare get_skill 失败: {e}"))?;

    stmt.query_row(params![id], row_to_skill)
        .optional()
        .map_err(|e| format!("get_skill 失败: {e}"))
}

pub fn update_skill_progress(
    conn: &Connection,
    id: &str,
    progress: f64,
    status: &str,
    updated_at: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE skills SET progress = ?1, status = ?2, updated_at = ?3 WHERE id = ?4",
        params![progress, status, updated_at, id],
    )
    .map_err(|e| format!("update_skill_progress 失败: {e}"))?;
    Ok(())
}

pub fn update_skill_ku_ids(
    conn: &Connection,
    id: &str,
    ku_ids_json: &str,
    updated_at: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE skills SET ku_ids = ?1, updated_at = ?2 WHERE id = ?3",
        params![ku_ids_json, updated_at, id],
    )
    .map_err(|e| format!("update_skill_ku_ids 失败: {e}"))?;
    Ok(())
}

pub fn update_skill_challenge(
    conn: &Connection,
    id: &str,
    challenge_json: &str,
    updated_at: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE skills SET last_challenge = ?1, updated_at = ?2 WHERE id = ?3",
        params![challenge_json, updated_at, id],
    )
    .map_err(|e| format!("update_skill_challenge 失败: {e}"))?;
    Ok(())
}

pub fn update_skill_evaluation(
    conn: &Connection,
    id: &str,
    evaluation_json: &str,
    status: &str,
    verified_at: Option<&str>,
    updated_at: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE skills SET last_evaluation = ?1, status = ?2, verified_at = ?3, updated_at = ?4
         WHERE id = ?5",
        params![evaluation_json, status, verified_at, updated_at, id],
    )
    .map_err(|e| format!("update_skill_evaluation 失败: {e}"))?;
    Ok(())
}

pub fn delete_skill(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM skills WHERE id = ?1", params![id])
        .map_err(|e| format!("delete_skill 失败: {e}"))?;
    Ok(())
}

// ─── 行转换 ───────────────────────────────────────────────────────────────────

fn row_to_skill(row: &rusqlite::Row) -> rusqlite::Result<Skill> {
    let ku_ids_json: String = row.get::<_, String>(4).unwrap_or_else(|_| "[]".to_string());
    Ok(Skill {
        id: row.get(0)?,
        library_id: row.get(1)?,
        name: row.get(2)?,
        description: row.get(3)?,
        ku_ids: serde_json::from_str(&ku_ids_json).unwrap_or_default(),
        status: row.get(5)?,
        progress: row.get(6)?,
        last_challenge: row.get(7)?,
        last_evaluation: row.get(8)?,
        verified_at: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}
