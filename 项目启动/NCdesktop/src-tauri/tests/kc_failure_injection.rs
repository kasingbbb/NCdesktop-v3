//! task_022 集成测试：KC 5 类失败注入端到端验证（PRD §4.3 + ADR-004 §"5 类失败映射"）。
//!
//! ## 测试目标（input.md AC-1 ~ AC-5）
//!
//! 用 `MockKcServer`（task_006）模拟 KC HTTP 端点的 5 类失败语义，验证：
//!
//! 1. **KcClient → KcCallError 分类正确**（直接经 `ingest_text` 触发，断言 err 变体）；
//! 2. **resolve_outcome 落地形态**（final_md / extractor_type / kc_enriched / failure_code_for_meta）
//!    严格符合 ADR-004 §"5 类失败映射"表；
//! 3. **failure_code 字面与 `FailureCode::EKc*.as_str()` 一致**（task_003 单一来源，禁止手写漂移）；
//! 4. **PRD 不可妥协底线 #2**：`extracted_content.status` 在 KC 失败兜底后仍是 `'extracted'`——
//!    KC 失败**不**污染主链路 extraction 状态。
//!
//! ## 为什么不直接调 `enrich(app, asset, raw_md).await`
//!
//! `enrich()` 需要 `tauri::AppHandle`（实际 Tauri runtime + 多个 `app.manage(Arc<...>)` 注入），
//! 在 integration test crate 构造真实 AppHandle 非常重。本测试参考 task_011 集成测试
//! `kc_enrichment_integration.rs` §模块文档的设计原则，直接调
//! `KcClient::ingest_text` 拿 `Result<_, KcCallError>`，再用本地复刻的
//! `call_error_to_outcome` 把 KcCallError 转 `KcEnrichmentOutcome`，最后调公开
//! `resolve_outcome` 验证落地形态。这等价于**完整覆盖了 `enrich()` 步骤 4-5（client 调用 +
//! 错误分类）**与 **`resolve_outcome` 全部三态**，但跳过纯 Tauri runtime 杂活。
//!
//! ## DB 断言粒度（task_022 vs task_011 的独特增量）
//!
//! 每个测试新建 in-memory SQLite + 跑完 v18 迁移 + **手工模拟 scheduler 写入序列**：
//!
//! 1. `upsert_extraction_result(..., extractor_type="markitdown", ...)` — 模拟"markitdown
//!    抽取成功"，写入 `extracted_content.status='extracted'`；
//! 2. 跑 KC enrich → 落地 `ResolvedEnrichment`；
//! 3. 调 `db_update_kc_fields` + `db_conversion_meta_kc_insert` + `update_failure_code`
//!    （模拟 `scheduler::kc_persist_resolved_with_conn`）；
//! 4. **断言 `extracted_content.status` 仍是 `'extracted'`**（KC 失败不污染主链路）；
//! 5. 断言 `conversion_meta.failure_code` 字面对齐 `FailureCode::EKc*.as_str()`。
//!
//! ## 5 类失败覆盖表（input.md AC-1 必含 2 P0 + 建议 3 补全）
//!
//! | 测试 | mock scenario | 期望 KcCallError | 期望 outcome | failure_code（DB） |
//! |--|--|--|--|--|
//! | `failure_a_unavailable_falls_back_to_markitdown_md`         | unavailable     | Unreachable      | Fallback(Unavailable)     | E_KC_UNAVAILABLE       |
//! | `failure_d_timeout_falls_back_to_markitdown_md`             | timeout         | Timeout          | Fallback(Timeout)         | E_KC_TIMEOUT           |
//! | `failure_b_internal_error_falls_back_with_failure_code`     | internal_error  | Internal         | Fallback(InternalError)   | E_KC_ENRICH_FAILED     |
//! | `failure_c_llm_unavailable_partial_writes_rule_only_md`     | llm_unavailable | LlmUnavailable{Some} | PartialLlmUnavailable | E_KC_LLM_UNAVAILABLE   |
//! | `failure_e_input_too_large_falls_back`                      | input_too_large | InputTooLarge    | Fallback(InputTooLarge)   | E_KC_INPUT_TOO_LARGE   |
//!
//! ## 测试隔离（input.md AC-5）
//!
//! - 每个测试用独立 wiremock server（动态空闲端口，无端口冲突）；
//! - 每个测试用独立 in-memory SQLite（`:memory:` 数据库间不共享状态）；
//! - 无外部网络依赖（wiremock 走 127.0.0.1）。

mod common;

use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use std::time::Duration;

use rusqlite::{params, Connection};

use app_lib::db::extraction::{get_extracted_content, upsert_extraction_result};
use app_lib::db::migration::run_migrations;
use app_lib::extraction::failure_code::FailureCode;
use app_lib::extraction::models::ExtractionResult;
use app_lib::extraction::scheduler::kc_persist_resolved_with_conn;
use app_lib::kc::client::{KcClient, KcIngestOptions};
use app_lib::kc::enrichment::resolve_outcome;
use app_lib::kc::errors::{KcCallError, KcEnrichmentOutcome, KcFallbackReason, KcMeta, KcTagsSource};
use app_lib::kc::process::PortProvider;

use common::mock_kc::MockKcServer;

// =====================================================================
// 测试用 helpers
// =====================================================================

/// 集成测试用 PortProvider stub：从 MockKcServer 取端口固化（与 task_011 同款）。
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

/// 构造 markitdown 抽取阶段产出的 `ExtractionResult` stub（structured_md 是 KC 失败兜底的 final_md）。
fn make_raw_extraction() -> ExtractionResult {
    ExtractionResult {
        raw_text: "原始 markitdown 文本".to_string(),
        structured_md: "# markitdown 原版\n\n这是 markitdown 抽取的 MD（KC 失败兜底回这里）。".to_string(),
        quality_level: 3,
        extractor_type: "markitdown".to_string(),
        segments: Vec::new(),
        needs_ocr_fallback: false,
    }
}

/// 测试用 frontmatter writer：把 meta 关键字段嵌进 frontmatter 用于 partial 路径断言。
fn test_frontmatter_writer(meta: &KcMeta) -> String {
    format!(
        "---\ndoc_id: {}\nkc_version: {}\ntags_source: {}\n---",
        meta.doc_id,
        meta.kc_version,
        meta.tags_source.as_str()
    )
}

/// `KcCallError → KcEnrichmentOutcome` 映射（与 task_011 集成测试同款，复刻
/// lib 内私有 `map_call_error_to_outcome` 的字面映射表，供集成测试使用）。
fn call_error_to_outcome(err: KcCallError, raw_md: &str) -> KcEnrichmentOutcome {
    match err {
        KcCallError::Unreachable => KcEnrichmentOutcome::Fallback {
            reason: KcFallbackReason::Unavailable,
            base_md: raw_md.to_string(),
        },
        KcCallError::Timeout => KcEnrichmentOutcome::Fallback {
            reason: KcFallbackReason::Timeout,
            base_md: raw_md.to_string(),
        },
        KcCallError::LlmUnavailable { partial_md: Some(md) } => {
            KcEnrichmentOutcome::PartialLlmUnavailable {
                rule_only_md: md,
                meta: KcMeta {
                    doc_id: "doc-partial".to_string(),
                    kc_version: "unknown".to_string(),
                    tags_source: KcTagsSource::RuleOnly,
                    ai_tags: Vec::new(),
                    rule_tags: Vec::new(),
                    ai_summary: None,
                    ai_qa_pairs: Vec::new(),
                    ai_paragraph_links: Vec::new(),
                    generated_at: String::new(),
                    paragraph_count: 0,
                    response_size_bytes: 0,
                    duration_ms: 0,
                },
            }
        }
        KcCallError::LlmUnavailable { partial_md: None } => KcEnrichmentOutcome::Fallback {
            reason: KcFallbackReason::InternalError("LLM unavailable, no partial".to_string()),
            base_md: raw_md.to_string(),
        },
        KcCallError::Internal { detail, .. } => KcEnrichmentOutcome::Fallback {
            reason: KcFallbackReason::InternalError(detail),
            base_md: raw_md.to_string(),
        },
        KcCallError::InputTooLarge => KcEnrichmentOutcome::Fallback {
            reason: KcFallbackReason::InputTooLarge,
            base_md: raw_md.to_string(),
        },
        KcCallError::Malformed { .. } => KcEnrichmentOutcome::Fallback {
            reason: KcFallbackReason::Malformed,
            base_md: raw_md.to_string(),
        },
    }
}

/// 建一个跑完 v18 迁移的 in-memory SQLite + 完整外键链路 + 一条 `extracted_content` 行
/// （status='extracted', extractor_type='markitdown'），模拟"markitdown 抽取成功" 状态。
///
/// 注：每个测试独立调用本函数，保证 in-memory DB 不共享（input.md AC-5 隔离要求）。
fn setup_db_with_markitdown_extracted(asset_id: &str) -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("enable FK");
    run_migrations(&conn).expect("run migrations (incl. v18)");

    // 建外键链路：library + project + asset（最小可行 fixture）
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

    // 模拟"markitdown 抽取成功"：写 extracted_content.status='extracted'
    upsert_extraction_result(
        &conn,
        asset_id,
        "原始 markitdown 文本",
        "# markitdown 原版",
        3,
        "markitdown",
        None,
    )
    .expect("upsert extracted_content (markitdown 阶段)");

    conn
}

/// 共用断言：每个失败注入测试结束后必须满足的"PRD 不可妥协底线 #2 + failure_code 守护"。
fn assert_db_invariants_post_kc_failure(
    conn: &Connection,
    asset_id: &str,
    expected_kc_enriched: &str,
    expected_failure_code: FailureCode,
) {
    // === PRD 不可妥协底线 #2：extracted_content.status 仍是 'extracted' ===
    let row = get_extracted_content(conn, asset_id)
        .expect("query extracted_content")
        .expect("row exists");
    assert_eq!(
        row.status, "extracted",
        "PRD 不可妥协底线 #2：KC 失败不得污染 extracted_content.status（仍应是 'extracted'）"
    );
    assert_eq!(
        row.extractor_type, "markitdown",
        "markitdown 抽取阶段写入的 extractor_type 不应被 KC 失败兜底覆盖"
    );
    assert_eq!(
        row.kc_enriched.as_deref(),
        Some(expected_kc_enriched),
        "extracted_content.kc_enriched 列应反映 ResolvedEnrichment.kc_enriched"
    );

    // === conversion_meta.failure_code 字面对齐 task_003 ===
    let failure_code_in_db: Option<String> = conn
        .query_row(
            "SELECT failure_code FROM conversion_meta
             WHERE source_asset_id = ?1
             ORDER BY converted_at DESC
             LIMIT 1",
            params![asset_id],
            |r| r.get(0),
        )
        .expect("query conversion_meta.failure_code");
    assert_eq!(
        failure_code_in_db.as_deref(),
        Some(expected_failure_code.as_str()),
        "conversion_meta.failure_code 字面值必须与 FailureCode::as_str() 严格一致（无漂移）"
    );
}

// =====================================================================
// AC-1.A（P0 必含）：E_KC_UNAVAILABLE → markitdown 兜底
// =====================================================================

/// **P0**：KC 不可达 → KcCallError::Unreachable → Fallback(Unavailable)
/// → ResolvedEnrichment {extractor_type: "markitdown", kc_enriched: "false",
/// failure_code_for_meta: Some("E_KC_UNAVAILABLE"), final_md: markitdown 原版}。
/// DB：extracted_content.status 仍是 'extracted'，conversion_meta.failure_code = E_KC_UNAVAILABLE。
#[tokio::test]
async fn failure_a_unavailable_falls_back_to_markitdown_md() {
    let asset_id = "asset-a-unavailable";

    // 1. mock：KC 不可达（监听后立即释放端口）
    let mock = MockKcServer::start_with_unavailable().await;
    let provider = StaticPortProvider::with_port(mock.port());
    let client = KcClient::new(provider);

    // 2. client 触发 → Unreachable
    let raw_md = "# 测试 unavailable";
    let result = client.ingest_text(raw_md, &KcIngestOptions::default()).await;
    let err = result.expect_err("KC 不可达应当抛错");
    assert!(
        matches!(err, KcCallError::Unreachable),
        "应当映射为 Unreachable，实际: {err:?}"
    );

    // 3. err → outcome → resolved
    let outcome = call_error_to_outcome(err, raw_md);
    let raw = make_raw_extraction();
    let resolved = resolve_outcome(&raw, outcome, test_frontmatter_writer);

    // 4. resolved 落地形态断言（ADR-004 §"5 类失败映射"）
    assert_eq!(resolved.extractor_type, "markitdown");
    assert_eq!(resolved.kc_enriched, "false");
    assert!(resolved.kc_meta_for_db.is_none(), "Fallback 路径不应有 meta");
    assert_eq!(
        resolved.failure_code_for_meta,
        Some(FailureCode::EKcUnavailable.as_str()),
        "Unreachable 必须映射到 E_KC_UNAVAILABLE（task_003 字面）"
    );
    // final_md 应当是 markitdown 原版（structured_md）
    assert_eq!(
        resolved.final_md, raw.structured_md,
        "Fallback 路径 final_md 必须是 markitdown 原版"
    );

    // 5. DB 写入并断言不可妥协底线 #2
    let conn = setup_db_with_markitdown_extracted(asset_id);
    kc_persist_resolved_with_conn(&conn, asset_id, "application/pdf", "h-a", &resolved);
    assert_db_invariants_post_kc_failure(
        &conn,
        asset_id,
        "false",
        FailureCode::EKcUnavailable,
    );

    mock.stop();
}

// =====================================================================
// AC-1.D（P0 必含）：E_KC_TIMEOUT → markitdown 兜底
// =====================================================================

/// **P0**：KC 超时 → KcCallError::Timeout → Fallback(Timeout)
/// → ResolvedEnrichment {failure_code_for_meta: Some("E_KC_TIMEOUT")}。
/// 用 mock 500ms 延迟 + client 100ms 超时触发（input.md "测试运行 < 10s" 要求）。
/// DB：extracted_content.status 仍是 'extracted'，conversion_meta.failure_code = E_KC_TIMEOUT。
#[tokio::test]
async fn failure_d_timeout_falls_back_to_markitdown_md() {
    let asset_id = "asset-d-timeout";

    // 1. mock 延迟 500ms；client 用短超时 100ms 注入（避开生产 60s 默认值）
    let mock = MockKcServer::start_with_timeout(Duration::from_millis(500)).await;
    let provider = StaticPortProvider::with_port(mock.port());

    let short_timeout_http = reqwest::Client::builder()
        .timeout(Duration::from_millis(100))
        .build()
        .expect("short-timeout http client");
    let client = KcClient::new_with_http_client(provider, short_timeout_http);

    // 2. client 触发 → Timeout
    let raw_md = "# 测试 timeout";
    let result = client.ingest_text(raw_md, &KcIngestOptions::default()).await;
    let err = result.expect_err("100ms client 超时应当抛错");
    assert!(
        matches!(err, KcCallError::Timeout),
        "应当映射为 Timeout，实际: {err:?}"
    );

    // 3. err → outcome → resolved
    let outcome = call_error_to_outcome(err, raw_md);
    let raw = make_raw_extraction();
    let resolved = resolve_outcome(&raw, outcome, test_frontmatter_writer);

    // 4. resolved 落地形态
    assert_eq!(resolved.extractor_type, "markitdown");
    assert_eq!(resolved.kc_enriched, "false");
    assert!(resolved.kc_meta_for_db.is_none());
    assert_eq!(
        resolved.failure_code_for_meta,
        Some(FailureCode::EKcTimeout.as_str()),
        "Timeout 必须映射到 E_KC_TIMEOUT"
    );
    assert_eq!(resolved.final_md, raw.structured_md);

    // 5. DB
    let conn = setup_db_with_markitdown_extracted(asset_id);
    kc_persist_resolved_with_conn(&conn, asset_id, "application/pdf", "h-d", &resolved);
    assert_db_invariants_post_kc_failure(&conn, asset_id, "false", FailureCode::EKcTimeout);

    mock.stop();
}

// =====================================================================
// AC-1.B（建议）：E_KC_ENRICH_FAILED → markitdown 兜底（B 类聚合）
// =====================================================================

/// KC 500 + KC_INTERNAL → KcCallError::Internal → Fallback(InternalError)
/// → ResolvedEnrichment {failure_code_for_meta: Some("E_KC_ENRICH_FAILED")}。
/// B 类聚合：KC_PARSE_ERROR / KC_INTERNAL / KC_OUTPUT_ERROR / malformed 200 全部进 E_KC_ENRICH_FAILED。
/// DB：extracted_content.status 仍是 'extracted'，conversion_meta.failure_code = E_KC_ENRICH_FAILED。
#[tokio::test]
async fn failure_b_internal_error_falls_back_with_failure_code() {
    let asset_id = "asset-b-internal";

    let mock = MockKcServer::start_with_internal_error().await;
    let provider = StaticPortProvider::with_port(mock.port());
    let client = KcClient::new(provider);

    let raw_md = "# 测试 internal_error";
    let result = client.ingest_text(raw_md, &KcIngestOptions::default()).await;
    let err = result.expect_err("KC 500 + KC_INTERNAL 应当抛错");
    match err {
        KcCallError::Internal { ref code, .. } => {
            assert_eq!(code, "KC_INTERNAL", "mock 配置的 error_code 字面");
        }
        other => panic!("expected Internal, got {other:?}"),
    }

    let outcome = call_error_to_outcome(err, raw_md);
    let raw = make_raw_extraction();
    let resolved = resolve_outcome(&raw, outcome, test_frontmatter_writer);

    assert_eq!(resolved.extractor_type, "markitdown");
    assert_eq!(resolved.kc_enriched, "false");
    assert!(resolved.kc_meta_for_db.is_none());
    assert_eq!(
        resolved.failure_code_for_meta,
        Some(FailureCode::EKcEnrichFailed.as_str()),
        "Internal 必须映射到 E_KC_ENRICH_FAILED（B 类聚合）"
    );
    assert_eq!(resolved.final_md, raw.structured_md);

    let conn = setup_db_with_markitdown_extracted(asset_id);
    kc_persist_resolved_with_conn(&conn, asset_id, "application/pdf", "h-b", &resolved);
    assert_db_invariants_post_kc_failure(
        &conn,
        asset_id,
        "false",
        FailureCode::EKcEnrichFailed,
    );

    mock.stop();
}

// =====================================================================
// AC-1.C（建议）：E_KC_LLM_UNAVAILABLE → 规则增强 MD（C 类 partial）
// =====================================================================

/// KC 500 + KC_LLM_UNAVAILABLE + partial_md → KcCallError::LlmUnavailable{Some}
/// → PartialLlmUnavailable {rule_only_md, meta: RuleOnly}
/// → ResolvedEnrichment {extractor_type: "markitdown+kc:partial", kc_enriched: "partial",
///    failure_code_for_meta: Some("E_KC_LLM_UNAVAILABLE")}。
/// 关键：本路径 kc_meta_for_db 有值（虽然是规则兜底的 RuleOnly meta），
/// final_md 应当含 partial 正文 + frontmatter（tags_source: rule_only）。
/// DB：extracted_content.status 仍是 'extracted'，extracted_content.kc_enriched='partial',
///     extracted_content.kc_tags_source='rule_only'，conversion_meta.failure_code = E_KC_LLM_UNAVAILABLE。
#[tokio::test]
async fn failure_c_llm_unavailable_partial_writes_rule_only_md() {
    let asset_id = "asset-c-llm-partial";
    let partial = "# 规则增强\n\n#tag1 #tag2\n\n（仅规则标签，无 AI 摘要）";

    let mock = MockKcServer::start_with_llm_unavailable(partial).await;
    let provider = StaticPortProvider::with_port(mock.port());
    let client = KcClient::new(provider);

    let raw_md = "# 测试 llm_unavailable";
    let result = client.ingest_text(raw_md, &KcIngestOptions::default()).await;
    let err = result.expect_err("KC 500 + KC_LLM_UNAVAILABLE 应当抛错");
    match err {
        KcCallError::LlmUnavailable { partial_md: Some(ref md) } => {
            assert_eq!(md, partial, "partial_md 应当与 mock 配置一致");
        }
        other => panic!("expected LlmUnavailable {{ Some(_) }}, got {other:?}"),
    }

    let outcome = call_error_to_outcome(err, raw_md);
    let raw = make_raw_extraction();
    let resolved = resolve_outcome(&raw, outcome, test_frontmatter_writer);

    // 关键：partial 路径 → markitdown+kc:partial / "partial" / E_KC_LLM_UNAVAILABLE
    assert_eq!(resolved.extractor_type, "markitdown+kc:partial");
    assert_eq!(resolved.kc_enriched, "partial");
    assert!(
        resolved.kc_meta_for_db.is_some(),
        "partial 路径必须带（合成的）meta"
    );
    assert_eq!(
        resolved.failure_code_for_meta,
        Some(FailureCode::EKcLlmUnavailable.as_str()),
        "partial 路径必须记 E_KC_LLM_UNAVAILABLE"
    );
    // partial 路径 meta 应当是 RuleOnly
    assert_eq!(
        resolved.kc_meta_for_db.as_ref().unwrap().tags_source,
        KcTagsSource::RuleOnly
    );
    // final_md 应当含 partial 正文（不是 markitdown 原版）
    assert!(
        resolved.final_md.contains("# 规则增强"),
        "final_md 应含 partial 正文，实际: {}",
        resolved.final_md
    );
    // frontmatter 应当反映 RuleOnly
    assert!(
        resolved.final_md.contains("tags_source: rule_only"),
        "frontmatter 应含 tags_source: rule_only，实际: {}",
        resolved.final_md
    );

    // DB：注意 kc_enriched='partial' 也是 task_022 的关键断言
    let conn = setup_db_with_markitdown_extracted(asset_id);
    kc_persist_resolved_with_conn(&conn, asset_id, "application/pdf", "h-c", &resolved);
    assert_db_invariants_post_kc_failure(
        &conn,
        asset_id,
        "partial",
        FailureCode::EKcLlmUnavailable,
    );

    // 追加断言：partial 路径还应当写 kc_tags_source='rule_only'（task_015 AC-1）
    let row = get_extracted_content(&conn, asset_id)
        .expect("query")
        .expect("row");
    // task_015 db_update_kc_fields 写的字段，从 row 不暴露 kc_tags_source，需直接 SQL 查询
    let tags_source: Option<String> = conn
        .query_row(
            "SELECT kc_tags_source FROM extracted_content WHERE asset_id = ?1",
            params![asset_id],
            |r| r.get(0),
        )
        .expect("query kc_tags_source");
    assert_eq!(
        tags_source.as_deref(),
        Some("rule_only"),
        "partial 路径 kc_tags_source 必须落 'rule_only'"
    );
    // row 复读时 status 仍 'extracted'（再次守护，避免 partial 路径误改 status）
    assert_eq!(row.status, "extracted");

    mock.stop();
}

// =====================================================================
// AC-1.E（建议）：E_KC_INPUT_TOO_LARGE → markitdown 兜底
// =====================================================================

/// KC 500 + KC_INPUT_TOO_LARGE → KcCallError::InputTooLarge → Fallback(InputTooLarge)
/// → ResolvedEnrichment {failure_code_for_meta: Some("E_KC_INPUT_TOO_LARGE")}。
/// DB：extracted_content.status 仍是 'extracted'，conversion_meta.failure_code = E_KC_INPUT_TOO_LARGE。
#[tokio::test]
async fn failure_e_input_too_large_falls_back() {
    let asset_id = "asset-e-too-large";

    let mock = MockKcServer::start_with_input_too_large().await;
    let provider = StaticPortProvider::with_port(mock.port());
    let client = KcClient::new(provider);

    let raw_md = "# 测试 input_too_large";
    let result = client.ingest_text(raw_md, &KcIngestOptions::default()).await;
    let err = result.expect_err("KC 500 + KC_INPUT_TOO_LARGE 应当抛错");
    assert!(
        matches!(err, KcCallError::InputTooLarge),
        "应当映射为 InputTooLarge，实际: {err:?}"
    );

    let outcome = call_error_to_outcome(err, raw_md);
    let raw = make_raw_extraction();
    let resolved = resolve_outcome(&raw, outcome, test_frontmatter_writer);

    assert_eq!(resolved.extractor_type, "markitdown");
    assert_eq!(resolved.kc_enriched, "false");
    assert!(resolved.kc_meta_for_db.is_none());
    assert_eq!(
        resolved.failure_code_for_meta,
        Some(FailureCode::EKcInputTooLarge.as_str()),
        "InputTooLarge 必须映射到 E_KC_INPUT_TOO_LARGE"
    );
    assert_eq!(resolved.final_md, raw.structured_md);

    let conn = setup_db_with_markitdown_extracted(asset_id);
    kc_persist_resolved_with_conn(&conn, asset_id, "application/pdf", "h-e", &resolved);
    assert_db_invariants_post_kc_failure(
        &conn,
        asset_id,
        "false",
        FailureCode::EKcInputTooLarge,
    );

    mock.stop();
}
