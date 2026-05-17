//! 用户自定义 Prompt 运行时层（custom_prompt_v1 / task_003）。
//!
//! 本模块在 task_002 的"数据 + IPC"基础上，叠加：
//! - 内置默认 Prompt 文本暴露（4 module → 4 常量；`classify` 通过 tagging + para 两段合成）
//! - 运行时合并（`runtime_prompt_for` 读 DB；`is_custom=1` 用用户文本，否则回退默认）
//! - 三层防御中的 Layer A（输出格式硬守卫永远 system 压底） + Layer B（保存时占位符校验）
//!   见 Architect output.md § ADR-003
//! - 双层字节/字符校验（保存 16 KiB / 调用前 64 KiB 字符）
//!   见 Architect output.md § ADR-004
//! - 统一组装入口 `assemble_messages_for_{classify,concept,aggregation}` —— 调用方
//!   （task_004 改造 `llm_classify_with_db` / `extract_concepts_for_library` /
//!   `synthesize_viewpoints`）只需调一次 assemble 即可拿到符合 ADR-003 的最终 messages。
//!
//! ## 不变量
//! - **输出格式守卫永远是最后一条 system message**：用户的自定义 prompt 不能绕过下游
//!   parser 对 JSON 格式的硬期望。
//! - **`classify_prompt_v2` 拆段**：`classify_prompt` 老接口保留为 deprecated wrapper
//!   转调 v2 + 默认段位（R8 兼容性）；不修改其他既有调用方（task_004 才做）。
//!
//! ## 设计取舍
//! - 默认文本从既有 `llm::prompts::classify_prompt` /
//!   `commands::knowledge::build_extraction_prompt` / `build_synthesis_prompt`
//!   **逐字摘抄**，避免改变 LLM 行为；后续若改默认 prompt，请同步本文件并补回归测试。
//! - `byte_len_check` 与 `assert_total_chars_within` 用字节 / 字符两种 proxy（MVP 不
//!   引入 tokenizer），阈值参见 ADR-004。
//! - `validate_required_placeholders` 只检查 module-specific 必含占位符（tagging /
//!   para 无强制占位符；concept 必含 `{content}`；aggregation 必含 `{concept_name}`）。
//!   其余占位符（`{asset_name}` / `{definition}` 等）作为可选 —— 用户即便删掉，模板
//!   引擎不会注入对应变量，但提示 LLM 输出会因此降级，本期可接受。

use rusqlite::Connection;

use crate::db::categories as db_categories;
use crate::db::library as db_library;
use crate::db::user_prompt as db_user_prompt;
use crate::llm::chat::ChatMessage;
use crate::llm::prompts;

// ============================================================================
// 1. 默认 Prompt 文本（4 module）
// ============================================================================

/// `classify_prompt` 中"四、与本系统字段的对应关系"内关于 tags 的那一段（原文行 55）。
///
/// 抽出后由 `classify_prompt_v2` 通过 `{tagging_seg}` 占位符注入。用户在 UI 自定义
/// 时编辑的就是这段文本。
pub const TAGGING_DEFAULT: &str =
    "tags：3～5 个，短词，偏行动与归宿（如「Q3交付」「会议纪要」「竞品」），避免空洞学科名与纯格式词堆砌。";

/// `classify_prompt` 中"一、核心路由(PARA Router)..."完整 5 行（原文行 35-39）。
///
/// 抽出后由 `classify_prompt_v2` 通过 `{para_seg}` 占位符注入。
pub const PARA_DEFAULT: &str = "一、核心路由（PARA Router）——自上而下穿透，直到唯一物理定位：
【P】1-项目：服务于有明确目标与截止期的短期活动？
【A】2-领域：无明确终点、但需长期维持标准的责任领域？
【R】3-资源：暂无任务、但有潜在利用价值的课题/兴趣？
【A】4-存档：已完结、取消或无限期搁置？";

/// 概念抽取 Prompt 模板（取自 `commands::knowledge::build_extraction_prompt`，原文行 447-468）。
///
/// 包含 3 个占位符：`{asset_name}` / `{project_name}` / `{content}`。
/// 用户自定义时可调整提示语气与抽取数量等，但删除 `{content}` 会导致校验失败
/// （`validate_required_placeholders`）。
pub const CONCEPT_DEFAULT: &str = "# Document Analysis Request

## Document
Title: {asset_name}
Project/Course: {project_name}
Content:
---
{content}
---

## Task
Extract all significant academic concepts from this document. For each concept:
1. name: The canonical English term
2. aliases: Alternative names (including translations if bilingual)
3. definition: A one-sentence definition as used in this context
4. excerpts: 1-2 direct quotes from the document that discuss this concept

Return as JSON array:
[{\"name\":\"...\",\"aliases\":[\"...\"],\"definition\":\"...\",\"excerpts\":[\"...\"]}]

Rules:
- Only extract substantive concepts (not generic terms like \"example\" or \"chapter\")
- Prefer established academic terminology
- Include 3-10 concepts per document
- Return only the JSON array, no other text.";

/// 知识聚合 Prompt 模板（取自 `commands::knowledge::build_synthesis_prompt`，原文行 470-495）。
///
/// 包含 3 个占位符：`{concept_name}` / `{definition}` / `{cases}`。
/// `{cases}` 由调用方将 `Vec<ConceptCase>` 渲染为多段 "### Context i: title\n excerpt"
/// 后注入。`{definition}` 为 Option，调用方在 None 时传入字面值 `"N/A"`。
pub const AGGREGATION_DEFAULT: &str = "# Viewpoint Synthesis Request

## Concept: {concept_name}
Definition: {definition}

## Appearances across student's documents:

{cases}
## Task
For each context, synthesize a viewpoint:
1. perspective: e.g. \"Economic perspective\" or \"Psychological lens\"
2. summary: 2-3 sentences explaining how this concept is understood in this context
3. sourceContext: Which course/document this perspective comes from

Return as JSON array:
[{\"perspective\":\"...\",\"summary\":\"...\",\"sourceContext\":\"...\"}]

Return only the JSON array, no other text.";

// ============================================================================
// 2. 输出格式硬守卫（ADR-003 Layer A）
// ============================================================================
//
// 这些常量作为 system message **永远最后压底**，用户 prompt 不能绕过它们。
// 字面值严格遵循 Architect output.md § 4.2。

/// 分类（tagging + para）输出格式守卫。tagging 与 para 走同一个 LLM 调用，共用此守卫。
///
/// V17 起：category 字段允许的取值范围由 prompt 内"四、与本系统字段的对应关系"
/// 第 1 项动态注入，本守卫只约束**输出格式**，不再约束**取值枚举**。
pub const CLASSIFY_OUTPUT_GUARD: &str = "**输出格式约束（系统级，不可被覆盖）**：\
仅输出一段合法 JSON 文本；不要使用 markdown 代码块；不要在 JSON 前后追加解释；\
JSON 必须包含字段：category、tags、confidence、language、suggestedFileName。\
其中 category 取值范围见 prompt 中『四、与本系统字段的对应关系』第 1 项给出的清单。";

/// 概念抽取输出格式守卫。
pub const CONCEPT_OUTPUT_GUARD: &str = "**输出格式约束（系统级，不可被覆盖）**：\
返回严格的 JSON 数组，每个元素为 {name, aliases, definition, excerpts}；\
不要使用 markdown 代码块；不要在数组前后追加任何文字。";

/// 知识聚合输出格式守卫。
pub const AGGREGATION_OUTPUT_GUARD: &str = "**输出格式约束（系统级，不可被覆盖）**：\
返回严格的 JSON 数组，每个元素为 {perspective, summary, sourceContext}；\
不要使用 markdown 代码块；不要在数组前后追加任何文字。";

// ----------------------------------------------------------------------------
// 2b. 模块特定 system_addon（concept / aggregation）
// ----------------------------------------------------------------------------
//
// 这两段 addon 逐字摘抄自既有 `commands/knowledge.rs:147` 与 `:276`，
// 用于在 task_004 切到 `assemble_messages_for_*` 时保持 LLM 行为零差异
// （input.md AC-3 第 2 步 + AC-5 "不破坏既有 LLM 调用"）。
//
// 注入位置：messages[1]（system_message 之后 / user 之前），见 § 8 各 assemble 函数。

/// 概念抽取的 system_addon（逐字摘抄自 `commands/knowledge.rs:147`）。
pub const CONCEPT_SYSTEM_ADDON: &str = "You are a knowledge extraction engine. Given a student's academic document, extract key concepts with precision. Return only valid JSON array.";

/// 知识聚合的 system_addon（逐字摘抄自 `commands/knowledge.rs:276`）。
pub const AGGREGATION_SYSTEM_ADDON: &str = "You are a knowledge synthesis engine. Help students see how the same concept appears across different courses and contexts. Return only valid JSON array.";

// ============================================================================
// 3. 字节 / 字符阈值（ADR-004）
// ============================================================================

/// 保存时单 prompt 字节上限。16 KiB ≈ 安全占用 ~4k tokens，给 LLM 输入 content 留余地。
pub const MAX_USER_PROMPT_BYTES: usize = 16 * 1024;

/// 调用前总 prompt 字符上限（system + system_addon + user + guard 总和）。
/// 64 KiB 字符是保守取 16k tokens 上限的字符等价（中英混合按 1 字符 ≈ 0.25 token）。
pub const MAX_TOTAL_PROMPT_CHARS: usize = 64 * 1024;

// ============================================================================
// 4. module 元数据访问
// ============================================================================

/// 返回 module 的内置默认 Prompt 文本。未知 module 返回空串（command 层应已做白名单过滤）。
pub fn default_for(module: &str) -> &'static str {
    match module {
        "tagging" => TAGGING_DEFAULT,
        "para" => PARA_DEFAULT,
        "concept" => CONCEPT_DEFAULT,
        "aggregation" => AGGREGATION_DEFAULT,
        _ => "",
    }
}

/// 返回 module 在前端展示的中文标题。
pub fn display_title(module: &str) -> &'static str {
    match module {
        "tagging" => "文件打标签",
        "para" => "PARA 分组",
        "concept" => "知识概念提取",
        "aggregation" => "知识聚合",
        _ => "",
    }
}

/// 返回 module 必含的占位符列表（用户自定义时不可删除）。
///
/// - tagging / para：纯文本片段，无强制占位符（其内容会被原样塞入 classify_prompt_v2）
/// - concept：必含 `{content}`（缺失 → 模板渲染后 LLM 收不到文档内容）
/// - aggregation：必含 `{concept_name}`（缺失 → LLM 不知聚合哪个概念）
pub fn required_placeholders(module: &str) -> Vec<&'static str> {
    match module {
        "tagging" | "para" => Vec::new(),
        "concept" => vec!["{content}"],
        "aggregation" => vec!["{concept_name}"],
        _ => Vec::new(),
    }
}

/// 返回 module 在调用 LLM 前需要 system 压底的输出格式守卫（ADR-003 Layer A）。
///
/// 注意：tagging 与 para 共享 `CLASSIFY_OUTPUT_GUARD`，因为它们在后端走同一个
/// `llm_classify_with_db` 调用链。
pub fn output_format_addon(module: &str) -> &'static str {
    match module {
        "tagging" | "para" => CLASSIFY_OUTPUT_GUARD,
        "concept" => CONCEPT_OUTPUT_GUARD,
        "aggregation" => AGGREGATION_OUTPUT_GUARD,
        _ => "",
    }
}

// ============================================================================
// 5. 运行时合并：DB 用户 prompt → 默认 fallback
// ============================================================================

/// 读取该 module 的运行时 Prompt。
///
/// 策略：
/// - DB 中存在记录 + `is_custom=1` + 文本非纯空白 → 返回用户文本
/// - 否则（记录不存在 / `is_custom=0` / 纯空白） → 返回内置默认（`default_for`）
///
/// 纯空白视为"等同于未自定义"，避免用户误操作清空全文却保留 is_custom=1 的退化态。
pub fn runtime_prompt_for(conn: &Connection, module: &str) -> Result<String, String> {
    let row = db_user_prompt::get(conn, module)?;
    let text = match row {
        Some(r) if r.is_custom && !r.prompt_text.trim().is_empty() => r.prompt_text,
        _ => default_for(module).to_string(),
    };
    Ok(text)
}

// ============================================================================
// 6. ADR-003 Layer B：保存时占位符静态校验
// ============================================================================

/// 校验用户自定义 prompt 是否包含 module 必含的占位符。
///
/// 缺占位符 → 返回中文错误（在 `save_user_prompt` 中触发，拒绝保存）。
pub fn validate_required_placeholders(module: &str, text: &str) -> Result<(), String> {
    for ph in required_placeholders(module) {
        if !text.contains(ph) {
            return Err(format!(
                "自定义 Prompt 缺少必需占位符 {ph}（{title}模块要求保留此占位符以注入运行时变量）",
                ph = ph,
                title = display_title(module),
            ));
        }
    }
    Ok(())
}

// ============================================================================
// 7. ADR-004：双层长度校验
// ============================================================================

/// 保存时单 prompt 字节校验（ADR-004 保存分支，与 `commands::user_prompt::validate_byte_len`
/// 同语义，本模块对外暴露以便 `commands/user_prompt.rs` 在回填时统一切到本实现）。
pub fn byte_len_check(text: &str) -> Result<(), String> {
    let n = text.as_bytes().len();
    if n > MAX_USER_PROMPT_BYTES {
        Err(format!(
            "自定义 Prompt 过长（{n} 字节，上限 {MAX_USER_PROMPT_BYTES} 字节），请精简"
        ))
    } else {
        Ok(())
    }
}

/// 调用前总 prompt 字符校验（ADR-004 调用分支）。
///
/// 把 messages 中所有 content 的 `chars().count()` 相加并比对 `MAX_TOTAL_PROMPT_CHARS`。
/// 超限：返回 `Err` 并**不发出 LLM 请求**（assemble 函数的最后一步）。
/// 实现上同时打 `log::warn!` 以便运维定位。
pub fn assert_total_chars_within(messages: &[ChatMessage]) -> Result<(), String> {
    let total: usize = messages.iter().map(|m| m.content.chars().count()).sum();
    if total > MAX_TOTAL_PROMPT_CHARS {
        log::warn!(
            "LLM 请求被拒：总字符数 {} 超过上限 {}",
            total,
            MAX_TOTAL_PROMPT_CHARS
        );
        return Err(format!(
            "LLM 请求过长（{total} 字符），请缩短 Prompt 或减少输入内容"
        ));
    }
    Ok(())
}

/// LLM 调用上下文摘要（AC-5 日志埋点用）。
///
/// 调用方在调用 `chat_completion` 之前用 `inspect_messages_for_log` 取得这份摘要，
/// 然后通过 `log::info!` 输出。不改变 assemble 函数签名，保持 task_003 既有契约稳定。
#[derive(Debug, Clone)]
pub struct LlmCallContext {
    /// 模块名（`"tagging"` / `"para"` / `"concept"` / `"aggregation"` / `"classify"`）。
    /// 注意 classify 是 tagging+para 合并后的实际调用名。
    pub module: &'static str,
    /// messages 中所有 content 的 UTF-8 字节总数。
    pub total_bytes: usize,
    /// 是否至少一个相关 module 在 DB 中是 `is_custom=1`（即用户已自定义）。
    pub user_overridden: bool,
}

/// 计算某次调用所用 messages 的总字节数（仅看 content，忽略 role 元数据）。
pub fn total_message_bytes(messages: &[ChatMessage]) -> usize {
    messages.iter().map(|m| m.content.as_bytes().len()).sum()
}

/// 判断某 module 当前是否由用户自定义（`is_custom=1` 且文本非纯空白）。
///
/// 与 `runtime_prompt_for` 的"空白视为未自定义"语义保持一致。
pub fn is_module_user_overridden(conn: &Connection, module: &str) -> Result<bool, String> {
    let row = db_user_prompt::get(conn, module)?;
    Ok(match row {
        Some(r) => r.is_custom && !r.prompt_text.trim().is_empty(),
        None => false,
    })
}

/// 组装 LLM 调用上下文（AC-5）。
///
/// - `module`：本次调用的"调用名"，由调用方传入字面（如 `"classify"` / `"concept"` /
///   `"aggregation"`）；与 4 module 的"模块名"概念近义但不完全一致——classify 调用
///   合并了 tagging+para 两个模块，因此查询自定义状态时取两者的并集。
/// - `messages`：assemble 函数已经组装好的最终 messages。
pub fn inspect_messages_for_log(
    conn: &Connection,
    module: &'static str,
    messages: &[ChatMessage],
) -> LlmCallContext {
    // classify 调用合并 tagging + para；其余 module 与名字 1:1
    let user_overridden = match module {
        "classify" => {
            is_module_user_overridden(conn, "tagging").unwrap_or(false)
                || is_module_user_overridden(conn, "para").unwrap_or(false)
        }
        "concept" | "aggregation" => is_module_user_overridden(conn, module).unwrap_or(false),
        _ => false,
    };
    LlmCallContext {
        module,
        total_bytes: total_message_bytes(messages),
        user_overridden,
    }
}

// ============================================================================
// 8. 统一组装入口（AC-3）
// ============================================================================

/// 分类调用参数（来自调用方 `llm_classify_with_db`）。
#[derive(Debug, Clone)]
pub struct ClassifyVars {
    pub content: String,
}

/// 概念抽取调用参数（来自 `extract_concepts_for_library`）。
#[derive(Debug, Clone)]
pub struct ConceptVars {
    pub asset_name: String,
    pub project_name: String,
    pub content: String,
}

/// 知识聚合调用参数（来自 `synthesize_viewpoints`）。
#[derive(Debug, Clone)]
pub struct AggregationVars {
    pub concept_name: String,
    pub definition: Option<String>,
    /// 由调用方将 `Vec<ConceptCase>` 渲染好的多段文本块（含末尾 `\n`）。
    pub cases_block: String,
}

/// 组装分类调用的 messages（tagging + para 合并到 `classify_prompt_v2`）。
///
/// 顺序：
/// 1. system: `prompts::system_message()`
/// 2. system: `prompts::classify_system_addon()`
/// 3. user:   `classify_prompt_v2(content, tagging_seg, para_seg, categories_section)`
///    - tagging_seg / para_seg 来自 `runtime_prompt_for(conn, "tagging" | "para")`
///    - categories_section（custom_para_v1 / V17）：从 `categories` 表实时渲染，
///      取**第一个 library** 的 active 类目（与 `dropzone::ensure_import_project_id`
///      的"首库"惯例对齐）。库为空或类目表为空时退化为 `CATEGORIES_SECTION_LEGACY`。
/// 4. system: `CLASSIFY_OUTPUT_GUARD`  ← ADR-003 A 永远最后压底
///
/// 最后跑 `assert_total_chars_within`。
pub fn assemble_messages_for_classify(
    conn: &Connection,
    vars: ClassifyVars,
) -> Result<Vec<ChatMessage>, String> {
    let tagging_seg = runtime_prompt_for(conn, "tagging")?;
    let para_seg = runtime_prompt_for(conn, "para")?;
    let categories_section = render_categories_section_from_db(conn)?;
    let user_body = prompts::classify_prompt_v2(
        &vars.content,
        &tagging_seg,
        &para_seg,
        &categories_section,
    );

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: prompts::system_message(),
        },
        ChatMessage {
            role: "system".to_string(),
            content: prompts::classify_system_addon().to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: user_body,
        },
        ChatMessage {
            role: "system".to_string(),
            content: CLASSIFY_OUTPUT_GUARD.to_string(),
        },
    ];

    assert_total_chars_within(&messages)?;
    Ok(messages)
}

/// custom_para_v1（V17）：从 `categories` 表渲染 classify_prompt_v2 的
/// `{categories_section}` 段落。
///
/// 策略：
/// - 取**第一个 library**（按 created_at DESC 排序的首行，与 `db::library::get_all`
///   语义一致）的 active 类目。
/// - 渲染格式：`   - \`<slug>\`（<label>）`，每行一条，sort_order 升序。
/// - 末尾固定追加两条特殊取值说明（`other` / `new:<名称>`）。
/// - 库为空 / 类目表为空 / 查询出错 → 退化为 `prompts::CATEGORIES_SECTION_LEGACY`
///   （保证 prompt 永远有合法 category 取值范围）。
fn render_categories_section_from_db(conn: &Connection) -> Result<String, String> {
    let libs = db_library::get_all(conn)?;
    let lib_id = match libs.first() {
        Some(l) => l.id.clone(),
        None => return Ok(prompts::CATEGORIES_SECTION_LEGACY.to_string()),
    };
    let cats = db_categories::list_active_for_prompt(conn, &lib_id)?;
    if cats.is_empty() {
        return Ok(prompts::CATEGORIES_SECTION_LEGACY.to_string());
    }
    let mut out = String::new();
    for c in &cats {
        out.push_str(&format!("   - `{}`（{}）\n", c.slug, c.label));
    }
    out.push_str("   特殊取值：\n");
    out.push_str("   - `other`：仅当完全无法判定时使用（系统不做目录整理，仅可能原地重命名）。\n");
    out.push_str("   - `new:<新类目名>`：若现有类目均不适用，可输出此格式请求系统建新类目（如 `new:读书笔记`）；名称 1-32 字符，仅含中文/数字/字母/_/-。");
    Ok(out)
}

/// 组装概念抽取调用的 messages。
///
/// 顺序：
/// 1. system: `prompts::system_message()`
/// 2. system: `CONCEPT_SYSTEM_ADDON`（逐字摘抄自 `commands/knowledge.rs:147`，
///    task_004 切到本函数时保持 LLM 行为零差异）
/// 3. user:   渲染后的 concept 模板
/// 4. system: `CONCEPT_OUTPUT_GUARD`  ← ADR-003 A 永远最后压底
pub fn assemble_messages_for_concept(
    conn: &Connection,
    vars: ConceptVars,
) -> Result<Vec<ChatMessage>, String> {
    let template = runtime_prompt_for(conn, "concept")?;
    let user_body = template
        .replace("{asset_name}", &vars.asset_name)
        .replace("{project_name}", &vars.project_name)
        .replace("{content}", &vars.content);

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: prompts::system_message(),
        },
        ChatMessage {
            role: "system".to_string(),
            content: CONCEPT_SYSTEM_ADDON.to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: user_body,
        },
        ChatMessage {
            role: "system".to_string(),
            content: CONCEPT_OUTPUT_GUARD.to_string(),
        },
    ];

    assert_total_chars_within(&messages)?;
    Ok(messages)
}

/// 组装知识聚合调用的 messages。
///
/// 顺序：
/// 1. system: `prompts::system_message()`
/// 2. system: `AGGREGATION_SYSTEM_ADDON`（逐字摘抄自 `commands/knowledge.rs:276`，
///    task_004 切到本函数时保持 LLM 行为零差异）
/// 3. user:   渲染后的 aggregation 模板（definition 为 None 时填 "N/A"，与既有
///    `build_synthesis_prompt` 行为一致）
/// 4. system: `AGGREGATION_OUTPUT_GUARD`  ← ADR-003 A 永远最后压底
pub fn assemble_messages_for_aggregation(
    conn: &Connection,
    vars: AggregationVars,
) -> Result<Vec<ChatMessage>, String> {
    let template = runtime_prompt_for(conn, "aggregation")?;
    let definition_text = vars.definition.as_deref().unwrap_or("N/A");
    let user_body = template
        .replace("{concept_name}", &vars.concept_name)
        .replace("{definition}", definition_text)
        .replace("{cases}", &vars.cases_block);

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: prompts::system_message(),
        },
        ChatMessage {
            role: "system".to_string(),
            content: AGGREGATION_SYSTEM_ADDON.to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: user_body,
        },
        ChatMessage {
            role: "system".to_string(),
            content: AGGREGATION_OUTPUT_GUARD.to_string(),
        },
    ];

    assert_total_chars_within(&messages)?;
    Ok(messages)
}

// ============================================================================
// 9. 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migration::run_migrations;
    use rusqlite::Connection;

    fn fresh_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in memory");
        run_migrations(&conn).expect("migrate");
        conn
    }

    // ---- default_for / display_title / required_placeholders ----

    #[test]
    fn default_for_returns_module_specific_text() {
        assert_eq!(default_for("tagging"), TAGGING_DEFAULT);
        assert_eq!(default_for("para"), PARA_DEFAULT);
        assert_eq!(default_for("concept"), CONCEPT_DEFAULT);
        assert_eq!(default_for("aggregation"), AGGREGATION_DEFAULT);
        assert_eq!(default_for("unknown"), "");
    }

    #[test]
    fn default_for_concept_contains_required_placeholder() {
        assert!(CONCEPT_DEFAULT.contains("{content}"));
        assert!(CONCEPT_DEFAULT.contains("{asset_name}"));
        assert!(CONCEPT_DEFAULT.contains("{project_name}"));
    }

    #[test]
    fn default_for_aggregation_contains_required_placeholder() {
        assert!(AGGREGATION_DEFAULT.contains("{concept_name}"));
        assert!(AGGREGATION_DEFAULT.contains("{definition}"));
        assert!(AGGREGATION_DEFAULT.contains("{cases}"));
    }

    #[test]
    fn display_title_returns_chinese_titles() {
        assert_eq!(display_title("tagging"), "文件打标签");
        assert_eq!(display_title("para"), "PARA 分组");
        assert_eq!(display_title("concept"), "知识概念提取");
        assert_eq!(display_title("aggregation"), "知识聚合");
        assert_eq!(display_title("unknown"), "");
    }

    #[test]
    fn required_placeholders_tagging_and_para_are_empty() {
        assert!(required_placeholders("tagging").is_empty());
        assert!(required_placeholders("para").is_empty());
    }

    #[test]
    fn required_placeholders_concept_requires_content() {
        assert_eq!(required_placeholders("concept"), vec!["{content}"]);
    }

    #[test]
    fn required_placeholders_aggregation_requires_concept_name() {
        assert_eq!(required_placeholders("aggregation"), vec!["{concept_name}"]);
    }

    // ---- output_format_addon ----

    #[test]
    fn output_format_addon_returns_correct_guard_per_module() {
        assert_eq!(output_format_addon("tagging"), CLASSIFY_OUTPUT_GUARD);
        assert_eq!(output_format_addon("para"), CLASSIFY_OUTPUT_GUARD);
        assert_eq!(output_format_addon("concept"), CONCEPT_OUTPUT_GUARD);
        assert_eq!(
            output_format_addon("aggregation"),
            AGGREGATION_OUTPUT_GUARD
        );
        assert_eq!(output_format_addon("unknown"), "");
    }

    #[test]
    fn guards_contain_explicit_system_marker() {
        // 用户无法通过 prompt 注入"输出格式约束"等字面值；硬守卫常量本身已含强字面提示
        assert!(CLASSIFY_OUTPUT_GUARD.contains("不可被覆盖"));
        assert!(CONCEPT_OUTPUT_GUARD.contains("不可被覆盖"));
        assert!(AGGREGATION_OUTPUT_GUARD.contains("不可被覆盖"));
    }

    /// Fix v2：concept / aggregation 的 system_addon 必须与 `commands/knowledge.rs:147` 与
    /// `:276` 的硬编码字面值**完全一致**（逐字摘抄），保证 task_004 切到 `assemble_messages_for_*`
    /// 时 LLM 行为零差异（input.md AC-5 "不破坏既有 LLM 调用"）。
    /// 若 knowledge.rs 中的 system_addon 字面值后续变更，本测试会立即失败，提示同步更新。
    #[test]
    fn system_addons_match_existing_knowledge_rs_literals() {
        assert_eq!(
            CONCEPT_SYSTEM_ADDON,
            "You are a knowledge extraction engine. Given a student's academic document, extract key concepts with precision. Return only valid JSON array."
        );
        assert_eq!(
            AGGREGATION_SYSTEM_ADDON,
            "You are a knowledge synthesis engine. Help students see how the same concept appears across different courses and contexts. Return only valid JSON array."
        );
    }

    // ---- validate_required_placeholders ----

    #[test]
    fn validate_required_placeholders_concept_accepts_default() {
        assert!(validate_required_placeholders("concept", CONCEPT_DEFAULT).is_ok());
    }

    #[test]
    fn validate_required_placeholders_concept_rejects_missing_content() {
        let err = validate_required_placeholders("concept", "抽取概念，不要输入 content")
            .expect_err("缺 {content} 应拒绝");
        assert!(err.contains("{content}"));
        assert!(err.contains("知识概念提取"), "错误消息应含模块中文名: {err}");
    }

    #[test]
    fn validate_required_placeholders_aggregation_rejects_missing_concept_name() {
        let err = validate_required_placeholders(
            "aggregation",
            "总结一下：{definition} {cases}",
        )
        .expect_err("缺 {concept_name} 应拒绝");
        assert!(err.contains("{concept_name}"));
    }

    #[test]
    fn validate_required_placeholders_aggregation_accepts_default() {
        assert!(validate_required_placeholders("aggregation", AGGREGATION_DEFAULT).is_ok());
    }

    #[test]
    fn validate_required_placeholders_tagging_para_accept_any_text() {
        // tagging 与 para 无强制占位符，纯文本片段亦视为合法
        assert!(validate_required_placeholders("tagging", "任意标签策略文字").is_ok());
        assert!(validate_required_placeholders("para", "我的 PARA 分类思想").is_ok());
        assert!(validate_required_placeholders("tagging", "").is_ok());
    }

    // ---- byte_len_check ----

    #[test]
    fn byte_len_check_passes_under_limit() {
        assert!(byte_len_check("").is_ok());
        let just_below = "a".repeat(MAX_USER_PROMPT_BYTES);
        assert!(byte_len_check(&just_below).is_ok(), "恰好 = 上限应通过");
    }

    #[test]
    fn byte_len_check_rejects_over_limit_with_chinese_message() {
        let too_long = "a".repeat(MAX_USER_PROMPT_BYTES + 1);
        let err = byte_len_check(&too_long).expect_err("超限应拒绝");
        assert!(err.contains("自定义 Prompt 过长"), "中文错误: {err}");
        assert!(err.contains(&format!("{}", MAX_USER_PROMPT_BYTES)));
    }

    #[test]
    fn byte_len_check_counts_bytes_not_chars() {
        // 单个中文字 3 字节；超 5462 字 = 16386 字节即超
        let chinese = "中".repeat(MAX_USER_PROMPT_BYTES / 3 + 2);
        let err = byte_len_check(&chinese).expect_err("超字节上限应拒绝");
        assert!(err.contains("字节"));
    }

    // ---- assert_total_chars_within ----

    #[test]
    fn assert_total_chars_within_passes_under_limit() {
        let msgs = vec![
            ChatMessage {
                role: "system".into(),
                content: "a".repeat(100),
            },
            ChatMessage {
                role: "user".into(),
                content: "b".repeat(200),
            },
        ];
        assert!(assert_total_chars_within(&msgs).is_ok());
    }

    #[test]
    fn assert_total_chars_within_at_limit_passes() {
        let msgs = vec![ChatMessage {
            role: "user".into(),
            content: "x".repeat(MAX_TOTAL_PROMPT_CHARS),
        }];
        assert!(assert_total_chars_within(&msgs).is_ok());
    }

    #[test]
    fn assert_total_chars_within_over_limit_rejects() {
        let msgs = vec![ChatMessage {
            role: "user".into(),
            content: "x".repeat(MAX_TOTAL_PROMPT_CHARS + 1),
        }];
        let err = assert_total_chars_within(&msgs).expect_err("应拒绝");
        assert!(err.contains("LLM 请求过长"));
        assert!(err.contains("字符"));
    }

    // ---- runtime_prompt_for ----

    #[test]
    fn runtime_prompt_for_returns_default_when_no_record() {
        let conn = fresh_conn();
        let t = runtime_prompt_for(&conn, "tagging").unwrap();
        assert_eq!(t, TAGGING_DEFAULT);
        let p = runtime_prompt_for(&conn, "para").unwrap();
        assert_eq!(p, PARA_DEFAULT);
    }

    #[test]
    fn runtime_prompt_for_returns_user_text_when_is_custom() {
        let conn = fresh_conn();
        db_user_prompt::upsert(&conn, "tagging", "我的标签策略").unwrap();
        let t = runtime_prompt_for(&conn, "tagging").unwrap();
        assert_eq!(t, "我的标签策略");
    }

    #[test]
    fn runtime_prompt_for_falls_back_when_user_text_only_whitespace() {
        // 纯空白视为"等同于未自定义"
        let conn = fresh_conn();
        db_user_prompt::upsert(&conn, "concept", "   \n\t  ").unwrap();
        let t = runtime_prompt_for(&conn, "concept").unwrap();
        assert_eq!(t, CONCEPT_DEFAULT);
    }

    // ---- assemble_messages_for_classify ----

    #[test]
    fn assemble_messages_for_classify_default_path_uses_builtin_segments() {
        let conn = fresh_conn();
        let messages = assemble_messages_for_classify(
            &conn,
            ClassifyVars {
                content: "测试文档".into(),
            },
        )
        .expect("assemble ok");

        assert_eq!(messages.len(), 4);
        assert_eq!(messages[0].role, "system");
        assert!(messages[0].content.contains("NoteCapt 知识管理助手"));
        assert_eq!(messages[1].role, "system");
        assert!(messages[1].content.contains("PARA 归宿分类"));
        assert_eq!(messages[2].role, "user");
        // user body 应含原 classify_prompt 中的默认 PARA 段与 tagging 段标志性文字
        assert!(messages[2].content.contains("【P】1-项目"));
        assert!(messages[2].content.contains("tags：3～5 个"));
        // 守卫永远最后压底
        assert_eq!(messages.last().unwrap().role, "system");
        assert_eq!(messages.last().unwrap().content, CLASSIFY_OUTPUT_GUARD);
    }

    #[test]
    fn assemble_messages_for_classify_uses_custom_tagging_when_saved() {
        let conn = fresh_conn();
        db_user_prompt::upsert(&conn, "tagging", "自定义 TAGGING ★彩蛋").unwrap();
        let messages = assemble_messages_for_classify(
            &conn,
            ClassifyVars {
                content: "x".into(),
            },
        )
        .unwrap();
        // 自定义文本应注入到 messages[2].content
        assert!(
            messages[2].content.contains("自定义 TAGGING ★彩蛋"),
            "应包含自定义 tagging 段; got: {}",
            messages[2].content
        );
        // 默认 tagging 文字应**已被替换掉**（不存在了）
        assert!(
            !messages[2].content.contains("tags：3～5 个"),
            "默认 tagging 段应已被覆盖"
        );
        // 最后一条仍是 GUARD
        assert_eq!(messages.last().unwrap().content, CLASSIFY_OUTPUT_GUARD);
    }

    #[test]
    fn assemble_messages_for_classify_uses_custom_para_when_saved() {
        let conn = fresh_conn();
        db_user_prompt::upsert(&conn, "para", "我的 PARA 分类哲学：A > P > R > A").unwrap();
        let messages = assemble_messages_for_classify(
            &conn,
            ClassifyVars {
                content: "y".into(),
            },
        )
        .unwrap();
        assert!(
            messages[2].content.contains("我的 PARA 分类哲学"),
            "应包含自定义 PARA 段"
        );
        assert!(
            !messages[2].content.contains("【P】1-项目"),
            "默认 PARA 段应已被覆盖"
        );
    }

    // ---- assemble_messages_for_concept ----

    #[test]
    fn assemble_messages_for_concept_replaces_all_placeholders() {
        let conn = fresh_conn();
        let messages = assemble_messages_for_concept(
            &conn,
            ConceptVars {
                asset_name: "操作系统.pdf".into(),
                project_name: "CS-101".into(),
                content: "进程是程序的一次执行...".into(),
            },
        )
        .unwrap();

        // 修复 v2：system_message → CONCEPT_SYSTEM_ADDON → user → GUARD，共 4 条
        assert_eq!(messages.len(), 4);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "system");
        // messages[1] 必须是逐字摘抄自 knowledge.rs:147 的 system_addon
        assert!(
            messages[1].content.contains("knowledge extraction engine"),
            "messages[1] 应是 CONCEPT_SYSTEM_ADDON: got {}",
            messages[1].content
        );
        assert_eq!(messages[1].content, CONCEPT_SYSTEM_ADDON);
        assert_eq!(messages[2].role, "user");
        // 占位符应被替换为变量值
        assert!(messages[2].content.contains("操作系统.pdf"));
        assert!(messages[2].content.contains("CS-101"));
        assert!(messages[2].content.contains("进程是程序的一次执行"));
        // 占位符自身不应残留
        assert!(!messages[2].content.contains("{asset_name}"));
        assert!(!messages[2].content.contains("{project_name}"));
        assert!(!messages[2].content.contains("{content}"));
        // 守卫永远是 messages.last()
        assert_eq!(messages.last().unwrap().role, "system");
        assert_eq!(messages.last().unwrap().content, CONCEPT_OUTPUT_GUARD);
    }

    #[test]
    fn assemble_messages_for_concept_uses_custom_template_when_saved() {
        let conn = fresh_conn();
        // 自定义但保留必含占位符 {content}（其他占位符可选）
        db_user_prompt::upsert(
            &conn,
            "concept",
            "请按我的方法分析：{content}\n要求：每条概念附中文译名。",
        )
        .unwrap();

        let messages = assemble_messages_for_concept(
            &conn,
            ConceptVars {
                asset_name: "a".into(),
                project_name: "p".into(),
                content: "TEST_BODY_XYZ".into(),
            },
        )
        .unwrap();
        // 修复 v2：user body 现在是 messages[2]（[1] 是 system_addon）
        assert!(messages[2].content.contains("请按我的方法分析"));
        assert!(messages[2].content.contains("TEST_BODY_XYZ"));
        assert!(messages[2].content.contains("中文译名"));
        // 因为自定义 prompt 没有 {asset_name}，所以 user body 不应含原默认中的 "# Document Analysis Request"
        assert!(!messages[2].content.contains("Document Analysis Request"));
        // system_addon 不受用户自定义模板影响（仍是 CONCEPT_SYSTEM_ADDON 字面）
        assert_eq!(messages[1].content, CONCEPT_SYSTEM_ADDON);
    }

    // ---- assemble_messages_for_aggregation ----

    #[test]
    fn assemble_messages_for_aggregation_replaces_placeholders_and_handles_none_definition() {
        let conn = fresh_conn();
        let messages = assemble_messages_for_aggregation(
            &conn,
            AggregationVars {
                concept_name: "认知偏差".into(),
                definition: None,
                cases_block: "### Context 1: T\nE\n\n".into(),
            },
        )
        .unwrap();
        // 修复 v2：system_message → AGGREGATION_SYSTEM_ADDON → user → GUARD，共 4 条
        assert_eq!(messages.len(), 4);
        // messages[1] 必须是逐字摘抄自 knowledge.rs:276 的 system_addon
        assert!(
            messages[1].content.contains("knowledge synthesis engine"),
            "messages[1] 应是 AGGREGATION_SYSTEM_ADDON: got {}",
            messages[1].content
        );
        assert_eq!(messages[1].content, AGGREGATION_SYSTEM_ADDON);
        // user body 现在是 messages[2]（[1] 是 system_addon）
        // None definition → "N/A"
        assert!(messages[2].content.contains("Definition: N/A"));
        assert!(messages[2].content.contains("认知偏差"));
        assert!(messages[2].content.contains("Context 1: T"));
        assert!(!messages[2].content.contains("{concept_name}"));
        assert!(!messages[2].content.contains("{definition}"));
        assert!(!messages[2].content.contains("{cases}"));
        assert_eq!(messages.last().unwrap().content, AGGREGATION_OUTPUT_GUARD);
    }

    #[test]
    fn assemble_messages_for_aggregation_with_some_definition() {
        let conn = fresh_conn();
        let messages = assemble_messages_for_aggregation(
            &conn,
            AggregationVars {
                concept_name: "认知偏差".into(),
                definition: Some("一种系统性偏差".into()),
                cases_block: String::new(),
            },
        )
        .unwrap();
        // 修复 v2：user body 现在是 messages[2]
        assert!(messages[2].content.contains("Definition: 一种系统性偏差"));
        assert_eq!(messages[1].content, AGGREGATION_SYSTEM_ADDON);
    }

    // ---- assemble + 字符上限 ----

    #[test]
    fn assemble_rejects_when_total_chars_over_limit() {
        let conn = fresh_conn();
        // 制造一个超长 content（必定让最终 messages 超 64 KiB 字符上限）
        let huge_content = "x".repeat(MAX_TOTAL_PROMPT_CHARS + 1);
        let r = assemble_messages_for_concept(
            &conn,
            ConceptVars {
                asset_name: "a".into(),
                project_name: "p".into(),
                content: huge_content,
            },
        );
        let err = r.expect_err("超字符应拒绝");
        assert!(err.contains("LLM 请求过长"), "错误: {err}");
    }

    // ---- custom_para_v1 / V17：assemble_messages_for_classify 注入类目清单 ----

    fn fresh_conn_with_lib_and_categories(lib_id: &str) -> Connection {
        let conn = fresh_conn();
        conn.execute(
            "INSERT INTO libraries (id, name, root_path, created_at) VALUES (?1, 'L', '/tmp/L', datetime('now'))",
            rusqlite::params![lib_id],
        )
        .unwrap();
        db_categories::seed_builtin_categories(&conn, lib_id).unwrap();
        conn
    }

    #[test]
    fn assemble_classify_falls_back_to_legacy_when_no_library() {
        // fresh_conn 没建任何 library → render_categories_section_from_db
        // 应退化为 CATEGORIES_SECTION_LEGACY，保证 prompt 永远有合法 category 取值范围
        let conn = fresh_conn();
        let messages = assemble_messages_for_classify(
            &conn,
            ClassifyVars {
                content: "x".into(),
            },
        )
        .unwrap();
        let body = &messages[2].content;
        // legacy 字面（V17 之前的硬枚举）
        assert!(
            body.contains("`1-项目` `2-领域` `3-资源` `4-存档`"),
            "无 library 时应注入 legacy 类目段"
        );
    }

    #[test]
    fn assemble_classify_injects_db_categories_when_library_exists() {
        let conn = fresh_conn_with_lib_and_categories("L1");
        // 加一个 LLM 自动生成的自定义类目
        db_categories::upsert_llm_generated(&conn, "L1", "读书笔记", "读书笔记").unwrap();

        let messages = assemble_messages_for_classify(
            &conn,
            ClassifyVars {
                content: "x".into(),
            },
        )
        .unwrap();
        let body = &messages[2].content;
        // 应注入 4 个 builtin + 自定义类目
        for slug in &["1-项目", "2-领域", "3-资源", "4-存档", "读书笔记"] {
            assert!(body.contains(slug), "应包含类目 slug {slug}; got: {body}");
        }
        // 不应残留 legacy 硬枚举行
        assert!(
            !body.contains("`1-项目` `2-领域` `3-资源` `4-存档`"),
            "动态注入后不应残留 legacy 横向列举"
        );
        // 特殊取值（other / new:）说明也应在
        assert!(body.contains("`other`"));
        assert!(body.contains("new:<新类目名>"));
    }

    #[test]
    fn assemble_classify_skips_disabled_categories_from_prompt() {
        let conn = fresh_conn_with_lib_and_categories("L1");
        conn.execute(
            "UPDATE categories SET is_disabled=1 WHERE library_id='L1' AND slug='4-存档'",
            [],
        )
        .unwrap();
        let messages = assemble_messages_for_classify(
            &conn,
            ClassifyVars {
                content: "x".into(),
            },
        )
        .unwrap();
        let body = &messages[2].content;
        // disabled 的类目不应出现在第四节类目枚举（注：'4-存档' 在 PARA 段一段会出现，
        // 但在第四节"- `4-存档`"的形式不应出现）
        assert!(!body.contains("- `4-存档`"));
        // 其余 3 个 builtin 仍应在
        assert!(body.contains("- `1-项目`"));
        assert!(body.contains("- `2-领域`"));
        assert!(body.contains("- `3-资源`"));
    }
}
