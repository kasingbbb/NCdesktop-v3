/// Prompt 模板管理（集中化、版本化）

pub const PROMPT_VERSION: &str = "1.1";

/// 智能摘要 Prompt
pub fn summarize_prompt(content: &str, language: &str) -> String {
    format!(
        r#"你是一位专业的学术笔记助手。请将以下多模态知识内容进行结构化摘要。

要求：
1. 使用 {} 语言输出
2. 保持关键信息和时间线结构
3. 区分不同来源（音频转录、OCR文本、手动笔记）
4. 提取核心观点和关键术语
5. 输出 Markdown 格式

内容：
{}

请输出结构化摘要："#,
        language, content
    )
}

/// AI 自动分类 Prompt（集成 PARA 动态分类与重命名底层协议）。
///
/// **DEPRECATED（custom_prompt_v1 / task_003）**：本函数保留为向后兼容 wrapper，
/// 内部转调 `classify_prompt_v2`，使用 `prompt_runtime::PARA_DEFAULT` 与
/// `prompt_runtime::TAGGING_DEFAULT` 默认段位填入，类目清单用 `CATEGORIES_SECTION_LEGACY`。
///
/// 新调用方应改用 `llm::prompt_runtime::assemble_messages_for_classify`（task_004 改造）。
#[deprecated(
    note = "用 llm::prompt_runtime::assemble_messages_for_classify 替代；本 wrapper 仅用于尚未迁移的旧调用"
)]
pub fn classify_prompt(content: &str) -> String {
    use crate::llm::prompt_runtime::{PARA_DEFAULT, TAGGING_DEFAULT};
    classify_prompt_v2(content, TAGGING_DEFAULT, PARA_DEFAULT, CATEGORIES_SECTION_LEGACY)
}

/// custom_para_v1：默认类目清单段（legacy 兼容用）。
///
/// 字面对应 V17 之前 4 个 PARA 内置类目的硬枚举。生产路径下
/// `prompt_runtime::assemble_messages_for_classify` 会用从 `categories` 表实时读出的
/// 清单覆盖本字面（含用户/LLM 自定义类目）；本常量只服务 deprecated
/// `classify_prompt` wrapper，使其与旧 prompt 字面继续等价（守护测试不漂移）。
pub const CATEGORIES_SECTION_LEGACY: &str = "   - `1-项目` `2-领域` `3-资源` `4-存档`
   - 仅当完全无法做 PARA 判定时才用 `other`（系统将不做目录整理，仅可能原地重命名）。";

/// AI 自动分类 Prompt 拆段版本（custom_prompt_v1 / task_003 + custom_para_v1 / V17）。
///
/// 段落映射：
/// - `{para_seg}` 占位 = "一、核心路由（PARA Router）..." 共 5 行
///   - 默认值 = `prompt_runtime::PARA_DEFAULT`
/// - `{tagging_seg}` 占位 = "四、与本系统字段的对应关系" 第 2 项 tags 段
///   - 默认值 = `prompt_runtime::TAGGING_DEFAULT`
/// - `{categories_section}` 占位 = "四、与本系统字段的对应关系" 第 1 项 category
///   字段的类目枚举（V17 起从 `categories` 表动态注入，含用户/LLM 自定义类目；
///   特殊取值 `other` / `new:<名称>` 由 prompt_runtime 渲染器统一附加在尾部）
///   - legacy 默认 = `CATEGORIES_SECTION_LEGACY`
///
/// **`classify_prompt_v2(content, TAGGING_DEFAULT, PARA_DEFAULT, CATEGORIES_SECTION_LEGACY)`
/// 与旧 `classify_prompt(content)` 输出字符串完全等价**（见 `classify_prompt_tests`）。
pub fn classify_prompt_v2(
    content: &str,
    tagging_seg: &str,
    para_seg: &str,
    categories_section: &str,
) -> String {
    format!(
        r#"【AI 逻辑与作业宪章：PARA 动态分类与重命名】

思想原则：
- 拒绝静态归档：分类服从「行动的引力」。不问「主题是什么」，只问「在哪个项目里最管用」。
- 绞杀「按信息来源/格式」建档（如单独建「读书笔记」「PPT模板」文件夹）；强制「按信息归宿」输出。
- 资料的存放位置应与其可执行性（Actionability）和紧急程度（Immediacy）一致：越贴近当前行动，层级越浅。

{para_seg}

二、策略过滤（禁止项）：
- 禁止按来源或格式建仓名（违背归宿原则）。
- 禁止大而空的学科名作为「唯一分类依据」（如单独用「心理学」「经济学」当全部标签）；标签应服务检索与下一步行动。
- 禁止为单条信息设计超过三层的嵌套子文件夹概念；细分过深时停在当前任务可用的粒度，依赖搜索补全。

三、归类前自检（内部完成，不要输出）：
1）这份文件要促成什么具体交付物？
2）用户赶进度时 10 秒内能否找到？
3）若是「半熟素材」，下一步最可能拼进哪类行动？

四、与本系统字段的对应关系：
1）category（主类别，字符串，**必须取下列之一**，用于磁盘 `organized/<category>/`）：
{categories_section}
2）{tagging_seg}
3）suggestedFileName：建议主文件名（**不含扩展名**），遵守「行动力榨取」：
   - 偏项目/任务：倾向「强动词 + 具象对象/目标 + 关键时间或版本」，如：设计2024Q3官网重构版、招聘前端工程师_05月。
   - 偏领域/资源：「核心责任或兴趣点 + 可选材料类型」，如：健康管理_年度体检汇总、建筑学参考_立面集。
   - 通用文件/素材：极简可检索，可用下划线连接要素，如：会议纪要_XX项目_240510、竞品分析_幻灯片草案。
   - 去掉无意义装饰词，保留可搜索关键词；不要使用路径分隔符或非法文件名字符。

五、待分析内容：
{content}

**必须严格遵守**：
- 只输出 **一段** 合法 JSON 文本；
- 不要使用 markdown 代码块（不要 ```）；
- 不要在 JSON 前后追加任何解释性句子。

要求 JSON 含：category、tags、confidence（0-1）、language、suggestedFileName。

JSON 模板示例：
{{"category":"1-项目","tags":["交付","原型","Q3"],"confidence":0.88,"language":"zh","suggestedFileName":"设计2024Q3官网重构版"}}"#,
        para_seg = para_seg,
        tagging_seg = tagging_seg,
        categories_section = categories_section,
        content = content,
    )
}

#[cfg(test)]
mod classify_prompt_tests {
    use super::*;
    use crate::llm::prompt_runtime::{PARA_DEFAULT, TAGGING_DEFAULT};

    /// AC-2：`classify_prompt_v2(content, TAGGING_DEFAULT, PARA_DEFAULT, CATEGORIES_SECTION_LEGACY)`
    /// 与既有 `classify_prompt(content)` 字符串等价。
    #[test]
    #[allow(deprecated)]
    fn classify_prompt_v2_with_defaults_matches_legacy_wrapper() {
        let content = "示例文档内容 123 abc";
        let legacy = classify_prompt(content);
        let v2 = classify_prompt_v2(content, TAGGING_DEFAULT, PARA_DEFAULT, CATEGORIES_SECTION_LEGACY);
        // 旧 wrapper 现已转调 v2，两者必然等价；该测试守护"段落映射"长期不漂移
        assert_eq!(legacy, v2);
    }

    #[test]
    fn classify_prompt_v2_injects_custom_tagging() {
        let out = classify_prompt_v2(
            "doc",
            "tags：MY_CUSTOM_TAG_RULE_★",
            PARA_DEFAULT,
            CATEGORIES_SECTION_LEGACY,
        );
        assert!(out.contains("MY_CUSTOM_TAG_RULE_★"));
        assert!(!out.contains("tags：3～5 个"));
        // PARA 段仍是默认
        assert!(out.contains("【P】1-项目"));
    }

    #[test]
    fn classify_prompt_v2_injects_custom_para() {
        let out = classify_prompt_v2(
            "doc",
            TAGGING_DEFAULT,
            "我的 PARA 哲学：优先 P 大于 A 大于 R",
            CATEGORIES_SECTION_LEGACY,
        );
        assert!(out.contains("我的 PARA 哲学"));
        assert!(!out.contains("【P】1-项目"));
        // tagging 默认仍在
        assert!(out.contains("tags：3～5 个"));
    }

    #[test]
    fn classify_prompt_v2_preserves_invariant_sections() {
        // 任意 seg 替换后，其余段落（输出约束、JSON 模板示例）应保留
        let out = classify_prompt_v2("X", "T", "P", CATEGORIES_SECTION_LEGACY);
        assert!(out.contains("**必须严格遵守**"));
        assert!(out.contains("JSON 模板示例"));
        assert!(out.contains("category"));
        assert!(out.contains("suggestedFileName"));
        assert!(out.contains("待分析内容"));
        assert!(out.contains("X"));
    }

    /// custom_para_v1：自定义类目清单段（V17 起，prompt_runtime 用本占位符注入实时清单）。
    #[test]
    fn classify_prompt_v2_injects_custom_categories_section() {
        let custom = "   - `1-项目`（项目）\n   - `读书笔记`（读书笔记）\n   - `other` / `new:<名称>`：见末尾说明";
        let out = classify_prompt_v2("doc", TAGGING_DEFAULT, PARA_DEFAULT, custom);
        // 自定义类目段被注入到 prompt
        assert!(out.contains("读书笔记"), "应包含自定义类目: {out}");
        // 默认 legacy 类目段应**已被替换掉**（不存在硬枚举的"`1-项目` `2-领域` `3-资源` `4-存档`"行）
        assert!(
            !out.contains("`1-项目` `2-领域` `3-资源` `4-存档`"),
            "默认硬枚举行应已被覆盖"
        );
    }
}

/// 分类专用 system：PARA + 仅 JSON 约束
pub fn classify_system_addon() -> &'static str {
    "你是 NoteCapt 分类器，严格执行 PARA 归宿分类与「行动力榨取」重命名。回复必须是纯 JSON 对象字符串，键为 category、tags、confidence、language、suggestedFileName（主文件名不含扩展名）。禁止输出其它文字。"
}

/// Markdown 导出 Prompt（增强版，用于 LLM 二次整理）
pub fn enhance_export_prompt(markdown: &str) -> String {
    format!(
        r#"以下是从多模态知识采集设备自动生成的原始 Markdown 笔记。请对其进行优化：

1. 修正明显的语音转录错误
2. 改善段落结构
3. 添加适当的标题层级
4. 保留所有时间戳和来源标注
5. 不要删除或修改原始内容的核心含义

原始 Markdown：
{}

输出优化后的 Markdown："#,
        markdown
    )
}

/// 构造 system message
pub fn system_message() -> String {
    format!(
        "你是 NoteCapt 知识管理助手（Prompt v{}）。\
         你帮助用户按 PARA 思路整理、摘要和导出多模态知识；\
         分类重命名以「行动与归宿」为先，尊重原始数据的准确性。",
        PROMPT_VERSION
    )
}

// ─── 知识理解辅助层 prompt builders（task_003） ──────────────────────────

/// 单段摘录素材（用于 summary prompt）
pub struct ExcerptItem {
    pub asset_name: String,
    pub project_name: String,
    pub text: String,
}

/// 单段文档片段（用于 explanation prompt）
pub struct DocumentSection {
    pub project_name: String,
    pub asset_name: String,
    pub content: String,
}

/// 关键要点（用于 mirror / 校对 prompt）
pub struct KeyPoint {
    pub text: String,
    pub source: String,
}

/// 摘要 prompt：把多个文档摘录整合成一段连贯说明，必须只用所给材料 + 每条要点都标来源。
pub fn build_summary_prompt(concept_name: &str, excerpts: &[ExcerptItem]) -> String {
    let mut body = String::new();
    for (i, e) in excerpts.iter().enumerate() {
        body.push_str(&format!(
            "[Source {}: {} / {}]\n{}\n\n",
            i + 1,
            e.asset_name,
            e.project_name,
            e.text
        ));
    }
    format!(
        "概念名称：{concept_name}\n\n\
         以下是来自学生文档的相关摘录：\n\n{body}\n\
         CRITICAL RULES:\n\
         1. ONLY use information from provided documents above.\n\
         2. Do NOT add any external knowledge or fabricate examples.\n\
         3. Cite source for EVERY point by referencing the source labels (e.g. [Source 1]).\n\
         4. Keep the summary concise, factual, and integrate overlapping points.\n\
         5. Respond in the same language as the documents.\n\n\
         请基于上述摘录撰写关于「{concept_name}」的整合摘要。",
        concept_name = concept_name,
        body = body.trim_end()
    )
}

/// 理解框架 prompt：要求 LLM 输出 JSON（mechanism/typical_scenarios/common_misconceptions/essence_sentence），
/// 每项都必须含 source 字段；不允许凭空构造。
pub fn build_explanation_prompt(
    concept_name: &str,
    definition: &str,
    sections: &[DocumentSection],
) -> String {
    let mut body = String::new();
    for (i, s) in sections.iter().enumerate() {
        body.push_str(&format!(
            "[Source {}: {} / {}]\n{}\n\n",
            i + 1,
            s.asset_name,
            s.project_name,
            s.content
        ));
    }
    format!(
        "概念名称：{concept_name}\n\
         已有定义：{definition}\n\n\
         以下是该概念在学生文档中的出现段落：\n\n{body}\n\
         CRITICAL RULES:\n\
         1. ONLY use information from provided documents above; never introduce outside knowledge.\n\
         2. Cite source for EVERY point — each item must include a `source` field referencing the originating document.\n\
         3. Do NOT fabricate mechanisms, scenarios, or misconceptions; if a category has no evidence in the documents, omit it.\n\
         4. `mechanism.source` MUST be non-empty; if no mechanism is observable in documents, leave the mechanism description blank but never invent one.\n\
         5. Respond in JSON only, no markdown fences or prose around it.\n\n\
         Output JSON shape:\n\
         {{\n\
           \"mechanism\": {{ \"text\": \"...\", \"source\": \"...\" }},\n\
           \"typical_scenarios\": [{{ \"text\": \"...\", \"source\": \"...\" }}],\n\
           \"common_misconceptions\": [{{ \"text\": \"...\", \"source\": \"...\" }}] | null,\n\
           \"essence_sentence\": \"...\"\n\
         }}",
        concept_name = concept_name,
        definition = definition,
        body = body.trim_end()
    )
}

/// 镜子核对 prompt：对照学生自己写的解释 vs 学生自己的文档，用探索式语言反馈，禁用 wrong/incorrect/incomplete/missing/failed to。
pub fn build_mirror_prompt(
    concept_name: &str,
    user_explanation: &str,
    key_points: &[KeyPoint],
) -> String {
    let mut points = String::new();
    for (i, p) in key_points.iter().enumerate() {
        points.push_str(&format!(
            "[Key Point {}: source = {}]\n{}\n\n",
            i + 1,
            p.source,
            p.text
        ));
    }
    format!(
        "概念名称：{concept_name}\n\n\
         学生自己写下的理解：\n{user_explanation}\n\n\
         学生自己文档中的关键要点：\n\n{points}\n\
         CRITICAL RULES:\n\
         1. Compare ONLY against the provided documents — never against any external standard.\n\
         2. Cite source for EVERY observation by referencing key-point labels (e.g. [Key Point 1]).\n\
         3. Use encouraging, exploratory language throughout.\n\
         4. NEVER use words like 'wrong', 'incorrect', 'incomplete', 'missing', 'failed to'.\n\
         5. Acknowledge what the student captured correctly first.\n\
         6. Present any uncovered points as additional perspectives or things to revisit, not as mistakes.\n\
         7. Respond in the same language as the student's explanation.\n\n\
         请基于以上规则给出反馈。",
        concept_name = concept_name,
        user_explanation = user_explanation,
        points = points.trim_end()
    )
}
