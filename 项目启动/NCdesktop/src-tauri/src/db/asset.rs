use crate::models::{AIAnalysisRow, Asset, AssetState};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;

/// `list_root_assets` 单查询返回的"非 Asset 字段"集合。
///
/// 与 `Asset`（root 行字段）配对返回，由 commands 层在拿到此结构后做
/// `Path::exists()` stat、调用 `compute_asset_state` 派生四态，最终拼成
/// `WorkspaceAssetView` —— **db 层不做任何 IO**（ADR-003 / 硬约束）。
#[derive(Debug, Clone)]
pub struct AssetListJoinRow {
    /// canonical markdown 衍生件 id
    pub rendition_id: Option<String>,
    /// canonical markdown 文件绝对路径
    pub rendition_path: Option<String>,
    /// canonical markdown 文件大小（来自 assets 行）
    pub rendition_size: Option<i64>,
    /// hotfix-H3: canonical markdown 文件展示名（用于工作区列表覆盖 root.name）
    pub rendition_name: Option<String>,
    /// hotfix-H3: canonical markdown mime（覆盖 root.mime_type，便于前端图标判定）
    pub rendition_mime: Option<String>,
    /// hotfix-H3: canonical markdown asset_type（覆盖 root.asset_type，前端类型筛选）
    pub rendition_asset_type: Option<String>,
    /// 最近一条 pipeline_tasks.status（queued / running / completed / failed / cancelled）
    pub pipeline_status: Option<String>,
    /// 最近一条 pipeline_tasks.error_message
    pub pipeline_error: Option<String>,
    /// 最近一条 conversion_meta.error_class（NULL = 成功 / 未尝试）
    pub latest_error_class: Option<String>,
    /// 最近一条 conversion_meta.fallback_used（用于诊断）
    pub latest_fallback_used: Option<bool>,
    /// extracted_content.status（pending / extracting / extracted / failed 等）
    pub extraction_status: Option<String>,
    /// task_014 Fix-A4：extracted_content.extractor_type，
    /// 前端用于区分 "placeholder_*"（占位 MD）vs 真 MD。空字符串视为未知。
    pub extractor_type: Option<String>,
    /// task_014 AC-4：最近一行 `conversion_meta.failure_code`。
    /// - `"legacy_unverified"`：V14 backfill 标注的"老成功 + 空内容"；
    /// - 8 错误码之一（`E_*`）：明确失败；
    /// - `None`：当前为成功 / 未尝试转换。
    /// 前端据此渲染三态 badge（success / legacy_unverified / failed）。
    pub latest_failure_code: Option<String>,
}

pub fn insert(conn: &Connection, a: &Asset) -> Result<(), String> {
    conn.execute(
        "INSERT INTO assets (id, project_id, asset_type, name, original_name, file_path, file_size,
         mime_type, captured_at, imported_at, source_type, source_data, is_starred,
         source_asset_id, derivative_version)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
        params![
            a.id,
            a.project_id,
            a.asset_type,
            a.name,
            a.original_name,
            a.file_path,
            a.file_size,
            a.mime_type,
            a.captured_at,
            a.imported_at,
            a.source_type,
            a.source_data,
            a.is_starred as i32,
            a.source_asset_id,
            a.derivative_version,
        ],
    )
    .map_err(|e| format!("插入素材失败: {e}"))?;
    Ok(())
}

const ASSET_SELECT: &str = "SELECT id, project_id, asset_type, name, original_name, file_path, file_size,
             mime_type, captured_at, imported_at, source_type, source_data, is_starred,
             source_asset_id, derivative_version
             FROM assets";

/// **已弃用**：工作区列表请改用 [`list_root_assets`]（ADR-002）。
///
/// 此函数返回项目内**所有** asset 行（含 markdown 衍生件），用于工作区
/// 列表时会出现"一个原件 + 一个 markdown 衍生件"的双条目（PRD §9 R1）。
/// 仅保留供非工作区视图（搜索、时间轴、导出、知识中枢）继续使用。
///
/// 调用方若仍是工作区路径，请迁移到 `list_root_assets`。
#[deprecated(note = "工作区列表请使用 list_root_assets；本函数仅供非工作区视图使用")]
pub fn get_by_project(conn: &Connection, project_id: &str) -> Result<Vec<Asset>, String> {
    let mut stmt = conn
        .prepare(
            &format!(
                "{ASSET_SELECT} WHERE project_id = ?1 ORDER BY imported_at DESC"
            ),
        )
        .map_err(|e| format!("查询素材失败: {e}"))?;

    let rows = stmt
        .query_map(params![project_id], |row| row_to_asset(row))
        .map_err(|e| format!("遍历素材失败: {e}"))?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| format!("读取行失败: {e}"))?);
    }
    Ok(result)
}

/// 当前项目中打了指定标签的素材
pub fn get_by_project_and_tag(
    conn: &Connection,
    project_id: &str,
    tag_id: &str,
) -> Result<Vec<Asset>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT a.id, a.project_id, a.asset_type, a.name, a.original_name, a.file_path, a.file_size,
             a.mime_type, a.captured_at, a.imported_at, a.source_type, a.source_data, a.is_starred,
             a.source_asset_id, a.derivative_version
             FROM assets a
             INNER JOIN asset_tags at ON a.id = at.asset_id
             WHERE a.project_id = ?1 AND at.tag_id = ?2
             ORDER BY a.imported_at DESC",
        )
        .map_err(|e| format!("按标签查询素材失败: {e}"))?;

    let rows = stmt
        .query_map(params![project_id, tag_id], |row| row_to_asset(row))
        .map_err(|e| format!("遍历素材失败: {e}"))?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| format!("读取行失败: {e}"))?);
    }
    Ok(result)
}

/// 解算 asset_id 到 (root, Option<derivative>) 配对（ADR-007）。
///
/// 工作区命令链以 asset_id 为唯一目标，但 UI 可能传入 root.id 也可能传入
/// markdown derivative.id（例如用户从知识中枢点了 .md 条目）。本函数把两种
/// 输入统一解算回 root + canonical markdown derivative：
///
/// - 传入 asset_id 是 root（`source_asset_id IS NULL`）：直接用 root，
///   derivative 通过 [`find_markdown_derivative`] 查；
/// - 传入 asset_id 是 derivative（`source_asset_id IS NOT NULL`）：按
///   `source_asset_id` 反查 root，再用 root 调 `find_markdown_derivative`
///   （结果应包含传入的 asset_id）；
/// - 两查都没命中 → `Err("素材不存在")`。
pub fn resolve_asset_pair(
    conn: &Connection,
    asset_id: &str,
) -> Result<(Asset, Option<Asset>), String> {
    let asset = get_by_id(conn, asset_id)?.ok_or_else(|| "素材不存在".to_string())?;

    if asset.source_asset_id.is_none() {
        // 自身是 root
        let derivative = find_markdown_derivative(conn, &asset.id)?;
        Ok((asset, derivative))
    } else {
        // 自身是 derivative —— 反查 root
        let root_id = asset
            .source_asset_id
            .as_deref()
            .expect("source_asset_id 已在 is_none 分支外");
        let root = get_by_id(conn, root_id)?.ok_or_else(|| "素材不存在".to_string())?;
        let derivative = find_markdown_derivative(conn, &root.id)?;
        Ok((root, derivative))
    }
}

pub fn get_by_id(conn: &Connection, id: &str) -> Result<Option<Asset>, String> {
    conn.query_row(
        &format!("{ASSET_SELECT} WHERE id = ?1"),
        params![id],
        |row| row_to_asset(row),
    )
    .optional()
    .map_err(|e| format!("查询素材失败: {e}"))
}

/// 查找 root asset 的 canonical markdown 衍生件（若存在）。
///
/// 同一 root asset 全系统应只存在唯一 canonical markdown 衍生件（见
/// `session_context.md` 不可妥协底线 §2 与 ADR-001）。本函数按 `imported_at DESC`
/// 取最新一条做防御：理论上结果集应 ≤ 1，但若历史数据残留多行，返回最近一条
/// 供 scheduler 走"更新而非新建"的幂等分支（ADR-006）。
pub fn find_markdown_derivative(
    conn: &Connection,
    root_asset_id: &str,
) -> Result<Option<Asset>, String> {
    conn.query_row(
        &format!(
            "{ASSET_SELECT} WHERE source_asset_id = ?1 AND asset_type = 'markdown' \
             ORDER BY imported_at DESC LIMIT 1"
        ),
        params![root_asset_id],
        |row| row_to_asset(row),
    )
    .optional()
    .map_err(|e| format!("查询 markdown 衍生件失败: {e}"))
}

/// 已存在衍生件被覆盖写入时，更新展示名、文件大小与导入时间三列。
/// 不动 `file_path`（canonical 路径稳定，见 ADR-006）与 `derivative_version`
/// （由 `set_derivative_version` 单独推进，保证 source/derivative 双写不分叉）。
pub fn update_markdown_derivative(
    conn: &Connection,
    derived_asset_id: &str,
    new_name: &str,
    new_file_size: i64,
    new_imported_at: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE assets SET name = ?2, file_size = ?3, imported_at = ?4 WHERE id = ?1",
        params![derived_asset_id, new_name, new_file_size, new_imported_at],
    )
    .map_err(|e| format!("更新 markdown 衍生件失败: {e}"))?;
    Ok(())
}

/// 推进单个 asset 的 `derivative_version`。
/// scheduler 在 source 与 derivative 两侧分别调用，保证版本号双写对齐
/// （session_context.md §6 审查重点）。
pub fn set_derivative_version(
    conn: &Connection,
    asset_id: &str,
    new_version: i32,
) -> Result<(), String> {
    conn.execute(
        "UPDATE assets SET derivative_version = ?2 WHERE id = ?1",
        params![asset_id, new_version],
    )
    .map_err(|e| format!("更新 derivative_version 失败: {e}"))?;
    Ok(())
}

pub fn update(conn: &Connection, a: &Asset) -> Result<(), String> {
    conn.execute(
        "UPDATE assets SET name=?2, is_starred=?3 WHERE id=?1",
        params![a.id, a.name, a.is_starred as i32],
    )
    .map_err(|e| format!("更新素材失败: {e}"))?;
    Ok(())
}

/// 更新展示名与磁盘路径（如 AI 分类后整理到子目录）
pub fn update_name_and_path(
    conn: &Connection,
    id: &str,
    name: &str,
    file_path: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE assets SET name = ?2, file_path = ?3 WHERE id = ?1",
        params![id, name, file_path],
    )
    .map_err(|e| format!("更新素材路径失败: {e}"))?;
    Ok(())
}

/// custom_para_v1 / V17：写入 `assets.category_slug` 弱外键。
///
/// 不强制 FK（categories 表为 library 级，跨库无法直接 FK 引用）；
/// 由 `dropzone::resolve_or_create_category` 保证 slug 已在 `categories` 表存在。
/// 传入 `None` 表示清空（如分类回退到 `other`，应保持 category_slug 为 NULL）。
pub fn set_category_slug(
    conn: &Connection,
    id: &str,
    category_slug: Option<&str>,
) -> Result<(), String> {
    conn.execute(
        "UPDATE assets SET category_slug = ?2 WHERE id = ?1",
        params![id, category_slug],
    )
    .map_err(|e| format!("更新素材 category_slug 失败: {e}"))?;
    Ok(())
}

/// 跨项目移动：更新 project_id 与磁盘路径（BatchToolbar"移动到"路径）
pub fn update_project_and_path(
    conn: &Connection,
    id: &str,
    project_id: &str,
    file_path: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE assets SET project_id = ?2, file_path = ?3 WHERE id = ?1",
        params![id, project_id, file_path],
    )
    .map_err(|e| format!("移动素材失败: {e}"))?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM assets WHERE id = ?1", params![id])
        .map_err(|e| format!("删除素材失败: {e}"))?;
    Ok(())
}

/// task_006 M6 删除级联报告：返回给命令层用于日志 / 测试断言。
///
/// 字段语义：
/// - `removed_root_file` / `removed_derivative_file`：物理文件是否真的被
///   `fs::remove_file` 删除（文件本就不存在视为 false 而非错误，与 AC-3 一致）；
/// - `derivative_existed`：DB 中是否解算到了 markdown derivative（若无 derivative，
///   `removed_derivative_file` 必定为 false）；
/// - `removed_pipeline_tasks`：手工 `DELETE FROM pipeline_tasks` 影响的行数
///   （V7 表无 FK，必须显式清，见 task_001_architect §十 风险登记 R-pipeline-fk）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteCascadeReport {
    pub root_asset_id: String,
    pub derivative_asset_id: Option<String>,
    pub derivative_existed: bool,
    pub removed_root_file: bool,
    pub removed_derivative_file: bool,
    pub removed_pipeline_tasks: usize,
}

/// task_006 AC-3：以 asset_id 为入口的级联删除底座。
///
/// 接受 root.id 或 markdown derivative.id（通过 [`resolve_asset_pair`] 统一解算
/// 回 root），并完成下述清理：
///
/// 1. **物理文件**：删 root 与（如有）derivative 的磁盘文件；文件不存在视为
///    成功，仅在 IO 失败时 `warn!` 不中断流程（AC-4 同款"删除不被 IO 阻断"原则）。
/// 2. **手工 DELETE pipeline_tasks**：因 `pipeline_tasks` 在 V7 没有 FK 指向
///    `assets`（task_001_architect §十），FK CASCADE 不会自动清理，必须显式
///    `DELETE WHERE asset_id IN (root.id, derivative.id)`。
/// 3. **DELETE assets WHERE id = root.id**：FK CASCADE 自动联动
///    derivative（V5 `source_asset_id` 不带 FK，但 derivative 与 root 同属 assets
///    表，需要二次显式 DELETE，见步骤 4）；同时清 `conversion_meta`（V6 source_asset_id
///    CASCADE / derived_asset_id SET NULL）、`extracted_content`（V8 CASCADE）、
///    `asset_tags`（V1 CASCADE）。
/// 4. **显式 DELETE derivative**：`assets.source_asset_id` V5 加列时未带 FK，
///    所以 root 行删除后 derivative 行不会被自动清理 —— 用 derivative_id 再
///    `DELETE FROM assets WHERE id = ?` 一遍。
///
/// **不**清理 outbound 缓存目录（属命令层 IO 职责，由 `commands::asset::delete_asset`
/// 调用 `commands::outbound::outbound_cache_dir_for` + `fs::remove_dir_all`）。
pub fn delete_with_cascade(
    conn: &Connection,
    asset_id: &str,
) -> Result<DeleteCascadeReport, String> {
    let (root, derivative) = resolve_asset_pair(conn, asset_id)?;

    // ── 1. 物理文件清理（IO 失败仅 warn，不阻断 DB 删除）
    let removed_root_file = remove_file_lenient(&root.file_path);
    let (derivative_existed, removed_derivative_file, derivative_id) = match &derivative {
        Some(d) => (true, remove_file_lenient(&d.file_path), Some(d.id.clone())),
        None => (false, false, None),
    };

    // ── 2. 手工 DELETE pipeline_tasks（V7 表无 FK，必须显式清）
    let mut removed_pipeline_tasks: usize = 0;
    removed_pipeline_tasks += conn
        .execute(
            "DELETE FROM pipeline_tasks WHERE asset_id = ?1",
            params![root.id],
        )
        .map_err(|e| format!("清理 root pipeline_tasks 失败: {e}"))?;
    if let Some(ref did) = derivative_id {
        removed_pipeline_tasks += conn
            .execute(
                "DELETE FROM pipeline_tasks WHERE asset_id = ?1",
                params![did],
            )
            .map_err(|e| format!("清理 derivative pipeline_tasks 失败: {e}"))?;
    }

    // ── 3. DELETE assets WHERE id = root.id
    //   FK CASCADE 自动清：
    //     - conversion_meta.source_asset_id（V6 ON DELETE CASCADE）
    //     - conversion_meta.derived_asset_id（V6 ON DELETE SET NULL，对 derivative 行）
    //     - extracted_content.asset_id（V8 ON DELETE CASCADE）
    //     - asset_tags.asset_id（V1 ON DELETE CASCADE）
    //   注：`assets.source_asset_id` 是 V5 加列，未带 FK 约束，derivative 行
    //       不会被 root 删除自动联动，需在步骤 4 显式删。
    conn.execute("DELETE FROM assets WHERE id = ?1", params![root.id])
        .map_err(|e| format!("删除 root asset 失败: {e}"))?;

    // ── 4. 显式 DELETE derivative（assets.source_asset_id 未带 FK，需补刀）
    if let Some(ref did) = derivative_id {
        conn.execute("DELETE FROM assets WHERE id = ?1", params![did])
            .map_err(|e| format!("删除 derivative asset 失败: {e}"))?;
    }

    Ok(DeleteCascadeReport {
        root_asset_id: root.id,
        derivative_asset_id: derivative_id,
        derivative_existed,
        removed_root_file,
        removed_derivative_file,
        removed_pipeline_tasks,
    })
}

/// 删除单个文件：不存在视为成功（返回 false），其它 IO 错误 warn 不阻断
/// 上层 DB 级联。返回值表示"确实从磁盘移除了文件"。
fn remove_file_lenient(path: &str) -> bool {
    use std::fs;
    use std::io::ErrorKind;
    match fs::remove_file(path) {
        Ok(()) => true,
        Err(e) if e.kind() == ErrorKind::NotFound => false,
        Err(e) => {
            log::warn!("删除物理文件失败（继续走 DB 级联）: {path} — {e}");
            false
        }
    }
}

pub fn toggle_star(conn: &Connection, id: &str) -> Result<bool, String> {
    let current: i32 = conn
        .query_row(
            "SELECT is_starred FROM assets WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .map_err(|e| format!("查询素材星标失败: {e}"))?;

    let new_val = if current == 0 { 1 } else { 0 };
    conn.execute(
        "UPDATE assets SET is_starred = ?2 WHERE id = ?1",
        params![id, new_val],
    )
    .map_err(|e| format!("切换星标失败: {e}"))?;

    Ok(new_val != 0)
}

/// 项目内各素材的标签名列表（用于工作区视图）
pub fn get_tag_names_by_project(
    conn: &Connection,
    project_id: &str,
) -> Result<HashMap<String, Vec<String>>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT at.asset_id, t.name
             FROM asset_tags at
             INNER JOIN tags t ON t.id = at.tag_id
             INNER JOIN assets a ON a.id = at.asset_id AND a.project_id = ?1
             ORDER BY at.asset_id, t.name",
        )
        .map_err(|e| format!("查询素材标签失败: {e}"))?;

    let rows = stmt
        .query_map(params![project_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| format!("查询素材标签失败: {e}"))?;

    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for row in rows {
        let (asset_id, tag_name) = row.map_err(|e| format!("读取行失败: {e}"))?;
        map.entry(asset_id).or_default().push(tag_name);
    }
    Ok(map)
}

// AI 分析
pub fn upsert_analysis(conn: &Connection, a: &AIAnalysisRow) -> Result<(), String> {
    conn.execute(
        "INSERT INTO ai_analyses (id, asset_id, summary, topics, ocr_text, language, suggested_tags, suggested_name)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
         ON CONFLICT(asset_id) DO UPDATE SET
           summary=excluded.summary, topics=excluded.topics, ocr_text=excluded.ocr_text,
           language=excluded.language, suggested_tags=excluded.suggested_tags, suggested_name=excluded.suggested_name",
        params![a.id, a.asset_id, a.summary, a.topics, a.ocr_text, a.language, a.suggested_tags, a.suggested_name],
    )
    .map_err(|e| format!("写入 AI 分析失败: {e}"))?;
    Ok(())
}

pub fn get_analysis(conn: &Connection, asset_id: &str) -> Result<Option<AIAnalysisRow>, String> {
    conn.query_row(
        "SELECT id, asset_id, summary, topics, ocr_text, language, suggested_tags, suggested_name
         FROM ai_analyses WHERE asset_id = ?1",
        params![asset_id],
        |row| {
            Ok(AIAnalysisRow {
                id: row.get(0)?,
                asset_id: row.get(1)?,
                summary: row.get(2)?,
                topics: row.get(3)?,
                ocr_text: row.get(4)?,
                language: row.get(5)?,
                suggested_tags: row.get(6)?,
                suggested_name: row.get(7)?,
            })
        },
    )
    .optional()
    .map_err(|e| format!("查询 AI 分析失败: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migration::run_migrations;
    use crate::models::Asset;
    use rusqlite::Connection;

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("打开内存库失败");
        run_migrations(&conn).expect("迁移失败");
        // 生产路径在 `Database::open` 内打开 FK；测试需手工打开以验证
        // `delete_with_cascade` 的 FK CASCADE 行为（task_006 AC-6）。
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .expect("启用 FK 失败");
        conn
    }

    fn insert_project(conn: &Connection, id: &str) {
        // libraries 是 projects 的 FK 父表；SQLite 默认 PRAGMA foreign_keys=OFF，
        // 但显式插入对应行，避免未来打开 FK 时测试失效。
        conn.execute(
            "INSERT OR IGNORE INTO libraries (id, name, root_path) VALUES (?1, ?2, ?3)",
            params!["lib_test", "test_lib", "/tmp/test_lib"],
        )
        .expect("插入 library 失败");
        conn.execute(
            "INSERT INTO projects (id, library_id, name) VALUES (?1, ?2, ?3)",
            params![id, "lib_test", "test_proj"],
        )
        .expect("插入 project 失败");
    }

    fn mk_asset(id: &str, project_id: &str, asset_type: &str, imported_at: &str) -> Asset {
        Asset {
            id: id.to_string(),
            project_id: project_id.to_string(),
            asset_type: asset_type.to_string(),
            name: format!("{id}.bin"),
            original_name: format!("{id}.bin"),
            file_path: format!("/tmp/{id}.bin"),
            file_size: 100,
            mime_type: "application/octet-stream".to_string(),
            captured_at: imported_at.to_string(),
            imported_at: imported_at.to_string(),
            source_type: "import".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn find_markdown_derivative_returns_latest_match() {
        let conn = setup_conn();
        insert_project(&conn, "p1");

        // root（pdf 原件）
        let root = mk_asset("root1", "p1", "pdf", "2025-01-01T00:00:00Z");
        insert(&conn, &root).unwrap();

        // 较早的 markdown 衍生件
        let mut d_old = mk_asset("d_old", "p1", "markdown", "2025-01-02T00:00:00Z");
        d_old.source_asset_id = Some("root1".to_string());
        insert(&conn, &d_old).unwrap();

        // 较新的 markdown 衍生件（应该被命中）
        let mut d_new = mk_asset("d_new", "p1", "markdown", "2025-01-05T00:00:00Z");
        d_new.source_asset_id = Some("root1".to_string());
        insert(&conn, &d_new).unwrap();

        // 噪声：另一个 root 的 markdown 衍生件
        let mut other = mk_asset("d_other", "p1", "markdown", "2025-02-01T00:00:00Z");
        other.source_asset_id = Some("other_root".to_string());
        insert(&conn, &other).unwrap();

        // 噪声：source_asset_id=root1 但 asset_type != 'markdown'（不应命中）
        let mut not_md = mk_asset("d_img", "p1", "image", "2025-03-01T00:00:00Z");
        not_md.source_asset_id = Some("root1".to_string());
        insert(&conn, &not_md).unwrap();

        let result = find_markdown_derivative(&conn, "root1")
            .expect("查询失败")
            .expect("应返回 Some");
        assert_eq!(result.id, "d_new", "应返回 imported_at 最新的那条");
        assert_eq!(result.source_asset_id.as_deref(), Some("root1"));
        assert_eq!(result.asset_type, "markdown");
    }

    #[test]
    fn find_markdown_derivative_returns_none_when_absent() {
        let conn = setup_conn();
        insert_project(&conn, "p1");
        let root = mk_asset("root_alone", "p1", "pdf", "2025-01-01T00:00:00Z");
        insert(&conn, &root).unwrap();

        let result = find_markdown_derivative(&conn, "root_alone").expect("查询失败");
        assert!(result.is_none(), "无衍生件时应返回 Ok(None)");

        // 查一个完全不存在的 root_id 也应返回 None
        let result2 = find_markdown_derivative(&conn, "no_such_root").expect("查询失败");
        assert!(result2.is_none());
    }

    #[test]
    fn update_markdown_derivative_changes_only_three_columns() {
        let conn = setup_conn();
        insert_project(&conn, "p1");

        let mut deriv = mk_asset("md1", "p1", "markdown", "2025-01-01T00:00:00Z");
        deriv.source_asset_id = Some("root_x".to_string());
        deriv.derivative_version = 3;
        deriv.file_path = "/tmp/canonical/md1.md".to_string();
        insert(&conn, &deriv).unwrap();

        update_markdown_derivative(&conn, "md1", "renamed.md", 9999, "2025-06-06T00:00:00Z")
            .expect("update 失败");

        let after = get_by_id(&conn, "md1").unwrap().expect("应存在");
        // 改变的 3 列
        assert_eq!(after.name, "renamed.md");
        assert_eq!(after.file_size, 9999);
        assert_eq!(after.imported_at, "2025-06-06T00:00:00Z");
        // 未触碰的列保持原值
        assert_eq!(after.file_path, "/tmp/canonical/md1.md", "file_path 不应被改");
        assert_eq!(after.derivative_version, 3, "derivative_version 不应被改");
        assert_eq!(after.source_asset_id.as_deref(), Some("root_x"));
        assert_eq!(after.asset_type, "markdown");
    }

    // ---- compute_asset_state 纯函数单测（AC-2，覆盖 ≥ 6 组合） ----

    #[test]
    fn compute_state_done_when_completed_and_rendition_present() {
        let s = compute_asset_state(Some("completed"), None, true, true, false);
        assert_eq!(s, AssetState::Done);
    }

    #[test]
    fn compute_state_converting_for_queued_or_running() {
        assert_eq!(
            compute_asset_state(Some("queued"), None, false, true, false),
            AssetState::Converting
        );
        assert_eq!(
            compute_asset_state(Some("running"), None, false, true, false),
            AssetState::Converting
        );
    }

    #[test]
    fn compute_state_failed_when_pipeline_failed() {
        let s = compute_asset_state(Some("failed"), None, false, true, false);
        assert_eq!(s, AssetState::Failed);
    }

    #[test]
    fn compute_state_failed_when_conversion_meta_has_error_class() {
        // 即使 pipeline 已无任务（None），最近一条转换记录失败 → Failed
        let s = compute_asset_state(None, Some("timeout"), false, true, false);
        assert_eq!(s, AssetState::Failed);
    }

    #[test]
    fn compute_state_offline_when_no_pipeline_no_meta() {
        let s = compute_asset_state(None, None, false, true, false);
        assert_eq!(s, AssetState::Offline);
    }

    #[test]
    fn compute_state_offline_when_cancelled() {
        // cancelled 不计入 failed/converting
        let s = compute_asset_state(Some("cancelled"), None, false, true, false);
        assert_eq!(s, AssetState::Offline);
    }

    #[test]
    fn compute_state_done_even_when_source_missing() {
        // source 缺失不应改变四态 —— ADR-006 / PRD §S4
        let s_done = compute_asset_state(Some("completed"), None, true, false, true);
        assert_eq!(s_done, AssetState::Done, "source 缺失不改变 Done");

        let s_off = compute_asset_state(None, None, false, false, true);
        assert_eq!(s_off, AssetState::Offline, "source 缺失不改变 Offline");
    }

    #[test]
    fn compute_state_completed_without_rendition_falls_through() {
        // pipeline 标记 completed 但 rendition 缺失（用户删了 .md） → 不是 Done
        // 落入兜底 Offline（无 error_class），用户可触发重试入队
        let s = compute_asset_state(Some("completed"), None, false, true, false);
        assert_eq!(s, AssetState::Offline);
    }

    // ---- list_root_assets 集成单测（AC-1 / AC-6） ----

    fn insert_pipeline_task(
        conn: &Connection,
        id: &str,
        asset_id: &str,
        status: &str,
        created_at: &str,
        error: Option<&str>,
    ) {
        conn.execute(
            "INSERT INTO pipeline_tasks
                (id, asset_id, task_type, status, retry_count, max_retries,
                 error_message, priority, batch_id, created_at, started_at, completed_at)
             VALUES (?1, ?2, 'extract', ?3, 0, 3, ?4, 100, NULL, ?5, NULL, NULL)",
            params![id, asset_id, status, error, created_at],
        )
        .expect("insert pipeline_task");
    }

    fn insert_conversion_meta(
        conn: &Connection,
        id: &str,
        source_asset_id: &str,
        error_class: Option<&str>,
        converted_at: &str,
    ) {
        conn.execute(
            "INSERT INTO conversion_meta (
                id, source_asset_id, derived_asset_id, converter_name, converter_version,
                source_mime, source_hash, quality_level, fallback_used,
                error_class, conversion_ms, converted_at
             ) VALUES (?1, ?2, NULL, 'markitdown', '0.0.1', 'application/pdf',
                       'h', 0, 0, ?3, 100, ?4)",
            params![id, source_asset_id, error_class, converted_at],
        )
        .expect("insert conversion_meta");
    }

    fn insert_extracted(conn: &Connection, asset_id: &str, status: &str) {
        conn.execute(
            "INSERT INTO extracted_content
                (id, asset_id, status, error_message, retry_count, raw_text, structured_md,
                 quality_level, extractor_type, segments_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, 0, NULL, NULL, 0, 'markitdown', NULL,
                     '2025-01-01T00:00:00Z', '2025-01-01T00:00:00Z')",
            params![uuid::Uuid::new_v4().to_string(), asset_id, status],
        )
        .expect("insert extracted_content");
    }

    #[test]
    fn list_root_assets_excludes_markdown_derivative() {
        let conn = setup_conn();
        insert_project(&conn, "p1");

        let root = mk_asset("root1", "p1", "pdf", "2025-01-01T00:00:00Z");
        insert(&conn, &root).unwrap();

        let mut deriv = mk_asset("d1", "p1", "markdown", "2025-01-02T00:00:00Z");
        deriv.source_asset_id = Some("root1".to_string());
        deriv.file_path = "/tmp/canonical/d1.md".to_string();
        deriv.file_size = 4096;
        insert(&conn, &deriv).unwrap();

        let rows = list_root_assets(&conn, "p1").expect("list");
        assert_eq!(rows.len(), 1, "derivative 不应出现在 root 列表");
        let (asset, join) = &rows[0];
        assert_eq!(asset.id, "root1");
        assert_eq!(join.rendition_id.as_deref(), Some("d1"));
        assert_eq!(join.rendition_path.as_deref(), Some("/tmp/canonical/d1.md"));
        assert_eq!(join.rendition_size, Some(4096));
    }

    #[test]
    fn list_root_assets_empty_project_returns_empty_vec() {
        let conn = setup_conn();
        insert_project(&conn, "p_empty");
        let rows = list_root_assets(&conn, "p_empty").expect("list");
        assert!(rows.is_empty());
    }

    #[test]
    fn list_root_assets_orders_by_imported_at_desc() {
        let conn = setup_conn();
        insert_project(&conn, "p1");

        insert(&conn, &mk_asset("a_old", "p1", "pdf", "2025-01-01T00:00:00Z")).unwrap();
        insert(&conn, &mk_asset("a_mid", "p1", "pdf", "2025-02-01T00:00:00Z")).unwrap();
        insert(&conn, &mk_asset("a_new", "p1", "pdf", "2025-03-01T00:00:00Z")).unwrap();

        let rows = list_root_assets(&conn, "p1").expect("list");
        let ids: Vec<&str> = rows.iter().map(|(a, _)| a.id.as_str()).collect();
        assert_eq!(ids, vec!["a_new", "a_mid", "a_old"]);
    }

    #[test]
    fn list_root_assets_joins_latest_pipeline_and_conversion_meta() {
        let conn = setup_conn();
        insert_project(&conn, "p1");
        insert(&conn, &mk_asset("r1", "p1", "pdf", "2025-01-01T00:00:00Z")).unwrap();

        // 旧任务：failed 在前，新任务：completed 在后 —— 应取最新
        insert_pipeline_task(&conn, "t1", "r1", "failed", "2025-01-01T10:00:00Z", Some("早期失败"));
        insert_pipeline_task(&conn, "t2", "r1", "completed", "2025-01-02T10:00:00Z", None);

        // 旧 conversion_meta 失败，新一次成功（error_class=None） —— 应取最新
        insert_conversion_meta(&conn, "cm1", "r1", Some("timeout"), "2025-01-01T11:00:00Z");
        insert_conversion_meta(&conn, "cm2", "r1", None, "2025-01-02T11:00:00Z");

        insert_extracted(&conn, "r1", "extracted");

        let rows = list_root_assets(&conn, "p1").expect("list");
        assert_eq!(rows.len(), 1);
        let (_, join) = &rows[0];
        assert_eq!(join.pipeline_status.as_deref(), Some("completed"));
        assert!(join.pipeline_error.is_none());
        assert!(
            join.latest_error_class.is_none(),
            "最近一条 conversion_meta 是成功，error_class 应为 None；实际 {:?}",
            join.latest_error_class
        );
        assert_eq!(join.latest_fallback_used, Some(false));
        assert_eq!(join.extraction_status.as_deref(), Some("extracted"));
    }

    #[test]
    fn list_root_assets_mixed_states_three_roots() {
        let conn = setup_conn();
        insert_project(&conn, "p1");

        // done
        insert(&conn, &mk_asset("done_root", "p1", "pdf", "2025-03-01T00:00:00Z")).unwrap();
        let mut deriv = mk_asset("done_md", "p1", "markdown", "2025-03-01T00:01:00Z");
        deriv.source_asset_id = Some("done_root".to_string());
        deriv.file_path = "/tmp/done_md.md".to_string();
        insert(&conn, &deriv).unwrap();
        insert_pipeline_task(&conn, "pt_done", "done_root", "completed", "2025-03-01T00:02:00Z", None);

        // converting
        insert(&conn, &mk_asset("conv_root", "p1", "pdf", "2025-02-01T00:00:00Z")).unwrap();
        insert_pipeline_task(&conn, "pt_run", "conv_root", "running", "2025-02-01T00:01:00Z", None);

        // failed
        insert(&conn, &mk_asset("fail_root", "p1", "pdf", "2025-01-01T00:00:00Z")).unwrap();
        insert_pipeline_task(&conn, "pt_fail", "fail_root", "failed", "2025-01-01T00:01:00Z", Some("boom"));
        insert_conversion_meta(&conn, "cm_fail", "fail_root", Some("converter_error"), "2025-01-01T00:02:00Z");

        let rows = list_root_assets(&conn, "p1").expect("list");
        assert_eq!(rows.len(), 3);
        // 排序：done(3月) → conv(2月) → fail(1月)
        let ids: Vec<&str> = rows.iter().map(|(a, _)| a.id.as_str()).collect();
        assert_eq!(ids, vec!["done_root", "conv_root", "fail_root"]);

        // done_root 字段
        let (_, j_done) = &rows[0];
        assert_eq!(j_done.rendition_id.as_deref(), Some("done_md"));
        assert_eq!(j_done.pipeline_status.as_deref(), Some("completed"));

        // conv_root
        let (_, j_conv) = &rows[1];
        assert!(j_conv.rendition_id.is_none(), "converting 态尚无 rendition");
        assert_eq!(j_conv.pipeline_status.as_deref(), Some("running"));

        // fail_root
        let (_, j_fail) = &rows[2];
        assert_eq!(j_fail.pipeline_status.as_deref(), Some("failed"));
        assert_eq!(j_fail.pipeline_error.as_deref(), Some("boom"));
        assert_eq!(j_fail.latest_error_class.as_deref(), Some("converter_error"));
    }

    #[test]
    fn list_root_assets_isolates_by_project_id() {
        let conn = setup_conn();
        insert_project(&conn, "p1");
        // 另起一个项目
        conn.execute(
            "INSERT INTO projects (id, library_id, name) VALUES (?1, ?2, ?3)",
            params!["p2", "lib_test", "proj2"],
        )
        .expect("insert project p2");

        insert(&conn, &mk_asset("a_in_p1", "p1", "pdf", "2025-01-01T00:00:00Z")).unwrap();
        insert(&conn, &mk_asset("a_in_p2", "p2", "pdf", "2025-01-01T00:00:00Z")).unwrap();

        let rows_p1 = list_root_assets(&conn, "p1").expect("list");
        assert_eq!(rows_p1.len(), 1);
        assert_eq!(rows_p1[0].0.id, "a_in_p1");
    }

    // ---- resolve_asset_pair（AC-1） ----

    #[test]
    fn resolve_asset_pair_returns_root_and_derivative_when_input_is_root() {
        let conn = setup_conn();
        insert_project(&conn, "p1");

        let root = mk_asset("root1", "p1", "pdf", "2025-01-01T00:00:00Z");
        insert(&conn, &root).unwrap();

        let mut deriv = mk_asset("d1", "p1", "markdown", "2025-01-02T00:00:00Z");
        deriv.source_asset_id = Some("root1".to_string());
        insert(&conn, &deriv).unwrap();

        let (got_root, got_deriv) = resolve_asset_pair(&conn, "root1").expect("解算失败");
        assert_eq!(got_root.id, "root1");
        assert_eq!(got_deriv.as_ref().map(|a| a.id.as_str()), Some("d1"));
    }

    #[test]
    fn resolve_asset_pair_resolves_via_derivative_id() {
        let conn = setup_conn();
        insert_project(&conn, "p1");

        let root = mk_asset("root1", "p1", "pdf", "2025-01-01T00:00:00Z");
        insert(&conn, &root).unwrap();

        let mut deriv = mk_asset("d1", "p1", "markdown", "2025-01-02T00:00:00Z");
        deriv.source_asset_id = Some("root1".to_string());
        insert(&conn, &deriv).unwrap();

        // 传入 derivative.id —— 应反查回 root
        let (got_root, got_deriv) = resolve_asset_pair(&conn, "d1").expect("解算失败");
        assert_eq!(got_root.id, "root1", "应反解到 root");
        assert_eq!(
            got_deriv.as_ref().map(|a| a.id.as_str()),
            Some("d1"),
            "derivative 应包含传入 id"
        );
    }

    #[test]
    fn resolve_asset_pair_root_without_derivative_returns_none() {
        let conn = setup_conn();
        insert_project(&conn, "p1");

        let root = mk_asset("root_only", "p1", "pdf", "2025-01-01T00:00:00Z");
        insert(&conn, &root).unwrap();

        let (got_root, got_deriv) = resolve_asset_pair(&conn, "root_only").expect("解算失败");
        assert_eq!(got_root.id, "root_only");
        assert!(got_deriv.is_none());
    }

    #[test]
    fn resolve_asset_pair_returns_err_when_asset_missing() {
        let conn = setup_conn();
        insert_project(&conn, "p1");
        let err = resolve_asset_pair(&conn, "nope").expect_err("应失败");
        assert_eq!(err, "素材不存在");
    }

    // ---- delete_with_cascade（AC-3 / AC-6 / AC-7） ----

    /// 把 conversion_meta 与 extracted_content 的"是否还存在"封装出来，便于断言。
    fn count_rows(conn: &Connection, sql: &str, params: &[&dyn rusqlite::ToSql]) -> i64 {
        conn.query_row(sql, params, |row| row.get::<_, i64>(0)).unwrap()
    }

    fn insert_asset_tag(conn: &Connection, asset_id: &str) -> String {
        let tag_id = format!("tag_{asset_id}");
        conn.execute(
            "INSERT INTO tags (id, name, color, source, usage_count)
             VALUES (?1, ?2, '#aaa', 'user', 0)",
            params![tag_id, format!("name_{asset_id}")],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO asset_tags (asset_id, tag_id) VALUES (?1, ?2)",
            params![asset_id, tag_id],
        )
        .unwrap();
        tag_id
    }

    /// AC-6：root + derivative + 2 conversion_meta + 1 pipeline_tasks +
    /// 1 extracted_content + 2 临时文件 → delete_with_cascade(root.id) →
    /// 全部行为 0、两文件均不存在。
    #[test]
    fn delete_with_cascade_no_orphans() {
        use std::fs;
        let conn = setup_conn();
        insert_project(&conn, "p1");

        // 构造 2 个真实临时文件（确保 fs::remove_file 真有东西可删）
        let tmp = tempfile::tempdir().expect("tempdir");
        let root_path = tmp.path().join("root.pdf");
        let deriv_path = tmp.path().join("root.md");
        fs::write(&root_path, b"PDF").unwrap();
        fs::write(&deriv_path, b"# md").unwrap();

        // root asset
        let mut root = mk_asset("root1", "p1", "pdf", "2025-01-01T00:00:00Z");
        root.file_path = root_path.to_string_lossy().to_string();
        insert(&conn, &root).unwrap();

        // markdown derivative
        let mut deriv = mk_asset("d1", "p1", "markdown", "2025-01-02T00:00:00Z");
        deriv.source_asset_id = Some("root1".to_string());
        deriv.file_path = deriv_path.to_string_lossy().to_string();
        insert(&conn, &deriv).unwrap();

        // 2 条 conversion_meta（source_asset_id=root1，最近一条已成功）
        insert_conversion_meta(&conn, "cm1", "root1", Some("timeout"), "2025-01-01T11:00Z");
        insert_conversion_meta(&conn, "cm2", "root1", None, "2025-01-02T11:00Z");

        // 1 条 pipeline_tasks
        insert_pipeline_task(&conn, "pt1", "root1", "completed", "2025-01-02T12:00Z", None);

        // 1 条 extracted_content
        insert_extracted(&conn, "root1", "extracted");

        // 顺便挂个 tag，验证 V1 asset_tags ON DELETE CASCADE 也走通
        let tag_id = insert_asset_tag(&conn, "root1");

        // pre-condition
        assert!(root_path.exists());
        assert!(deriv_path.exists());
        assert_eq!(count_rows(&conn, "SELECT COUNT(*) FROM assets WHERE id IN ('root1','d1')", &[]), 2);

        let report = delete_with_cascade(&conn, "root1").expect("级联删除应成功");

        // ── report 字段
        assert_eq!(report.root_asset_id, "root1");
        assert_eq!(report.derivative_asset_id.as_deref(), Some("d1"));
        assert!(report.derivative_existed);
        assert!(report.removed_root_file, "root 物理文件应被删除");
        assert!(report.removed_derivative_file, "derivative 物理文件应被删除");
        assert_eq!(report.removed_pipeline_tasks, 1);

        // ── 文件物理消失
        assert!(!root_path.exists(), "root 文件应已不存在");
        assert!(!deriv_path.exists(), "derivative 文件应已不存在");

        // ── DB 行全部为 0
        assert_eq!(
            count_rows(&conn, "SELECT COUNT(*) FROM assets WHERE id IN ('root1','d1')", &[]),
            0
        );
        assert_eq!(
            count_rows(&conn, "SELECT COUNT(*) FROM pipeline_tasks WHERE asset_id IN ('root1','d1')", &[]),
            0
        );
        assert_eq!(
            count_rows(&conn, "SELECT COUNT(*) FROM conversion_meta WHERE source_asset_id = 'root1'", &[]),
            0,
            "conversion_meta 应被 FK CASCADE 清空"
        );
        assert_eq!(
            count_rows(&conn, "SELECT COUNT(*) FROM extracted_content WHERE asset_id IN ('root1','d1')", &[]),
            0,
            "extracted_content 应被 FK CASCADE 清空"
        );
        assert_eq!(
            count_rows(
                &conn,
                "SELECT COUNT(*) FROM asset_tags WHERE asset_id = ?1",
                &[&tag_id]
            ),
            0,
            "asset_tags 应被 FK CASCADE 清空"
        );
    }

    /// AC-7：传入 derivative.id 也应级联到 root（resolve_asset_pair 反解）。
    #[test]
    fn delete_with_cascade_resolves_via_derivative_id() {
        let conn = setup_conn();
        insert_project(&conn, "p1");

        let root = mk_asset("root2", "p1", "pdf", "2025-01-01T00:00:00Z");
        insert(&conn, &root).unwrap();
        let mut deriv = mk_asset("d2", "p1", "markdown", "2025-01-02T00:00:00Z");
        deriv.source_asset_id = Some("root2".to_string());
        insert(&conn, &deriv).unwrap();
        insert_pipeline_task(&conn, "pt2", "root2", "completed", "2025-01-02T12:00Z", None);

        // 传入 derivative.id，应能级联清掉 root + derivative
        let report = delete_with_cascade(&conn, "d2").expect("via derivative.id 失败");
        assert_eq!(report.root_asset_id, "root2");
        assert_eq!(report.derivative_asset_id.as_deref(), Some("d2"));

        assert_eq!(
            count_rows(&conn, "SELECT COUNT(*) FROM assets WHERE id IN ('root2','d2')", &[]),
            0
        );
        assert_eq!(
            count_rows(&conn, "SELECT COUNT(*) FROM pipeline_tasks WHERE asset_id = 'root2'", &[]),
            0
        );
    }

    /// 物理文件本就不存在时 `delete_with_cascade` 仍应成功并把
    /// `removed_root_file` 标 false（AC-3 文件不存在视为成功）。
    #[test]
    fn delete_with_cascade_missing_file_is_ok() {
        let conn = setup_conn();
        insert_project(&conn, "p1");

        let mut root = mk_asset("root_ghost", "p1", "pdf", "2025-01-01T00:00:00Z");
        root.file_path = "/tmp/does_not_exist_42424242.pdf".to_string();
        insert(&conn, &root).unwrap();

        let report = delete_with_cascade(&conn, "root_ghost").expect("应成功");
        assert!(!report.removed_root_file, "文件不存在应记 false");
        assert!(!report.derivative_existed);
        assert!(!report.removed_derivative_file);
        assert_eq!(
            count_rows(&conn, "SELECT COUNT(*) FROM assets WHERE id = 'root_ghost'", &[]),
            0
        );
    }

    /// 不存在的 asset_id → Err("素材不存在") 透传自 resolve_asset_pair。
    #[test]
    fn delete_with_cascade_returns_err_when_asset_missing() {
        let conn = setup_conn();
        insert_project(&conn, "p1");
        let err = delete_with_cascade(&conn, "nope").expect_err("应失败");
        assert_eq!(err, "素材不存在");
    }

    #[test]
    fn set_derivative_version_advances_value() {
        let conn = setup_conn();
        insert_project(&conn, "p1");
        let a = mk_asset("a1", "p1", "pdf", "2025-01-01T00:00:00Z");
        insert(&conn, &a).unwrap();

        let before = get_by_id(&conn, "a1").unwrap().unwrap();
        assert_eq!(before.derivative_version, 0);

        set_derivative_version(&conn, "a1", 7).expect("set 失败");

        let after = get_by_id(&conn, "a1").unwrap().unwrap();
        assert_eq!(after.derivative_version, 7);
        // 不影响其他列
        assert_eq!(after.name, before.name);
        assert_eq!(after.file_path, before.file_path);
    }
}

/// 工作区列表唯一查询入口（task_001_architect ADR-002）。
///
/// 单 SQL 一次性返回当前项目内**所有 root asset**（`source_asset_id IS NULL`）
/// 配上派生关联：
/// - LEFT JOIN canonical markdown 衍生件 → `rendition_id` / `rendition_path` /
///   `rendition_size`；
/// - LEFT JOIN `extracted_content`（V8 已建 `UNIQUE(asset_id)`，每个 root 至多一行）→
///   `extraction_status`；
/// - LEFT JOIN 最近一条 `pipeline_tasks`（ROW_NUMBER OVER PARTITION BY asset_id
///   ORDER BY created_at DESC, rowid DESC）→ `pipeline_status` / `pipeline_error`；
/// - LEFT JOIN 最近一条 `conversion_meta`（同窗口策略）→ `latest_error_class` /
///   `latest_fallback_used`。
/// 结果按 `assets.imported_at DESC` 排序。
///
/// **不做任何 IO** —— `Path::exists()` 等 stat 留给命令层，避免在 db/ 内引入
/// 文件系统依赖（硬约束 / ADR-003）。
pub fn list_root_assets(
    conn: &Connection,
    project_id: &str,
) -> Result<Vec<(Asset, AssetListJoinRow)>, String> {
    // 注：rusqlite 默认 bundled SQLite ≥ 3.40，window function 自 3.25 起可用。
    // 子查询用 ROW_NUMBER 取每 asset_id 最新一条，避免相关子查询的 N+1。
    let sql = format!(
        "SELECT {root_cols},
                md.id            AS rendition_id,
                md.file_path     AS rendition_path,
                md.file_size     AS rendition_size,
                pt.status        AS pipeline_status,
                pt.error_message AS pipeline_error,
                cm.error_class   AS latest_error_class,
                cm.fallback_used AS latest_fallback_used,
                ec.status        AS extraction_status,
                ec.extractor_type AS ec_extractor_type,
                cm.failure_code  AS latest_failure_code,
                md.name          AS rendition_name,
                md.mime_type     AS rendition_mime,
                md.asset_type    AS rendition_asset_type
         FROM assets root
         LEFT JOIN assets md
                ON md.source_asset_id = root.id AND md.asset_type = 'markdown'
         LEFT JOIN extracted_content ec
                ON ec.asset_id = root.id
         LEFT JOIN (
            SELECT asset_id, status, error_message,
                   ROW_NUMBER() OVER (
                       PARTITION BY asset_id
                       ORDER BY created_at DESC, rowid DESC
                   ) AS rn
            FROM pipeline_tasks
         ) pt ON pt.asset_id = root.id AND pt.rn = 1
         LEFT JOIN (
            SELECT source_asset_id, error_class, fallback_used, failure_code,
                   ROW_NUMBER() OVER (
                       PARTITION BY source_asset_id
                       ORDER BY converted_at DESC, rowid DESC
                   ) AS rn
            FROM conversion_meta
         ) cm ON cm.source_asset_id = root.id AND cm.rn = 1
         WHERE root.project_id = ?1 AND root.source_asset_id IS NULL
         ORDER BY root.imported_at DESC",
        root_cols = ROOT_ASSET_COLS,
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("准备 list_root_assets 语句失败: {e}"))?;

    let rows = stmt
        .query_map(params![project_id], |row| {
            let asset = row_to_asset(row)?;
            // 索引从 ASSET_SELECT 的 15 列之后开始
            let rendition_id: Option<String> = row.get(15)?;
            let rendition_path: Option<String> = row.get(16)?;
            let rendition_size: Option<i64> = row.get(17)?;
            let pipeline_status: Option<String> = row.get(18)?;
            let pipeline_error: Option<String> = row.get(19)?;
            let latest_error_class: Option<String> = row.get(20)?;
            // fallback_used 在 conversion_meta 中是 INTEGER → 用 Option<i64> 拿
            let fallback_int: Option<i64> = row.get(21)?;
            let extraction_status: Option<String> = row.get(22)?;
            // task_014 Fix-A4：ec.extractor_type；空字符串与 NULL 一并视为 None
            let raw_et: Option<String> = row.get(23)?;
            let extractor_type = raw_et.filter(|s| !s.is_empty());
            // task_014 AC-4：最近一行 conversion_meta.failure_code
            let latest_failure_code: Option<String> = row.get(24)?;
            // hotfix-H3：derivative 展示字段（25/26/27），有 .md 衍生时覆盖 root 展示
            let rendition_name: Option<String> = row.get(25)?;
            let rendition_mime: Option<String> = row.get(26)?;
            let rendition_asset_type: Option<String> = row.get(27)?;
            let join = AssetListJoinRow {
                rendition_id,
                rendition_path,
                rendition_size,
                pipeline_status,
                pipeline_error,
                latest_error_class,
                latest_fallback_used: fallback_int.map(|v| v != 0),
                extraction_status,
                extractor_type,
                latest_failure_code,
                rendition_name,
                rendition_mime,
                rendition_asset_type,
            };
            Ok((asset, join))
        })
        .map_err(|e| format!("执行 list_root_assets 失败: {e}"))?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| format!("解析 list_root_assets 行失败: {e}"))?);
    }
    Ok(out)
}

/// root.* 的列序与 `row_to_asset` 完全对齐（与 `ASSET_SELECT` 同形，但带 `root.` 前缀）。
const ROOT_ASSET_COLS: &str = "root.id, root.project_id, root.asset_type, root.name, root.original_name, \
     root.file_path, root.file_size, root.mime_type, root.captured_at, root.imported_at, \
     root.source_type, root.source_data, root.is_starred, root.source_asset_id, root.derivative_version";

/// 四态派生纯函数（ADR-003）。
///
/// 输入完全是值类型，便于单测穷举；调用方需先做以下解算：
/// - `pipeline_status`：最近一条 `pipeline_tasks.status`（None = 该 asset 从未入队）；
/// - `latest_error_class`：最近一条 `conversion_meta.error_class`（None = 成功 / 未尝试）；
/// - `rendition_exists`：磁盘上 canonical markdown 文件是否存在（命令层 `Path::exists()`）；
/// - `source_exists`：磁盘上 source 文件是否存在（命令层 `Path::exists()`）；
/// - `source_missing_known`：task_007 内存态 `SourceMissingSet` 是否已记录该资产为缺失
///   （仅影响 UI 提示，不改变四态本身 —— 硬性要求）。
///
/// 派生规则：
/// 1. `rendition_exists && pipeline_status == Some("completed")` → `Done`
/// 2. `pipeline_status` ∈ {`queued`, `running`} → `Converting`
/// 3. `pipeline_status == Some("failed")` 或 `latest_error_class.is_some()` → `Failed`
/// 4. 其余 → `Offline`
///
/// **source-missing 不改变 state**：即使 source 缺失，若 rendition 仍在且
/// pipeline completed，依旧是 `Done`（用户仍可正常 outbound）；这是 ADR-006
/// 与 PRD §S4 的明确选择，避免 source 短暂离线把已经成功的资产降级为 failed。
pub fn compute_asset_state(
    pipeline_status: Option<&str>,
    latest_error_class: Option<&str>,
    rendition_exists: bool,
    _source_exists: bool,
    _source_missing_known: bool,
) -> AssetState {
    // 规则 1：成功完结
    if rendition_exists && pipeline_status == Some("completed") {
        return AssetState::Done;
    }
    // 规则 2：进行中
    if matches!(pipeline_status, Some("queued") | Some("running")) {
        return AssetState::Converting;
    }
    // 规则 3：失败（pipeline 显式失败 或 最近一次转换记录有 error_class）
    if pipeline_status == Some("failed") || latest_error_class.is_some() {
        return AssetState::Failed;
    }
    // 规则 4：兜底（含 pipeline_status == None / Some("cancelled") 等）
    AssetState::Offline
}

fn row_to_asset(row: &rusqlite::Row) -> rusqlite::Result<Asset> {
    let starred: i32 = row.get(12)?;
    Ok(Asset {
        id: row.get(0)?,
        project_id: row.get(1)?,
        asset_type: row.get(2)?,
        name: row.get(3)?,
        original_name: row.get(4)?,
        file_path: row.get(5)?,
        file_size: row.get(6)?,
        mime_type: row.get(7)?,
        captured_at: row.get(8)?,
        imported_at: row.get(9)?,
        source_type: row.get(10)?,
        source_data: row.get(11)?,
        is_starred: starred != 0,
        source_asset_id: row.get(13)?,
        derivative_version: row.get(14)?,
    })
}
