//! task_013：KC frontmatter writer——把 `KcMeta` + `Asset` + `ExtractionResult` 序列化为衍生件
//! `.md` 头部的 YAML frontmatter 块。
//!
//! ## 设计依据
//!
//! - **Architect output.md §"Frontmatter Schema（衍生件 .md 头部）"**：5 NC schema 主键
//!   + 10 KC 扩展字段（kc_doc_id / kc_generated_at / kc_version / kc_tags_source /
//!   kc_enriched / ai_tags / rule_tags / ai_summary / ai_qa_pairs_count / paragraph_count），
//!   **字段顺序固定**，便于跨工具 diff 与 grep。
//! - **task_011 enrichment.rs**：`resolve_outcome` 通过 `frontmatter_writer: impl Fn(&KcMeta) -> String`
//!   注入 builder；scheduler 接入时把本模块的 `build_kc_frontmatter` 适配为闭包传入。
//! - **task_017 前端解析（`src/utils/parseFrontmatter.ts`）**：YAML key 用 **snake_case**
//!   且**严格匹配**前端 whitelist —— 本模块写入的字段名必须与前端 `mapToCamelCase` 函数
//!   接受的 key 一一对应（任何漂移都会导致前端静默丢字段）。
//!
//! ## 字段对照表（与 task_017 前端 snake_case key 一一对齐）
//!
//! | YAML key | 来源 | 类型 | 取值约束 |
//! |--|--|--|--|
//! | `source_asset_id`   | `&Asset.id` | string | UUID 字面 |
//! | `derivative_version`| `&Asset.derivative_version + 1` | number | i32 |
//! | `extracted_at`      | `chrono::Utc::now().to_rfc3339()` | string | ISO 8601 |
//! | `extractor_type`    | 由 `KcMeta.tags_source` 推断 | string | `"markitdown+kc"` / `"markitdown+kc:partial"` |
//! | `quality_level`     | `&ExtractionResult.quality_level` | number | i32 |
//! | `kc_doc_id`         | `&KcMeta.doc_id` | string | KC 端 doc-* |
//! | `kc_generated_at`   | `&KcMeta.generated_at` | string | ISO 8601 |
//! | `kc_version`        | `&KcMeta.kc_version` | string | "0.9" / "unknown"(partial) |
//! | `kc_tags_source`    | `&KcMeta.tags_source.as_str()` | string | `"ai+rule"` / `"rule_only"` |
//! | `kc_enriched`       | 由 tags_source 推断 | string | `"true"` / `"partial"` |
//! | `ai_tags`           | `&KcMeta.ai_tags` | string[] | flow array |
//! | `rule_tags`         | `&KcMeta.rule_tags` | string[] | flow array |
//! | `ai_summary`        | `&KcMeta.ai_summary` | string | block scalar `|` 用于多行 |
//! | `ai_qa_pairs_count` | `&KcMeta.ai_qa_pairs.len()` | number | usize |
//! | `paragraph_count`   | `&KcMeta.paragraph_count` | number | u32 |
//!
//! **不写**：`response_size_bytes` / `duration_ms` / `ai_paragraph_links*` —— 这些是
//! `conversion_meta` DB 列内容（task_015 范畴），不进 frontmatter（前端无对应 schema）。
//!
//! ## 设计决策
//!
//! 1. **手动 YAML 序列化**（不引入 `serde_yaml` 依赖）：
//!    - 字段集合极简（仅 string / number / string[]）+ 字段顺序固定 + 字面值需逐位与
//!      task_017 前端 snake_case 严格对齐——手写比 serde-derive 更可控；
//!    - 避免新增依赖触发 Cargo.lock 全量解析；
//!    - 转义 / block scalar 等 corner 由 5+ 单测显式覆盖。
//! 2. **`kc_enriched` 由 `tags_source` 推断**：
//!    - `AiAndRule` → `"true"`（Success 路径）；
//!    - `RuleOnly`  → `"partial"`（PartialLlmUnavailable 路径）。
//!    - 这与 `enrichment.rs::resolve_outcome` 的 `extractor_type` 字面值约定共享同一隐式映射，
//!      Fallback 路径不会调本 builder（因为 `resolve_outcome` 走 `raw.structured_md` 直出，
//!      不拼 frontmatter），所以**没有 `"false"` 情况**进入本函数。
//! 3. **多行字符串用 YAML block scalar `|`**：
//!    - 当 `ai_summary` 含 `\n` 时——`"`-quoted 风格会显式 escape `\n`，导致前端 js-yaml
//!      JSON_SCHEMA 解析时显示为字面 `\n` 而非真实换行（YAML 1.2 规范明确：JSON_SCHEMA
//!      下双引号串 `\n` 是合法 escape，但 block scalar 更接近"原文本"，对人类阅读更友好）；
//!    - 单行字符串仍用 `"`-quoted 风格（避免空字符串 → 难辨识 / leading space → 解析歧义）。
//! 4. **空数组用 flow style `[]`**：保证 YAML 字面紧凑且与前端 `Array.isArray` 判定兼容。
//! 5. **返回 `---\n<YAML>\n---`**（不含尾部 `\n\n`）：
//!    - 调用方（`enrichment::resolve_outcome` → `join_frontmatter_body`）已经负责拼接
//!      `frontmatter + "\n\n" + body`；本函数不二次拼接尾部空行，避免出现 `---\n\n\n# body`。
//!
//! ## 不变量
//!
//! 1. 字段顺序与 Architect output.md §"Frontmatter Schema" 严格一致（5 NC + 10 KC）；
//! 2. 所有字符串值进 `escape_yaml_string` 或 `block_scalar_string`，禁止任何位置裸拼 `format!("{}: {s}")`；
//! 3. `extractor_type` / `kc_enriched` 字面值与 `enrichment::resolve_outcome` 输出严格 round-trip
//!    （`"markitdown+kc"` / `"markitdown+kc:partial"` / `"true"` / `"partial"`）。
//!
//! ## 测试矩阵（AC-4）
//!
//! 见本文件 `tests` 模块，5 个核心场景 + 5 个边界场景 = 10 个单测。

use crate::extraction::models::ExtractionResult;
use crate::kc::errors::{KcMeta, KcTagsSource};
use crate::models::asset::Asset;

// =====================================================================
// 1. 公开 API：build_kc_frontmatter
// =====================================================================

/// 把 `KcMeta` + `Asset` + `ExtractionResult` 序列化为衍生件 `.md` 头部 YAML frontmatter 块。
///
/// **返回**：`---\n<YAML>\n---` 三行块（无尾部 `\n\n`，由调用方拼接 body）。
///
/// **调用方**：
/// - `enrichment::resolve_outcome` 在 Success / PartialLlmUnavailable 路径调用（task_011 已注入
///   `frontmatter_writer: impl Fn(&KcMeta) -> String`，scheduler 在 task_012 注入时用闭包
///   `|meta| build_kc_frontmatter(asset, raw, meta)` 把三参数压成单参数）。
/// - Fallback 路径**不调**本函数（走 `raw.structured_md` 原版 markitdown MD，无 KC 字段）。
///
/// **字段顺序**：见模块文档表格——5 个 NC schema 主键 + 10 个 KC 扩展字段，
/// 与 Architect output.md §"Frontmatter Schema" 字面对齐。
pub fn build_kc_frontmatter(
    asset: &Asset,
    raw: &ExtractionResult,
    meta: &KcMeta,
) -> String {
    let mut out = String::with_capacity(512);
    out.push_str("---\n");

    // ---- NC schema 主键（5 字段，与 build_frontmatter 同序）----
    push_string_field(&mut out, "source_asset_id", &asset.id);
    push_number_field(&mut out, "derivative_version", i64::from(asset.derivative_version + 1));
    push_string_field(&mut out, "extracted_at", &chrono::Utc::now().to_rfc3339());
    push_string_field(&mut out, "extractor_type", extractor_type_for(meta));
    push_number_field(&mut out, "quality_level", raw.quality_level as i64);

    // ---- KC 扩展字段（10 字段，与 Architect output.md §"Frontmatter Schema" 同序）----
    push_string_field(&mut out, "kc_doc_id", &meta.doc_id);
    push_string_field(&mut out, "kc_generated_at", &meta.generated_at);
    push_string_field(&mut out, "kc_version", &meta.kc_version);
    push_string_field(&mut out, "kc_tags_source", meta.tags_source.as_str());
    push_string_field(&mut out, "kc_enriched", kc_enriched_for(meta));
    push_string_array_field(&mut out, "ai_tags", &meta.ai_tags);
    push_string_array_field(&mut out, "rule_tags", &meta.rule_tags);
    if let Some(summary) = &meta.ai_summary {
        push_summary_field(&mut out, "ai_summary", summary);
    }
    push_number_field(&mut out, "ai_qa_pairs_count", meta.ai_qa_pairs.len() as i64);
    push_number_field(&mut out, "paragraph_count", meta.paragraph_count as i64);

    out.push_str("---");
    out
}

// =====================================================================
// 2. 推断 helpers（与 enrichment::resolve_outcome 共享字面约定）
// =====================================================================

/// `extractor_type` 字段取值：AiAndRule → "markitdown+kc"，RuleOnly → "markitdown+kc:partial"。
///
/// 与 `enrichment::resolve_outcome` 中对应 `ResolvedEnrichment.extractor_type` 字面严格一致。
fn extractor_type_for(meta: &KcMeta) -> &'static str {
    match meta.tags_source {
        KcTagsSource::AiAndRule => "markitdown+kc",
        KcTagsSource::RuleOnly => "markitdown+kc:partial",
    }
}

/// `kc_enriched` 字段取值：AiAndRule → "true"，RuleOnly → "partial"。
///
/// **不变量**：本 builder **不被** Fallback 路径调用（resolve_outcome 在 Fallback 时
/// 直接用 raw.structured_md 不拼 frontmatter），故没有 `"false"` 情况进入。
fn kc_enriched_for(meta: &KcMeta) -> &'static str {
    match meta.tags_source {
        KcTagsSource::AiAndRule => "true",
        KcTagsSource::RuleOnly => "partial",
    }
}

// =====================================================================
// 3. YAML 字段拼装 helpers（手动序列化 + 转义）
// =====================================================================

/// 追加 `<key>: "<escaped string>"\n`。
///
/// 字符串值统一走双引号 escape——简单、稳定、可读；不走 plain style 避免与 YAML
/// 保留字（`yes` / `no` / `null` / `true` 等）冲突。
fn push_string_field(out: &mut String, key: &str, value: &str) {
    out.push_str(key);
    out.push_str(": ");
    push_yaml_quoted_string(out, value);
    out.push('\n');
}

/// 追加 `<key>: <number>\n`（i64 / usize 等数字字面，无需转义）。
fn push_number_field(out: &mut String, key: &str, value: i64) {
    out.push_str(key);
    out.push_str(": ");
    out.push_str(&value.to_string());
    out.push('\n');
}

/// 追加 `<key>: [<v1>, <v2>, ...]\n`（YAML flow array）。
///
/// 空数组 → `[]`；单元素仍走 flow style 保持一致性。元素逐个走双引号 escape。
fn push_string_array_field(out: &mut String, key: &str, values: &[String]) {
    out.push_str(key);
    out.push_str(": [");
    let mut first = true;
    for v in values {
        if !first {
            out.push_str(", ");
        }
        first = false;
        push_yaml_quoted_string(out, v);
    }
    out.push_str("]\n");
}

/// 追加 `<key>:` + block scalar（多行）或 quoted（单行）字符串。
///
/// 多行（含 `\n`）→ block scalar `|` 风格（YAML 1.2 literal block，保留换行）：
/// ```yaml
/// ai_summary: |
///   line 1
///   line 2
/// ```
///
/// 单行 → 双引号 escape 一行搞定：
/// ```yaml
/// ai_summary: "single line summary"
/// ```
fn push_summary_field(out: &mut String, key: &str, value: &str) {
    if value.contains('\n') {
        // block scalar：用 `|` 后跟换行 + 每行 2 空格缩进
        out.push_str(key);
        out.push_str(": |\n");
        for line in value.split('\n') {
            out.push_str("  ");
            out.push_str(line);
            out.push('\n');
        }
    } else {
        push_string_field(out, key, value);
    }
}

/// YAML 双引号字符串 escape（参考 YAML 1.2 spec §5.7 "Escaped Characters"）。
///
/// 转义规则：
/// - `\` → `\\`
/// - `"` → `\"`
/// - `\n` → `\n`（字面）
/// - `\r` → `\r`
/// - `\t` → `\t`
/// - 其他控制字符（U+0000-U+001F 除上述）→ `\xNN`
/// - 其他可打印字符（含中文、`:`、`#`、单引号等）→ 原样
///
/// 双引号策略避免 plain style 与 YAML 保留字冲突；同时简化"何时引用"判定。
fn push_yaml_quoted_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                // 其他 C0 控制字符（U+0000-U+001F 但不在上面 4 个），用 \xNN
                out.push_str(&format!("\\x{:02X}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

// =====================================================================
// 4. 单元测试（AC-4：5 个核心 + 5 个边界）
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extraction::models::ExtractionResult;
    use crate::kc::errors::{KcMeta, KcParagraphLink, KcQaPair, KcTagsSource};
    use crate::models::asset::Asset;

    // ---------- 辅助构造器 ----------

    fn make_asset() -> Asset {
        Asset {
            id: "asset-001".to_string(),
            project_id: "proj-A".to_string(),
            asset_type: "pdf".to_string(),
            name: "doc.pdf".to_string(),
            original_name: "doc.pdf".to_string(),
            file_path: "/tmp/doc.pdf".to_string(),
            file_size: 1024,
            mime_type: "application/pdf".to_string(),
            captured_at: "2026-05-27T00:00:00Z".to_string(),
            imported_at: "2026-05-27T00:00:00Z".to_string(),
            source_type: "manual".to_string(),
            source_data: None,
            is_starred: false,
            source_asset_id: None,
            derivative_version: 2, // 期望 frontmatter 写 3（+1）
        }
    }

    fn make_raw(quality: i32) -> ExtractionResult {
        ExtractionResult {
            raw_text: "raw".to_string(),
            structured_md: "# md".to_string(),
            quality_level: quality,
            extractor_type: "markitdown".to_string(),
            segments: Vec::new(),
            needs_ocr_fallback: false,
        }
    }

    fn make_full_meta() -> KcMeta {
        KcMeta {
            doc_id: "doc-abc12345".to_string(),
            kc_version: "0.9".to_string(),
            tags_source: KcTagsSource::AiAndRule,
            ai_tags: vec!["AI".to_string(), "机器学习".to_string(), "深度学习".to_string()],
            rule_tags: vec!["AI".to_string(), "ML".to_string()],
            ai_summary: Some("本文介绍了人工智能的基本概念".to_string()),
            ai_qa_pairs: vec![
                KcQaPair { question: "Q1".to_string(), answer: "A1".to_string() },
                KcQaPair { question: "Q2".to_string(), answer: "A2".to_string() },
                KcQaPair { question: "Q3".to_string(), answer: "A3".to_string() },
            ],
            ai_paragraph_links: vec![KcParagraphLink {
                paragraph_id: "paragraph-0".to_string(),
                related_text: "rel".to_string(),
            }],
            generated_at: "2026-05-27T07:59:50Z".to_string(),
            paragraph_count: 7,
            response_size_bytes: 4096,
            duration_ms: 1200,
        }
    }

    // =================================================================
    // AC-4 核心测试 1：所有字段都有 → 完整 frontmatter
    // =================================================================

    #[test]
    fn build_kc_frontmatter_success_full_meta() {
        let asset = make_asset();
        let raw = make_raw(3);
        let meta = make_full_meta();

        let fm = build_kc_frontmatter(&asset, &raw, &meta);

        // 块结构
        assert!(fm.starts_with("---\n"), "frontmatter 必须以 ---\\n 开头");
        assert!(fm.ends_with("---"), "frontmatter 必须以 --- 结尾（无尾部 \\n\\n）");

        // 5 个 NC schema 主键
        assert!(fm.contains("source_asset_id: \"asset-001\"\n"));
        assert!(
            fm.contains("derivative_version: 3\n"),
            "derivative_version 应是 asset.derivative_version + 1 = 3"
        );
        assert!(fm.contains("extracted_at: \""), "extracted_at 应有值（动态时间戳，仅断言前缀）");
        assert!(fm.contains("extractor_type: \"markitdown+kc\"\n"));
        assert!(fm.contains("quality_level: 3\n"));

        // 10 个 KC 扩展字段
        assert!(fm.contains("kc_doc_id: \"doc-abc12345\"\n"));
        assert!(fm.contains("kc_generated_at: \"2026-05-27T07:59:50Z\"\n"));
        assert!(fm.contains("kc_version: \"0.9\"\n"));
        assert!(fm.contains("kc_tags_source: \"ai+rule\"\n"));
        assert!(fm.contains("kc_enriched: \"true\"\n"));
        assert!(
            fm.contains("ai_tags: [\"AI\", \"机器学习\", \"深度学习\"]\n"),
            "ai_tags 应为 flow array 且含中文原样，实际: {fm}"
        );
        assert!(fm.contains("rule_tags: [\"AI\", \"ML\"]\n"));
        assert!(fm.contains("ai_summary: \"本文介绍了人工智能的基本概念\"\n"));
        assert!(fm.contains("ai_qa_pairs_count: 3\n"));
        assert!(fm.contains("paragraph_count: 7\n"));

        // 字段顺序：NC 5 个在 KC 10 个之前
        let pos_quality = fm.find("quality_level").unwrap();
        let pos_kc_doc = fm.find("kc_doc_id").unwrap();
        assert!(pos_quality < pos_kc_doc, "NC schema 应排在 KC 扩展之前");
    }

    // =================================================================
    // AC-4 核心测试 2：RuleOnly 模式只有 rule_tags，无 ai_summary
    // =================================================================

    #[test]
    fn build_kc_frontmatter_partial_no_ai_summary() {
        let asset = make_asset();
        let raw = make_raw(2);
        let meta = KcMeta {
            doc_id: "doc-partial".to_string(),
            kc_version: "unknown".to_string(),
            tags_source: KcTagsSource::RuleOnly,
            ai_tags: Vec::new(),
            rule_tags: vec!["AI".to_string(), "ML".to_string()],
            ai_summary: None, // partial 路径无摘要
            ai_qa_pairs: Vec::new(),
            ai_paragraph_links: Vec::new(),
            generated_at: String::new(),
            paragraph_count: 0,
            response_size_bytes: 0,
            duration_ms: 0,
        };

        let fm = build_kc_frontmatter(&asset, &raw, &meta);

        // RuleOnly → kc_enriched = "partial" + extractor_type = "markitdown+kc:partial"
        assert!(
            fm.contains("extractor_type: \"markitdown+kc:partial\"\n"),
            "RuleOnly tags_source 应映射到 markitdown+kc:partial，实际: {fm}"
        );
        assert!(fm.contains("kc_enriched: \"partial\"\n"));
        assert!(fm.contains("kc_tags_source: \"rule_only\"\n"));

        // ai_summary 字段完全不出现（Option::None 时跳过）
        assert!(
            !fm.contains("ai_summary"),
            "ai_summary = None 时不应出现该字段，实际: {fm}"
        );

        // 空数组 ai_tags → []
        assert!(
            fm.contains("ai_tags: []\n"),
            "空 ai_tags 应序列化为 []，实际: {fm}"
        );

        // rule_tags 有值
        assert!(fm.contains("rule_tags: [\"AI\", \"ML\"]\n"));

        // 计数字段
        assert!(fm.contains("ai_qa_pairs_count: 0\n"));
        assert!(fm.contains("paragraph_count: 0\n"));
    }

    // =================================================================
    // AC-4 核心测试 3：标题/摘要含 YAML 特殊字符（:, #, ", ', \）
    // =================================================================

    #[test]
    fn build_kc_frontmatter_escapes_special_chars() {
        let asset = make_asset();
        let raw = make_raw(3);
        let meta = KcMeta {
            doc_id: "doc-special".to_string(),
            kc_version: "0.9".to_string(),
            tags_source: KcTagsSource::AiAndRule,
            // tag 含 `:` 和 `#`，应保留原字符（双引号 quoted 模式下不会被 YAML 误解为映射或注释）
            ai_tags: vec![
                "tag: with colon".to_string(),
                "tag #with hash".to_string(),
                "tag 'with quote".to_string(),
            ],
            rule_tags: vec!["tag \"with double quote".to_string(), "tag\\with backslash".to_string()],
            ai_summary: Some(
                "Summary with: colon, # hash, \"double quote\", 'single quote', and \\ backslash"
                    .to_string(),
            ),
            ai_qa_pairs: Vec::new(),
            ai_paragraph_links: Vec::new(),
            generated_at: "2026-05-27T00:00:00Z".to_string(),
            paragraph_count: 1,
            response_size_bytes: 0,
            duration_ms: 0,
        };

        let fm = build_kc_frontmatter(&asset, &raw, &meta);

        // ai_tags 含 `:` `#` `'` 原样（双引号包裹下安全）
        assert!(
            fm.contains("\"tag: with colon\""),
            "tag 含 : 应原样保留（双引号包裹），实际: {fm}"
        );
        assert!(
            fm.contains("\"tag #with hash\""),
            "tag 含 # 应原样保留，实际: {fm}"
        );
        assert!(
            fm.contains("\"tag 'with quote\""),
            "tag 含 ' 应原样保留（双引号包裹下无需 escape），实际: {fm}"
        );

        // rule_tags：` " ` 应被 escape 为 `\"`；`\` 应被 escape 为 `\\`
        assert!(
            fm.contains("\"tag \\\"with double quote\""),
            "tag 含双引号应被 escape 为 \\\"，实际: {fm}"
        );
        assert!(
            fm.contains("\"tag\\\\with backslash\""),
            "tag 含反斜杠应被 escape 为 \\\\，实际: {fm}"
        );

        // ai_summary：相同 escape 规则适用
        assert!(
            fm.contains("\\\"double quote\\\""),
            "summary 含双引号应被 escape，实际: {fm}"
        );
        assert!(
            fm.contains("\\\\ backslash"),
            "summary 含反斜杠应被 escape 为 \\\\，实际: {fm}"
        );

        // 验证 YAML 解析后能 round-trip（粗略：不存在裸 `: ` 在 tag 值内导致歧义）
        // 这一行作为冗余守护——任何 escape bug 都会破坏 frontmatter 头部 `---\n` 结构
        assert!(fm.starts_with("---\n"));
        assert!(fm.ends_with("---"));
    }

    // =================================================================
    // AC-4 核心测试 4：多行 ai_summary 用 block scalar `|`
    // =================================================================

    #[test]
    fn build_kc_frontmatter_multiline_summary_uses_block_scalar() {
        let asset = make_asset();
        let raw = make_raw(3);
        let meta = KcMeta {
            doc_id: "doc-multiline".to_string(),
            kc_version: "0.9".to_string(),
            tags_source: KcTagsSource::AiAndRule,
            ai_tags: vec!["X".to_string()],
            rule_tags: Vec::new(),
            ai_summary: Some("第一行\n第二行\n第三行".to_string()),
            ai_qa_pairs: Vec::new(),
            ai_paragraph_links: Vec::new(),
            generated_at: "2026-05-27T00:00:00Z".to_string(),
            paragraph_count: 1,
            response_size_bytes: 0,
            duration_ms: 0,
        };

        let fm = build_kc_frontmatter(&asset, &raw, &meta);

        // 多行 summary 应走 block scalar，而**不**走双引号 escape 风格
        assert!(
            fm.contains("ai_summary: |\n"),
            "多行 summary 应触发 block scalar |，实际: {fm}"
        );
        assert!(
            fm.contains("  第一行\n  第二行\n  第三行\n"),
            "block scalar 每行应有 2 空格缩进，实际: {fm}"
        );
        // 反例：不应出现 `\n` 字面 escape
        assert!(
            !fm.contains("ai_summary: \"第一行\\n"),
            "多行 summary 不应走双引号 \\n escape，实际: {fm}"
        );
    }

    // =================================================================
    // AC-4 核心测试 5：空数组 ai_tags / rule_tags
    // =================================================================

    #[test]
    fn build_kc_frontmatter_empty_arrays_serialize_as_empty_list() {
        let asset = make_asset();
        let raw = make_raw(1);
        let meta = KcMeta {
            doc_id: "doc-empty".to_string(),
            kc_version: "0.9".to_string(),
            tags_source: KcTagsSource::AiAndRule,
            ai_tags: Vec::new(),
            rule_tags: Vec::new(),
            ai_summary: None,
            ai_qa_pairs: Vec::new(),
            ai_paragraph_links: Vec::new(),
            generated_at: "2026-05-27T00:00:00Z".to_string(),
            paragraph_count: 0,
            response_size_bytes: 0,
            duration_ms: 0,
        };

        let fm = build_kc_frontmatter(&asset, &raw, &meta);

        assert!(
            fm.contains("ai_tags: []\n"),
            "空 ai_tags 应序列化为 []，实际: {fm}"
        );
        assert!(
            fm.contains("rule_tags: []\n"),
            "空 rule_tags 应序列化为 []，实际: {fm}"
        );
        // 与 task_017 前端 `Array.isArray(raw.ai_tags)` 判定兼容（js-yaml 解析 [] 为 []）
        // 同时 ai_summary = None 时不应出现该字段
        assert!(!fm.contains("ai_summary"));
    }

    // =================================================================
    // 边界测试 6：ai_tags 含 emoji / 高 Unicode 码点
    // =================================================================

    #[test]
    fn build_kc_frontmatter_handles_emoji_and_high_unicode() {
        let asset = make_asset();
        let raw = make_raw(3);
        let mut meta = make_full_meta();
        meta.ai_tags = vec!["🎉 庆祝".to_string(), "✨".to_string()];
        meta.ai_summary = Some("含 emoji 的摘要 🚀".to_string());

        let fm = build_kc_frontmatter(&asset, &raw, &meta);

        // emoji 应原样保留（不走 \xNN escape——它们是高码点，不在 C0 控制字符范围）
        assert!(
            fm.contains("\"🎉 庆祝\""),
            "emoji 应原样保留，实际: {fm}"
        );
        assert!(fm.contains("\"✨\""));
        assert!(fm.contains("含 emoji 的摘要 🚀"));
    }

    // =================================================================
    // 边界测试 7：summary 含 \r\n（CRLF）—— 也应触发 block scalar
    // =================================================================

    #[test]
    fn build_kc_frontmatter_crlf_summary_uses_block_scalar() {
        let asset = make_asset();
        let raw = make_raw(3);
        let mut meta = make_full_meta();
        meta.ai_summary = Some("第一行\r\n第二行".to_string());

        let fm = build_kc_frontmatter(&asset, &raw, &meta);

        // 当前实现只检测 `\n`——CRLF 包含 `\n` 也会走 block scalar
        assert!(
            fm.contains("ai_summary: |\n"),
            "CRLF summary 也应触发 block scalar（基于 \\n 检测），实际: {fm}"
        );
    }

    // =================================================================
    // 边界测试 8：source_asset_id 含 `:` 等特殊字符
    // =================================================================

    #[test]
    fn build_kc_frontmatter_escapes_asset_id_special_chars() {
        let mut asset = make_asset();
        asset.id = "asset:with:colon".to_string();
        let raw = make_raw(3);
        let meta = make_full_meta();

        let fm = build_kc_frontmatter(&asset, &raw, &meta);

        assert!(
            fm.contains("source_asset_id: \"asset:with:colon\"\n"),
            "asset.id 含冒号应被双引号包裹，实际: {fm}"
        );
    }

    // =================================================================
    // 边界测试 9：derivative_version + 1 进位（i32 边界）
    // =================================================================

    #[test]
    fn build_kc_frontmatter_derivative_version_incremented() {
        let mut asset = make_asset();
        asset.derivative_version = 99;
        let raw = make_raw(3);
        let meta = make_full_meta();

        let fm = build_kc_frontmatter(&asset, &raw, &meta);
        assert!(
            fm.contains("derivative_version: 100\n"),
            "version 应是 99 + 1 = 100，实际: {fm}"
        );
    }

    // =================================================================
    // 边界测试 10：summary 内含 tab / control char → 双引号 escape
    // =================================================================

    #[test]
    fn build_kc_frontmatter_escapes_tab_and_control_chars_in_single_line_summary() {
        let asset = make_asset();
        let raw = make_raw(3);
        let mut meta = make_full_meta();
        // 单行（无 \n），含 tab + bell（U+0007）
        meta.ai_summary = Some("foo\tbar\u{0007}baz".to_string());

        let fm = build_kc_frontmatter(&asset, &raw, &meta);

        assert!(
            fm.contains("ai_summary: \"foo\\tbar\\x07baz\"\n"),
            "tab 应 escape 为 \\t，bell 应 escape 为 \\x07，实际: {fm}"
        );
    }

    // =================================================================
    // 不变量守护测试 11：字段顺序严格固定（防 IDE 自动排序）
    // =================================================================

    #[test]
    fn build_kc_frontmatter_field_order_is_stable() {
        let asset = make_asset();
        let raw = make_raw(3);
        let meta = make_full_meta();
        let fm = build_kc_frontmatter(&asset, &raw, &meta);

        // 提取所有 `key: ` 形式的字段名（按出现顺序）
        let expected_order = [
            "source_asset_id",
            "derivative_version",
            "extracted_at",
            "extractor_type",
            "quality_level",
            "kc_doc_id",
            "kc_generated_at",
            "kc_version",
            "kc_tags_source",
            "kc_enriched",
            "ai_tags",
            "rule_tags",
            "ai_summary",
            "ai_qa_pairs_count",
            "paragraph_count",
        ];
        let mut last_pos: usize = 0;
        for key in expected_order {
            let needle = format!("\n{key}: ");
            // 头部第一个字段 source_asset_id 前是 `---\n`，所以 `\n<key>: ` 都能找到
            let pos = fm.find(&needle).unwrap_or_else(|| {
                panic!("字段 {key} 缺失，frontmatter: {fm}");
            });
            assert!(
                pos >= last_pos,
                "字段 {key} 顺序错误（pos={pos} < last_pos={last_pos}），frontmatter: {fm}"
            );
            last_pos = pos;
        }
    }
}
