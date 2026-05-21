//! `conversion_meta` 表的 CRUD —— ADR-004 append-only 转换日志。
//!
//! 每次转换尝试（成功 / fallback / 失败 / placeholder）都追加一行，
//! 用于失败率统计、诊断与三态展示。**不**对
//! `(source_asset_id, converter_name)` 加唯一约束（见迁移 V6）。
//!
//! - `id` 由调用方生成 UUID（与 `db::asset::insert` 风格一致）
//! - 时间戳 RFC3339（`chrono::Utc::now().to_rfc3339()`）
//! - SQL 全部参数化（`params![]`）

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::extraction::failure_code::FailureCode;

/// 转换元数据行（与 `extraction::conversion::ConversionAttempt` 字段一一对应，
/// 外加 `id` / `source_asset_id` / `derived_asset_id`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionMetaRow {
    pub id: String,
    pub source_asset_id: String,
    pub derived_asset_id: Option<String>,
    pub converter_name: String,
    pub converter_version: String,
    pub source_mime: String,
    pub source_hash: String,
    pub quality_level: i32,
    pub fallback_used: bool,
    pub error_class: Option<String>,
    pub conversion_ms: Option<i64>,
    pub converted_at: String,
}

/// 插入一条 conversion_meta 记录。`id` 由调用方生成。
pub fn insert(conn: &Connection, row: &ConversionMetaRow) -> Result<(), String> {
    conn.execute(
        "INSERT INTO conversion_meta (
            id, source_asset_id, derived_asset_id, converter_name, converter_version,
            source_mime, source_hash, quality_level, fallback_used,
            error_class, conversion_ms, converted_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            row.id,
            row.source_asset_id,
            row.derived_asset_id,
            row.converter_name,
            row.converter_version,
            row.source_mime,
            row.source_hash,
            row.quality_level,
            row.fallback_used as i32,
            row.error_class,
            row.conversion_ms,
            row.converted_at,
        ],
    )
    .map_err(|e| format!("插入 conversion_meta 失败: {e}"))?;
    Ok(())
}

/// 列出某 source_asset_id 的所有转换记录，按 `converted_at` 倒序。
pub fn list_by_source(
    conn: &Connection,
    source_asset_id: &str,
) -> Result<Vec<ConversionMetaRow>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, source_asset_id, derived_asset_id, converter_name, converter_version,
                    source_mime, source_hash, quality_level, fallback_used,
                    error_class, conversion_ms, converted_at
             FROM conversion_meta
             WHERE source_asset_id = ?1
             ORDER BY converted_at DESC",
        )
        .map_err(|e| format!("准备 list_by_source 语句失败: {e}"))?;

    let rows = stmt
        .query_map(params![source_asset_id], row_to_meta)
        .map_err(|e| format!("查询 conversion_meta 失败: {e}"))?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| format!("解析 conversion_meta 行失败: {e}"))?);
    }
    Ok(out)
}

/// 取某 source_asset_id 的最新一条转换记录；无记录返回 `Ok(None)`。
pub fn latest_for_source(
    conn: &Connection,
    source_asset_id: &str,
) -> Result<Option<ConversionMetaRow>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, source_asset_id, derived_asset_id, converter_name, converter_version,
                    source_mime, source_hash, quality_level, fallback_used,
                    error_class, conversion_ms, converted_at
             FROM conversion_meta
             WHERE source_asset_id = ?1
             ORDER BY converted_at DESC
             LIMIT 1",
        )
        .map_err(|e| format!("准备 latest_for_source 语句失败: {e}"))?;

    let mut rows = stmt
        .query_map(params![source_asset_id], row_to_meta)
        .map_err(|e| format!("查询 latest conversion_meta 失败: {e}"))?;

    match rows.next() {
        Some(r) => Ok(Some(
            r.map_err(|e| format!("解析 conversion_meta 行失败: {e}"))?,
        )),
        None => Ok(None),
    }
}

/// task_008 AC-3：更新 `conversion_meta.failure_code`。
///
/// **更新策略**：按 `source_asset_id` 找该 asset **最近一行** conversion_meta，
/// 仅更新其 `failure_code` 字段（成功时显式写 NULL）。
/// 设计依据：
/// - `conversion_meta` 为 append-only，本方法不新增行（落库由 scheduler 写入完整行时一并完成）；
/// - 在 markitdown.rs 这种内层 extract 失败 / 成功路径 不持有 conversion_meta.id，
///   按 asset_id 取最近一行是稳定锚点；
/// - 若该 asset 还没有 conversion_meta 行（极少见，例如尚未走 scheduler 即直接调 extract），
///   则 0 行受影响，调用方需容忍 Ok（不视为错误）。
///
/// 调用方语义：
/// - `success` 路径必须显式调 `update_failure_code(asset_id, None)`，
///   将历史可能的 failure_code 清为 NULL；
/// - `failure` 路径调 `update_failure_code(asset_id, Some(code))`。
pub fn update_failure_code(
    conn: &Connection,
    source_asset_id: &str,
    code: Option<FailureCode>,
) -> Result<(), String> {
    // SQLite NULL 用 rusqlite 的 None 即可：`Option<&str>` 序列化为 NULL。
    let code_str: Option<&str> = code.map(|c| c.as_str());

    conn.execute(
        "UPDATE conversion_meta
         SET failure_code = ?1
         WHERE id = (
           SELECT id FROM conversion_meta
           WHERE source_asset_id = ?2
           ORDER BY converted_at DESC
           LIMIT 1
         )",
        params![code_str, source_asset_id],
    )
    .map_err(|e| format!("更新 conversion_meta.failure_code 失败: {e}"))?;
    Ok(())
}

// ===== task_014 AC-3：三态查询 =====

/// 转换三态（task_014 AC-3）。
///
/// 由 `conversion_meta.failure_code` + `extracted_content` 联合判定：
/// - `Success(content)`：有抽取结果，且 raw_text 或 structured_md 至少一项非空；
/// - `LegacyUnverified`：`conversion_meta.failure_code = 'legacy_unverified'`
///   （由 V14 backfill 标注的"成功但内容空"老数据，或并发竞态保护下后端补判）；
/// - `Failed(FailureCode)`：`conversion_meta.failure_code` 是 8 错误码字面之一。
///
/// 设计约束：
/// - 仅作用于"最新一行 conversion_meta"，与 V14 / task_008 `update_failure_code`
///   的"按 asset 最新一行"语义对齐；
/// - 不修改 `failure_code.rs`，本地用 `parse_failure_code` 做字符串 → 枚举映射；
/// - 不喂下游知识进化系统：消费侧 filter 由 follow-up task 落地（AC-6）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversionState {
    /// 抽取内容非空（来自 extracted_content.raw_text 优先 / 否则 structured_md）。
    Success(String),
    /// 旧版本未验证：升级前转换"显示成功但内容空"的老记录。
    LegacyUnverified,
    /// 明确失败：携带 8 错误码之一。
    Failed(FailureCode),
}

/// 将 `conversion_meta.failure_code` 字符串字面映射为 [`FailureCode`] 枚举。
///
/// `legacy_unverified` **不**在 8 枚举中（PRD R-④"老用户感知不退步"），
/// 因此本函数对它返回 `None`，由调用方按 `LegacyUnverified` 单独分支处理。
/// 未知字符串（理论上不应出现）同样返回 `None`，由调用方决定容错策略。
fn parse_failure_code(code: &str) -> Option<FailureCode> {
    match code {
        "E_RUNTIME_MISSING" => Some(FailureCode::ERuntimeMissing),
        "E_EXTRA_MISSING_EPUB" => Some(FailureCode::EExtraMissingEpub),
        "E_SCAN_PDF_UNSUPPORTED" => Some(FailureCode::EScanPdfUnsupported),
        "E_AUDIO_WRONG_ROUTE" => Some(FailureCode::EAudioWrongRoute),
        "E_OUTPUT_EMPTY" => Some(FailureCode::EOutputEmpty),
        "E_OUTPUT_GIBBERISH" => Some(FailureCode::EOutputGibberish),
        "E_OUTPUT_NO_STRUCTURE" => Some(FailureCode::EOutputNoStructure),
        "E_TIMEOUT_90S" => Some(FailureCode::ETimeout90s),
        _ => None,
    }
}

/// 取某 asset 的转换三态。无 conversion_meta 行返回 `Ok(None)`。
///
/// 判定顺序：
/// 1. 取最新一行 conversion_meta。
/// 2. `failure_code`：
///    - `'legacy_unverified'` → [`ConversionState::LegacyUnverified`]
///    - 8 错误码之一 → [`ConversionState::Failed`]
///    - NULL → 看 extracted_content：raw_text 优先 / structured_md 兜底；
///      任一非空 → [`ConversionState::Success`]；都空 → 保守判
///      [`ConversionState::LegacyUnverified`]（与 V14 backfill 语义对齐，
///      防御并发竞态下尚未写 failure_code 的中间状态）。
pub fn get_conversion_state(
    conn: &Connection,
    asset_id: &str,
) -> Result<Option<ConversionState>, String> {
    // 1) 取最新一行 conversion_meta.failure_code
    let latest: Option<Option<String>> = conn
        .query_row(
            "SELECT failure_code FROM conversion_meta
             WHERE source_asset_id = ?1
             ORDER BY converted_at DESC LIMIT 1",
            params![asset_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|e| format!("查询最新 conversion_meta 失败: {e}"))?;

    let failure_code = match latest {
        None => return Ok(None), // 没有任何转换记录
        Some(fc) => fc,
    };

    // 2) 按 failure_code 分派
    if let Some(code_str) = failure_code.as_deref() {
        if code_str == "legacy_unverified" {
            return Ok(Some(ConversionState::LegacyUnverified));
        }
        if let Some(fc) = parse_failure_code(code_str) {
            return Ok(Some(ConversionState::Failed(fc)));
        }
        // 未知字面：保守归为 LegacyUnverified（前端会提示"重新转录"）。
        return Ok(Some(ConversionState::LegacyUnverified));
    }

    // 3) failure_code 为 NULL：看 extracted_content
    let ec: Option<(Option<String>, Option<String>)> = conn
        .query_row(
            "SELECT raw_text, structured_md FROM extracted_content WHERE asset_id = ?1",
            params![asset_id],
            |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .optional()
        .map_err(|e| format!("查询 extracted_content 失败: {e}"))?;

    match ec {
        Some((raw_opt, md_opt)) => {
            let raw_nonempty = raw_opt
                .as_deref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);
            let md_nonempty = md_opt
                .as_deref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);
            if raw_nonempty {
                Ok(Some(ConversionState::Success(raw_opt.unwrap())))
            } else if md_nonempty {
                Ok(Some(ConversionState::Success(md_opt.unwrap())))
            } else {
                // 保守：与 V14 backfill 语义对齐，处理并发竞态下尚未回填的 NULL 行
                Ok(Some(ConversionState::LegacyUnverified))
            }
        }
        // 有 conversion_meta（failure_code=NULL）但无 extracted_content：
        // 这种情况通常意味着 scheduler 刚写完 conversion_meta，extract 还没落库。
        // 视为 LegacyUnverified（前端提示"重新转录"是安全的兜底）。
        None => Ok(Some(ConversionState::LegacyUnverified)),
    }
}

fn row_to_meta(row: &rusqlite::Row<'_>) -> rusqlite::Result<ConversionMetaRow> {
    let fallback_int: i64 = row.get(8)?;
    Ok(ConversionMetaRow {
        id: row.get(0)?,
        source_asset_id: row.get(1)?,
        derived_asset_id: row.get(2)?,
        converter_name: row.get(3)?,
        converter_version: row.get(4)?,
        source_mime: row.get(5)?,
        source_hash: row.get(6)?,
        quality_level: row.get(7)?,
        fallback_used: fallback_int != 0,
        error_class: row.get(9)?,
        conversion_ms: row.get(10)?,
        converted_at: row.get(11)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migration::run_migrations;

    /// 建一个跑完整迁移（含 V6）的内存库 + 一个用于外键 source 的 asset 行。
    fn setup_db_with_asset(asset_id: &str) -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .expect("enable FK");
        run_migrations(&conn).expect("run migrations");

        // 建库 + 项目 + 素材，用于外键引用
        conn.execute(
            "INSERT INTO libraries (id, name, root_path) VALUES (?1, 'lib', '/tmp/lib')",
            params!["lib1"],
        )
        .expect("insert library");
        conn.execute(
            "INSERT INTO projects (id, library_id, name) VALUES (?1, ?2, 'p')",
            params!["proj1", "lib1"],
        )
        .expect("insert project");
        conn.execute(
            "INSERT INTO assets (id, project_id, asset_type, name, file_path)
             VALUES (?1, ?2, 'document', 'a.pdf', '/tmp/a.pdf')",
            params![asset_id, "proj1"],
        )
        .expect("insert asset");
        conn
    }

    fn sample_row(id: &str, source: &str, converter: &str, when: &str) -> ConversionMetaRow {
        ConversionMetaRow {
            id: id.to_string(),
            source_asset_id: source.to_string(),
            derived_asset_id: None,
            converter_name: converter.to_string(),
            converter_version: "0.0.1".to_string(),
            source_mime: "application/pdf".to_string(),
            source_hash: "abc123".to_string(),
            quality_level: 2,
            fallback_used: false,
            error_class: None,
            conversion_ms: Some(500),
            converted_at: when.to_string(),
        }
    }

    /// AC-5：插入 3 条 → list_by_source 按 converted_at DESC 返回 3 条
    #[test]
    fn list_by_source_returns_rows_desc() {
        let conn = setup_db_with_asset("src1");
        insert(&conn, &sample_row("m1", "src1", "markitdown", "2026-05-12T10:00:00Z"))
            .expect("insert m1");
        insert(&conn, &sample_row("m2", "src1", "pdf-text", "2026-05-12T11:00:00Z"))
            .expect("insert m2");
        insert(&conn, &sample_row("m3", "src1", "placeholder", "2026-05-12T12:00:00Z"))
            .expect("insert m3");

        let rows = list_by_source(&conn, "src1").expect("list");
        assert_eq!(rows.len(), 3);
        // 倒序：最新（12:00 placeholder）在前
        assert_eq!(rows[0].converter_name, "placeholder");
        assert_eq!(rows[1].converter_name, "pdf-text");
        assert_eq!(rows[2].converter_name, "markitdown");
    }

    /// AC-5：latest_for_source 返回最新一行；查不到时返回 Ok(None)
    #[test]
    fn latest_for_source_picks_most_recent_and_handles_missing() {
        let conn = setup_db_with_asset("src1");
        // 查不到时
        let none = latest_for_source(&conn, "src1").expect("latest none");
        assert!(none.is_none());

        insert(&conn, &sample_row("m1", "src1", "markitdown", "2026-05-12T10:00:00Z"))
            .expect("insert m1");
        insert(&conn, &sample_row("m2", "src1", "pdf-text", "2026-05-12T11:00:00Z"))
            .expect("insert m2");

        let latest = latest_for_source(&conn, "src1").expect("latest");
        let row = latest.expect("some");
        assert_eq!(row.converter_name, "pdf-text");
        assert_eq!(row.converted_at, "2026-05-12T11:00:00Z");
    }

    /// AC-5：ON DELETE CASCADE —— 删 source asset 后 conversion_meta 也被清掉
    #[test]
    fn deleting_source_asset_cascades_conversion_meta() {
        let conn = setup_db_with_asset("src1");
        insert(&conn, &sample_row("m1", "src1", "markitdown", "2026-05-12T10:00:00Z"))
            .expect("insert");
        insert(&conn, &sample_row("m2", "src1", "pdf-text", "2026-05-12T11:00:00Z"))
            .expect("insert");

        assert_eq!(list_by_source(&conn, "src1").expect("pre").len(), 2);

        conn.execute("DELETE FROM assets WHERE id = ?1", params!["src1"])
            .expect("delete asset");

        let after = list_by_source(&conn, "src1").expect("post");
        assert!(after.is_empty(), "CASCADE 应清空相关 conversion_meta");
    }

    /// task_008 AC-3：写 success（code=None）必须显式落 NULL；
    /// 写 failure 时 failure_code 字符串与 `FailureCode::as_str()` 一致。
    #[test]
    fn update_failure_code_writes_screaming_snake_and_clears_on_none() {
        let conn = setup_db_with_asset("src1");
        insert(
            &conn,
            &sample_row("m1", "src1", "markitdown", "2026-05-12T10:00:00Z"),
        )
        .expect("insert");

        // 1) 写失败码
        update_failure_code(&conn, "src1", Some(FailureCode::EOutputEmpty))
            .expect("update fail");
        let got: Option<String> = conn
            .query_row(
                "SELECT failure_code FROM conversion_meta WHERE id = 'm1'",
                [],
                |row| row.get(0),
            )
            .expect("read fc");
        assert_eq!(got.as_deref(), Some("E_OUTPUT_EMPTY"));

        // 2) success 路径：显式写 None → DB 落 NULL
        update_failure_code(&conn, "src1", None).expect("update success");
        let cleared: Option<String> = conn
            .query_row(
                "SELECT failure_code FROM conversion_meta WHERE id = 'm1'",
                [],
                |row| row.get(0),
            )
            .expect("read fc cleared");
        assert!(cleared.is_none(), "success 路径必须显式落 NULL");
    }

    /// task_008 AC-3：多条 conversion_meta 行时只更新最近一行（按 converted_at DESC）。
    #[test]
    fn update_failure_code_targets_latest_row_only() {
        let conn = setup_db_with_asset("src1");
        insert(
            &conn,
            &sample_row("m1", "src1", "markitdown", "2026-05-12T10:00:00Z"),
        )
        .expect("insert m1");
        insert(
            &conn,
            &sample_row("m2", "src1", "pdf-text", "2026-05-12T11:00:00Z"),
        )
        .expect("insert m2");

        update_failure_code(&conn, "src1", Some(FailureCode::ETimeout90s))
            .expect("update");

        let m1: Option<String> = conn
            .query_row(
                "SELECT failure_code FROM conversion_meta WHERE id = 'm1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let m2: Option<String> = conn
            .query_row(
                "SELECT failure_code FROM conversion_meta WHERE id = 'm2'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(m1.is_none(), "旧行不应被改动");
        assert_eq!(m2.as_deref(), Some("E_TIMEOUT_90S"));
    }

    /// 边界：asset 还没 conversion_meta 行时调 update 不应报错（0 row affected 视为 Ok）。
    #[test]
    fn update_failure_code_no_row_is_not_error() {
        let conn = setup_db_with_asset("src1");
        // 不插任何 conversion_meta；直接调 update。
        let r = update_failure_code(&conn, "src1", Some(FailureCode::ERuntimeMissing));
        assert!(r.is_ok(), "无行也应 Ok（容忍上层调用顺序）");
    }

    // ===== task_014 AC-3：get_conversion_state 三态 =====

    fn upsert_ec(
        conn: &Connection,
        asset_id: &str,
        raw_text: Option<&str>,
        structured_md: Option<&str>,
    ) {
        let now = "2026-05-01T00:00:00Z";
        conn.execute(
            "INSERT INTO extracted_content
                (id, asset_id, status, error_message, retry_count, raw_text, structured_md,
                 quality_level, extractor_type, segments_json, created_at, updated_at)
             VALUES (?1, ?2, 'extracted', NULL, 0, ?3, ?4, 0, NULL, NULL, ?5, ?5)",
            params![format!("ec_{}", asset_id), asset_id, raw_text, structured_md, now],
        )
        .expect("insert ec");
    }

    #[test]
    fn get_conversion_state_returns_success_when_content_present() {
        let conn = setup_db_with_asset("src1");
        insert(&conn, &sample_row("m1", "src1", "markitdown", "2026-05-12T10:00:00Z"))
            .expect("insert");
        upsert_ec(&conn, "src1", Some("正常正文"), Some("# 正常 MD"));

        let st = get_conversion_state(&conn, "src1").expect("query");
        match st {
            Some(ConversionState::Success(content)) => {
                assert_eq!(content, "正常正文", "应优先返回 raw_text 内容");
            }
            other => panic!("应为 Success(_)，实际 {:?}", other),
        }
    }

    #[test]
    fn get_conversion_state_returns_legacy_unverified() {
        let conn = setup_db_with_asset("src1");
        insert(&conn, &sample_row("m1", "src1", "markitdown", "2026-05-12T10:00:00Z"))
            .expect("insert");
        // 直接把 failure_code 写为 'legacy_unverified'（模拟 V14 回填后的状态）
        conn.execute(
            "UPDATE conversion_meta SET failure_code = 'legacy_unverified' WHERE id = 'm1'",
            [],
        )
        .unwrap();

        let st = get_conversion_state(&conn, "src1").expect("query");
        assert_eq!(st, Some(ConversionState::LegacyUnverified));
    }

    #[test]
    fn get_conversion_state_returns_failed_when_8code_present() {
        let conn = setup_db_with_asset("src1");
        insert(&conn, &sample_row("m1", "src1", "markitdown", "2026-05-12T10:00:00Z"))
            .expect("insert");
        update_failure_code(&conn, "src1", Some(FailureCode::EOutputEmpty)).expect("update");

        let st = get_conversion_state(&conn, "src1").expect("query");
        assert_eq!(st, Some(ConversionState::Failed(FailureCode::EOutputEmpty)));
    }

    #[test]
    fn get_conversion_state_returns_none_when_no_meta() {
        let conn = setup_db_with_asset("src1");
        let st = get_conversion_state(&conn, "src1").expect("query");
        assert!(st.is_none(), "无 conversion_meta 行应返回 None");
    }

    /// 保守边界：failure_code=NULL + extracted_content 内容全空 → LegacyUnverified。
    #[test]
    fn get_conversion_state_null_fc_empty_content_is_legacy_unverified() {
        let conn = setup_db_with_asset("src1");
        insert(&conn, &sample_row("m1", "src1", "markitdown", "2026-05-12T10:00:00Z"))
            .expect("insert");
        upsert_ec(&conn, "src1", Some(""), Some("   "));

        let st = get_conversion_state(&conn, "src1").expect("query");
        assert_eq!(st, Some(ConversionState::LegacyUnverified));
    }

    /// AC-4：derived_asset_id 为 None 时 JSON 序列化为 null，可往返
    #[test]
    fn serde_derived_asset_id_none_is_json_null() {
        let row = ConversionMetaRow {
            id: "x".into(),
            source_asset_id: "src1".into(),
            derived_asset_id: None,
            converter_name: "markitdown".into(),
            converter_version: "0.0.1".into(),
            source_mime: "application/pdf".into(),
            source_hash: "deadbeef".into(),
            quality_level: 0,
            fallback_used: true,
            error_class: Some("timeout".into()),
            conversion_ms: None,
            converted_at: "2026-05-12T10:00:00Z".into(),
        };
        let json = serde_json::to_string(&row).expect("ser");
        assert!(json.contains("\"derivedAssetId\":null"));
        assert!(json.contains("\"conversionMs\":null"));
        assert!(json.contains("\"fallbackUsed\":true"));
        // 确保 camelCase 生效
        assert!(!json.contains("derived_asset_id"));

        let back: ConversionMetaRow = serde_json::from_str(&json).expect("de");
        assert!(back.derived_asset_id.is_none());
        assert!(back.conversion_ms.is_none());
        assert_eq!(back.fallback_used, true);
        assert_eq!(back.error_class.as_deref(), Some("timeout"));
    }
}
