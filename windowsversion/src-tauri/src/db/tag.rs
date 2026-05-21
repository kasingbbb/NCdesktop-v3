use crate::models::Tag;
use rusqlite::{params, Connection, OptionalExtension};

pub fn insert(conn: &Connection, tag: &Tag) -> Result<(), String> {
    conn.execute(
        "INSERT INTO tags (id, name, color, source, usage_count) VALUES (?1,?2,?3,?4,?5)",
        params![tag.id, tag.name, tag.color, tag.source, tag.usage_count],
    )
    .map_err(|e| format!("插入标签失败: {e}"))?;
    Ok(())
}

pub fn get_all(conn: &Connection) -> Result<Vec<Tag>, String> {
    let mut stmt = conn
        .prepare("SELECT id, name, color, source, usage_count FROM tags ORDER BY usage_count DESC")
        .map_err(|e| format!("查询标签失败: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                source: row.get(3)?,
                usage_count: row.get(4)?,
            })
        })
        .map_err(|e| format!("遍历标签失败: {e}"))?;

    let mut result = Vec::new();
    for r in rows {
        result.push(r.map_err(|e| format!("读取行失败: {e}"))?);
    }
    Ok(result)
}

pub fn get_by_id(conn: &Connection, id: &str) -> Result<Option<Tag>, String> {
    conn.query_row(
        "SELECT id, name, color, source, usage_count FROM tags WHERE id = ?1",
        params![id],
        |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                source: row.get(3)?,
                usage_count: row.get(4)?,
            })
        },
    )
    .optional()
    .map_err(|e| format!("查询标签失败: {e}"))
}

pub fn get_or_create_by_name(conn: &Connection, name: &str, source: &str) -> Result<Tag, String> {
    if let Some(existing) = conn
        .query_row(
            "SELECT id, name, color, source, usage_count FROM tags WHERE name = ?1",
            params![name],
            |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                    source: row.get(3)?,
                    usage_count: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(|e| format!("查询标签失败: {e}"))?
    {
        return Ok(existing);
    }

    let tag = Tag {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        color: "#808080".to_string(),
        source: source.to_string(),
        usage_count: 0,
    };
    insert(conn, &tag)?;
    Ok(tag)
}

fn refresh_tag_usage_count(conn: &Connection, tag_id: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE tags SET usage_count = (SELECT COUNT(*) FROM asset_tags WHERE tag_id = ?1) + (SELECT COUNT(*) FROM project_tags WHERE tag_id = ?1) WHERE id = ?1",
        params![tag_id],
    )
    .map_err(|e| format!("更新标签计数失败: {e}"))?;
    Ok(())
}

pub fn unlink_from_asset(conn: &Connection, asset_id: &str, tag_id: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM asset_tags WHERE asset_id = ?1 AND tag_id = ?2",
        params![asset_id, tag_id],
    )
    .map_err(|e| format!("解除素材标签失败: {e}"))?;
    refresh_tag_usage_count(conn, tag_id)?;
    Ok(())
}

pub fn link_to_asset(conn: &Connection, asset_id: &str, tag_id: &str) -> Result<(), String> {
    conn.execute(
        "INSERT OR IGNORE INTO asset_tags (asset_id, tag_id) VALUES (?1, ?2)",
        params![asset_id, tag_id],
    )
    .map_err(|e| format!("关联素材标签失败: {e}"))?;

    refresh_tag_usage_count(conn, tag_id)?;

    Ok(())
}

/// 将原件（root）已绑定的全部标签复制到一个具体的衍生件（derivative）上。
///
/// 用于：MarkItDown 转换完成、刚刚 INSERT 出 .md 衍生件时调用，
/// 让衍生件继承原件当前已有的所有标签。
///
/// 使用 `INSERT OR IGNORE` 保证幂等：同一对 (asset_id, tag_id) 重复调用不会产生多行。
/// 不在此处刷新 tags.usage_count——上游的 link_to_asset 已经维护过；
/// 同一标签关联到衍生件后，usage_count 的语义是"被多少 asset_tags 行引用"，
/// 应该跟随实际行数变化，因此我们在最后单独刷新所有被影响的 tag_id。
///
/// 返回写入的新行数（已存在被 IGNORE 的不计入）。
pub fn propagate_tags_to_derivative(
    conn: &Connection,
    root_asset_id: &str,
    derived_asset_id: &str,
) -> Result<usize, String> {
    let inserted = conn
        .execute(
            "INSERT OR IGNORE INTO asset_tags (asset_id, tag_id) \
             SELECT ?1, tag_id FROM asset_tags WHERE asset_id = ?2",
            params![derived_asset_id, root_asset_id],
        )
        .map_err(|e| format!("传播原件标签到衍生件失败: {e}"))?;

    if inserted > 0 {
        refresh_usage_count_for_asset_tags(conn, derived_asset_id)?;
    }

    Ok(inserted)
}

/// 将原件（root）当前全部标签，同步到该原件下所有 `asset_type='markdown'` 的衍生件。
///
/// 用于：AI 打标流程在原件上写入新标签后调用——确保已经存在的 .md 衍生件
/// 也能拿到这些新增标签（解决"先转换、后 AI 打标"的时序漏洞）。
///
/// 同样使用 `INSERT OR IGNORE`，幂等。
/// 返回写入的新行数。
pub fn sync_tags_to_canonical_derivatives(
    conn: &Connection,
    root_asset_id: &str,
) -> Result<usize, String> {
    let inserted = conn
        .execute(
            "INSERT OR IGNORE INTO asset_tags (asset_id, tag_id) \
             SELECT a.id, at.tag_id FROM assets a \
             JOIN asset_tags at ON at.asset_id = ?1 \
             WHERE a.source_asset_id = ?1 AND a.asset_type = 'markdown'",
            params![root_asset_id],
        )
        .map_err(|e| format!("同步原件标签到 markdown 衍生件失败: {e}"))?;

    if inserted > 0 {
        // 把所有受影响的 tag_id 的 usage_count 刷新一遍
        let mut stmt = conn
            .prepare("SELECT DISTINCT tag_id FROM asset_tags WHERE asset_id = ?1")
            .map_err(|e| format!("准备刷新 usage_count 查询失败: {e}"))?;
        let tag_ids: Vec<String> = stmt
            .query_map(params![root_asset_id], |row| row.get::<_, String>(0))
            .map_err(|e| format!("查询原件标签失败: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("读取标签 id 失败: {e}"))?;
        for tag_id in tag_ids {
            refresh_tag_usage_count(conn, &tag_id)?;
        }
    }

    Ok(inserted)
}

/// 辅助：刷新指定 asset 当前所有标签的 usage_count。
fn refresh_usage_count_for_asset_tags(conn: &Connection, asset_id: &str) -> Result<(), String> {
    let mut stmt = conn
        .prepare("SELECT tag_id FROM asset_tags WHERE asset_id = ?1")
        .map_err(|e| format!("准备刷新 usage_count 查询失败: {e}"))?;
    let tag_ids: Vec<String> = stmt
        .query_map(params![asset_id], |row| row.get::<_, String>(0))
        .map_err(|e| format!("查询素材标签失败: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("读取标签 id 失败: {e}"))?;
    for tag_id in tag_ids {
        refresh_tag_usage_count(conn, &tag_id)?;
    }
    Ok(())
}

pub fn link_to_project(conn: &Connection, project_id: &str, tag_id: &str) -> Result<(), String> {
    conn.execute(
        "INSERT OR IGNORE INTO project_tags (project_id, tag_id) VALUES (?1, ?2)",
        params![project_id, tag_id],
    )
    .map_err(|e| format!("关联项目标签失败: {e}"))?;

    refresh_tag_usage_count(conn, tag_id)?;

    Ok(())
}

pub fn get_tags_for_asset(conn: &Connection, asset_id: &str) -> Result<Vec<Tag>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT t.id, t.name, t.color, t.source, t.usage_count
             FROM tags t INNER JOIN asset_tags at ON t.id = at.tag_id
             WHERE at.asset_id = ?1 ORDER BY t.name",
        )
        .map_err(|e| format!("查询素材标签失败: {e}"))?;

    let rows = stmt
        .query_map(params![asset_id], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                source: row.get(3)?,
                usage_count: row.get(4)?,
            })
        })
        .map_err(|e| format!("遍历标签失败: {e}"))?;

    let mut result = Vec::new();
    for r in rows {
        result.push(r.map_err(|e| format!("读取行失败: {e}"))?);
    }
    Ok(result)
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM tags WHERE id = ?1", params![id])
        .map_err(|e| format!("删除标签失败: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migration;
    use crate::models::Tag;

    /// 准备一个跑完全部 migration 的内存库，并写入 1 个 library / 1 个 project。
    fn setup_db() -> (Connection, String) {
        let conn = Connection::open_in_memory().expect("打开内存库失败");
        migration::run_migrations(&conn).expect("迁移失败");

        let library_id = "lib_test".to_string();
        conn.execute(
            "INSERT INTO libraries (id, name, root_path) VALUES (?1, ?2, ?3)",
            params![library_id, "test_lib", "/tmp/test"],
        )
        .expect("插入 library 失败");

        let project_id = "proj_test".to_string();
        conn.execute(
            "INSERT INTO projects (id, library_id, name) VALUES (?1, ?2, ?3)",
            params![project_id, library_id, "test_proj"],
        )
        .expect("插入 project 失败");

        (conn, project_id)
    }

    fn insert_asset(
        conn: &Connection,
        project_id: &str,
        id: &str,
        asset_type: &str,
        source_asset_id: Option<&str>,
    ) {
        conn.execute(
            "INSERT INTO assets (id, project_id, asset_type, name, file_path, source_asset_id) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id,
                project_id,
                asset_type,
                format!("{id}.bin"),
                format!("/tmp/{id}"),
                source_asset_id
            ],
        )
        .expect("插入 asset 失败");
    }

    fn make_tag(conn: &Connection, name: &str) -> Tag {
        get_or_create_by_name(conn, name, "ai").expect("创建标签失败")
    }

    fn count_asset_tags(conn: &Connection, asset_id: &str) -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM asset_tags WHERE asset_id = ?1",
            params![asset_id],
            |row| row.get::<_, i64>(0),
        )
        .expect("count 查询失败")
    }

    /// 场景 A：原件已有衍生 .md，propagate_tags_to_derivative 把全部标签复制过去。
    #[test]
    fn scenario_a_propagate_copies_all_tags() {
        let (conn, project_id) = setup_db();
        insert_asset(&conn, &project_id, "root1", "pdf", None);
        insert_asset(&conn, &project_id, "deriv1", "markdown", Some("root1"));

        let t1 = make_tag(&conn, "数学");
        let t2 = make_tag(&conn, "物理");
        link_to_asset(&conn, "root1", &t1.id).unwrap();
        link_to_asset(&conn, "root1", &t2.id).unwrap();

        assert_eq!(count_asset_tags(&conn, "deriv1"), 0);

        let inserted =
            propagate_tags_to_derivative(&conn, "root1", "deriv1").expect("propagate 失败");
        assert_eq!(inserted, 2);
        assert_eq!(count_asset_tags(&conn, "deriv1"), 2);
    }

    /// 场景 B：衍生件已存在，原件后补 AI 标签 → sync 把新增标签同步过去。
    #[test]
    fn scenario_b_sync_to_existing_derivatives() {
        let (conn, project_id) = setup_db();
        insert_asset(&conn, &project_id, "root2", "pdf", None);
        insert_asset(&conn, &project_id, "deriv2a", "markdown", Some("root2"));
        insert_asset(&conn, &project_id, "deriv2b", "markdown", Some("root2"));
        // 一个非 markdown 衍生件，应被忽略
        insert_asset(&conn, &project_id, "deriv2c", "image", Some("root2"));

        let t1 = make_tag(&conn, "课程");
        let t2 = make_tag(&conn, "AI生成");
        link_to_asset(&conn, "root2", &t1.id).unwrap();
        link_to_asset(&conn, "root2", &t2.id).unwrap();

        let inserted =
            sync_tags_to_canonical_derivatives(&conn, "root2").expect("sync 失败");
        // 两个 markdown 衍生件 × 2 个标签 = 4 行
        assert_eq!(inserted, 4);
        assert_eq!(count_asset_tags(&conn, "deriv2a"), 2);
        assert_eq!(count_asset_tags(&conn, "deriv2b"), 2);
        // 非 markdown 衍生件不应被同步
        assert_eq!(count_asset_tags(&conn, "deriv2c"), 0);
    }

    /// 场景 C：重复调用 propagate 不产生重复行。
    #[test]
    fn scenario_c_propagate_is_idempotent() {
        let (conn, project_id) = setup_db();
        insert_asset(&conn, &project_id, "root3", "pdf", None);
        insert_asset(&conn, &project_id, "deriv3", "markdown", Some("root3"));

        let t1 = make_tag(&conn, "笔记");
        link_to_asset(&conn, "root3", &t1.id).unwrap();

        let first = propagate_tags_to_derivative(&conn, "root3", "deriv3").unwrap();
        let second = propagate_tags_to_derivative(&conn, "root3", "deriv3").unwrap();
        let third = propagate_tags_to_derivative(&conn, "root3", "deriv3").unwrap();

        assert_eq!(first, 1);
        assert_eq!(second, 0);
        assert_eq!(third, 0);
        assert_eq!(count_asset_tags(&conn, "deriv3"), 1);
    }
}
