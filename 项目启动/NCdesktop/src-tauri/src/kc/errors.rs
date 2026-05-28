//! task_005：KC 模块错误与结果类型骨架。
//!
//! 本文件落盘的是"类型契约"，而非业务逻辑：
//! - `KcCallError`：HTTP 客户端层错误（task_007 `kc::client` 调 KC 后的错误分类）；
//! - `KcEnrichmentOutcome`：enrichment step 对外的三态结果（task_011 `kc::enrichment::enrich` 的返回类型）；
//! - `KcFallbackReason`：fallback 分支的成因细分（决定 conversion_meta.failure_code 列写哪个）；
//! - `KcMeta` / `KcQaPair` / `KcParagraphLink` / `KcTagsSource`：从 KC HTTP 响应反序列化后的领域模型，
//!   也是写 frontmatter 与 conversion_meta 的数据源（task_013 / task_015 消费）。
//!
//! **架构依据**：
//! - ADR-002：HTTP 客户端错误分类（5 类失败映射到 `KcCallError` 变体）；
//! - ADR-004：5 类失败兜底状态机（`KcEnrichmentOutcome` + `KcFallbackReason`）；
//! - Architect output.md §"KcMeta 结构"：11 字段定义；
//! - task_003 已在 `extraction::failure_code::FailureCode` 中追加 5 个 `EKc*` 变体，
//!   本文件提供 `KcCallError::to_failure_code()` 单向映射占位，保证后续 task_011 enrichment
//!   能把客户端错误码翻译成 `failure_code` 列字面值。
//!
//! **本 task 范围**：
//! - 全部 `pub` 类型 + 字段 + `From<&KcCallError> for FailureCode` 映射 + 1 个单测验证映射不漏；
//! - 不实装 HTTP 解析 / enrichment 调度 / DB 写入逻辑（这些由依赖本类型的下游 task 承接）；
//! - `KcMeta` / `KcQaPair` 等不加 `serde::Deserialize`（task_007 实装客户端时再决定是否要 serde 反序列化层）。
//!
//! **不变量**：5 个 `KcCallError` 变体与 5 个 `FailureCode::EKc*` 变体严格一一对应；
//! `KcCallError::Malformed` 与 KC 200 但 `enhanced_markdown` 缺失同义，归到 `EKcEnrichFailed`（ADR-004 表）。

use crate::extraction::failure_code::FailureCode;

// =====================================================================
// 1. HTTP 客户端层错误（task_007 `kc::client` 实装时返回此类型）
// =====================================================================

/// KC HTTP 调用错误（5 类失败分类 + 1 类响应解析失败）。
///
/// 与 ADR-002 §"错误分类"表对应：
/// - HTTP 连不上 / DNS / 端口拒绝 → `Unreachable`；
/// - 60s 客户端超时 → `Timeout`；
/// - 500 + body 含 `KC_LLM_UNAVAILABLE` → `LlmUnavailable { partial_md }`；
/// - 500 + body 含 `KC_INTERNAL / KC_PARSE_ERROR / KC_OUTPUT_ERROR` → `Internal { detail, code }`；
/// - 500 + body 含 `KC_INPUT_TOO_LARGE` → `InputTooLarge`；
/// - 200 但 `enhanced_markdown` 字段缺失（KC-MOD-1 未到位）→ `Malformed { reason }`。
///
/// 注意：`LlmUnavailable.partial_md` 是 KC-MOD-3 "类型 C 用"的可选 fallback 文本
/// （规则标签 + 锚点 + 索引段，无 AI 标签 / 问答对）。
#[derive(Debug, Clone)]
pub enum KcCallError {
    /// KC 子进程不可达（HTTP connect 失败、端口拒绝、健康检查 fail）。
    Unreachable,
    /// 60s 客户端总超时（reqwest 层超时）。
    Timeout,
    /// KC 返回 `KC_LLM_UNAVAILABLE`：智谱 / OpenAI 不可达，可降级到规则增强 MD。
    LlmUnavailable {
        /// KC-MOD-3 提供的 `partial_enhanced_markdown`（规则标签 + 锚点 + 索引段），
        /// 缺失时为 None（视为完全 fallback 到 markitdown 原 MD）。
        partial_md: Option<String>,
    },
    /// KC 内部错误（`KC_INTERNAL` / `KC_PARSE_ERROR` / `KC_OUTPUT_ERROR` 三合一）。
    Internal {
        /// 透传 KC 返回的 `detail.message`，仅供日志 / debugging。
        detail: String,
        /// KC 端 `error_code` 原值（`KC_INTERNAL` 等），用于日志精确分类。
        code: String,
    },
    /// KC 返回 `KC_INPUT_TOO_LARGE`：输入 markdown 超过 KC 内部限制。
    InputTooLarge,
    /// KC 返回 200 但响应体 `enhanced_markdown` 字段缺失或格式不符（KC-MOD-1 未到位）。
    Malformed {
        /// 解析失败的具体原因（用于日志，例如 "missing field `enhanced_markdown`"）。
        reason: String,
    },
}

impl KcCallError {
    /// 单向映射：客户端错误 → `extraction::failure_code::FailureCode`。
    ///
    /// 映射规则来自 ADR-004 §"5 类失败映射"表：
    /// | KcCallError 变体 | FailureCode |
    /// |--|--|
    /// | `Unreachable`            | `EKcUnavailable`     |
    /// | `Timeout`                | `EKcTimeout`         |
    /// | `LlmUnavailable {..}`    | `EKcLlmUnavailable`  |
    /// | `Internal {..}`          | `EKcEnrichFailed`    |
    /// | `InputTooLarge`          | `EKcInputTooLarge`   |
    /// | `Malformed {..}`         | `EKcEnrichFailed`    |
    ///
    /// **设计**：取 `&self` 而非 `self`，调用方不需要 move 错误对象就能取 code
    /// （task_011 enrichment 在 log + DB 写入两处都要消费同一份错误）。
    pub fn to_failure_code(&self) -> FailureCode {
        match self {
            KcCallError::Unreachable => FailureCode::EKcUnavailable,
            KcCallError::Timeout => FailureCode::EKcTimeout,
            KcCallError::LlmUnavailable { .. } => FailureCode::EKcLlmUnavailable,
            KcCallError::Internal { .. } => FailureCode::EKcEnrichFailed,
            KcCallError::InputTooLarge => FailureCode::EKcInputTooLarge,
            KcCallError::Malformed { .. } => FailureCode::EKcEnrichFailed,
        }
    }
}

// =====================================================================
// 2. enrichment step 三态结果（task_011 `kc::enrichment::enrich` 返回此类型）
// =====================================================================

/// KC enrichment step 对外结果（ADR-004 §"5 类失败兜底状态机"）。
///
/// 决定 scheduler 落地哪种 MD + 写哪种 `extractor_type` + `kc_enriched` 标记：
/// - `Success`              → `extractor_type = "markitdown+kc"`、`kc_enriched = "true"`、落地 enhanced_md；
/// - `PartialLlmUnavailable`→ `extractor_type = "markitdown+kc:partial"`、`kc_enriched = "partial"`、落地 rule_only_md；
/// - `Fallback`             → `extractor_type = "markitdown"`、`kc_enriched = "false"`、落地 base_md（markitdown 原版）。
#[derive(Debug, Clone)]
pub enum KcEnrichmentOutcome {
    /// KC 调用成功，拿到完整增强 MD（含 AI 标签 + 摘要 + 问答对）。
    Success {
        /// KC 返回的完整 v6 增强 markdown（不含 NC frontmatter，由 task_013 拼接）。
        enhanced_md: String,
        /// 元数据（供 frontmatter 写入 + conversion_meta 写入消费）。
        meta: KcMeta,
    },
    /// KC LLM 不可用，但拿到了规则增强 MD（KC-MOD-3 类型 C）。
    PartialLlmUnavailable {
        /// 仅含规则标签 + 锚点 + 索引段（无 AI 标签 / 摘要 / 问答对）。
        rule_only_md: String,
        /// 元数据：`tags_source = KcTagsSource::RuleOnly`，`ai_summary = None`。
        meta: KcMeta,
    },
    /// 完全 fallback 到 markitdown 原 MD（无 KC 增强）。
    Fallback {
        /// 成因（决定写哪个 `failure_code` 到 conversion_meta）。
        reason: KcFallbackReason,
        /// markitdown 原 MD（即 enrichment step 的输入 raw_md）。
        base_md: String,
    },
}

/// Fallback 成因分类（ADR-004 §"5 类失败映射"）。
///
/// 决定 `conversion_meta.failure_code` 列字面值（None → 不写）：
/// - `Disabled`             → 不写 failure_code（用户主动关 KC，非失败）；
/// - `Unavailable`          → `E_KC_UNAVAILABLE`；
/// - `Timeout`              → `E_KC_TIMEOUT`；
/// - `InternalError(_)`     → `E_KC_ENRICH_FAILED`（detail 字符串仅用于日志）；
/// - `InputTooLarge`        → `E_KC_INPUT_TOO_LARGE`；
/// - `Malformed`            → `E_KC_ENRICH_FAILED`（与 InternalError 同 code，区分仅在日志层）。
#[derive(Debug, Clone)]
pub enum KcFallbackReason {
    /// 用户通过 `kc.enabled = false` 主动关闭 KC（不是失败）。
    Disabled,
    /// KC 子进程不可达（healthcheck 失败 / 端口拒绝 / 进程未起）。
    Unavailable,
    /// 客户端 60s 超时。
    Timeout,
    /// KC 500 + `KC_INTERNAL` / `KC_PARSE_ERROR` / `KC_OUTPUT_ERROR`，detail 透传到日志。
    InternalError(String),
    /// KC 500 + `KC_INPUT_TOO_LARGE`。
    InputTooLarge,
    /// KC 200 但响应体 malformed（KC-MOD-1 未到位）。
    Malformed,
}

impl KcFallbackReason {
    /// 映射到 `failure_code` 列字面值。
    ///
    /// 返回 `None` 表示不写 `failure_code` 列（仅 `Disabled` 分支）。
    /// 与 ADR-004 §"5 类失败映射"表的 failure_code 列严格一致。
    pub fn to_failure_code(&self) -> Option<FailureCode> {
        match self {
            KcFallbackReason::Disabled => None,
            KcFallbackReason::Unavailable => Some(FailureCode::EKcUnavailable),
            KcFallbackReason::Timeout => Some(FailureCode::EKcTimeout),
            KcFallbackReason::InternalError(_) => Some(FailureCode::EKcEnrichFailed),
            KcFallbackReason::InputTooLarge => Some(FailureCode::EKcInputTooLarge),
            KcFallbackReason::Malformed => Some(FailureCode::EKcEnrichFailed),
        }
    }
}

// =====================================================================
// 3. KC 元数据领域模型（Architect output.md §"KcMeta 结构"，11 字段）
// =====================================================================

/// KC 标签来源（决定 frontmatter `kc_tags_source` 字段值）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KcTagsSource {
    /// AI 标签 + 规则标签合并（KC 正常成功路径）。
    AiAndRule,
    /// 仅规则标签（KC LLM 不可用 → `PartialLlmUnavailable` 路径）。
    RuleOnly,
}

impl KcTagsSource {
    /// 序列化为 frontmatter / DB 中的字面值。
    pub fn as_str(&self) -> &'static str {
        match self {
            KcTagsSource::AiAndRule => "ai+rule",
            KcTagsSource::RuleOnly => "rule_only",
        }
    }
}

/// KC AI 问答对（KC 返回的 `ai_qa_pairs` 数组元素）。
///
/// 详细内容写在 .md 正文 `## 问答增强` 节；frontmatter 仅存 `ai_qa_pairs_count`。
#[derive(Debug, Clone)]
pub struct KcQaPair {
    /// 问题文本。
    pub question: String,
    /// 答案文本。
    pub answer: String,
}

/// KC AI 段落关联（KC 返回的 `ai_paragraph_links` 数组元素）。
///
/// 详细内容写在 .md 正文 `## 段落关联` 节。
#[derive(Debug, Clone)]
pub struct KcParagraphLink {
    /// 段落锚点 ID（如 `paragraph-0`）。
    pub paragraph_id: String,
    /// 关联文本（关键句 / 摘要）。
    pub related_text: String,
}

/// KC 元数据完整结构（Architect output.md §"KcMeta 结构"，**11 字段**）。
///
/// 来源：KC `/api/v1/ingest` 200 响应 + 客户端层注入的 `response_size_bytes` / `duration_ms`。
///
/// 消费方：
/// - frontmatter writer（task_013）：把 9 个 KC 字段写入衍生件 .md 头部；
/// - DB writer（task_015）：把 `doc_id` / `response_size_bytes` / `duration_ms` 写入 `conversion_meta`，
///   把 `kc_version` / `tags_source` 写入 `extracted_content.kc_*` 列。
#[derive(Debug, Clone)]
pub struct KcMeta {
    /// KC 端为本次 ingest 生成的 doc_id（如 `doc-abc12345`），写 `conversion_meta.kc_doc_id`。
    pub doc_id: String,
    /// KC 版本字符串（如 `"0.9"`），写 `extracted_content.kc_version`。
    pub kc_version: String,
    /// 标签来源（AI+规则 / 仅规则），写 `extracted_content.kc_tags_source`。
    pub tags_source: KcTagsSource,
    /// AI 生成的标签数组（写 frontmatter `ai_tags`）。
    pub ai_tags: Vec<String>,
    /// 规则提取的标签数组（写 frontmatter `rule_tags`）。
    pub rule_tags: Vec<String>,
    /// AI 摘要（写 frontmatter `ai_summary`，`RuleOnly` 路径下为 None）。
    pub ai_summary: Option<String>,
    /// AI 问答对（写 .md 正文 `## 问答增强` 节，frontmatter 仅存计数）。
    pub ai_qa_pairs: Vec<KcQaPair>,
    /// AI 段落关联（写 .md 正文 `## 段落关联` 节）。
    pub ai_paragraph_links: Vec<KcParagraphLink>,
    /// KC 端生成时间戳（ISO 8601 字符串），写 frontmatter `kc_generated_at`。
    pub generated_at: String,
    /// 段落总数（写 frontmatter `paragraph_count`）。
    pub paragraph_count: u32,
    /// HTTP 响应体字节数（客户端注入），写 `conversion_meta.kc_response_size`。
    pub response_size_bytes: usize,
    /// 客户端 ingest 调用耗时（毫秒，客户端注入），写 `conversion_meta.kc_duration_ms`。
    pub duration_ms: u64,
}

// =====================================================================
// 4. 单测：保证 5 类失败映射不漂移（不触碰任何 unimplemented! 的逻辑）
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 6 个 `KcCallError` 变体 → 5 个 `FailureCode::EKc*`（Internal/Malformed 同映射）。
    /// 出错时立刻能看到是哪个变体漏配，防止后续 task_011 enrichment 静默漏码。
    #[test]
    fn kc_call_error_maps_to_failure_code() {
        assert_eq!(KcCallError::Unreachable.to_failure_code(), FailureCode::EKcUnavailable);
        assert_eq!(KcCallError::Timeout.to_failure_code(), FailureCode::EKcTimeout);
        assert_eq!(
            KcCallError::LlmUnavailable { partial_md: None }.to_failure_code(),
            FailureCode::EKcLlmUnavailable,
        );
        assert_eq!(
            KcCallError::Internal { detail: "x".into(), code: "KC_INTERNAL".into() }
                .to_failure_code(),
            FailureCode::EKcEnrichFailed,
        );
        assert_eq!(KcCallError::InputTooLarge.to_failure_code(), FailureCode::EKcInputTooLarge);
        assert_eq!(
            KcCallError::Malformed { reason: "missing field".into() }.to_failure_code(),
            FailureCode::EKcEnrichFailed,
        );
    }

    /// `KcFallbackReason::Disabled` 不写 failure_code（None）；其他 5 个映射到对应 `EKc*`。
    #[test]
    fn kc_fallback_reason_maps_to_failure_code() {
        assert_eq!(KcFallbackReason::Disabled.to_failure_code(), None);
        assert_eq!(
            KcFallbackReason::Unavailable.to_failure_code(),
            Some(FailureCode::EKcUnavailable),
        );
        assert_eq!(
            KcFallbackReason::Timeout.to_failure_code(),
            Some(FailureCode::EKcTimeout),
        );
        assert_eq!(
            KcFallbackReason::InternalError("x".into()).to_failure_code(),
            Some(FailureCode::EKcEnrichFailed),
        );
        assert_eq!(
            KcFallbackReason::InputTooLarge.to_failure_code(),
            Some(FailureCode::EKcInputTooLarge),
        );
        assert_eq!(
            KcFallbackReason::Malformed.to_failure_code(),
            Some(FailureCode::EKcEnrichFailed),
        );
    }

    /// `KcTagsSource::as_str` 字面值与 frontmatter / DB 列写入值一致（ADR-005 §"v18"）。
    #[test]
    fn kc_tags_source_as_str() {
        assert_eq!(KcTagsSource::AiAndRule.as_str(), "ai+rule");
        assert_eq!(KcTagsSource::RuleOnly.as_str(), "rule_only");
    }

    /// `KcEnrichmentOutcome` 3 个变体可构造（保证 `KcMeta` 字段类型自洽）。
    /// 编译过即视为 PASS（运行时不消费 unimplemented! 的下游函数）。
    #[test]
    fn kc_enrichment_outcome_variants_constructible() {
        let meta = KcMeta {
            doc_id: "doc-test".into(),
            kc_version: "0.9".into(),
            tags_source: KcTagsSource::AiAndRule,
            ai_tags: vec!["AI".into()],
            rule_tags: vec!["ML".into()],
            ai_summary: Some("summary".into()),
            ai_qa_pairs: vec![KcQaPair { question: "Q".into(), answer: "A".into() }],
            ai_paragraph_links: vec![KcParagraphLink {
                paragraph_id: "paragraph-0".into(),
                related_text: "rel".into(),
            }],
            generated_at: "2026-05-27T00:00:00Z".into(),
            paragraph_count: 1,
            response_size_bytes: 1024,
            duration_ms: 500,
        };
        let _success = KcEnrichmentOutcome::Success {
            enhanced_md: "# x".into(),
            meta: meta.clone(),
        };
        let _partial = KcEnrichmentOutcome::PartialLlmUnavailable {
            rule_only_md: "# y".into(),
            meta: meta.clone(),
        };
        let _fallback = KcEnrichmentOutcome::Fallback {
            reason: KcFallbackReason::Unavailable,
            base_md: "# z".into(),
        };
    }
}
