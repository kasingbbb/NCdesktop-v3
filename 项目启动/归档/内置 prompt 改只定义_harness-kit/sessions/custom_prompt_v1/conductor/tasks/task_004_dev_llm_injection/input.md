# Task 输入 — task_004_dev_llm_injection

## 目标

将 NCdesktop 现有 3 处 LLM 调用链（分类 / 概念抽取 / 知识聚合）改造为通过 `prompt_runtime::assemble_messages_for_*` 组装消息，使用户自定义 Prompt 与输出守卫真正生效，并完成端到端日志埋点。

## 前置条件

- 依赖 task：`task_003_dev_backend_validation` 必须 DONE（提供 `assemble_messages_for_classify / concept / aggregation`）
- 必须先存在的文件/接口：
  - `src-tauri/src/llm/prompt_runtime.rs`（task_003 产物）
  - `src-tauri/src/llm/prompts.rs::classify_prompt_v2`（task_003 产物）

## 验收标准（Acceptance Criteria）

0. **AC-0（chat.rs 多 system 合并修复 / 前置）** —
   - **现状缺陷**：`src-tauri/src/llm/chat.rs:58-66` 当前 `let mut system_text = None; for msg in &messages { if msg.role == "system" { system_text = Some(msg.content.clone()); } }` 在循环里**覆盖**而非合并，最终 `system_text` 只保留最后一条 system。
   - **影响**：task_003 的 3 个 `assemble_messages_for_*` 产出的 messages 含 3 条 system（system_message + system_addon + GUARD），但 chat.rs 当前实现只发 GUARD 给 Anthropic。AC-1/2/3 改造后若不修该 bug，则 system_message + system_addon 静默丢失，违反"LLM 行为零差异"原则。
   - **修复方案（推荐）**：把 `chat.rs:58-66` 改为：
     ```rust
     let collected: Vec<&str> = messages
         .iter()
         .filter(|m| m.role == "system")
         .map(|m| m.content.as_str())
         .collect();
     let system_text = if collected.is_empty() {
         None
     } else {
         Some(collected.join("\n\n"))
     };
     ```
     替代方案（次选）：在 `assemble_messages_for_*` 内预合并多条 system 为单条 —— 但会让 messages 序列化语义偏离 ChatMessage 设计。**优先选 chat.rs 修复方案。**
   - **AC-0 验证**：
     - 新增 `chat.rs::tests::multiple_system_messages_are_joined_with_double_newline`，构造 3 条 system + 1 条 user 的 messages，断言传给 Anthropic 的 system 字段为 `"a\n\nb\n\nc"`
     - `cargo test --lib chat` 全绿
   - **AC-0 必须在 AC-1/2/3 改造前完成**（否则改造后端到端测试会拿到错的 system 内容）

1. **AC-1（llm_classify_with_db 改造）** — 修改 `src-tauri/src/commands/llm.rs::llm_classify_with_db`：
   - 删除内联的 `system / user` `ChatMessage` 手工构造
   - 改为：

     ```rust
     let messages = {
         let conn = database.conn.lock().map_err(|e| format!("数据库锁: {e}"))?;
         crate::llm::prompt_runtime::assemble_messages_for_classify(
             &conn,
             crate::llm::prompt_runtime::ClassifyVars { content }
         )?
     };
     let response = chat_completion(&client, messages).await?;
     parse_classify_response(&response)
     ```
   - 单测（如已有）保持 PASS；新增单测：mock `user_custom_prompt` 表插入自定义 tagging 段后，`assemble_messages_for_classify` 返回的 messages 在 user content 中包含该自定义文本

2. **AC-2（extract_concepts_for_library 改造）** — 修改 `src-tauri/src/commands/knowledge.rs::extract_concepts_for_library`（约 `:143-153`）：
   - 删除内联 `let prompt = build_extraction_prompt(...)` + 内联 `messages = vec![...]`
   - 改为：
     ```rust
     let messages = {
         let conn = db.conn.lock().map_err(|e| format!("数据库锁: {e}"))?;
         crate::llm::prompt_runtime::assemble_messages_for_concept(
             &conn,
             crate::llm::prompt_runtime::ConceptVars {
                 asset_name: asset_name.clone(),
                 project_name: project_name.clone(),
                 content: content_snippet.clone(),
             }
         )?
     };
     ```
   - 既有 `build_extraction_prompt` 函数保留为 deprecated（标 `#[allow(dead_code)]`），防止删除导致测试 import 失败
   - 既有日志 / 错误处理路径保留不动（包括 F-8 增量、共现计算等）

3. **AC-3（synthesize_viewpoints 改造）** — 修改 `src-tauri/src/commands/knowledge.rs::synthesize_viewpoints`（约 `:272-279`）：
   - 改造方式同 AC-2，使用 `assemble_messages_for_aggregation` + `AggregationVars`
   - `cases` 转换为 `cases_block: String`（按既有 `build_synthesis_prompt` 中循环拼接逻辑，写一个 helper `fn build_cases_block(cases: &[ConceptCase]) -> String`，可放在 `commands/knowledge.rs` 内私有）

4. **AC-4（不改造的调用清单 / 明确边界）** — 以下 LLM 调用**本期不动**，原因写入 commit message：
   - `llm_summarize`：不在 PRD 4 模块内
   - `llm_enhance_export`：不在 PRD 4 模块内
   - `knowledge::generate_extensions` 中 inline format prompt：当前不在 PRD 4 模块语义内，且改造成本不在本期 MVP 收益范围
   - `commands/knowledge_understanding.rs` 中 `build_summary_prompt / build_explanation_prompt / build_mirror_prompt`：属于"知识理解辅助层"，与 PRD 4 模块不重叠
   在本 task 的 output.md「已知局限」中显式列出以上 4 项

5. **AC-5（call-site 日志埋点）** — 每个被改造的调用点在调用 `chat_completion` 之前加一行 `log::info!`，格式：
   `log::info!("LLM call: module={module} bytes={total_bytes} user_overridden={is_custom}");`
   其中 `module` / `is_custom` / `total_bytes` 由 `assemble_messages_for_*` 在返回值之外通过新增的 `LlmCallContext` 结构同时返回（即 assemble 函数签名建议改为 `Result<(Vec<ChatMessage>, LlmCallContext), String>`，或单独 helper `inspect_messages(&messages)` 计算 bytes，二选一）
   - 单测：插入自定义 prompt 后调用 assemble 函数，返回的 context.is_custom == true；未自定义时 false

6. **AC-6（cargo test 全绿 + cargo build 通过）** — `cd src-tauri && cargo test --lib` 全表 PASS（不仅本 task 引入的，所有 inherited 测试均通过，预期 ≥ 330）；`cargo build` 通过；`commands/llm.rs:120` 的 deprecated warning 在 AC-1 改造后自动消失，**最终 cargo build 应不再有 deprecated warning**

7. **AC-7（迁移信号灯清理）** — `src-tauri/src/llm/prompt_runtime.rs` 中 3 个 `assemble_messages_for_*` 函数 doc 注释末尾各有一行 `// FIXME(task_004): chat.rs:58-66 ...` 注释（task_003 v2 植入）。task_004 完成 AC-0 + AC-1/2/3 后，**必须移除**这 3 行 FIXME 注释（迁移已完成）。

8. **AC-8（concept / aggregation system_addon 字面回归断言）** — 新增端到端断言测试：mock `chat_completion` 收到的 messages，验证：
   - concept 调用（task_003 选项 A 复刻）：system 字段（修 chat.rs 后的合并产物）必须**逐字包含** `"knowledge extraction engine"`
   - aggregation 调用：system 字段必须**逐字包含** `"knowledge synthesis engine"`
   - 这是对 task_003 Fix v2 "LLM 行为零差异"承诺的回归验证（task_003 MAJOR-1 选项 A 落地的等价性守护）

## 技术约束

- **不破坏 LLM 行为**：改造前后默认行为（is_custom=0 时）应与改造前**逐字符等价**或差异仅在 system message 排列顺序上。这需要 task_003 的 `default_for(module)` 字面值严格取自既有 prompts.rs / knowledge.rs；本 task 在 AC-1/2/3 完成后跑一次端到端比对（可在 e2e task_008 中再做严格比对）
- **不动并发与锁**：现有 `db.conn.lock()` 模式保留；不引入 RwLock 或异步锁
- **不删除既有 build_*_prompt 函数**：标 deprecated 即可，方便回退

## 参考文件

**必读**：
- Architect output.md `§ 4.3`（数据流详图）
- Architect output.md `§ 9 R8 / R9`
- task_003 input.md（理解 `assemble_messages_for_*` 设计）
- **task_003 output.md `§ 留给 task_004 的信号灯`（行 348-367）** — AC-0 / AC-7 / AC-8 的来源依据
- **task_003 output.md `§ 已知局限 6`（行 330-335）** — chat.rs:58-66 bug 现象与影响的完整说明

**代码参考（必读）**：
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/commands/llm.rs:94-126` — `llm_classify_with_db` 原文
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/commands/knowledge.rs:108-247` — `extract_concepts_for_library` 原文，含 F-8 增量逻辑
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/commands/knowledge.rs:254-294` — `synthesize_viewpoints` 原文

## 预估影响范围

- **修改文件**：
  - `src-tauri/src/llm/chat.rs`（AC-0 多 system 合并修复，~10 行 + 1 个测试 ~20 行）
  - `src-tauri/src/llm/prompt_runtime.rs`（AC-7 移除 3 行 FIXME 注释）
  - `src-tauri/src/commands/llm.rs`（`llm_classify_with_db` 改造，~20 行替换）
  - `src-tauri/src/commands/knowledge.rs`（两处改造 + `build_cases_block` helper，~80 行）
- **预估变更**：~430 行（含测试 ~180 行）
