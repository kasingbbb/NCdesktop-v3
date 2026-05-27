//! task_011 集成测试：`kc::enrichment` 模块的端到端验证（AC-5）。
//!
//! ## 测试范围
//!
//! 用 `MockKcServer`（task_006）模拟 KC HTTP 端点，验证 `KcClient` → `KcCallError` 分类
//! → `kc::enrichment` 5 类失败映射的完整链路；以及 `resolve_outcome` 纯函数在每个分支下的输出
//! 形态（final_md / extractor_type / kc_enriched / failure_code_for_meta）严格符合 ADR-004 §"5
//! 类失败映射"表。
//!
//! ## 为什么不直接调 `enrich(app, asset, raw_md)`
//!
//! `enrich()` 需要 `tauri::AppHandle`（实际 Tauri runtime + `app.manage(Arc<KcClient>)` /
//! `app.manage(Arc<KcProcessManager>)` 注入），在 integration test crate 中构造真实 AppHandle
//! 非常重（要 mock window / window_event / db state / ...）。本测试改为**直接调
//! `KcClient::ingest_text` 拿到 `Result<KcIngestOutcome, KcCallError>`**，然后通过公开类型
//! `KcEnrichmentOutcome` / `KcFallbackReason` 手动构造对应的 outcome，再调 `resolve_outcome`
//! 验证落地形态——这等价于**完整验证了 `enrich()` 步骤 4-5（client 调用 + 错误分类）**与
//! **`resolve_outcome` 全部三态**，但跳过纯 Tauri runtime 杂活（emit 事件由单测覆盖；
//! emit 失败已在 enrichment.rs 兜底为 `log::warn`）。
//!
//! `enrich` 的"步骤 1（disabled 短路）+ 步骤 2-3（依赖缺失）"由 lib 内单测
//! `outcome_to_event_strings_for_all_variants` + 集成场景"disabled 短路"覆盖。
//!
//! ## 5 + 2 个集成测试
//!
//! | 测试 | mock scenario | 期望 KcCallError | 期望 outcome | 期望 resolve_outcome 字段 |
//! |--|--|--|--|--|
//! | `enrich_success_returns_full_outcome`      | success         | -                | Success           | extractor_type="markitdown+kc", kc_enriched="true" |
//! | `enrich_unavailable_returns_fallback`      | unavailable     | Unreachable      | Fallback(Unavailable) | failure_code="E_KC_UNAVAILABLE" |
//! | `enrich_timeout_returns_fallback_timeout`  | timeout         | Timeout          | Fallback(Timeout)     | failure_code="E_KC_TIMEOUT" |
//! | `enrich_internal_error_returns_fallback`   | internal_error  | Internal         | Fallback(InternalError) | failure_code="E_KC_ENRICH_FAILED" |
//! | `enrich_llm_unavailable_with_partial_returns_partial` | llm_unavailable | LlmUnavailable{Some} | PartialLlmUnavailable | extractor_type="markitdown+kc:partial", kc_enriched="partial", failure_code="E_KC_LLM_UNAVAILABLE" |
//! | `enrich_input_too_large_returns_fallback`  | input_too_large | InputTooLarge    | Fallback(InputTooLarge) | failure_code="E_KC_INPUT_TOO_LARGE" |
//! | `enrich_disabled_short_circuits`           | （不发请求）    | -                | Fallback(Disabled)    | failure_code=None |

mod common;

use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use std::time::Duration;

use app_lib::extraction::models::ExtractionResult;
use app_lib::kc::client::{KcClient, KcIngestOptions, KcIngestOutcome};
use app_lib::kc::enrichment::{resolve_outcome, ResolvedEnrichment};
use app_lib::kc::errors::{KcCallError, KcEnrichmentOutcome, KcFallbackReason, KcMeta, KcTagsSource};
use app_lib::kc::process::PortProvider;

use common::mock_kc::{KcMockMeta, MockKcServer};

// =====================================================================
// 测试用 helpers
// =====================================================================

/// 集成测试用 PortProvider stub：从 MockKcServer 取端口固化（不模拟"端口变化"）。
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

/// 构造一个 `ExtractionResult` stub（structured_md 用于 Fallback 路径的 final_md 验证）。
fn make_raw_extraction() -> ExtractionResult {
    ExtractionResult {
        raw_text: "原始文本".to_string(),
        structured_md: "# markitdown 原版\n\n这是 markitdown 生成的 MD".to_string(),
        quality_level: 3,
        extractor_type: "markitdown".to_string(),
        segments: Vec::new(),
        needs_ocr_fallback: false,
    }
}

/// 测试用 frontmatter writer：把 meta.doc_id / kc_version 嵌进 frontmatter 用于断言。
fn test_frontmatter_writer(meta: &KcMeta) -> String {
    format!(
        "---\ndoc_id: {}\nkc_version: {}\ntags_source: {}\n---",
        meta.doc_id,
        meta.kc_version,
        meta.tags_source.as_str()
    )
}

/// 把 `KcCallError` 转 `KcEnrichmentOutcome`（复刻 `enrichment.rs::map_call_error_to_outcome` 私有逻辑）。
///
/// 该函数与 lib 内 `map_call_error_to_outcome` **严格同义**（同样的映射表），但仅用于集成测试
/// 不能引用 lib 私有函数的场景。生产路径走 `enrich()` 时由 lib 内函数处理。
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

// =====================================================================
// AC-5.1: success scenario → KcEnrichmentOutcome::Success → ResolvedEnrichment
// =====================================================================

#[tokio::test]
async fn enrich_success_returns_full_outcome() {
    let enhanced = "# 增强后的文档\n\n#AI #ML\n\n## 摘要\n\n这是 AI 生成的摘要。\n";
    let meta = KcMockMeta::default();
    let mock = MockKcServer::start_with_success(enhanced, meta.clone()).await;
    let provider = StaticPortProvider::with_port(mock.port());
    let client = KcClient::new(provider);

    // 走 client 端到端
    let result = client
        .ingest_text("# 原始", &KcIngestOptions::default())
        .await;

    let outcome = match result {
        Ok(KcIngestOutcome::Success { enhanced_md, meta }) => {
            assert_eq!(enhanced_md, enhanced);
            assert_eq!(meta.kc_version, "0.9");
            // 转 enrichment outcome
            KcEnrichmentOutcome::Success { enhanced_md, meta }
        }
        other => panic!("expected Ok(Success), got {other:?}"),
    };

    // resolve_outcome 验证落地形态
    let raw = make_raw_extraction();
    let resolved: ResolvedEnrichment = resolve_outcome(&raw, outcome, test_frontmatter_writer);

    assert_eq!(resolved.extractor_type, "markitdown+kc");
    assert_eq!(resolved.kc_enriched, "true");
    assert!(
        resolved.kc_meta_for_db.is_some(),
        "Success 路径必须带 meta"
    );
    assert_eq!(resolved.failure_code_for_meta, None);
    // final_md 应当包含 frontmatter + enhanced 正文
    assert!(
        resolved.final_md.contains("doc_id: doc-mocktest"),
        "应含 mock doc_id（来自 KcMockMeta::default），实际: {}",
        resolved.final_md
    );
    assert!(resolved.final_md.contains("# 增强后的文档"));

    mock.stop();
}

// =====================================================================
// AC-5.2: unavailable scenario → KcCallError::Unreachable → Fallback(Unavailable)
// =====================================================================

#[tokio::test]
async fn enrich_unavailable_returns_fallback() {
    let mock = MockKcServer::start_with_unavailable().await;
    let provider = StaticPortProvider::with_port(mock.port());
    let client = KcClient::new(provider);

    let raw_md = "# 原始 MD";
    let result = client.ingest_text(raw_md, &KcIngestOptions::default()).await;
    let err = result.expect_err("KC 不可达应当抛错");
    assert!(
        matches!(err, KcCallError::Unreachable),
        "应当映射为 Unreachable，实际: {err:?}"
    );

    let outcome = call_error_to_outcome(err, raw_md);
    let raw = make_raw_extraction();
    let resolved = resolve_outcome(&raw, outcome, test_frontmatter_writer);

    assert_eq!(resolved.kc_enriched, "false");
    assert_eq!(resolved.extractor_type, "markitdown");
    assert!(resolved.kc_meta_for_db.is_none());
    assert_eq!(
        resolved.failure_code_for_meta,
        Some("E_KC_UNAVAILABLE"),
        "Unreachable 必须映射到 E_KC_UNAVAILABLE"
    );
    // final_md 必须是 markitdown 原版
    assert_eq!(resolved.final_md, raw.structured_md);

    mock.stop();
}

// =====================================================================
// AC-5.3（追加）：timeout scenario → KcCallError::Timeout → Fallback(Timeout)
// =====================================================================

#[tokio::test]
async fn enrich_timeout_returns_fallback_timeout() {
    // mock 延迟 500ms；client 用短超时 100ms（new_with_http_client 注入）
    let mock = MockKcServer::start_with_timeout(Duration::from_millis(500)).await;
    let provider = StaticPortProvider::with_port(mock.port());

    // 注入 100ms 超时的 reqwest client（覆盖 KcClient::new 写死的 60s）
    let short_timeout_http = reqwest::Client::builder()
        .timeout(Duration::from_millis(100))
        .build()
        .expect("short-timeout http client");
    let client = KcClient::new_with_http_client(provider, short_timeout_http);

    let raw_md = "# 测试";
    let result = client.ingest_text(raw_md, &KcIngestOptions::default()).await;
    let err = result.expect_err("100ms 超时应当抛错");
    assert!(
        matches!(err, KcCallError::Timeout),
        "应当映射为 Timeout，实际: {err:?}"
    );

    let outcome = call_error_to_outcome(err, raw_md);
    let raw = make_raw_extraction();
    let resolved = resolve_outcome(&raw, outcome, test_frontmatter_writer);

    assert_eq!(resolved.kc_enriched, "false");
    assert_eq!(
        resolved.failure_code_for_meta,
        Some("E_KC_TIMEOUT"),
        "Timeout 必须映射到 E_KC_TIMEOUT"
    );

    mock.stop();
}

// =====================================================================
// AC-5.4（追加）：internal_error scenario → KcCallError::Internal → Fallback(InternalError)
// =====================================================================

#[tokio::test]
async fn enrich_internal_error_returns_fallback() {
    let mock = MockKcServer::start_with_internal_error().await;
    let provider = StaticPortProvider::with_port(mock.port());
    let client = KcClient::new(provider);

    let raw_md = "# 测试";
    let result = client.ingest_text(raw_md, &KcIngestOptions::default()).await;
    let err = result.expect_err("KC 500 应当抛错");
    match err {
        KcCallError::Internal { ref code, .. } => {
            assert_eq!(code, "KC_INTERNAL");
        }
        other => panic!("expected Internal, got {other:?}"),
    }

    let outcome = call_error_to_outcome(err, raw_md);
    let raw = make_raw_extraction();
    let resolved = resolve_outcome(&raw, outcome, test_frontmatter_writer);

    assert_eq!(resolved.kc_enriched, "false");
    assert_eq!(
        resolved.failure_code_for_meta,
        Some("E_KC_ENRICH_FAILED"),
        "Internal 必须映射到 E_KC_ENRICH_FAILED（B 类聚合）"
    );

    mock.stop();
}

// =====================================================================
// AC-5.5（追加）：llm_unavailable + partial → PartialLlmUnavailable
// =====================================================================

#[tokio::test]
async fn enrich_llm_unavailable_with_partial_returns_partial() {
    let partial = "# 规则增强\n\n#tag1 #tag2\n\n（仅规则标签，无 AI 摘要）";
    let mock = MockKcServer::start_with_llm_unavailable(partial).await;
    let provider = StaticPortProvider::with_port(mock.port());
    let client = KcClient::new(provider);

    let raw_md = "# 测试";
    let result = client.ingest_text(raw_md, &KcIngestOptions::default()).await;
    let err = result.expect_err("KC 500 LLM_UNAVAILABLE 应当抛错");
    match err {
        KcCallError::LlmUnavailable { partial_md: Some(ref md) } => {
            assert_eq!(md, partial);
        }
        other => panic!("expected LlmUnavailable {{ Some }}, got {other:?}"),
    }

    let outcome = call_error_to_outcome(err, raw_md);
    let raw = make_raw_extraction();
    let resolved = resolve_outcome(&raw, outcome, test_frontmatter_writer);

    // **关键**：partial 路径 → markitdown+kc:partial / "partial" / E_KC_LLM_UNAVAILABLE
    assert_eq!(resolved.extractor_type, "markitdown+kc:partial");
    assert_eq!(resolved.kc_enriched, "partial");
    assert!(
        resolved.kc_meta_for_db.is_some(),
        "partial 路径必须带 (合成的) meta"
    );
    assert_eq!(
        resolved.failure_code_for_meta,
        Some("E_KC_LLM_UNAVAILABLE"),
        "partial 路径必须记 E_KC_LLM_UNAVAILABLE"
    );
    // partial 路径 meta 应当是 RuleOnly
    assert_eq!(
        resolved.kc_meta_for_db.as_ref().unwrap().tags_source,
        KcTagsSource::RuleOnly
    );
    // final_md 应当含 partial 内容
    assert!(
        resolved.final_md.contains("# 规则增强"),
        "final_md 应含 partial 正文，实际: {}",
        resolved.final_md
    );
    // frontmatter 应当反映 RuleOnly
    assert!(
        resolved.final_md.contains("tags_source: rule_only"),
        "frontmatter 应当含 tags_source: rule_only，实际: {}",
        resolved.final_md
    );

    mock.stop();
}

// =====================================================================
// AC-5.6（追加）：input_too_large scenario → Fallback(InputTooLarge)
// =====================================================================

#[tokio::test]
async fn enrich_input_too_large_returns_fallback() {
    let mock = MockKcServer::start_with_input_too_large().await;
    let provider = StaticPortProvider::with_port(mock.port());
    let client = KcClient::new(provider);

    let raw_md = "# 测试";
    let result = client.ingest_text(raw_md, &KcIngestOptions::default()).await;
    let err = result.expect_err("KC 500 INPUT_TOO_LARGE 应当抛错");
    assert!(
        matches!(err, KcCallError::InputTooLarge),
        "应当映射为 InputTooLarge，实际: {err:?}"
    );

    let outcome = call_error_to_outcome(err, raw_md);
    let raw = make_raw_extraction();
    let resolved = resolve_outcome(&raw, outcome, test_frontmatter_writer);

    assert_eq!(resolved.kc_enriched, "false");
    assert_eq!(
        resolved.failure_code_for_meta,
        Some("E_KC_INPUT_TOO_LARGE"),
        "InputTooLarge 必须映射到 E_KC_INPUT_TOO_LARGE"
    );

    mock.stop();
}

// =====================================================================
// AC-5.7（追加）：disabled 短路（不发请求，模拟 KcSettings.enabled=false 的本地等价语义）
// =====================================================================
//
// 注：真实的 `enrich(app, ...)` 在 `!settings.enabled` 时**完全短路**——不会构造 KcClient
// 也不会发请求。本测试无 AppHandle 上下文（参考模块文档解释），故直接构造
// `Fallback { reason: Disabled, .. }` outcome，验证 `resolve_outcome` 在 Disabled 分支下：
// - kc_enriched = "false"
// - extractor_type = "markitdown"
// - kc_meta_for_db = None
// - **failure_code_for_meta = None**（关键：Disabled 不写 failure_code，与其他 Fallback 区分）
// - final_md 回到 markitdown 原版

#[tokio::test]
async fn enrich_disabled_short_circuits() {
    let raw_md = "# 测试";
    let outcome = KcEnrichmentOutcome::Fallback {
        reason: KcFallbackReason::Disabled,
        base_md: raw_md.to_string(),
    };
    let raw = make_raw_extraction();
    let resolved = resolve_outcome(&raw, outcome, test_frontmatter_writer);

    assert_eq!(resolved.kc_enriched, "false");
    assert_eq!(resolved.extractor_type, "markitdown");
    assert!(resolved.kc_meta_for_db.is_none());
    assert_eq!(
        resolved.failure_code_for_meta, None,
        "Disabled 路径**不**写 failure_code（与 5 类失败区分）"
    );
    assert_eq!(
        resolved.final_md, raw.structured_md,
        "Disabled 路径 final_md 必须是 markitdown 原版"
    );
}
