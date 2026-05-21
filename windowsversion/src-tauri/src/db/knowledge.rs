use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// 数据结构
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Concept {
    pub id: String,
    pub library_id: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub definition: Option<String>,
    pub source_asset_ids: Vec<String>,
    pub source_project_ids: Vec<String>,
    pub user_edited: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// 概念摘要 + 统计（左侧列表用）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptWithStats {
    pub id: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub definition: Option<String>,
    pub source_project_count: usize,
    pub viewpoint_count: usize,
    pub case_count: usize,
    pub user_edited: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptViewpoint {
    pub id: String,
    pub concept_id: String,
    pub perspective: String,
    pub summary: String,
    pub source_context: Option<String>,
    pub source_asset_id: Option<String>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptCase {
    pub id: String,
    pub concept_id: String,
    pub title: String,
    pub excerpt: String,
    pub source_asset_id: Option<String>,
    pub source_location: Option<String>,
    pub relevance_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptExtension {
    pub id: String,
    pub concept_id: String,
    pub direction: String, // "upstream" | "downstream"
    pub name: String,
    pub description: Option<String>,
    pub relationship: Option<String>,
}

/// 概念完整详情（右侧面板）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptDetail {
    pub concept: Concept,
    pub viewpoints: Vec<ConceptViewpoint>,
    pub cases: Vec<ConceptCase>,
    pub extensions: Vec<ConceptExtension>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Concept CRUD
// ─────────────────────────────────────────────────────────────────────────────

/// 插入概念（幂等：基于 library_id + name，忽略重复）
pub fn insert_concept(conn: &Connection, c: &Concept) -> Result<(), String> {
    let aliases_json = serde_json::to_string(&c.aliases).unwrap_or_default();
    let asset_ids_json = serde_json::to_string(&c.source_asset_ids).unwrap_or_default();
    let proj_ids_json = serde_json::to_string(&c.source_project_ids).unwrap_or_default();
    conn.execute(
        "INSERT OR IGNORE INTO concepts
         (id, library_id, name, aliases, definition, source_asset_ids, source_project_ids,
          user_edited, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            c.id, c.library_id, c.name, aliases_json, c.definition,
            asset_ids_json, proj_ids_json, c.user_edited as i32,
            c.created_at, c.updated_at,
        ],
    )
    .map_err(|e| format!("插入概念失败: {e}"))?;
    Ok(())
}

/// 获取概念列表（含统计，用于左侧面板）
pub fn get_concepts_with_stats(
    conn: &Connection,
    library_id: &str,
) -> Result<Vec<ConceptWithStats>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT
               c.id, c.name, c.aliases, c.definition,
               c.source_project_ids, c.user_edited,
               (SELECT count(*) FROM concept_viewpoints WHERE concept_id = c.id) AS vp_count,
               (SELECT count(*) FROM concept_cases WHERE concept_id = c.id) AS case_count
             FROM concepts c
             WHERE c.library_id = ?1
             ORDER BY c.name ASC",
        )
        .map_err(|e| format!("查询概念列表失败: {e}"))?;

    let rows = stmt
        .query_map(params![library_id], |row| {
            let aliases_json: Option<String> = row.get(2)?;
            let proj_ids_json: Option<String> = row.get(4)?;
            let user_edited: i32 = row.get(5)?;
            let vp_count: i64 = row.get(6)?;
            let case_count: i64 = row.get(7)?;

            let aliases: Vec<String> = aliases_json
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();
            let proj_ids: Vec<String> = proj_ids_json
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();

            Ok(ConceptWithStats {
                id: row.get(0)?,
                name: row.get(1)?,
                aliases,
                definition: row.get(3)?,
                source_project_count: proj_ids.len(),
                viewpoint_count: vp_count as usize,
                case_count: case_count as usize,
                user_edited: user_edited != 0,
            })
        })
        .map_err(|e| format!("遍历概念失败: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("读取概念行失败: {e}"))?;

    Ok(rows)
}

/// 按 ID 获取单个概念完整详情（含观点/案例/拓展）
pub fn get_concept_detail(
    conn: &Connection,
    concept_id: &str,
) -> Result<Option<ConceptDetail>, String> {
    let concept = conn
        .query_row(
            "SELECT id, library_id, name, aliases, definition, source_asset_ids,
                    source_project_ids, user_edited, created_at, updated_at
             FROM concepts WHERE id = ?1",
            params![concept_id],
            row_to_concept,
        )
        .optional()
        .map_err(|e| format!("查询概念失败: {e}"))?;

    let concept = match concept {
        Some(c) => c,
        None => return Ok(None),
    };

    let viewpoints = get_viewpoints(conn, concept_id)?;
    let cases = get_cases(conn, concept_id)?;
    let extensions = get_extensions(conn, concept_id)?;

    Ok(Some(ConceptDetail { concept, viewpoints, cases, extensions }))
}

/// 更新概念名/定义（标记 user_edited）
pub fn update_concept(
    conn: &Connection,
    id: &str,
    name: Option<&str>,
    definition: Option<&str>,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    if let Some(n) = name {
        conn.execute(
            "UPDATE concepts SET name=?2, user_edited=1, updated_at=?3 WHERE id=?1",
            params![id, n, now],
        )
        .map_err(|e| format!("更新概念名失败: {e}"))?;
    }
    if let Some(d) = definition {
        conn.execute(
            "UPDATE concepts SET definition=?2, user_edited=1, updated_at=?3 WHERE id=?1",
            params![id, d, now],
        )
        .map_err(|e| format!("更新概念定义失败: {e}"))?;
    }
    Ok(())
}

/// 删除概念（级联删除观点/案例/拓展）
pub fn delete_concept(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM concepts WHERE id = ?1", params![id])
        .map_err(|e| format!("删除概念失败: {e}"))?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Viewpoint CRUD
// ─────────────────────────────────────────────────────────────────────────────

pub fn insert_viewpoint(conn: &Connection, v: &ConceptViewpoint) -> Result<(), String> {
    conn.execute(
        "INSERT OR IGNORE INTO concept_viewpoints
         (id, concept_id, perspective, summary, source_context, source_asset_id, generated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            v.id, v.concept_id, v.perspective, v.summary,
            v.source_context, v.source_asset_id, v.generated_at,
        ],
    )
    .map_err(|e| format!("插入观点失败: {e}"))?;
    Ok(())
}

/// 删除某概念的全部观点（重新生成前清空）
pub fn delete_viewpoints_for_concept(conn: &Connection, concept_id: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM concept_viewpoints WHERE concept_id = ?1",
        params![concept_id],
    )
    .map_err(|e| format!("删除观点失败: {e}"))?;
    Ok(())
}

pub fn get_viewpoints(conn: &Connection, concept_id: &str) -> Result<Vec<ConceptViewpoint>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, concept_id, perspective, summary, source_context, source_asset_id, generated_at
             FROM concept_viewpoints WHERE concept_id = ?1 ORDER BY generated_at ASC",
        )
        .map_err(|e| format!("查询观点失败: {e}"))?;

    let rows: Result<Vec<_>, _> = stmt
        .query_map(params![concept_id], |row| {
            Ok(ConceptViewpoint {
                id: row.get(0)?,
                concept_id: row.get(1)?,
                perspective: row.get(2)?,
                summary: row.get(3)?,
                source_context: row.get(4)?,
                source_asset_id: row.get(5)?,
                generated_at: row.get(6)?,
            })
        })
        .map_err(|e| format!("遍历观点失败: {e}"))?
        .collect();

    rows.map_err(|e| format!("读取观点行失败: {e}"))
}

// ─────────────────────────────────────────────────────────────────────────────
// Case CRUD
// ─────────────────────────────────────────────────────────────────────────────

pub fn insert_case(conn: &Connection, c: &ConceptCase) -> Result<(), String> {
    conn.execute(
        "INSERT OR IGNORE INTO concept_cases
         (id, concept_id, title, excerpt, source_asset_id, source_location, relevance_note)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            c.id, c.concept_id, c.title, c.excerpt,
            c.source_asset_id, c.source_location, c.relevance_note,
        ],
    )
    .map_err(|e| format!("插入案例失败: {e}"))?;
    Ok(())
}

pub fn get_cases(conn: &Connection, concept_id: &str) -> Result<Vec<ConceptCase>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, concept_id, title, excerpt, source_asset_id, source_location, relevance_note
             FROM concept_cases WHERE concept_id = ?1",
        )
        .map_err(|e| format!("查询案例失败: {e}"))?;

    let rows: Result<Vec<_>, _> = stmt
        .query_map(params![concept_id], |row| {
            Ok(ConceptCase {
                id: row.get(0)?,
                concept_id: row.get(1)?,
                title: row.get(2)?,
                excerpt: row.get(3)?,
                source_asset_id: row.get(4)?,
                source_location: row.get(5)?,
                relevance_note: row.get(6)?,
            })
        })
        .map_err(|e| format!("遍历案例失败: {e}"))?
        .collect();

    rows.map_err(|e| format!("读取案例行失败: {e}"))
}

// ─────────────────────────────────────────────────────────────────────────────
// Extension CRUD
// ─────────────────────────────────────────────────────────────────────────────

pub fn insert_extension(conn: &Connection, e: &ConceptExtension) -> Result<(), String> {
    conn.execute(
        "INSERT OR IGNORE INTO concept_extensions
         (id, concept_id, direction, name, description, relationship)
         VALUES (?1,?2,?3,?4,?5,?6)",
        params![
            e.id, e.concept_id, e.direction, e.name, e.description, e.relationship,
        ],
    )
    .map_err(|err| format!("插入拓展失败: {err}"))?;
    Ok(())
}

pub fn delete_extensions_for_concept(conn: &Connection, concept_id: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM concept_extensions WHERE concept_id = ?1",
        params![concept_id],
    )
    .map_err(|e| format!("删除拓展失败: {e}"))?;
    Ok(())
}

pub fn get_extensions(conn: &Connection, concept_id: &str) -> Result<Vec<ConceptExtension>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, concept_id, direction, name, description, relationship
             FROM concept_extensions WHERE concept_id = ?1 ORDER BY direction, name",
        )
        .map_err(|e| format!("查询拓展失败: {e}"))?;

    let rows: Result<Vec<_>, _> = stmt
        .query_map(params![concept_id], |row| {
            Ok(ConceptExtension {
                id: row.get(0)?,
                concept_id: row.get(1)?,
                direction: row.get(2)?,
                name: row.get(3)?,
                description: row.get(4)?,
                relationship: row.get(5)?,
            })
        })
        .map_err(|e| format!("遍历拓展失败: {e}"))?
        .collect();

    rows.map_err(|e| format!("读取拓展行失败: {e}"))
}

// ─────────────────────────────────────────────────────────────────────────────
// 工具
// ─────────────────────────────────────────────────────────────────────────────

fn row_to_concept(row: &rusqlite::Row) -> rusqlite::Result<Concept> {
    let aliases_json: Option<String> = row.get(3)?;
    let asset_ids_json: Option<String> = row.get(5)?;
    let proj_ids_json: Option<String> = row.get(6)?;
    let user_edited: i32 = row.get(7)?;

    Ok(Concept {
        id: row.get(0)?,
        library_id: row.get(1)?,
        name: row.get(2)?,
        aliases: aliases_json.as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default(),
        definition: row.get(4)?,
        source_asset_ids: asset_ids_json.as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default(),
        source_project_ids: proj_ids_json.as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default(),
        user_edited: user_edited != 0,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// 单元测试
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn open_db() -> Database {
        let dir = tempfile::tempdir().expect("tempdir");
        Database::open(&dir.path().join("test.db")).expect("open db")
    }

    fn make_concept(library_id: &str, name: &str) -> Concept {
        let now = chrono::Utc::now().to_rfc3339();
        Concept {
            id: uuid::Uuid::new_v4().to_string(),
            library_id: library_id.to_string(),
            name: name.to_string(),
            aliases: vec!["alias1".to_string()],
            definition: Some(format!("{name} 的定义")),
            source_asset_ids: vec!["asset-1".to_string()],
            source_project_ids: vec!["proj-1".to_string()],
            user_edited: false,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    #[test]
    fn insert_and_get_concept() {
        let db = open_db();
        let conn = db.conn.lock().unwrap();
        let c = make_concept("lib-1", "边际效用递减");
        insert_concept(&conn, &c).unwrap();

        let detail = get_concept_detail(&conn, &c.id).unwrap().expect("应能查到");
        assert_eq!(detail.concept.name, "边际效用递减");
        assert_eq!(detail.concept.aliases, vec!["alias1"]);
    }

    #[test]
    fn get_concepts_with_stats_returns_counts() {
        let db = open_db();
        let conn = db.conn.lock().unwrap();

        let c = make_concept("lib-1", "供需均衡");
        insert_concept(&conn, &c).unwrap();

        // 插入 2 个观点
        for i in 0..2 {
            insert_viewpoint(&conn, &ConceptViewpoint {
                id: uuid::Uuid::new_v4().to_string(),
                concept_id: c.id.clone(),
                perspective: format!("视角{i}"),
                summary: "摘要".to_string(),
                source_context: None,
                source_asset_id: None,
                generated_at: chrono::Utc::now().to_rfc3339(),
            }).unwrap();
        }
        // 插入 1 个案例
        insert_case(&conn, &ConceptCase {
            id: uuid::Uuid::new_v4().to_string(),
            concept_id: c.id.clone(),
            title: "案例1".to_string(),
            excerpt: "摘录内容".to_string(),
            source_asset_id: None,
            source_location: None,
            relevance_note: None,
        }).unwrap();

        let list = get_concepts_with_stats(&conn, "lib-1").unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].viewpoint_count, 2);
        assert_eq!(list[0].case_count, 1);
        assert_eq!(list[0].source_project_count, 1); // source_project_ids: ["proj-1"]
    }

    #[test]
    fn update_concept_marks_user_edited() {
        let db = open_db();
        let conn = db.conn.lock().unwrap();
        let c = make_concept("lib-1", "认知偏差");
        insert_concept(&conn, &c).unwrap();

        update_concept(&conn, &c.id, None, Some("认知偏差的新定义")).unwrap();

        let detail = get_concept_detail(&conn, &c.id).unwrap().unwrap();
        assert!(detail.concept.user_edited);
        assert_eq!(detail.concept.definition.as_deref(), Some("认知偏差的新定义"));
    }

    #[test]
    fn delete_concept_cascades() {
        let db = open_db();
        let conn = db.conn.lock().unwrap();
        let c = make_concept("lib-1", "享乐适应");
        insert_concept(&conn, &c).unwrap();

        insert_viewpoint(&conn, &ConceptViewpoint {
            id: uuid::Uuid::new_v4().to_string(),
            concept_id: c.id.clone(),
            perspective: "心理学视角".to_string(),
            summary: "内容".to_string(),
            source_context: None, source_asset_id: None,
            generated_at: chrono::Utc::now().to_rfc3339(),
        }).unwrap();

        insert_extension(&conn, &ConceptExtension {
            id: uuid::Uuid::new_v4().to_string(),
            concept_id: c.id.clone(),
            direction: "upstream".to_string(),
            name: "效用理论".to_string(),
            description: None, relationship: None,
        }).unwrap();

        delete_concept(&conn, &c.id).unwrap();

        // 概念不存在
        assert!(get_concept_detail(&conn, &c.id).unwrap().is_none());
        // 观点 & 拓展级联删除
        assert!(get_viewpoints(&conn, &c.id).unwrap().is_empty());
        assert!(get_extensions(&conn, &c.id).unwrap().is_empty());
    }

    #[test]
    fn insert_ignore_duplicate_concept() {
        let db = open_db();
        let conn = db.conn.lock().unwrap();
        let c = make_concept("lib-1", "信息不对称");
        insert_concept(&conn, &c).unwrap();
        // 相同 id 再插入，不报错
        insert_concept(&conn, &c).unwrap();
        let list = get_concepts_with_stats(&conn, "lib-1").unwrap();
        assert_eq!(list.len(), 1, "不应重复插入");
    }
}
