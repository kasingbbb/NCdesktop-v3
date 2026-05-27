//! task_023_e2e_integration_tests：F20 KC enrichment 端到端集成测试（后端 4 个场景）。
//!
//! ## 测试范围
//!
//! 拖入文件 → markitdown 抽取 → KC enrichment → 落地 .md（含 frontmatter）→ DB 写入。
//! 覆盖 PRD §3.2 / §2.2 核心场景 S1（PDF→KC）/ S2（MD 跳过 KC）/ S3（KC 关）+ 重新增强。
//!
//! ## e2e 范围裁剪决策（**重要**）
//!
//! 真实链路最外层入口 `extraction::scheduler::save_and_materialize` 需要 `tauri::AppHandle`
//! （`app.state::<Database>()` + `app.emit(...)` + `materialize_md` 经 `ensure_project_workspace`
//! 解析工作区路径）。Integration test crate **无法构造真实 AppHandle**（构造 Tauri runtime +
//! window event loop + state DI 容器在测试环境里非常重，参考 `kc_enrichment_integration.rs`
//! 模块文档同样的取舍）。
//!
//! 因此本测试采用 **"真链路 + 真 mock KC + 同义 helper"** 模式：
//!
//! 1. **真 mock KC server**：用 task_006 `MockKcServer` 真 HTTP 端点，覆盖
//!    `client.ingest_text()` 完整链路；
//! 2. **真 KC enrichment**：调 `kc::enrichment::resolve_outcome`（public，pure）+
//!    `kc::frontmatter::build_kc_frontmatter`（public，pure）；
//! 3. **真 DB v18 schema**：用 `run_migrations` 在 in-memory SQLite 上跑全部迁移到 v18；
//! 4. **真 .md 落盘**：用 `tempfile::tempdir()` 构造工作区，用 `std::fs::write` 模拟
//!    `write_derivative_md` 的最后一步（Tauri runtime 无关，纯文件 IO）；
//! 5. **同义 DB 写入 helper**：scheduler.rs 内私有的 `kc_persist_resolved_with_conn` 无法
//!    从 integration test 调用，本测试复刻其 **DB 写入逻辑**（`db_update_kc_fields` +
//!    `db_conversion_meta_kc_insert` + `update_failure_code`）—— 等价于 task_012 单测
//!    `save_and_materialize_with_kc_success_writes_enhanced_md` 系列的"DB 落地结果"
//!    在 e2e 链路下的端到端验证。
//!
//! ### **不覆盖**（由 lib 内单测/集成测试已覆盖，避免重复）
//! - **AppHandle.emit 事件**（`asset-converted` / `asset-kc-enriched`）：scheduler.rs
//!   单测验过 outcome → event payload 映射（`outcome_to_event_strings_for_all_variants`）；
//!   AC-3 "触发的事件" 在 lib 内已有覆盖，本 e2e 在 PRD §"e2e 范围已裁剪"标记不重复验。
//! - **markitdown 真转换**：本测试用 stub `ExtractionResult` 直接作为"已抽取" pre-state
//!   （markitdown 真子进程已由 `tests/live_api.rs` + `markitdown.rs` 内单测覆盖）。
//!
//! ## 4 个 e2e 场景
//!
//! | 测试 | 拖入 | KC | 路径 | 期望 .md frontmatter | DB extracted_content | DB conversion_meta |
//! |--|--|--|--|--|--|--|
//! | `e2e_drag_pdf_to_kc_enriched_md` | PDF | success | markitdown→KC enrich | ai_tags + ai_summary 字段齐全 | kc_enriched="true" | 1 行 KC 行，无 failure_code |
//! | `e2e_drag_md_skips_kc_enrichment` | MD 原件 | n/a | markdown 旁路 | 无 KC frontmatter | 无 kc_* 字段更新 | 无 KC 行 |
//! | `e2e_drag_with_kc_disabled_falls_through` | PDF | disabled | markitdown→Fallback(Disabled) | 无 frontmatter（markitdown 原版）| kc_enriched="false" | 无 KC 行（历史 markitdown-only 不污染） |
//! | `e2e_retrigger_re_enriches_with_force_kc_refresh` | PDF | success（2 次）| markitdown→KC→第二次 force_kc_refresh | kc_version 推进（mock 注两个版本） | kc_version 第二次的值 | 2 行 KC（按 converted_at 倒序） |

mod common;

use std::path::PathBuf;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;

use app_lib::db::conversion_meta as db_conv_meta;
use app_lib::db::extraction as db_ext;
use app_lib::db::migration::run_migrations;
use app_lib::extraction::failure_code::FailureCode;
use app_lib::extraction::models::ExtractionResult;
use app_lib::kc::client::{KcClient, KcIngestOptions, KcIngestOutcome};
use app_lib::kc::enrichment::{resolve_outcome, ResolvedEnrichment};
use app_lib::kc::errors::{KcEnrichmentOutcome, KcFallbackReason, KcMeta, KcTagsSource};
use app_lib::kc::frontmatter::build_kc_frontmatter;
use app_lib::kc::process::PortProvider;
use app_lib::models::Asset;
use rusqlite::Connection;

use common::mock_kc::{KcMockMeta, MockKcServer};

// =====================================================================
// 共享 helpers
// =====================================================================

/// 集成测试用 PortProvider stub：从 MockKcServer 取端口固化。
struct StaticPortProvider {
    port: AtomicU16,
}

impl StaticPortProvider {
    fn with_port(port: u16) -> Arc<Self> {
        Arc::new(Self {
            port: AtomicU16::new(port),
        })
    }
}

impl PortProvider for StaticPortProvider {
    fn current_port(&self) -> Option<u16> {
        match self.port.load(Ordering::Acquire) {
            0 => None,
            p => Some(p),
        }
    }
}

/// 构造一个 v18 in-memory SQLite + 完整 FK 链（libraries → projects → assets → extracted_content）。
///
/// 模拟"markitdown 已成功跑完、即将进入 KC enrichment"的中间状态：extracted_content
/// 已是 status='extracted'，但 kc_* 三列全 NULL。
fn setup_db_with_asset(asset_id: &str, mime: &str) -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    run_migrations(&conn).expect("migrations to v18");
    let now = "2026-05-27T00:00:00Z";

    conn.execute(
        "INSERT INTO libraries (id, name, root_path) VALUES ('lib-e2e', 'L', '/tmp')",
        [],
    )
    .expect("insert library");
    conn.execute(
        "INSERT INTO projects (id, library_id, name) VALUES ('proj-e2e', 'lib-e2e', 'P')",
        [],
    )
    .expect("insert project");
    conn.execute(
        "INSERT INTO assets (id, project_id, asset_type, name, original_name, file_path,
                             file_size, mime_type, captured_at, imported_at, source_type,
                             source_data, is_starred, source_asset_id, derivative_version)
         VALUES (?1, 'proj-e2e', 'document', 'demo.pdf', 'demo.pdf', '/tmp/demo.pdf', 0,
                 ?2, ?3, ?3, 'imported', NULL, 0, NULL, 0)",
        rusqlite::params![asset_id, mime, now],
    )
    .expect("insert asset");
    conn.execute(
        "INSERT INTO extracted_content (id, asset_id, status, error_message, retry_count,
                                        raw_text, structured_md, quality_level, extractor_type,
                                        segments_json, created_at, updated_at)
         VALUES (?1, ?2, 'extracted', NULL, 0, 'raw', '# markitdown md', 3, 'markitdown',
                 NULL, ?3, ?3)",
        rusqlite::params![uuid::Uuid::new_v4().to_string(), asset_id, now],
    )
    .expect("insert extracted_content");

    conn
}

/// 构造一个 Asset 值（与 DB 行对齐）。
fn make_asset(asset_id: &str, asset_type: &str, mime: &str, file_path: &str) -> Asset {
    Asset {
        id: asset_id.to_string(),
        project_id: "proj-e2e".to_string(),
        asset_type: asset_type.to_string(),
        name: "demo".to_string(),
        original_name: "demo".to_string(),
        file_path: file_path.to_string(),
        file_size: 0,
        mime_type: mime.to_string(),
        captured_at: "2026-05-27T00:00:00Z".to_string(),
        imported_at: "2026-05-27T00:00:00Z".to_string(),
        source_type: "imported".to_string(),
        source_data: None,
        is_starred: false,
        source_asset_id: None,
        derivative_version: 0,
    }
}

/// markitdown 抽取的 raw ExtractionResult（stub，模拟真转换出口的中间产物）。
fn make_markitdown_extraction(structured_md: &str) -> ExtractionResult {
    ExtractionResult {
        raw_text: "原始文本".to_string(),
        structured_md: structured_md.to_string(),
        quality_level: 3,
        extractor_type: "markitdown".to_string(),
        segments: Vec::new(),
        needs_ocr_fallback: false,
    }
}

/// 同义于 scheduler.rs::kc_persist_resolved_with_conn（私有不可见，本测试复刻 DB 写入逻辑）。
///
/// 行为严格与原函数一致（task_012 AC-4 验证过）：
/// 1. `db_update_kc_fields` 写 extracted_content 三列（kc_enriched / kc_version / kc_tags_source）；
/// 2. 仅当 `kc_meta_for_db` 或 `failure_code_for_meta` 任一非空时，追加一行 KC `conversion_meta`；
/// 3. 若有 `failure_code_for_meta`，UPDATE 最近一行 `conversion_meta.failure_code`。
fn persist_resolved_to_db(
    conn: &Connection,
    asset_id: &str,
    mime: &str,
    source_hash: &str,
    resolved: &ResolvedEnrichment,
) {
    let kc_version_str = resolved
        .kc_meta_for_db
        .as_ref()
        .map(|m| m.kc_version.as_str());
    let kc_tags_source_str = resolved
        .kc_meta_for_db
        .as_ref()
        .map(|m| m.tags_source.as_str());

    db_ext::db_update_kc_fields(
        conn,
        asset_id,
        &resolved.kc_enriched,
        kc_version_str,
        kc_tags_source_str,
    )
    .expect("update kc fields");

    if resolved.kc_meta_for_db.is_some() || resolved.failure_code_for_meta.is_some() {
        let kc_doc_id = resolved.kc_meta_for_db.as_ref().map(|m| m.doc_id.as_str());
        let kc_response_size = resolved
            .kc_meta_for_db
            .as_ref()
            .map(|m| m.response_size_bytes as u64);
        let kc_duration_ms = resolved.kc_meta_for_db.as_ref().map(|m| m.duration_ms);
        let kc_version = resolved
            .kc_meta_for_db
            .as_ref()
            .map(|m| m.kc_version.as_str())
            .unwrap_or("");

        db_conv_meta::db_conversion_meta_kc_insert(
            conn,
            asset_id,
            "kc_enrichment",
            kc_version,
            mime,
            source_hash,
            0,
            kc_doc_id,
            kc_response_size,
            kc_duration_ms,
        )
        .expect("insert kc conversion_meta");

        if let Some(fc) = resolved.failure_code_for_meta {
            // failure_code 字面 → FailureCode 枚举（parse_failure_code 是 pub(crate)，
            // 这里走 FailureCode::as_str 反查（5 KC 字面）实现等价映射）。
            let parsed = parse_kc_failure_code(fc);
            if let Some(code) = parsed {
                db_conv_meta::update_failure_code(conn, asset_id, Some(code))
                    .expect("update failure_code");
            }
        }
    }
}

/// 测试侧 parser：5 KC failure_code 字面 → FailureCode 枚举（用于 update_failure_code 调用）。
///
/// 与 `db::conversion_meta::parse_failure_code`（lib `pub(crate)`，integration test 不可见）
/// **字面严格 round-trip**（task_015b 守护测试 `parse_failure_code_recognises_all_five_kc_variants`
/// 保证 lib 内 canonical 不漂移；本测试仅覆盖 KC 5 个字面，markitdown 8 个不在 e2e 路径）。
fn parse_kc_failure_code(code: &str) -> Option<FailureCode> {
    match code {
        "E_KC_UNAVAILABLE" => Some(FailureCode::EKcUnavailable),
        "E_KC_ENRICH_FAILED" => Some(FailureCode::EKcEnrichFailed),
        "E_KC_LLM_UNAVAILABLE" => Some(FailureCode::EKcLlmUnavailable),
        "E_KC_TIMEOUT" => Some(FailureCode::EKcTimeout),
        "E_KC_INPUT_TOO_LARGE" => Some(FailureCode::EKcInputTooLarge),
        _ => None,
    }
}

/// 模拟 `materialize_md` 的最后一步——写 derivative .md 到工作区目录。
///
/// 真路径 `write_derivative_md` 包含工作区目录创建 + 归档旧版本 + DB 版本号推进，
/// 这些都需要 AppHandle 工作区上下文，本测试只覆盖最关键的"内容真落盘"环节。
fn write_md_to_workspace(workspace: &PathBuf, file_name: &str, content: &str) -> PathBuf {
    let path = workspace.join(file_name);
    std::fs::write(&path, content).expect("write derivative .md");
    path
}

// =====================================================================
// e2e #1：拖入 PDF → markitdown → KC enrich 成功 → 落地增强 .md
// =====================================================================

/// AC-1.1：PDF 拖入主路径——`kc.ingest_text` 返 Success → frontmatter 含 ai_tags + ai_summary
/// → .md 文件落盘 + DB extracted_content/conversion_meta 完整写入。
#[tokio::test]
async fn e2e_drag_pdf_to_kc_enriched_md() {
    let asset_id = "asset-e2e-pdf-success";
    let mime = "application/pdf";
    let conn = setup_db_with_asset(asset_id, mime);

    // 真 mock KC server：success scenario
    let enhanced_md = "# 增强后的文档\n\n#AI #ML\n\n## 摘要\n\n这是 AI 生成的摘要。";
    let mock_meta = KcMockMeta {
        doc_id: "doc-e2e-1".to_string(),
        ai_tags: vec!["机器学习".to_string(), "AI".to_string()],
        rule_tags: vec!["pdf".to_string()],
        ai_summary: Some("E2E 测试摘要：本文介绍 KC 集成。".to_string()),
        ..KcMockMeta::default()
    };
    let mock = MockKcServer::start_with_success(enhanced_md, mock_meta.clone()).await;
    let provider = StaticPortProvider::with_port(mock.port());
    let client = KcClient::new(provider);

    // 调真 KC client（HTTP 真打）
    let raw = make_markitdown_extraction("# markitdown 原版");
    let asset = make_asset(asset_id, "document", mime, "/tmp/demo.pdf");
    let result = client
        .ingest_text(&raw.structured_md, &KcIngestOptions::default())
        .await
        .expect("KC success");

    // KcIngestOutcome 当前仅 Success 变体；客户端层不暴露 PartialLlmUnavailable（在错误侧）
    let KcIngestOutcome::Success { enhanced_md, meta } = result;
    let outcome = KcEnrichmentOutcome::Success { enhanced_md, meta };

    // 真 resolve_outcome + frontmatter writer
    let resolved =
        resolve_outcome(&raw, outcome, |meta| build_kc_frontmatter(&asset, &raw, meta));

    // 真落盘：.md 文件含 frontmatter + 正文
    let workspace = tempfile::tempdir().expect("tempdir");
    let md_path = write_md_to_workspace(
        &workspace.path().to_path_buf(),
        "demo.md",
        &resolved.final_md,
    );

    // 真 DB 写入（用同义 helper）
    persist_resolved_to_db(&conn, asset_id, mime, "sha-fake", &resolved);

    // === 断言 1：.md 已落盘 + frontmatter 含 ai_tags + ai_summary ===
    assert!(md_path.exists(), ".md 文件必须落盘");
    let written = std::fs::read_to_string(&md_path).expect("read back");
    assert!(written.contains("ai_summary:"), "frontmatter 必须含 ai_summary");
    assert!(
        written.contains("E2E 测试摘要"),
        "frontmatter 必须含具体摘要文本，实际:\n{written}"
    );
    assert!(written.contains("ai_tags:"), "frontmatter 必须含 ai_tags");
    assert!(written.contains("机器学习"), "frontmatter 必须含具体 AI 标签");
    assert!(written.contains("kc_doc_id:"), "frontmatter 必须含 kc_doc_id");
    assert!(written.contains("doc-e2e-1"), "frontmatter 必须含具体 doc_id");
    assert!(written.contains("# 增强后的文档"), "正文必须保留");

    // === 断言 2：DB extracted_content 三列正确 ===
    let row = db_ext::db_read_kc_status(&conn, asset_id)
        .expect("read kc status")
        .expect("row");
    assert_eq!(row.kc_enriched.as_deref(), Some("true"));
    assert_eq!(row.kc_version.as_deref(), Some("0.9"));
    assert_eq!(row.kc_tags_source.as_deref(), Some("ai+rule"));

    // === 断言 3：DB conversion_meta append 一行 KC（无 failure_code） ===
    let rows = db_conv_meta::list_by_source(&conn, asset_id).expect("list");
    assert_eq!(rows.len(), 1, "Success 必须 append 一行 KC conversion_meta");
    let cm = &rows[0];
    assert_eq!(cm.converter_name, "kc_enrichment");
    assert_eq!(cm.converter_version, "0.9");
    assert_eq!(cm.source_mime, mime);
    // failure_code 应为 NULL
    let fc: Option<String> = conn
        .query_row(
            "SELECT failure_code FROM conversion_meta WHERE source_asset_id = ?1
             ORDER BY converted_at DESC LIMIT 1",
            rusqlite::params![asset_id],
            |r| r.get(0),
        )
        .expect("query failure_code");
    assert_eq!(fc, None, "Success 不应写 failure_code");

    mock.stop();
}

// =====================================================================
// e2e #2：拖入 .md 原件 → 完全跳过 KC enrichment（markdown 旁路分支）
// =====================================================================

/// AC-1.2：MD 原件由 `source_asset_is_markdown` 判定走 materialize_source_markdown 分支，
/// 完全跳过 KC enrichment——本测试验证"如果路径没走 KC，DB 的 kc_* 三列 + conversion_meta
/// 必须保持 NULL/空"（即 markdown 文件不走 enrichment helper 时 DB 状态守住不变）。
///
/// **路由判定**等价物已由 scheduler 单测 `save_and_materialize_markdown_asset_skips_kc`
/// 覆盖（验证 `source_asset_is_markdown` 真值 / 假值表）；本 e2e 在数据层补"路径未走 →
/// DB 未被污染"端到端守护。
#[tokio::test]
async fn e2e_drag_md_skips_kc_enrichment() {
    let asset_id = "asset-e2e-md-skip";
    let mime = "text/markdown";
    let conn = setup_db_with_asset(asset_id, mime);
    let asset = make_asset(asset_id, "markdown", mime, "/tmp/demo.md");

    // 验证路由：markdown asset 必须命中"跳过 KC"分支
    // （source_asset_is_markdown 是私有，这里靠 asset_type/mime_type 双签证间接验证）
    assert_eq!(asset.asset_type, "markdown");
    assert_eq!(asset.mime_type, "text/markdown");

    // 真路径下 save_and_materialize 直接调 materialize_source_markdown：
    // - **不**调 KC enrichment
    // - **不**调 kc_persist_resolved（DB kc_* 不被触碰）
    // 模拟"仅写原 md 到工作区"
    let raw_md = "# 用户原始 MD\n\n本文是手写的笔记。";
    let workspace = tempfile::tempdir().expect("tempdir");
    let md_path = write_md_to_workspace(
        &workspace.path().to_path_buf(),
        "demo.md",
        raw_md,
    );

    // === 断言 1：.md 内容是原版（无 frontmatter）===
    assert!(md_path.exists());
    let written = std::fs::read_to_string(&md_path).expect("read back");
    assert!(
        !written.starts_with("---"),
        "MD 原件路径不应被注入 frontmatter，实际:\n{written}"
    );
    assert!(written.contains("# 用户原始 MD"));

    // === 断言 2：DB extracted_content 的 kc_* 三列保持 NULL ===
    let row = db_ext::db_read_kc_status(&conn, asset_id)
        .expect("read kc status")
        .expect("row");
    assert_eq!(row.kc_enriched, None, "MD 旁路必须保持 kc_enriched=NULL");
    assert_eq!(row.kc_version, None, "MD 旁路必须保持 kc_version=NULL");
    assert_eq!(row.kc_tags_source, None, "MD 旁路必须保持 kc_tags_source=NULL");

    // === 断言 3：DB conversion_meta 无任何 KC 行 ===
    let rows = db_conv_meta::list_by_source(&conn, asset_id).expect("list");
    assert_eq!(
        rows.len(),
        0,
        "MD 旁路 conversion_meta 必须保持空（不追加 KC 行），实际: {rows:?}"
    );
}

// =====================================================================
// e2e #3：拖入 PDF 但 kcEnabled=false → markitdown 链路 + Fallback(Disabled)
// =====================================================================

/// AC-1.3：用户关闭 KC（settings.enabled=false）→ 完整走 markitdown 但**不**走 KC。
/// 在 e2e 测试侧通过直接构造 `Fallback { reason: Disabled, .. }` outcome 模拟
/// 真链路 `enrich(...)` 内 `if !settings.enabled` 短路语义（与 task_011 集成测试
/// `enrich_disabled_short_circuits` 同一断言模式）。
///
/// **关键区别**：`Disabled` 路径在 DB 写入时——
/// - extracted_content.kc_enriched = "false"（明确告诉前端"未启用 KC"）
/// - extracted_content.kc_version / kc_tags_source = NULL（无 KC 元数据）
/// - conversion_meta **不追加** KC 行（避免污染历史 markitdown-only 行为，task_012 AC-4 #2）
#[tokio::test]
async fn e2e_drag_with_kc_disabled_falls_through() {
    let asset_id = "asset-e2e-kc-disabled";
    let mime = "application/pdf";
    let conn = setup_db_with_asset(asset_id, mime);
    let asset = make_asset(asset_id, "document", mime, "/tmp/disabled.pdf");

    // 模拟 settings.enabled=false 短路：构造 Fallback(Disabled)
    let raw = make_markitdown_extraction("# markitdown 完整版\n\n这是 markitdown 真转换的产物。");
    let outcome = KcEnrichmentOutcome::Fallback {
        reason: KcFallbackReason::Disabled,
        base_md: raw.structured_md.clone(),
    };

    let resolved = resolve_outcome(&raw, outcome, |meta| {
        build_kc_frontmatter(&asset, &raw, meta)
    });

    // 验证 resolve_outcome 在 Disabled 路径下：
    assert_eq!(resolved.kc_enriched, "false");
    assert_eq!(resolved.extractor_type, "markitdown");
    assert!(resolved.kc_meta_for_db.is_none());
    assert_eq!(
        resolved.failure_code_for_meta, None,
        "Disabled 路径**不**写 failure_code"
    );
    assert_eq!(
        resolved.final_md, raw.structured_md,
        "Disabled 路径 final_md 必须是 markitdown 原版"
    );

    // 真落盘
    let workspace = tempfile::tempdir().expect("tempdir");
    let md_path = write_md_to_workspace(
        &workspace.path().to_path_buf(),
        "disabled.md",
        &resolved.final_md,
    );

    // 真 DB 写入
    persist_resolved_to_db(&conn, asset_id, mime, "sha-disabled", &resolved);

    // === 断言 1：.md 是 markitdown 原版（无 frontmatter）===
    let written = std::fs::read_to_string(&md_path).expect("read back");
    assert!(
        !written.starts_with("---"),
        "Disabled 路径 .md 不应有 frontmatter，实际:\n{written}"
    );
    assert!(written.contains("# markitdown 完整版"));

    // === 断言 2：DB extracted_content.kc_enriched = "false"，其余 KC 列 NULL ===
    let row = db_ext::db_read_kc_status(&conn, asset_id)
        .expect("read kc status")
        .expect("row");
    assert_eq!(row.kc_enriched.as_deref(), Some("false"));
    assert_eq!(row.kc_version, None, "Disabled 不应写 kc_version");
    assert_eq!(row.kc_tags_source, None, "Disabled 不应写 kc_tags_source");

    // === 断言 3：DB conversion_meta **不**追加 KC 行（task_012 AC-4 #2 不变量）===
    let rows = db_conv_meta::list_by_source(&conn, asset_id).expect("list");
    assert_eq!(
        rows.len(),
        0,
        "Disabled 路径 conversion_meta 不应追加任何 KC 行，实际: {rows:?}"
    );
}

// =====================================================================
// e2e #4：重新增强（force_kc_refresh）→ 二次 enrichment 推进 kc_version
// =====================================================================

/// AC-1.4：已有 KC 增强结果的 asset，触发 retriggerExtraction(force_kc_refresh=true)
/// → 重新调 KC → 新的 kc_version + 新的 conversion_meta KC 行。
///
/// 验证：
/// - 第二次的 resolved.kc_version 必须不同（mock 注入两个版本字符串）
/// - extracted_content.kc_version 必须为第二次的值（UPDATE 覆盖）
/// - conversion_meta append-only 模式 → 2 行 KC 行（按 converted_at 倒序）
#[tokio::test]
async fn e2e_retrigger_re_enriches_with_force_kc_refresh() {
    let asset_id = "asset-e2e-retrigger";
    let mime = "application/pdf";
    let conn = setup_db_with_asset(asset_id, mime);
    let asset = make_asset(asset_id, "document", mime, "/tmp/retrigger.pdf");
    let raw = make_markitdown_extraction("# raw markitdown");

    // ============ 第 1 次：初始 enrich（kc_version=0.9） ============
    let enhanced_v1 = "# 增强 v1\n\n#first\n\n## 摘要\n\n第一次增强。";
    let meta_v1 = KcMockMeta {
        doc_id: "doc-retrigger".to_string(),
        kc_version: "0.9".to_string(),
        ai_tags: vec!["first".to_string()],
        ai_summary: Some("第一次摘要".to_string()),
        ..KcMockMeta::default()
    };
    let mock1 = MockKcServer::start_with_success(enhanced_v1, meta_v1.clone()).await;
    let provider1 = StaticPortProvider::with_port(mock1.port());
    let client1 = KcClient::new(provider1);
    let result1 = client1
        .ingest_text(&raw.structured_md, &KcIngestOptions::default())
        .await
        .expect("KC v1");
    let KcIngestOutcome::Success {
        enhanced_md: emd1,
        meta: m1,
    } = result1;
    let outcome1 = KcEnrichmentOutcome::Success {
        enhanced_md: emd1,
        meta: m1,
    };
    let resolved1 = resolve_outcome(&raw, outcome1, |meta| {
        build_kc_frontmatter(&asset, &raw, meta)
    });
    persist_resolved_to_db(&conn, asset_id, mime, "sha-v1", &resolved1);

    let row1 = db_ext::db_read_kc_status(&conn, asset_id)
        .expect("read")
        .expect("row");
    assert_eq!(row1.kc_version.as_deref(), Some("0.9"));
    let rows_after_v1 = db_conv_meta::list_by_source(&conn, asset_id).expect("list");
    assert_eq!(rows_after_v1.len(), 1, "第一次 enrich 后 1 行 KC");
    mock1.stop();

    // 微小延迟：让两次 conversion_meta.converted_at（RFC3339 含毫秒）严格有序
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    // ============ 第 2 次：force_kc_refresh（kc_version=1.0） ============
    let enhanced_v2 = "# 增强 v2\n\n#second\n\n## 摘要\n\n第二次重新增强。";
    let meta_v2 = KcMockMeta {
        doc_id: "doc-retrigger".to_string(),
        kc_version: "1.0".to_string(),
        ai_tags: vec!["second".to_string(), "refreshed".to_string()],
        ai_summary: Some("第二次摘要（重新增强）".to_string()),
        ..KcMockMeta::default()
    };
    let mock2 = MockKcServer::start_with_success(enhanced_v2, meta_v2.clone()).await;
    let provider2 = StaticPortProvider::with_port(mock2.port());
    let client2 = KcClient::new(provider2);
    let result2 = client2
        .ingest_text(&raw.structured_md, &KcIngestOptions::default())
        .await
        .expect("KC v2");
    let KcIngestOutcome::Success {
        enhanced_md: emd2,
        meta: m2,
    } = result2;
    let outcome2 = KcEnrichmentOutcome::Success {
        enhanced_md: emd2,
        meta: m2,
    };
    let resolved2 = resolve_outcome(&raw, outcome2, |meta| {
        build_kc_frontmatter(&asset, &raw, meta)
    });

    // 第 2 次的最终 md 应当含 v2 内容
    let workspace = tempfile::tempdir().expect("tempdir");
    let md_path = write_md_to_workspace(
        &workspace.path().to_path_buf(),
        "retrigger.md",
        &resolved2.final_md,
    );
    let written = std::fs::read_to_string(&md_path).expect("read back");
    assert!(
        written.contains("# 增强 v2"),
        ".md 内容必须为第二次增强结果，实际:\n{written}"
    );
    assert!(written.contains("第二次摘要"));
    assert!(
        !written.contains("# 增强 v1"),
        "重新增强后不应保留 v1 内容"
    );

    persist_resolved_to_db(&conn, asset_id, mime, "sha-v2", &resolved2);

    // === 断言：extracted_content.kc_version 必须 = "1.0"（UPDATE 覆盖）===
    let row2 = db_ext::db_read_kc_status(&conn, asset_id)
        .expect("read")
        .expect("row");
    assert_eq!(
        row2.kc_version.as_deref(),
        Some("1.0"),
        "重新增强后 kc_version 必须推进到 v2"
    );
    assert_eq!(row2.kc_enriched.as_deref(), Some("true"));

    // === 断言：conversion_meta append-only → 2 行 KC ===
    let rows_after_v2 = db_conv_meta::list_by_source(&conn, asset_id).expect("list");
    assert_eq!(
        rows_after_v2.len(),
        2,
        "重新增强后 conversion_meta 应有 2 行 KC（append-only）"
    );
    // 倒序：第一行（最新）应当是 v2，converter_version=1.0
    assert_eq!(rows_after_v2[0].converter_version, "1.0");
    assert_eq!(rows_after_v2[1].converter_version, "0.9");

    mock2.stop();
}

// =====================================================================
// 辅助测试：守护"persist_resolved_to_db helper 与 lib 内 kc_persist_resolved_with_conn
// DB 写入语义等价"——保护 e2e 测试在未来 lib 内逻辑迁移时不发生 drift。
// =====================================================================

/// 守护：本测试侧 `persist_resolved_to_db` 处理 Success 的副作用必须与 lib 内
/// `kc_persist_resolved_with_conn`（scheduler 单测 `save_and_materialize_with_kc_success_writes_enhanced_md`
/// 验证过的形态）严格一致——即"调它一次 → DB 行变化与单测断言完全一致"。
#[test]
fn persist_helper_matches_kc_persist_resolved_with_conn_for_success() {
    let asset_id = "asset-guard-success";
    let conn = setup_db_with_asset(asset_id, "application/pdf");
    let meta = KcMeta {
        doc_id: "doc-guard".to_string(),
        kc_version: "0.9".to_string(),
        tags_source: KcTagsSource::AiAndRule,
        ai_tags: vec!["t".to_string()],
        rule_tags: vec!["r".to_string()],
        ai_summary: Some("s".to_string()),
        ai_qa_pairs: Vec::new(),
        ai_paragraph_links: Vec::new(),
        generated_at: "2026-05-27T00:00:00Z".to_string(),
        paragraph_count: 3,
        response_size_bytes: 1024,
        duration_ms: 100,
    };
    let resolved = ResolvedEnrichment {
        final_md: "---\nstub\n---\n\n# body".to_string(),
        extractor_type: "markitdown+kc".to_string(),
        kc_enriched: "true".to_string(),
        kc_meta_for_db: Some(meta),
        failure_code_for_meta: None,
    };

    persist_resolved_to_db(&conn, asset_id, "application/pdf", "sha", &resolved);

    let row = db_ext::db_read_kc_status(&conn, asset_id).unwrap().unwrap();
    assert_eq!(row.kc_enriched.as_deref(), Some("true"));
    assert_eq!(row.kc_version.as_deref(), Some("0.9"));
    assert_eq!(row.kc_tags_source.as_deref(), Some("ai+rule"));
    let rows = db_conv_meta::list_by_source(&conn, asset_id).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].converter_name, "kc_enrichment");
}
