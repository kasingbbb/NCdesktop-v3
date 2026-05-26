# Task 输入 — task_003_dev_backend_validation

## 目标

在 NCdesktop 后端实现 `llm/prompt_runtime.rs` 运行时层：默认 Prompt 暴露、运行时合并、输出格式硬守卫、占位符静态校验、字节长度校验、`assemble_messages_for` 统一组装入口；并将既有 `classify_prompt` 拆为接受外部 tagging/para 段位的 v2 版本。

## 前置条件

- 依赖 task：`task_002_dev_backend_data` 必须 DONE（提供 `db::user_prompt::get` / 4 个 IPC 占位字段填充入口）
- 必须先存在的文件/接口：
  - `src-tauri/src/db/user_prompt.rs` 与 `commands/user_prompt.rs`（task_002 产物）
  - `src-tauri/src/llm/prompts.rs`（既有，需要修改）

## 验收标准（Acceptance Criteria）

1. **AC-1（prompt_runtime.rs 全套函数）** — 新建 `src-tauri/src/llm/prompt_runtime.rs` 并通过 `llm/mod.rs` 暴露 `pub mod prompt_runtime;`。包含以下公开项：

   ```rust
   pub const TAGGING_DEFAULT: &str = "...";   // 从 prompts.rs 中 classify_prompt 的 tagging 段抽出
   pub const PARA_DEFAULT: &str = "...";       // 同上，PARA 段
   pub const CONCEPT_DEFAULT: &str = "...";    // 从 commands/knowledge.rs::build_extraction_prompt 抽出（保留 {asset_name} {project_name} {content} 占位符）
   pub const AGGREGATION_DEFAULT: &str = "..."; // 从 commands/knowledge.rs::build_synthesis_prompt 抽出（保留 {concept_name} {definition} {cases} 占位符）

   pub const CLASSIFY_OUTPUT_GUARD: &str = "...";
   pub const CONCEPT_OUTPUT_GUARD: &str = "...";
   pub const AGGREGATION_OUTPUT_GUARD: &str = "...";

   pub const MAX_USER_PROMPT_BYTES: usize = 16 * 1024;
   pub const MAX_TOTAL_PROMPT_CHARS: usize = 64 * 1024;

   pub fn default_for(module: &str) -> &'static str;          // 4 module → 4 个默认全文
   pub fn display_title(module: &str) -> &'static str;        // 中文展示标题
   pub fn required_placeholders(module: &str) -> Vec<&'static str>;  // tagging/para 返回 vec![], concept 返回 ["{content}"], aggregation 返回 ["{concept_name}"]（其余占位符可选）
   pub fn output_format_addon(module: &str) -> &'static str;  // ADR-003 A

   pub fn runtime_prompt_for(conn: &Connection, module: &str) -> Result<String, String>;  // 用 db::user_prompt::get → is_custom=1 且非空白 → 返回 prompt_text，否则返回 default_for(module).to_string()
   pub fn validate_required_placeholders(module: &str, text: &str) -> Result<(), String>;  // ADR-003 B
   pub fn byte_len_check(text: &str) -> Result<(), String>;  // ADR-004 保存时
   pub fn assert_total_chars_within(messages: &[ChatMessage]) -> Result<(), String>;  // ADR-004 调用前
   ```

   - 单测覆盖每个函数（含占位符校验 PASS/FAIL、byte_len_check 临界、runtime_prompt_for 在 is_custom=0 时 fallback、output_format_addon 4 module 各自返回正确常量）

2. **AC-2（classify_prompt 拆段）** — 修改 `src-tauri/src/llm/prompts.rs`：
   - 保留现有 `classify_prompt(content)` 作为 deprecated wrapper（内部调用新函数，使用默认段位填入），防止破坏既有未迁移调用（如 `llm_summarize` 等不在本期改造范围的命令暂可继续用旧 wrapper）
   - 新增 `pub fn classify_prompt_v2(content: &str, tagging_seg: &str, para_seg: &str) -> String`：把 `tagging` 段（`tags：3～5 个...`）与 `PARA` 段（`核心路由(PARA Router)...`）从 `classify_prompt` 主体中抽出，由参数注入；其余文本（思想原则、策略过滤、归类前自检、JSON 模板示例等）保持原样
   - 单测：`classify_prompt_v2(content, DEFAULT_TAGGING, DEFAULT_PARA)` 输出与旧 `classify_prompt(content)` 字符串等价（或差异仅为空白/段落顺序，可接受）；变更 `tagging_seg` 后输出包含新文本
   - **注意**：`classify_system_addon` 与 `system_message` 不动

3. **AC-3（assemble_messages_for）** — 在 `prompt_runtime.rs` 新增统一组装入口：

   ```rust
   pub struct ClassifyVars { pub content: String }
   pub struct ConceptVars { pub asset_name: String, pub project_name: String, pub content: String }
   pub struct AggregationVars { pub concept_name: String, pub definition: Option<String>, pub cases_block: String }

   pub fn assemble_messages_for_classify(conn: &Connection, vars: ClassifyVars) -> Result<Vec<ChatMessage>, String>;
   pub fn assemble_messages_for_concept(conn: &Connection, vars: ConceptVars) -> Result<Vec<ChatMessage>, String>;
   pub fn assemble_messages_for_aggregation(conn: &Connection, vars: AggregationVars) -> Result<Vec<ChatMessage>, String>;
   ```

   每个 assemble 函数内部按以下顺序构造 `messages`：
   1. system: `prompts::system_message()`
   2. system: 模块特定 system_addon（classify 用 `prompts::classify_system_addon()`；concept / aggregation 各自的固定文案）
   3. user: 拼接后的 prompt 文本（含 `runtime_prompt_for` 注入用户段 + variable 替换）
   4. system: `output_format_addon(module)` ← **永远最后压底**（ADR-003 A）
   - 最后调用 `assert_total_chars_within(&messages)?` 校验总字符数
   - 单测：mock conn 中插入"自定义 tagging 段"，调用 `assemble_messages_for_classify` 后 messages[2].content 包含该自定义文本；最后一条 message 是 system 且 = `CLASSIFY_OUTPUT_GUARD`

4. **AC-4（task_002 占位符回填）** — 修改 `commands/user_prompt.rs`：
   - `list_user_prompts` / `get_user_prompt` 中的 `default_text` 改为调用 `prompt_runtime::default_for(module).to_string()`
   - `required_placeholders` 改为调用 `prompt_runtime::required_placeholders(module).iter().map(|s| s.to_string()).collect()`
   - `max_bytes` 改为 `prompt_runtime::MAX_USER_PROMPT_BYTES`
   - `display_title` 字段写入 `prompt_runtime::display_title(module).to_string()`
   - `save_user_prompt` 中 placeholder stub 替换为 `prompt_runtime::validate_required_placeholders(&module, &text)?`
   - 单测：保存"缺 `{content}`" 的 concept prompt 返回错误；保存合法的 PASS

5. **AC-5（不破坏既有 LLM 调用）** — `cargo build` 通过；`cargo test --lib` 全表 PASS；`commands/llm.rs` 与 `commands/knowledge.rs` 此时仍使用旧 `classify_prompt` wrapper 与旧 `build_extraction_prompt` / `build_synthesis_prompt`（这些函数在本 task 中**不动**，由 task_004 改造）

## 技术约束

- **代码规范**：
  - 所有 default 常量必须从既有源码中**逐字摘抄**（不要"改写得更好"），避免改变 LLM 行为
  - `display_title` 使用中文：tagging→"文件打标签"、para→"PARA 分组"、concept→"知识概念提取"、aggregation→"知识聚合"
- **Architect 方案约束**：
  - `output_format_addon` 的三个常量字面值严格按 output.md § 4.2 抄入
  - 字节阈值 16 KiB（`MAX_USER_PROMPT_BYTES = 16 * 1024`）与字符阈值 64 KiB（`MAX_TOTAL_PROMPT_CHARS = 64 * 1024`）固定，不允许调整
  - `assemble_messages_for_*` 的 messages 顺序严格按 AC-3 描述（输出守卫永远最后）

## 参考文件

**必读**：
- Architect output.md `§ 4.2`（输出格式守卫常量字面值）
- Architect output.md `§ 5`（数据模型）
- Architect output.md `§ ADR-003`（三层防御）
- Architect output.md `§ ADR-004`（双层字节/字符校验）
- Architect output.md `§ R8`（classify_prompt 签名变更兼容性）

**代码参考（必读）**：
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/llm/prompts.rs:26-76` — `classify_prompt` 原文，AC-2 拆段时严格 mapping
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/commands/knowledge.rs:447-468` — `build_extraction_prompt` 原文，CONCEPT_DEFAULT 来源
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/commands/knowledge.rs:470-496` — `build_synthesis_prompt` 原文，AGGREGATION_DEFAULT 来源
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/llm/chat.rs:22-26` — `ChatMessage` 结构定义

## 预估影响范围

- **新建文件**：
  - `src-tauri/src/llm/prompt_runtime.rs`（核心实现 + 单测）
- **修改文件**：
  - `src-tauri/src/llm/mod.rs`（加 `pub mod prompt_runtime;`）
  - `src-tauri/src/llm/prompts.rs`（增 `classify_prompt_v2`，将旧 `classify_prompt` 改为 wrapper）
  - `src-tauri/src/commands/user_prompt.rs`（task_002 留下的占位字段全部回填，stub 校验函数替换）
- **预估变更**：~600 行（含测试 ~250 行）
