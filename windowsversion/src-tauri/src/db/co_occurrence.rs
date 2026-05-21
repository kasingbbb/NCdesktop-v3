use rusqlite::{params, Connection};
use std::collections::HashSet;

/// 计算 library 内所有概念的共现关系并写入 concept_relations 表。
///
/// 算法：
/// 1. 读取 library 内全部概念 (id, source_asset_ids)
/// 2. 两两配对（O(n²)），确保 concept_a_id < concept_b_id（字典序），避免重复边
/// 3. 检查两个概念的 source_asset_ids 是否有交集
/// 4. 有交集 → INSERT ... ON CONFLICT DO UPDATE，更新 co_occurrence_count 与 source_asset_ids
/// 5. 全程在事务内执行
/// 6. 返回处理的共现对数（含新增和更新）
pub fn compute_co_occurrence(
    conn: &Connection,
    library_id: &str,
) -> Result<usize, String> {
    let start = std::time::Instant::now();

    // ── 1. 读取全部概念 ──────────────────────────────────────────────────────
    let concepts = fetch_concepts_with_asset_ids(conn, library_id)?;

    let n = concepts.len();
    if n < 2 {
        eprintln!("[co_occurrence] library={library_id} concepts={n}, 少于 2 个概念，跳过");
        return Ok(0);
    }

    eprintln!("[co_occurrence] library={library_id} concepts={n}, 开始两两配对 ({} 对)", n * (n - 1) / 2);

    // ── 2. 开启事务 ──────────────────────────────────────────────────────────
    let tx = conn.unchecked_transaction()
        .map_err(|e| format!("事务开启失败: {e}"))?;

    let mut relation_count = 0usize;

    // ── 3. 两两配对 ──────────────────────────────────────────────────────────
    for i in 0..n {
        for j in (i + 1)..n {
            let (id_a_raw, asset_ids_a) = &concepts[i];
            let (id_b_raw, asset_ids_b) = &concepts[j];

            // 字典序排序，确保 a < b
            let (concept_a_id, asset_set_a, concept_b_id, asset_set_b) =
                if id_a_raw.as_str() < id_b_raw.as_str() {
                    (id_a_raw, asset_ids_a, id_b_raw, asset_ids_b)
                } else {
                    (id_b_raw, asset_ids_b, id_a_raw, asset_ids_a)
                };

            // ── 4. 计算交集 ────────────────────────────────────────────────
            let intersection: Vec<&String> = asset_set_a
                .iter()
                .filter(|aid| asset_set_b.contains(*aid))
                .collect();

            if intersection.is_empty() {
                continue;
            }

            // 交集 asset IDs → JSON
            let shared_ids: Vec<String> = intersection.into_iter().cloned().collect();
            let shared_json = serde_json::to_string(&shared_ids)
                .unwrap_or_else(|_| "[]".to_string());

            let relation_id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();

            // ── 5. 写入 concept_relations ───────────────────────────────────
            tx.execute(
                "INSERT INTO concept_relations
                   (id, concept_a_id, concept_b_id, relation_type, source_asset_ids, co_occurrence_count, created_at)
                 VALUES (?1, ?2, ?3, 'co_occurrence', ?4, 1, ?5)
                 ON CONFLICT(concept_a_id, concept_b_id, relation_type) DO UPDATE SET
                   co_occurrence_count = co_occurrence_count + 1,
                   source_asset_ids = excluded.source_asset_ids",
                params![
                    relation_id,
                    concept_a_id,
                    concept_b_id,
                    shared_json,
                    now,
                ],
            )
            .map_err(|e| format!("写入共现关系失败: {e}"))?;

            relation_count += 1;
        }
    }

    // ── 6. 提交事务 ──────────────────────────────────────────────────────────
    tx.commit().map_err(|e| format!("事务提交失败: {e}"))?;

    let elapsed = start.elapsed();
    eprintln!(
        "[co_occurrence] 完成：library={library_id}, 关系数={relation_count}, 耗时={:.3}s",
        elapsed.as_secs_f64()
    );

    Ok(relation_count)
}

/// 从数据库读取指定 library 的全部概念 (id, source_asset_ids HashSet)
fn fetch_concepts_with_asset_ids(
    conn: &Connection,
    library_id: &str,
) -> Result<Vec<(String, HashSet<String>)>, String> {
    let mut stmt = conn
        .prepare("SELECT id, source_asset_ids FROM concepts WHERE library_id = ?1")
        .map_err(|e| format!("准备查询概念失败: {e}"))?;

    let rows = stmt
        .query_map(params![library_id], |row| {
            let id: String = row.get(0)?;
            let asset_ids_json: Option<String> = row.get(1)?;
            Ok((id, asset_ids_json))
        })
        .map_err(|e| format!("查询概念失败: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("读取概念行失败: {e}"))?;

    let result = rows
        .into_iter()
        .map(|(id, asset_ids_json)| {
            let ids: HashSet<String> = asset_ids_json
                .as_deref()
                .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
                .unwrap_or_default()
                .into_iter()
                .collect();
            (id, ids)
        })
        .collect();

    Ok(result)
}

// ─────────────────────────────────────────────────────────────────────────────
// 单元测试
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::db::knowledge::{insert_concept, Concept};

    fn open_db() -> Database {
        let dir = tempfile::tempdir().expect("tempdir");
        Database::open(&dir.path().join("co_occ_test.db")).expect("open db")
    }

    fn make_concept(library_id: &str, name: &str, asset_ids: Vec<&str>) -> Concept {
        let now = chrono::Utc::now().to_rfc3339();
        Concept {
            id: uuid::Uuid::new_v4().to_string(),
            library_id: library_id.to_string(),
            name: name.to_string(),
            aliases: vec![],
            definition: None,
            source_asset_ids: asset_ids.into_iter().map(|s| s.to_string()).collect(),
            source_project_ids: vec![],
            user_edited: false,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    // AC-5: 空 library 返回 0，不报错
    #[test]
    fn empty_library_returns_zero() {
        let db = open_db();
        let conn = db.conn.lock().unwrap();
        let count = compute_co_occurrence(&conn, "lib-empty").unwrap();
        assert_eq!(count, 0, "空 library 应返回 0");
    }

    // AC-5: 只有 1 个概念，无法配对，返回 0
    #[test]
    fn single_concept_returns_zero() {
        let db = open_db();
        let conn = db.conn.lock().unwrap();
        let c = make_concept("lib-1", "概念A", vec!["asset-1"]);
        insert_concept(&conn, &c).unwrap();
        let count = compute_co_occurrence(&conn, "lib-1").unwrap();
        assert_eq!(count, 0, "单个概念无法配对，应返回 0");
    }

    // AC-1 + AC-3 + AC-4: 2 个概念共享 1 个 asset → 产生 1 条关系，方向正确
    #[test]
    fn two_concepts_shared_asset_produces_one_relation() {
        let db = open_db();
        let conn = db.conn.lock().unwrap();

        let c1 = make_concept("lib-1", "概念A", vec!["asset-shared"]);
        let c2 = make_concept("lib-1", "概念B", vec!["asset-shared", "asset-extra"]);
        insert_concept(&conn, &c1).unwrap();
        insert_concept(&conn, &c2).unwrap();

        let count = compute_co_occurrence(&conn, "lib-1").unwrap();
        assert_eq!(count, 1, "有共享 asset 应产生 1 条关系");

        // 验证关系记录存在
        let rel_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM concept_relations WHERE relation_type = 'co_occurrence'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(rel_count, 1, "concept_relations 表应有 1 条记录");

        // 验证方向性：concept_a_id < concept_b_id
        let (a_id, b_id): (String, String) = conn
            .query_row(
                "SELECT concept_a_id, concept_b_id FROM concept_relations WHERE relation_type = 'co_occurrence' LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert!(
            a_id.as_str() < b_id.as_str(),
            "concept_a_id 应小于 concept_b_id（字典序）: a={a_id}, b={b_id}"
        );
    }

    // 2 个概念，无共享 asset → 不产生关系
    #[test]
    fn two_concepts_no_shared_asset_produces_no_relation() {
        let db = open_db();
        let conn = db.conn.lock().unwrap();

        let c1 = make_concept("lib-1", "概念X", vec!["asset-1"]);
        let c2 = make_concept("lib-1", "概念Y", vec!["asset-2"]);
        insert_concept(&conn, &c1).unwrap();
        insert_concept(&conn, &c2).unwrap();

        let count = compute_co_occurrence(&conn, "lib-1").unwrap();
        assert_eq!(count, 0, "无共享 asset 不应产生关系");

        let rel_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM concept_relations WHERE relation_type = 'co_occurrence'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(rel_count, 0, "concept_relations 表应为空");
    }

    // AC-4: 重复计算（ON CONFLICT DO UPDATE）更新计数，不重复插入
    #[test]
    fn repeated_compute_updates_count_not_duplicate() {
        let db = open_db();
        let conn = db.conn.lock().unwrap();

        let c1 = make_concept("lib-1", "概念P", vec!["asset-shared"]);
        let c2 = make_concept("lib-1", "概念Q", vec!["asset-shared"]);
        insert_concept(&conn, &c1).unwrap();
        insert_concept(&conn, &c2).unwrap();

        // 第一次计算
        let count1 = compute_co_occurrence(&conn, "lib-1").unwrap();
        assert_eq!(count1, 1);

        // 第二次计算（重复）
        let count2 = compute_co_occurrence(&conn, "lib-1").unwrap();
        assert_eq!(count2, 1);

        // 记录数仍为 1（ON CONFLICT DO UPDATE，不重复插入）
        let rel_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM concept_relations WHERE relation_type = 'co_occurrence'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(rel_count, 1, "重复计算不应产生重复行");

        // co_occurrence_count 已被更新（+1）
        let occ_count: i64 = conn
            .query_row(
                "SELECT co_occurrence_count FROM concept_relations WHERE relation_type = 'co_occurrence' LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(occ_count, 2, "重复计算后 co_occurrence_count 应为 2");
    }

    // AC-3: 方向性验证 — 无论哪个概念 ID 字典序更大，写入时都保证 a < b
    #[test]
    fn direction_always_a_less_than_b() {
        let db = open_db();
        let conn = db.conn.lock().unwrap();

        // 构造两个具体 ID，确保 id_b < id_a（故意逆序）
        let now = chrono::Utc::now().to_rfc3339();
        // 用固定 UUID 以确定字典序
        let id_larger = "ffffffff-ffff-ffff-ffff-ffffffffffff".to_string();
        let id_smaller = "00000000-0000-0000-0000-000000000001".to_string();

        let c_large = Concept {
            id: id_larger.clone(),
            library_id: "lib-dir".to_string(),
            name: "概念Large".to_string(),
            aliases: vec![],
            definition: None,
            source_asset_ids: vec!["asset-x".to_string()],
            source_project_ids: vec![],
            user_edited: false,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let c_small = Concept {
            id: id_smaller.clone(),
            library_id: "lib-dir".to_string(),
            name: "概念Small".to_string(),
            aliases: vec![],
            definition: None,
            source_asset_ids: vec!["asset-x".to_string()],
            source_project_ids: vec![],
            user_edited: false,
            created_at: now.clone(),
            updated_at: now,
        };

        insert_concept(&conn, &c_large).unwrap();
        insert_concept(&conn, &c_small).unwrap();

        compute_co_occurrence(&conn, "lib-dir").unwrap();

        let (a_id, b_id): (String, String) = conn
            .query_row(
                "SELECT concept_a_id, concept_b_id FROM concept_relations WHERE relation_type = 'co_occurrence' LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();

        assert_eq!(a_id, id_smaller, "a_id 应为字典序更小的 ID");
        assert_eq!(b_id, id_larger, "b_id 应为字典序更大的 ID");
        assert!(a_id < b_id, "concept_a_id < concept_b_id 必须成立");
    }

    // 3 个概念：A↔B 共享 asset, A↔C 共享 asset, B↔C 不共享 → 产生 2 条关系
    #[test]
    fn three_concepts_partial_overlap() {
        let db = open_db();
        let conn = db.conn.lock().unwrap();

        let ca = make_concept("lib-3", "概念A", vec!["asset-1", "asset-2"]);
        let cb = make_concept("lib-3", "概念B", vec!["asset-1"]);          // 与A共享 asset-1
        let cc = make_concept("lib-3", "概念C", vec!["asset-2"]);          // 与A共享 asset-2，B↔C无共享
        insert_concept(&conn, &ca).unwrap();
        insert_concept(&conn, &cb).unwrap();
        insert_concept(&conn, &cc).unwrap();

        let count = compute_co_occurrence(&conn, "lib-3").unwrap();
        assert_eq!(count, 2, "A↔B 和 A↔C 各有共享，应产生 2 条关系");

        let rel_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM concept_relations WHERE relation_type = 'co_occurrence'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(rel_count, 2);
    }
}
