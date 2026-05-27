//! task_011：`kc::enrichment` 模块——`enrich()` async 入口 + `resolve_outcome()` 纯函数。
//!
//! ## 设计依据
//!
//! - **ADR-003**（Architect output.md §"ADR-003"）：enrichment 注入点 = `scheduler::save_and_materialize`
//!   内部，签名 `kc::enrich(&app, asset, &raw_md).await -> KcEnrichmentOutcome`；之后
//!   `kc::resolve_outcome(&extraction_result, outcome, frontmatter_writer) -> ResolvedEnrichment`
//!   被 scheduler 消费。
//! - **ADR-004**（同文档 §"ADR-004 5 类失败兜底状态机"）：5 类 `KcCallError` 到三态
//!   `KcEnrichmentOutcome`（Success / PartialLlmUnavailable / Fallback）的映射表 + `failure_code`
//!   字符串映射（仅写 `conversion_meta`，不污染 `extracted_content.status`）。
//! - **PRD §4.3**：KC 失败不阻断主链路，UI 暴露"重新增强"按钮（F14），不自动重试。
//! - **session_context §3 不可妥协底线 #3**：KC 失败必须能拿 MarkItDown 原版 MD 落地。
//! - **input.md AC-1 ~ AC-5**：详细的步骤 + 5 类失败映射表 + ResolvedEnrichment 字段定义。
//!
//! ## 模块结构
//!
//! - [`ResolvedEnrichment`] —— `resolve_outcome()` 返回的"落地形态四元组 + failure_code"（AC-3）；
//! - [`enrich`] —— 主入口（AC-1），调 `KcClient::ingest_text` + 5 类失败映射 + emit 事件；
//! - [`resolve_outcome`] —— 纯函数（AC-2），outcome → ResolvedEnrichment，注入 `frontmatter_writer`
//!   保持解耦（task_013 实装真实 builder，本 task 单测注入 stub）。
//!
//! ## 共享接口唯一来源（Conductor 协调规则）
//!
//! 本模块**只 import**已有类型与 helper，**严禁自写**已存在的 mask / settings 加载函数：
//! - `KcClient` / `KcIngestOptions` / `KcIngestOutcome` —— `kc::client`（task_007）；
//! - `KcCallError` / `KcEnrichmentOutcome` / `KcFallbackReason` / `KcMeta` / `KcTagsSource`
//!   —— `kc::errors`（task_005）；
//! - `KcSettings` / `log_with_mask` —— `kc::settings`（task_004 / task_010）；
//! - `KcProcessManager` / `KcStatus` —— `kc::process`（task_008）。
//!
//! ## 不变量
//!
//! 1. **failure_code 字面严格对齐**：`ResolvedEnrichment.failure_code_for_meta` 字面值由
//!    `FailureCode::EKc*.as_str()` 静态返回（不允许手写字符串），与 task_003 落地保持一致。
//! 2. **5 类失败全覆盖**：每个 `KcCallError` 变体（6 个）都有明确的 `KcEnrichmentOutcome` 映射，
//!    单测守护（防漏）。
//! 3. **emit 失败不影响落地**：emit `notecapt/asset-kc-enriched` 失败仅 `log::warn`，不向上抛错。
//! 4. **enrich 不写 DB**：DB 写入由 task_015 的 `db_update_kc_fields` 在 scheduler 拿到
//!    `ResolvedEnrichment` 之后做，本模块只做"产出 outcome + outcome → 落地形态"两件事。
//!
//! ## 前置技术债登记
//!
//! - **TD-3**（progress.md 登记）：`src/db/conversion_meta.rs::parse_failure_code()` 当前仅 8 个
//!   markitdown 字面值，缺 5 个 KC 字面值（`E_KC_UNAVAILABLE` / `E_KC_ENRICH_FAILED` /
//!   `E_KC_LLM_UNAVAILABLE` / `E_KC_TIMEOUT` / `E_KC_INPUT_TOO_LARGE`）。
//!   - **本 task 不补**（task_015 已并发跑同一治理点；为避免冲突由 task_015 单一来源补）；
//!   - **本 task 假设 TD-3 已 / 将由 task_015 补完**——本模块逻辑不依赖 `parse_failure_code`，
//!     仅写 `conversion_meta.failure_code` 字面（写值是 `FailureCode::EKc*.as_str()`，
//!     与 task_015 后续 `parse_failure_code` 读值严格 round-trip）。

use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};

use crate::extraction::failure_code::FailureCode;
use crate::extraction::models::ExtractionResult;
use crate::kc::client::{KcClient, KcIngestOptions, KcIngestOutcome};
use crate::kc::errors::{
    KcCallError, KcEnrichmentOutcome, KcFallbackReason, KcMeta, KcTagsSource,
};
use crate::kc::process::{KcProcessManager, KcStatus};
use crate::kc::settings::{log_with_mask, KcSettings};
use crate::models::asset::Asset;

// =====================================================================
// 1. 早期错误（enrich 内"前置依赖缺失"分类，仅供 log，向上仍走 Fallback 三态）
// =====================================================================

/// `enrich` 阶段的"前置依赖缺失"分类（仅日志用，不向上抛——所有路径都返回 `KcEnrichmentOutcome`）。
///
/// 该 enum 不参与公共契约，仅用于内部判定"为什么走 Fallback"以便选择正确的 `KcFallbackReason`。
#[derive(Debug)]
enum SpawnSkipReason {
    /// Tauri state 中没有注入 `Arc<KcClient>`（lib.rs setup 异常 / 测试态）。
    NoKcClient,
    /// Tauri state 中没有注入 `Arc<KcProcessManager>`（同上）。
    NoKcManager,
    /// `KcProcessManager` 状态非 Ready（Starting / Unavailable / Stopped）。
    ProcessNotReady(KcStatus),
}

// =====================================================================
// 2. ResolvedEnrichment（AC-3：5 字段，scheduler 直接消费）
// =====================================================================

/// `resolve_outcome` 返回的"落地形态"——scheduler 据此写 .md 文件 + DB 列。
///
/// **字段语义**（与 ADR-004 §"5 类失败映射"严格一致）：
///
/// | 字段 | Success | PartialLlmUnavailable | Fallback |
/// |--|--|--|--|
/// | `final_md` | frontmatter + enhanced_md | frontmatter + rule_only_md | markitdown 原 MD |
/// | `extractor_type` | `"markitdown+kc"` | `"markitdown+kc:partial"` | `"markitdown"` |
/// | `kc_enriched` | `"true"` | `"partial"` | `"false"` |
/// | `kc_meta_for_db` | `Some(meta)` | `Some(meta)` | `None` |
/// | `failure_code_for_meta` | `None` | `Some("E_KC_LLM_UNAVAILABLE")` | 按 reason 映射 |
///
/// **不变量**：`failure_code_for_meta` 是 `Option<&'static str>`——字面值由
/// `FailureCode::EKc*.as_str()` 提供，避免任何位置手写字符串导致字面漂移
/// （单测 `failure_code_strings_match_failure_code_enum` 守护）。
#[derive(Debug, Clone)]
pub struct ResolvedEnrichment {
    /// 最终落地 .md 文件内容（含 frontmatter + 正文）。
    pub final_md: String,
    /// `extracted_content.extractor_type` 列值。
    pub extractor_type: String,
    /// `extracted_content.kc_enriched` 列值（`"true"` / `"partial"` / `"false"`）。
    pub kc_enriched: String,
    /// KC 元数据（仅 Success / PartialLlmUnavailable 有；Fallback 时 None）。
    pub kc_meta_for_db: Option<KcMeta>,
    /// `conversion_meta.failure_code` 列值（None 表示"不写 failure_code"，仅 Success / Disabled）。
    pub failure_code_for_meta: Option<&'static str>,
}

// =====================================================================
// 3. enrich()：主入口（AC-1）
// =====================================================================

/// KC enrichment step 主入口（AC-1）——调 `KcClient::ingest_text`，把结果映射到三态
/// `KcEnrichmentOutcome`，并 emit `notecapt/asset-kc-enriched` 事件。
///
/// ## 流程（input.md AC-1 步骤 1-5）
///
/// 1. **读 `KcSettings`**；若 `!settings.enabled` → 立即返回
///    `Fallback { reason: Disabled, base_md: raw_md.to_string() }`（用户主动关 KC，**不是失败**）；
/// 2. **取 `KcProcessManager` state**；若状态非 `Ready` → `Fallback { reason: Unavailable, .. }`；
/// 3. **取 `KcClient` state**；缺失（理论上不会，lib.rs setup 阶段已 `app.manage()`）→
///    `Fallback { reason: Unavailable, .. }`；
/// 4. **调 `client.ingest_text`** with `KcIngestOptions { use_ai, enable_qa, enable_links,
///    persist: false }`（persist 永远 false，ADR-006 层 1）；
/// 5. **Result 分流**（ADR-004 §"5 类失败映射"完整表，详见 [`map_call_error_to_outcome`]）；
/// 6. **emit `notecapt/asset-kc-enriched`** 含 `{ assetId, kcEnriched, failureCode }`（emit 失败
///    仅 log，不影响落地）。
///
/// ## 返回值
///
/// 永远返回 `KcEnrichmentOutcome` 三态之一——**不向上抛 Result**：
/// - `Success { enhanced_md, meta }`：KC 正常 + 完整增强 MD；
/// - `PartialLlmUnavailable { rule_only_md, meta }`：LLM 不可用但有规则增强 MD；
/// - `Fallback { reason, base_md }`：完全降级到 markitdown 原版（base_md = `raw_md.to_string()`）。
///
/// ## 安全
///
/// - **不输出 Key**：所有日志走 `kc::settings::log_with_mask(&settings)`（task_010）；
/// - **emit payload 不含 Key**：仅 `assetId` / `kcEnriched` / `failureCode` 三个字段。
pub async fn enrich(app: &AppHandle, asset: &Asset, raw_md: &str) -> KcEnrichmentOutcome {
    // ---- 步骤 1：读 KcSettings ----
    let settings = read_kc_settings(app);

    if !settings.enabled {
        log_with_mask(
            log::Level::Info,
            &format!(
                "[kc::enrich] asset={} kc.enabled=false，短路走 markitdown 原 MD",
                asset.id
            ),
            &settings,
        );
        let outcome = KcEnrichmentOutcome::Fallback {
            reason: KcFallbackReason::Disabled,
            base_md: raw_md.to_string(),
        };
        emit_kc_enriched(app, &asset.id, &outcome);
        return outcome;
    }

    // ---- 步骤 2 + 3：取 KcProcessManager + KcClient state ----
    let client = match resolve_kc_dependencies(app) {
        Ok(c) => c,
        Err(skip) => {
            // 早期 skip：根据原因 log + 走 Fallback(Unavailable)
            if let SpawnSkipReason::ProcessNotReady(ref status) = skip {
                log_with_mask(
                    log::Level::Warn,
                    &format!(
                        "[kc::enrich] asset={} KcProcessManager 状态={}，走 markitdown 原 MD",
                        asset.id,
                        status.as_event_str()
                    ),
                    &settings,
                );
            } else {
                log_with_mask(
                    log::Level::Warn,
                    &format!(
                        "[kc::enrich] asset={} KC 依赖缺失 ({:?})，走 markitdown 原 MD",
                        asset.id, skip
                    ),
                    &settings,
                );
            }
            let outcome = KcEnrichmentOutcome::Fallback {
                reason: KcFallbackReason::Unavailable,
                base_md: raw_md.to_string(),
            };
            emit_kc_enriched(app, &asset.id, &outcome);
            return outcome;
        }
    };

    // ---- 步骤 4：调 client.ingest_text ----
    let options = KcIngestOptions {
        use_ai: settings.use_ai,
        enable_qa: settings.enable_qa,
        enable_links: settings.enable_links,
        persist: false, // ADR-006 层 1：永远 false
    };

    // task_025：在 ingest 真实开始前 emit `notecapt/asset-kc-queued`，
    // 让前端 toast 在依赖解析通过、即将真正占用 KC 时显示队列长度。
    // 前置失败（!enabled / Unavailable）路径不 emit，避免噪音（已 fallthrough 到 enriched 事件）。
    emit_kc_queued(app, &asset.id);

    let result = client.ingest_text(raw_md, &options).await;

    // ---- 步骤 5：Result 分流（5 类失败映射） ----
    let outcome = match result {
        Ok(KcIngestOutcome::Success { enhanced_md, meta }) => {
            KcEnrichmentOutcome::Success { enhanced_md, meta }
        }
        Err(err) => map_call_error_to_outcome(err, raw_md, &settings, &asset.id),
    };

    // ---- 步骤 6：emit 事件 ----
    emit_kc_enriched(app, &asset.id, &outcome);

    outcome
}

/// 把 `KcCallError` 6 变体映射为 `KcEnrichmentOutcome` 三态（ADR-004 §"5 类失败映射"完整表）。
///
/// **映射表**（与 input.md AC-1 步骤 4 严格一致）：
///
/// | KcCallError 变体 | KcEnrichmentOutcome | 说明 |
/// |--|--|--|
/// | `Unreachable`            | `Fallback(Unavailable)`    | A 类：KC 子进程不可达 |
/// | `Timeout`                | `Fallback(Timeout)`        | D 类：60s 客户端超时 |
/// | `LlmUnavailable { Some }` | `PartialLlmUnavailable`   | C 类：拿到 KC 规则增强 partial MD |
/// | `LlmUnavailable { None }` | `Fallback(InternalError)` | C 类无 partial：当 InternalError 处理 |
/// | `Internal { detail, code }` | `Fallback(InternalError(detail))` | B 类：KC 内部错误 |
/// | `InputTooLarge`          | `Fallback(InputTooLarge)`  | E 类 |
/// | `Malformed { reason }`   | `Fallback(Malformed)`     | B 类：KC-MOD-1 未到位，emit warn |
///
/// **PartialLlmUnavailable 的 meta 合成**：客户端层 `KcCallError::LlmUnavailable.partial_md`
/// 不带 meta（仅有 `Option<String>`），本函数合成一个 `KcMeta`，把：
/// - `tags_source = KcTagsSource::RuleOnly`（规则增强 → 仅规则标签）；
/// - `ai_*` 字段全空（无 AI 摘要 / 问答对 / 段落关联）；
/// - `doc_id = "doc-partial"`（占位，task_015 写 DB 时可识别）；
/// - `kc_version = "unknown"`（partial 路径不带版本信息）。
fn map_call_error_to_outcome(
    err: KcCallError,
    raw_md: &str,
    settings: &KcSettings,
    asset_id: &str,
) -> KcEnrichmentOutcome {
    match err {
        KcCallError::Unreachable => {
            log_with_mask(
                log::Level::Warn,
                &format!(
                    "[kc::enrich] asset={asset_id} KC 不可达（Unreachable），降级 markitdown 原 MD"
                ),
                settings,
            );
            KcEnrichmentOutcome::Fallback {
                reason: KcFallbackReason::Unavailable,
                base_md: raw_md.to_string(),
            }
        }
        KcCallError::Timeout => {
            log_with_mask(
                log::Level::Warn,
                &format!(
                    "[kc::enrich] asset={asset_id} KC 60s 超时（Timeout），降级 markitdown 原 MD"
                ),
                settings,
            );
            KcEnrichmentOutcome::Fallback {
                reason: KcFallbackReason::Timeout,
                base_md: raw_md.to_string(),
            }
        }
        KcCallError::LlmUnavailable { partial_md: Some(md) } => {
            log_with_mask(
                log::Level::Info,
                &format!(
                    "[kc::enrich] asset={asset_id} LLM 不可用但拿到 partial（{} 字节），走 PartialLlmUnavailable",
                    md.len()
                ),
                settings,
            );
            KcEnrichmentOutcome::PartialLlmUnavailable {
                rule_only_md: md,
                meta: synthesize_partial_meta(),
            }
        }
        KcCallError::LlmUnavailable { partial_md: None } => {
            // C 类但 KC 端未启用 KC-MOD-3 "类型 C"，无 partial 字符串可用——降级为 InternalError。
            log_with_mask(
                log::Level::Warn,
                &format!(
                    "[kc::enrich] asset={asset_id} LLM 不可用且无 partial_md（KC-MOD-3 未到位），降级 markitdown"
                ),
                settings,
            );
            KcEnrichmentOutcome::Fallback {
                reason: KcFallbackReason::InternalError("LLM unavailable, no partial".to_string()),
                base_md: raw_md.to_string(),
            }
        }
        KcCallError::Internal { detail, code } => {
            log_with_mask(
                log::Level::Warn,
                &format!(
                    "[kc::enrich] asset={asset_id} KC 内部错误 code={code} detail={detail}，降级 markitdown"
                ),
                settings,
            );
            KcEnrichmentOutcome::Fallback {
                reason: KcFallbackReason::InternalError(detail),
                base_md: raw_md.to_string(),
            }
        }
        KcCallError::InputTooLarge => {
            log_with_mask(
                log::Level::Warn,
                &format!(
                    "[kc::enrich] asset={asset_id} 输入超 1MB（InputTooLarge），降级 markitdown 原 MD"
                ),
                settings,
            );
            KcEnrichmentOutcome::Fallback {
                reason: KcFallbackReason::InputTooLarge,
                base_md: raw_md.to_string(),
            }
        }
        KcCallError::Malformed { reason } => {
            // 关键失败：KC 200 但 enhanced_markdown 字段缺失——大概率是 KC-MOD-1 未落地，
            // emit warn 以便 reviewer / dev 看到信号。
            log_with_mask(
                log::Level::Warn,
                &format!(
                    "[kc::enrich] asset={asset_id} KC 响应 Malformed（reason={reason}）；很可能 KC-MOD-1 未到位，降级 markitdown"
                ),
                settings,
            );
            KcEnrichmentOutcome::Fallback {
                reason: KcFallbackReason::Malformed,
                base_md: raw_md.to_string(),
            }
        }
    }
}

/// 合成 `PartialLlmUnavailable` 路径的 KcMeta 占位（无 AI 字段 + RuleOnly 标记）。
///
/// 设计：partial 路径下客户端只有一个 `partial_md: String`——无法从 KC 响应拿到 `ai_*` /
/// `kc_version` / `doc_id`。本函数生成"语义安全"的 meta：
/// - `tags_source = RuleOnly`，让 frontmatter / DB 一致表达"非 AI 增强"；
/// - 所有 ai_* 字段空；
/// - `doc_id = "doc-partial"` 让 task_015 写 conversion_meta.kc_doc_id 时能区分"完整 success"
///   与"partial 路径"。
///
/// **不变量**：调用方（scheduler / db）拿到此 meta 后写 DB 时，frontmatter / kc_version 应当
/// 处理 "unknown" 字面（不视为 bug）。
fn synthesize_partial_meta() -> KcMeta {
    KcMeta {
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
    }
}

// =====================================================================
// 4. resolve_outcome()：纯函数（AC-2）
// =====================================================================

/// 把 `KcEnrichmentOutcome` 转换为 `ResolvedEnrichment`（AC-2，**纯函数**——无 IO / 无 await）。
///
/// ## 三态分支
///
/// 1. **Success { enhanced_md, meta }**：
///    - `final_md = frontmatter_writer(&meta) + "\n\n" + enhanced_md`
///    - `extractor_type = "markitdown+kc"`
///    - `kc_enriched = "true"`
///    - `kc_meta_for_db = Some(meta)`
///    - `failure_code_for_meta = None`（成功，不写 failure_code）
/// 2. **PartialLlmUnavailable { rule_only_md, meta }**：
///    - `final_md = frontmatter_writer(&meta) + "\n\n" + rule_only_md`
///    - `extractor_type = "markitdown+kc:partial"`
///    - `kc_enriched = "partial"`
///    - `kc_meta_for_db = Some(meta)`
///    - `failure_code_for_meta = Some("E_KC_LLM_UNAVAILABLE")`（partial 仍记 LLM 不可用码）
/// 3. **Fallback { reason, base_md }**：
///    - `final_md = raw.structured_md.clone()`（用 ExtractionResult 原版 markitdown MD，而**非**
///      `base_md`——确保严格回归到 NC scheduler 已经验证过的 markitdown 输出，避免 enrich 入参
///      raw_md 在传递链路中被改写带来的不确定性）
///    - `extractor_type = "markitdown"`
///    - `kc_enriched = "false"`
///    - `kc_meta_for_db = None`
///    - `failure_code_for_meta = ` 按 reason 映射：
///      - `Disabled` → `None`（用户关 KC 不是失败）；
///      - `Unavailable` → `Some("E_KC_UNAVAILABLE")`；
///      - `Timeout` → `Some("E_KC_TIMEOUT")`；
///      - `InternalError(_)` → `Some("E_KC_ENRICH_FAILED")`；
///      - `InputTooLarge` → `Some("E_KC_INPUT_TOO_LARGE")`；
///      - `Malformed` → `Some("E_KC_ENRICH_FAILED")`。
///
/// ## frontmatter_writer 注入
///
/// `Fn(&KcMeta) -> String` —— scheduler 真实使用时传 task_013 实装的 `build_kc_frontmatter`；
/// 单测可注入 stub（如 `|_| "---\nstub\n---".to_string()`）。这样：
/// - 本函数**不依赖** task_013 是否实装；
/// - task_013 单测可独立验证 frontmatter 字面值；
/// - 集成测试用真实 builder 验证端到端形态。
///
/// **不变量**：本函数无 `await` / 无 `std::io` / 无 `log`——可在任意线程上下文 / 单元测试中调用。
pub fn resolve_outcome(
    raw: &ExtractionResult,
    outcome: KcEnrichmentOutcome,
    frontmatter_writer: impl Fn(&KcMeta) -> String,
) -> ResolvedEnrichment {
    match outcome {
        KcEnrichmentOutcome::Success { enhanced_md, meta } => {
            let fm = frontmatter_writer(&meta);
            ResolvedEnrichment {
                final_md: join_frontmatter_body(&fm, &enhanced_md),
                extractor_type: "markitdown+kc".to_string(),
                kc_enriched: "true".to_string(),
                kc_meta_for_db: Some(meta),
                failure_code_for_meta: None,
            }
        }
        KcEnrichmentOutcome::PartialLlmUnavailable { rule_only_md, meta } => {
            let fm = frontmatter_writer(&meta);
            ResolvedEnrichment {
                final_md: join_frontmatter_body(&fm, &rule_only_md),
                extractor_type: "markitdown+kc:partial".to_string(),
                kc_enriched: "partial".to_string(),
                kc_meta_for_db: Some(meta),
                failure_code_for_meta: Some(FailureCode::EKcLlmUnavailable.as_str()),
            }
        }
        KcEnrichmentOutcome::Fallback { reason, base_md: _ } => {
            // 注意：用 raw.structured_md 而非 outcome.base_md——retire enrich 入参链路的不确定性，
            // 让 scheduler 落地的 final_md 与 markitdown 阶段产物字面一致。
            ResolvedEnrichment {
                final_md: raw.structured_md.clone(),
                extractor_type: "markitdown".to_string(),
                kc_enriched: "false".to_string(),
                kc_meta_for_db: None,
                failure_code_for_meta: fallback_reason_to_failure_code(&reason),
            }
        }
    }
}

/// 把 `KcFallbackReason` 映射为 `Option<&'static str>` failure_code 字面值。
///
/// 复用 `KcFallbackReason::to_failure_code()`（task_005 已实装的 `Option<FailureCode>`），
/// 再走 `as_str()` 拿静态字面。这样：
/// - 字面值唯一源在 `FailureCode::EKc*.as_str()`；
/// - `KcFallbackReason::to_failure_code()` 改变映射时本函数自动跟进；
/// - 单测 `failure_code_strings_match_failure_code_enum` 守护字面值不漂移。
fn fallback_reason_to_failure_code(reason: &KcFallbackReason) -> Option<&'static str> {
    reason.to_failure_code().map(|fc| fc.as_str())
}

/// 拼接 frontmatter + 正文（中间空一行）。
///
/// 设计：
/// - frontmatter 末尾可能已经带 `\n`，也可能没带——本函数用 `trim_end_matches('\n')` 归一化，
///   再加固定 `\n\n` 分隔，避免出现 `\n\n\n\n` 这种连续空行；
/// - 空 frontmatter（如 `""`）也允许——直接返回 body，不前置分隔符。
fn join_frontmatter_body(frontmatter: &str, body: &str) -> String {
    if frontmatter.is_empty() {
        return body.to_string();
    }
    let trimmed_fm = frontmatter.trim_end_matches('\n');
    format!("{trimmed_fm}\n\n{body}")
}

// =====================================================================
// 5. 内部 helpers
// =====================================================================

/// 从 `AppHandle` 读 `KcSettings`，失败兜底走 `Default::default()`。
///
/// 与 `kc::process::read_kc_settings` 同模式（已在 task_008 实装），但 process 那份是模块私有，
/// 这里**复制**而非 import——因为 process.rs 在 Conductor 协调规则中标记"不动"。
/// （如果以后想合并 helper，可在后续重构 task 提取为 `kc::settings::load_from_app`。）
fn read_kc_settings(app: &AppHandle) -> KcSettings {
    let db_state = match app.try_state::<crate::db::Database>() {
        Some(s) => s,
        None => return KcSettings::default(),
    };
    let conn = match db_state.conn.lock() {
        Ok(c) => c,
        Err(_) => return KcSettings::default(),
    };
    KcSettings::load(&conn).unwrap_or_default()
}

/// 从 `AppHandle` state 取 `Arc<KcClient>` + `Arc<KcProcessManager>`，返回 client 或 skip 原因。
///
/// 三种 skip 路径：
/// 1. KcProcessManager state 未注入（lib.rs setup 失败 / 测试态）→ `NoKcManager`；
/// 2. KcProcessManager 状态非 Ready → `ProcessNotReady(status)`；
/// 3. KcClient state 未注入（同上）→ `NoKcClient`。
///
/// 1+3 理论上不会在生产路径出现（lib.rs setup 必然 manage 两个）。
fn resolve_kc_dependencies(app: &AppHandle) -> Result<Arc<KcClient>, SpawnSkipReason> {
    // 取 KcProcessManager 检查状态
    let manager_state = app.try_state::<Arc<KcProcessManager>>();
    let manager = match manager_state {
        Some(m) => m.inner().clone(),
        None => return Err(SpawnSkipReason::NoKcManager),
    };
    let status = manager.current_status();
    if !matches!(status, KcStatus::Ready) {
        return Err(SpawnSkipReason::ProcessNotReady(status));
    }

    // 取 KcClient
    let client_state = app.try_state::<Arc<KcClient>>();
    match client_state {
        Some(c) => Ok(c.inner().clone()),
        None => Err(SpawnSkipReason::NoKcClient),
    }
}

/// task_025：emit `notecapt/asset-kc-queued`（KC ingest 即将开始 → 前端 toast 队列计数 +1）。
///
/// payload schema（与 task_025 input.md AC-2 严格一致）：
/// ```json
/// { "assetId": "<asset.id>" }
/// ```
///
/// **触发时机**：在 `client.ingest_text` 调用前一行，且仅在前置依赖解析（KcSettings.enabled
/// 通过 + KcProcessManager Ready + KcClient 存在）全部通过时 emit；早期 Fallback（Disabled /
/// Unavailable）路径不 emit `queued`，因为这些 asset 实际并未进入 KC 队列。前端订阅这个事件 +
/// `notecapt/asset-kc-enriched`（开始 / 结束配对）即可维护队列长度。
///
/// **不影响落地**：emit 失败仅 `log::warn`，与 `emit_kc_enriched` 同保护策略。
fn emit_kc_queued(app: &AppHandle, asset_id: &str) {
    let payload = build_kc_queued_payload(asset_id);
    if let Err(e) = app.emit("notecapt/asset-kc-queued", payload) {
        log::warn!("[kc::enrich] emit notecapt/asset-kc-queued 失败: {e}");
    }
}

/// 构造 `notecapt/asset-kc-queued` 的 payload（提取为纯函数以便单测覆盖 payload 字面）。
fn build_kc_queued_payload(asset_id: &str) -> serde_json::Value {
    serde_json::json!({ "assetId": asset_id })
}

/// emit `notecapt/asset-kc-enriched` 事件（AC-1 步骤 5）。
///
/// payload schema（与 Architect output.md §"NC 事件"严格一致）：
/// ```json
/// {
///   "assetId": "<asset.id>",
///   "kcEnriched": "true" | "partial" | "false",
///   "failureCode": "E_KC_*" | null
/// }
/// ```
///
/// **不影响落地**：emit 失败（如 Tauri runtime 不可达）仅 `log::warn`，主链路不受影响。
fn emit_kc_enriched(app: &AppHandle, asset_id: &str, outcome: &KcEnrichmentOutcome) {
    let (kc_enriched, failure_code) = outcome_to_event_strings(outcome);
    let payload = serde_json::json!({
        "assetId": asset_id,
        "kcEnriched": kc_enriched,
        "failureCode": failure_code,
    });
    if let Err(e) = app.emit("notecapt/asset-kc-enriched", payload) {
        log::warn!("[kc::enrich] emit notecapt/asset-kc-enriched 失败: {e}");
    }
}

/// 把 `KcEnrichmentOutcome` 转换为 emit payload 用的 (kc_enriched, failure_code)。
///
/// 与 `resolve_outcome` 共享语义但**独立维护**——因为 enrich 阶段就 emit 事件（不依赖
/// resolve_outcome 调用，让前端在 enrich 完成 → resolve_outcome 还未跑完时就能更新 UI）。
fn outcome_to_event_strings(
    outcome: &KcEnrichmentOutcome,
) -> (&'static str, Option<&'static str>) {
    match outcome {
        KcEnrichmentOutcome::Success { .. } => ("true", None),
        KcEnrichmentOutcome::PartialLlmUnavailable { .. } => {
            ("partial", Some(FailureCode::EKcLlmUnavailable.as_str()))
        }
        KcEnrichmentOutcome::Fallback { reason, .. } => {
            ("false", reason.to_failure_code().map(|fc| fc.as_str()))
        }
    }
}

// =====================================================================
// 6. 单元测试（AC-4：覆盖所有 outcome → ResolvedEnrichment 路径）
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kc::errors::{KcMeta, KcTagsSource};

    // ---------- 辅助构造器 ----------

    fn make_meta() -> KcMeta {
        KcMeta {
            doc_id: "doc-test".to_string(),
            kc_version: "0.9".to_string(),
            tags_source: KcTagsSource::AiAndRule,
            ai_tags: vec!["AI".to_string()],
            rule_tags: vec!["Rule".to_string()],
            ai_summary: Some("summary".to_string()),
            ai_qa_pairs: Vec::new(),
            ai_paragraph_links: Vec::new(),
            generated_at: "2026-05-27T00:00:00Z".to_string(),
            paragraph_count: 3,
            response_size_bytes: 1024,
            duration_ms: 100,
        }
    }

    fn make_raw() -> ExtractionResult {
        ExtractionResult {
            raw_text: "raw text".to_string(),
            structured_md: "# markitdown 原版".to_string(),
            quality_level: 3,
            extractor_type: "markitdown".to_string(),
            segments: Vec::new(),
            needs_ocr_fallback: false,
        }
    }

    /// stub frontmatter writer：把 meta.doc_id 嵌进 frontmatter 用于断言。
    fn stub_writer(meta: &KcMeta) -> String {
        format!("---\ndoc_id: {}\n---", meta.doc_id)
    }

    // =================================================================
    // 路径 1：Success → markitdown+kc / "true" / Some(meta) / None failure
    // =================================================================

    #[test]
    fn resolve_outcome_success_path() {
        let raw = make_raw();
        let meta = make_meta();
        let outcome = KcEnrichmentOutcome::Success {
            enhanced_md: "# 增强 MD".to_string(),
            meta: meta.clone(),
        };

        let resolved = resolve_outcome(&raw, outcome, stub_writer);

        assert!(
            resolved.final_md.contains("doc_id: doc-test"),
            "final_md 应包含 stub frontmatter，实际: {}",
            resolved.final_md
        );
        assert!(
            resolved.final_md.contains("# 增强 MD"),
            "final_md 应包含 enhanced_md 正文，实际: {}",
            resolved.final_md
        );
        // frontmatter + 空行 + body 的拼接结构
        assert!(
            resolved.final_md.contains("---\n\n# 增强 MD"),
            "frontmatter 和 body 中间应有一个空行，实际: {}",
            resolved.final_md
        );

        assert_eq!(resolved.extractor_type, "markitdown+kc");
        assert_eq!(resolved.kc_enriched, "true");
        assert!(resolved.kc_meta_for_db.is_some(), "Success 应带 meta");
        assert_eq!(
            resolved.kc_meta_for_db.as_ref().unwrap().doc_id,
            meta.doc_id
        );
        assert_eq!(
            resolved.failure_code_for_meta, None,
            "Success 不写 failure_code"
        );
    }

    // =================================================================
    // 路径 2：PartialLlmUnavailable → markitdown+kc:partial / "partial" / Some / E_KC_LLM_UNAVAILABLE
    // =================================================================

    #[test]
    fn resolve_outcome_partial_llm_unavailable_path() {
        let raw = make_raw();
        let meta = make_meta();
        let outcome = KcEnrichmentOutcome::PartialLlmUnavailable {
            rule_only_md: "# 规则增强".to_string(),
            meta: meta.clone(),
        };

        let resolved = resolve_outcome(&raw, outcome, stub_writer);

        assert!(resolved.final_md.contains("# 规则增强"));
        assert_eq!(resolved.extractor_type, "markitdown+kc:partial");
        assert_eq!(resolved.kc_enriched, "partial");
        assert!(
            resolved.kc_meta_for_db.is_some(),
            "PartialLlmUnavailable 也应带 meta"
        );
        assert_eq!(
            resolved.failure_code_for_meta,
            Some("E_KC_LLM_UNAVAILABLE"),
            "partial 路径必须记 E_KC_LLM_UNAVAILABLE"
        );
        // 严格对齐 FailureCode enum
        assert_eq!(
            resolved.failure_code_for_meta.unwrap(),
            FailureCode::EKcLlmUnavailable.as_str()
        );
    }

    // =================================================================
    // 路径 3 ~ 8：Fallback 六种 reason
    // =================================================================

    #[test]
    fn resolve_outcome_fallback_disabled_path() {
        let raw = make_raw();
        let outcome = KcEnrichmentOutcome::Fallback {
            reason: KcFallbackReason::Disabled,
            base_md: "ignored".to_string(),
        };

        let resolved = resolve_outcome(&raw, outcome, stub_writer);

        // Fallback 路径 final_md 走 raw.structured_md（而非 base_md）
        assert_eq!(resolved.final_md, raw.structured_md);
        assert_eq!(resolved.extractor_type, "markitdown");
        assert_eq!(resolved.kc_enriched, "false");
        assert!(resolved.kc_meta_for_db.is_none());
        assert_eq!(
            resolved.failure_code_for_meta, None,
            "Disabled 不写 failure_code（用户主动关 KC 不是失败）"
        );
    }

    #[test]
    fn resolve_outcome_fallback_unavailable_path() {
        let raw = make_raw();
        let outcome = KcEnrichmentOutcome::Fallback {
            reason: KcFallbackReason::Unavailable,
            base_md: "ignored".to_string(),
        };
        let resolved = resolve_outcome(&raw, outcome, stub_writer);
        assert_eq!(resolved.failure_code_for_meta, Some("E_KC_UNAVAILABLE"));
        assert_eq!(resolved.kc_enriched, "false");
        assert_eq!(resolved.extractor_type, "markitdown");
        assert_eq!(resolved.final_md, raw.structured_md);
    }

    #[test]
    fn resolve_outcome_fallback_timeout_path() {
        let raw = make_raw();
        let outcome = KcEnrichmentOutcome::Fallback {
            reason: KcFallbackReason::Timeout,
            base_md: "ignored".to_string(),
        };
        let resolved = resolve_outcome(&raw, outcome, stub_writer);
        assert_eq!(resolved.failure_code_for_meta, Some("E_KC_TIMEOUT"));
    }

    #[test]
    fn resolve_outcome_fallback_internal_error_path() {
        let raw = make_raw();
        let outcome = KcEnrichmentOutcome::Fallback {
            reason: KcFallbackReason::InternalError("KC died".to_string()),
            base_md: "ignored".to_string(),
        };
        let resolved = resolve_outcome(&raw, outcome, stub_writer);
        assert_eq!(resolved.failure_code_for_meta, Some("E_KC_ENRICH_FAILED"));
    }

    #[test]
    fn resolve_outcome_fallback_input_too_large_path() {
        let raw = make_raw();
        let outcome = KcEnrichmentOutcome::Fallback {
            reason: KcFallbackReason::InputTooLarge,
            base_md: "ignored".to_string(),
        };
        let resolved = resolve_outcome(&raw, outcome, stub_writer);
        assert_eq!(resolved.failure_code_for_meta, Some("E_KC_INPUT_TOO_LARGE"));
    }

    #[test]
    fn resolve_outcome_fallback_malformed_path() {
        let raw = make_raw();
        let outcome = KcEnrichmentOutcome::Fallback {
            reason: KcFallbackReason::Malformed,
            base_md: "ignored".to_string(),
        };
        let resolved = resolve_outcome(&raw, outcome, stub_writer);
        assert_eq!(
            resolved.failure_code_for_meta,
            Some("E_KC_ENRICH_FAILED"),
            "Malformed 与 InternalError 同 code（ADR-004 §'5 类失败映射'）"
        );
    }

    // =================================================================
    // 不变量守护：failure_code 字面与 FailureCode enum 严格一一对齐
    // =================================================================

    /// 守护：本模块所有 `failure_code_for_meta` 字面值必须来自 `FailureCode::EKc*.as_str()`，
    /// 不能手写字符串。任何字面漂移由此 fail。
    #[test]
    fn failure_code_strings_match_failure_code_enum() {
        // 6 个 reason → 各自字面值
        let pairs = [
            (KcFallbackReason::Disabled, None),
            (
                KcFallbackReason::Unavailable,
                Some(FailureCode::EKcUnavailable.as_str()),
            ),
            (
                KcFallbackReason::Timeout,
                Some(FailureCode::EKcTimeout.as_str()),
            ),
            (
                KcFallbackReason::InternalError("x".to_string()),
                Some(FailureCode::EKcEnrichFailed.as_str()),
            ),
            (
                KcFallbackReason::InputTooLarge,
                Some(FailureCode::EKcInputTooLarge.as_str()),
            ),
            (
                KcFallbackReason::Malformed,
                Some(FailureCode::EKcEnrichFailed.as_str()),
            ),
        ];
        for (reason, expected) in pairs {
            assert_eq!(
                fallback_reason_to_failure_code(&reason),
                expected,
                "reason={reason:?} 应映射到 {expected:?}"
            );
        }
    }

    // =================================================================
    // map_call_error_to_outcome：6 个 KcCallError 变体的桥接路径（不走 HTTP）
    // =================================================================

    #[test]
    fn map_call_error_unreachable_returns_fallback_unavailable() {
        let outcome = map_call_error_to_outcome(
            KcCallError::Unreachable,
            "raw",
            &KcSettings::default(),
            "asset-1",
        );
        match outcome {
            KcEnrichmentOutcome::Fallback {
                reason: KcFallbackReason::Unavailable,
                base_md,
            } => assert_eq!(base_md, "raw"),
            other => panic!("expected Fallback(Unavailable), got {other:?}"),
        }
    }

    #[test]
    fn map_call_error_timeout_returns_fallback_timeout() {
        let outcome = map_call_error_to_outcome(
            KcCallError::Timeout,
            "raw",
            &KcSettings::default(),
            "asset-2",
        );
        assert!(matches!(
            outcome,
            KcEnrichmentOutcome::Fallback {
                reason: KcFallbackReason::Timeout,
                ..
            }
        ));
    }

    #[test]
    fn map_call_error_llm_unavailable_with_partial_returns_partial_outcome() {
        let outcome = map_call_error_to_outcome(
            KcCallError::LlmUnavailable {
                partial_md: Some("# partial".to_string()),
            },
            "raw",
            &KcSettings::default(),
            "asset-3",
        );
        match outcome {
            KcEnrichmentOutcome::PartialLlmUnavailable { rule_only_md, meta } => {
                assert_eq!(rule_only_md, "# partial");
                assert_eq!(
                    meta.tags_source,
                    KcTagsSource::RuleOnly,
                    "partial 路径 tags_source 必须是 RuleOnly"
                );
                assert!(meta.ai_tags.is_empty(), "partial 路径无 AI 标签");
                assert!(meta.ai_summary.is_none());
                assert!(meta.ai_qa_pairs.is_empty());
                assert_eq!(meta.doc_id, "doc-partial");
                assert_eq!(meta.kc_version, "unknown");
            }
            other => panic!("expected PartialLlmUnavailable, got {other:?}"),
        }
    }

    #[test]
    fn map_call_error_llm_unavailable_without_partial_returns_fallback_internal() {
        // 无 partial_md → 降级为 InternalError（input.md AC-1 步骤 4 显式约定）
        let outcome = map_call_error_to_outcome(
            KcCallError::LlmUnavailable { partial_md: None },
            "raw",
            &KcSettings::default(),
            "asset-4",
        );
        match outcome {
            KcEnrichmentOutcome::Fallback {
                reason: KcFallbackReason::InternalError(detail),
                base_md,
            } => {
                assert!(
                    detail.contains("LLM unavailable"),
                    "detail 应描述 LLM unavailable，实际: {detail}"
                );
                assert!(detail.contains("no partial"));
                assert_eq!(base_md, "raw");
            }
            other => panic!("expected Fallback(InternalError), got {other:?}"),
        }
    }

    #[test]
    fn map_call_error_internal_returns_fallback_internal_with_detail() {
        let outcome = map_call_error_to_outcome(
            KcCallError::Internal {
                detail: "specific detail".to_string(),
                code: "KC_INTERNAL".to_string(),
            },
            "raw",
            &KcSettings::default(),
            "asset-5",
        );
        match outcome {
            KcEnrichmentOutcome::Fallback {
                reason: KcFallbackReason::InternalError(detail),
                ..
            } => {
                assert_eq!(detail, "specific detail", "detail 必须透传");
            }
            other => panic!("expected Fallback(InternalError), got {other:?}"),
        }
    }

    #[test]
    fn map_call_error_input_too_large_returns_fallback_input_too_large() {
        let outcome = map_call_error_to_outcome(
            KcCallError::InputTooLarge,
            "raw",
            &KcSettings::default(),
            "asset-6",
        );
        assert!(matches!(
            outcome,
            KcEnrichmentOutcome::Fallback {
                reason: KcFallbackReason::InputTooLarge,
                ..
            }
        ));
    }

    #[test]
    fn map_call_error_malformed_returns_fallback_malformed() {
        let outcome = map_call_error_to_outcome(
            KcCallError::Malformed {
                reason: "missing field".to_string(),
            },
            "raw",
            &KcSettings::default(),
            "asset-7",
        );
        assert!(matches!(
            outcome,
            KcEnrichmentOutcome::Fallback {
                reason: KcFallbackReason::Malformed,
                ..
            }
        ));
    }

    // =================================================================
    // join_frontmatter_body：拼接边界
    // =================================================================

    #[test]
    fn join_empty_frontmatter_returns_body_only() {
        assert_eq!(join_frontmatter_body("", "# body"), "# body");
    }

    #[test]
    fn join_frontmatter_normalizes_trailing_newlines() {
        // frontmatter 末尾带多个 \n，应被归一化
        let fm = "---\nkey: val\n---\n\n";
        let body = "# body";
        let joined = join_frontmatter_body(fm, body);
        // 不允许 \n\n\n 这种 3 个以上连续换行
        assert!(
            !joined.contains("\n\n\n"),
            "应归一化末尾换行，实际: {joined:?}"
        );
        assert!(joined.contains("---\n\n# body"));
    }

    // =================================================================
    // outcome_to_event_strings：emit payload 字面
    // =================================================================

    #[test]
    fn outcome_to_event_strings_for_all_variants() {
        // Success
        let (ke, fc) = outcome_to_event_strings(&KcEnrichmentOutcome::Success {
            enhanced_md: "x".to_string(),
            meta: make_meta(),
        });
        assert_eq!(ke, "true");
        assert_eq!(fc, None);

        // PartialLlmUnavailable
        let (ke, fc) =
            outcome_to_event_strings(&KcEnrichmentOutcome::PartialLlmUnavailable {
                rule_only_md: "y".to_string(),
                meta: make_meta(),
            });
        assert_eq!(ke, "partial");
        assert_eq!(fc, Some("E_KC_LLM_UNAVAILABLE"));

        // Fallback(Disabled)
        let (ke, fc) = outcome_to_event_strings(&KcEnrichmentOutcome::Fallback {
            reason: KcFallbackReason::Disabled,
            base_md: "z".to_string(),
        });
        assert_eq!(ke, "false");
        assert_eq!(fc, None);

        // Fallback(Timeout)
        let (ke, fc) = outcome_to_event_strings(&KcEnrichmentOutcome::Fallback {
            reason: KcFallbackReason::Timeout,
            base_md: "z".to_string(),
        });
        assert_eq!(ke, "false");
        assert_eq!(fc, Some("E_KC_TIMEOUT"));

        // Fallback(Malformed)
        let (ke, fc) = outcome_to_event_strings(&KcEnrichmentOutcome::Fallback {
            reason: KcFallbackReason::Malformed,
            base_md: "z".to_string(),
        });
        assert_eq!(ke, "false");
        assert_eq!(fc, Some("E_KC_ENRICH_FAILED"));
    }

    // =================================================================
    // synthesize_partial_meta：固定字段
    // =================================================================

    #[test]
    fn synthesize_partial_meta_has_rule_only_tags_source() {
        let meta = synthesize_partial_meta();
        assert_eq!(meta.tags_source, KcTagsSource::RuleOnly);
        assert!(meta.ai_tags.is_empty());
        assert!(meta.rule_tags.is_empty());
        assert!(meta.ai_summary.is_none());
        assert!(meta.ai_qa_pairs.is_empty());
        assert!(meta.ai_paragraph_links.is_empty());
        assert_eq!(meta.doc_id, "doc-partial");
        assert_eq!(meta.kc_version, "unknown");
        assert_eq!(meta.paragraph_count, 0);
    }

    // =================================================================
    // task_025：build_kc_queued_payload 字面对齐前端订阅
    // =================================================================

    /// 守护：`notecapt/asset-kc-queued` payload 必须严格是 `{"assetId": "<id>"}`，
    /// 字段名为 `assetId`（驼峰，与 `notecapt/asset-kc-enriched` 一致），不含其他字段。
    /// 前端 `kcQueueStore` 依此读 `payload.assetId` 维护队列长度。
    #[test]
    fn build_kc_queued_payload_has_correct_shape() {
        let payload = build_kc_queued_payload("asset-xyz");
        assert_eq!(payload["assetId"], serde_json::json!("asset-xyz"));
        // 严格只有一个字段，防止未来无意识扩展 payload 导致前端 schema 漂移
        let obj = payload.as_object().expect("payload should be object");
        assert_eq!(obj.len(), 1, "payload 仅应含 assetId 字段，实际: {payload:?}");
    }

    /// 守护：不同 asset_id 产出独立 payload（不共享内部状态 / 不被缓存）。
    #[test]
    fn build_kc_queued_payload_per_asset_id() {
        let p1 = build_kc_queued_payload("a-1");
        let p2 = build_kc_queued_payload("a-2");
        assert_ne!(p1, p2);
        assert_eq!(p1["assetId"], serde_json::json!("a-1"));
        assert_eq!(p2["assetId"], serde_json::json!("a-2"));
    }

    // =================================================================
    // ResolvedEnrichment Clone（防御性，方便 scheduler 多次复用）
    // =================================================================

    #[test]
    fn resolved_enrichment_is_clonable() {
        let raw = make_raw();
        let outcome = KcEnrichmentOutcome::Success {
            enhanced_md: "x".to_string(),
            meta: make_meta(),
        };
        let resolved = resolve_outcome(&raw, outcome, stub_writer);
        let cloned = resolved.clone();
        assert_eq!(resolved.final_md, cloned.final_md);
        assert_eq!(resolved.kc_enriched, cloned.kc_enriched);
    }
}
