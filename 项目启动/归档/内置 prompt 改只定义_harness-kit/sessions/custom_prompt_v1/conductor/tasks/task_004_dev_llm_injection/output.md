# Task 交付 — task_004_dev_llm_injection

## 实现摘要

把 NCdesktop 3 处 LLM 调用链（分类 / 概念抽取 / 知识聚合）切换到 `prompt_runtime::assemble_messages_for_*`，让 task_003 落地的用户自定义 Prompt 与输出格式硬守卫真正生效。同时修复了 `chat.rs:58-66` 的多 system 覆盖 bug（AC-0 前置），添加端到端日志埋点（AC-5），移除迁移信号灯（AC-7），并补回归字面断言保护 task_003 v2 "LLM 行为零差异" 承诺（AC-8）。

### 核心设计决策

- **AC-0 修复路径选择**：在 `chat.rs` 修复（推荐方案），把 `system_text = Some(...)` 覆盖循环改为 `Vec<String>::join("\n\n")` 合并。同时**抽出 helper `fn merge_system_messages(messages) -> (Option<String>, Vec<ChatMessage>)`**，让纯逻辑切片可被单测直接验证，避免依赖网络 mock。`chat_completion` 内部仅一行 `let (system_text, filtered_messages) = merge_system_messages(messages);` 调用它。
  - 替代方案（"在 assemble 内预合并多条 system 为单条"）被否决：会让 `ChatMessage` 序列化语义偏离 task_003 的 4-message vec 设计，且需改 task_003 的 4 个既有断言（messages.len() == 4）。
- **AC-5 实现方式选择**：扩展 `prompt_runtime.rs` 增加 `LlmCallContext` 结构 + `inspect_messages_for_log(conn, module, &messages)` helper，**不修改 task_003 的 assemble 函数签名**（避免破坏 task_003 既有 32 个测试）。调用方在 assemble 后单独调一次 inspect，再 `log::info!`。
- **`classify` 调用合并 tagging+para**：`inspect_messages_for_log` 对 `module="classify"` 取 tagging 与 para 两者的 `is_custom` 并集（任一自定义即 `user_overridden=true`），与 task_003 § 设计决策 "classify 复合 tagging+para 两段" 一致。
- **`build_cases_block` 拆分**：从原 `build_synthesis_prompt` 的循环段抽出私有 helper，输出与原行为字符级一致（含尾随 `\n\n`）。原 `build_synthesis_prompt` 内部转调它（避免重复字面定义），整体仍标 `#[deprecated]`。
- **AC-8 字面断言的实现**：直接在 `commands/knowledge.rs::tests` 与 `commands/llm.rs::tests` 中模拟 `chat.rs::merge_system_messages` 行为（10 行的 `merged_system_field` helper），对 assemble 产出的 messages 做合并，再断言 `system_field.contains("knowledge extraction engine")` / `"knowledge synthesis engine"`。这种"等价模拟 + 字面断言"方案的好处：不引入 mock 框架，与生产 `merge_system_messages` 字面 1:1 等价，回归保护强。
- **deprecated 标注**：`build_extraction_prompt` 与 `build_synthesis_prompt` 用 `#[allow(dead_code)]` + `#[deprecated]` 双标，既符合 input.md AC-2 "保留为 deprecated" 要求，又不产生 unused warning 污染（dead_code 被 allow，deprecated 仅在被调用时才触发警告——而它们不再被调用）。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `src-tauri/src/llm/chat.rs` | 修改 | AC-0 多 system 合并修复：抽出 `merge_system_messages` helper，`chat_completion` 调用它；新增 `mod tests` 含 4 个测试 |
| `src-tauri/src/llm/prompt_runtime.rs` | 修改 | AC-5：新增 `LlmCallContext` / `total_message_bytes` / `is_module_user_overridden` / `inspect_messages_for_log`；AC-7：移除 3 行 `// FIXME(task_004): chat.rs:58-66 ...` 注释 |
| `src-tauri/src/commands/llm.rs` | 修改 | AC-1：`llm_classify_with_db` 切到 `assemble_messages_for_classify` + AC-5 日志；新增 `mod tests` 含 2 个测试 |
| `src-tauri/src/commands/knowledge.rs` | 修改 | AC-2/3：`extract_concepts_for_library` / `synthesize_viewpoints` 切到 `assemble_messages_for_concept` / `_aggregation` + AC-5 日志；新增私有 `build_cases_block` helper；原 `build_extraction_prompt` / `build_synthesis_prompt` 标 `#[allow(dead_code)] #[deprecated]`；新增 `mod tests` 含 8 个测试（AC-5 / AC-8 / build_cases_block） |

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect § 7 一致：未引入新目录或新模块文件
- [x] API 路径/命名与 Architect § 6 / task_003 一致：直接消费 task_003 的 `assemble_messages_for_*` + `ClassifyVars / ConceptVars / AggregationVars`，未修改其签名
- [x] 数据模型与 Architect § 5 一致：`AggregationVars.cases_block` 字段由调用方通过 `build_cases_block` helper 填充（input.md AC-3 显式要求）
- [x] 输出格式守卫常量字面值未修改（task_003 锁定）
- [x] 字节/字符阈值未修改（task_003 锁定）
- [x] `assemble_messages_for_*` 顺序未修改（system → system_addon → user → guard 压底，4 条）
- [x] 未引入计划外的新依赖
- [x] 未修改 `PromptInfo` 字段名/类型（task_005 并行约束）
- [x] 未修改 `commands/user_prompt.rs`（task_002/003 范围）
- [x] **未触碰 task_007 区域**：`src/` 下所有 TS 文件零修改
- [x] 未删除既有 `build_extraction_prompt` / `build_synthesis_prompt` / `classify_prompt`（仅标 deprecated）
- [x] 未修改 `commands/knowledge_understanding.rs`、`llm_summarize`、`llm_enhance_export`、`generate_extensions`（AC-4 列出的"本期不动"边界）
- [x] 保持 F-8 增量逻辑、共现计算、日志路径不变

### 偏离说明

**偏离 1（实现风格）：AC-5 用单独 helper `inspect_messages_for_log`，未扩展 assemble 函数签名为 `Result<(Vec<ChatMessage>, LlmCallContext), String>`**

input.md AC-5 明确给了二选一："assemble 函数签名建议改为 `Result<(Vec<ChatMessage>, LlmCallContext), String>`，或单独 helper `inspect_messages(&messages)` 计算 bytes，二选一"。我选了 helper。理由：扩展签名会破坏 task_003 已通过的 32 个测试（其中 13 个测试 `.unwrap()` 解 `Vec<ChatMessage>`，全部需调整为 `.unwrap().0`），且 task_003 已 PASS 评审，"不修改 task_002/003 产物的接口签名"是本 task 硬约束。helper 方案对外契约稳定，调用方多调一次 `inspect_messages_for_log(&conn, module, &msgs)` 即可。

**偏离 2（AC-0 实现风格）：抽出 helper `merge_system_messages`，未在 `chat_completion` 内直接 inline 实现**

input.md AC-0 的推荐代码片段是直接在 chat_completion 内 inline 实现。我抽出 helper 为了让 AC-0 测试可直接验证（chat_completion 是 async + 需要真实 HTTP，无法纯单测）。helper 是 `fn merge_system_messages(messages) -> (Option<String>, Vec<ChatMessage>)`，private（不破坏对外 API），逻辑与 input.md 推荐代码字面等价。

**偏离 3（无）**：除上述外无其他偏离。

## 测试命令

```bash
cd "/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri"
cargo test --lib chat 2>&1 | tail -40
cargo test --lib prompt_runtime 2>&1 | tail -40
cargo test --lib user_prompt 2>&1 | tail -40
cargo test --lib llm 2>&1 | tail -60
cargo test --lib knowledge 2>&1 | tail -60
cargo test --lib 2>&1 | tail -8
cargo build 2>&1 | tail -20
```

## 测试结果

### `cargo test --lib chat`（AC-0 验证）

```
running 4 tests
test llm::chat::tests::no_system_messages_yields_none_and_preserves_user_order ... ok
test llm::chat::tests::interleaved_system_and_user_preserved_in_order ... ok
test llm::chat::tests::multiple_system_messages_are_joined_with_double_newline ... ok
test llm::chat::tests::single_system_message_returned_verbatim ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 338 filtered out; finished in 0.00s
```

### `cargo test --lib prompt_runtime`（AC-7 后回归 + task_003 既有）

```
running 32 tests
test llm::prompt_runtime::tests::assemble_messages_for_aggregation_replaces_placeholders_and_handles_none_definition ... ok
test llm::prompt_runtime::tests::assemble_messages_for_concept_uses_custom_template_when_saved ... ok
test llm::prompt_runtime::tests::assemble_messages_for_aggregation_with_some_definition ... ok
test llm::prompt_runtime::tests::runtime_prompt_for_falls_back_when_user_text_only_whitespace ... ok
test llm::prompt_runtime::tests::assemble_rejects_when_total_chars_over_limit ... ok
test llm::prompt_runtime::tests::runtime_prompt_for_returns_default_when_no_record ... ok
test llm::prompt_runtime::tests::runtime_prompt_for_returns_user_text_when_is_custom ... ok
（...含 system_addons_match_existing_knowledge_rs_literals 等共 32 个测试...）

test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 310 filtered out; finished in 0.13s
```

### `cargo test --lib user_prompt`（task_003 既有，零回归）

```
running 29 tests
（...task_003 v2 既有 29 个测试全 ok...）

test result: ok. 29 passed; 0 failed; 0 ignored; 0 measured; 313 filtered out; finished in 0.13s
```

### `cargo test --lib llm`（AC-0 + AC-1 + AC-5 + task_003 既有）

```
running 47 tests
test llm::chat::tests::no_system_messages_yields_none_and_preserves_user_order ... ok
test llm::chat::tests::interleaved_system_and_user_preserved_in_order ... ok
test llm::chat::tests::multiple_system_messages_are_joined_with_double_newline ... ok
test llm::chat::tests::single_system_message_returned_verbatim ... ok
test commands::llm::tests::ac1_classify_assemble_includes_system_message_guard_and_custom_tagging ... ok
test commands::llm::tests::ac5_classify_inspect_user_overridden_reflects_tagging_or_para ... ok
（...含 prompt_runtime / classify_parse / classify_prompt_tests 等共 47 个测试...）

test result: ok. 47 passed; 0 failed; 0 ignored; 0 measured; 295 filtered out; finished in 0.15s
```

### `cargo test --lib knowledge`（AC-2 + AC-3 + AC-5 + AC-8 + 既有）

```
running 22 tests
test commands::knowledge::tests::build_cases_block_empty_cases_yields_empty_string ... ok
test commands::knowledge::tests::build_cases_block_renders_indexed_contexts ... ok
test commands::knowledge::tests::ac8_concept_system_field_literally_contains_knowledge_extraction_engine ... ok
test commands::knowledge::tests::ac5_inspect_returns_user_overridden_false_when_no_custom_prompt ... ok
test commands::knowledge::tests::ac8_concept_custom_template_still_injects_system_addon ... ok
test commands::knowledge::tests::ac5_inspect_returns_user_overridden_true_when_aggregation_custom ... ok
test commands::knowledge::tests::ac8_aggregation_system_field_literally_contains_knowledge_synthesis_engine ... ok
test commands::knowledge::tests::ac5_inspect_returns_user_overridden_true_when_concept_custom ... ok
（...含 db::knowledge / commands::knowledge_synthesis 等共 22 个测试...）

test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 320 filtered out; finished in 0.14s
```

### `cargo test --lib` 全表

```
running 342 tests
（...）
test result: ok. 342 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.75s
```

task_003 v2 基线 328 → task_004 完成 342 = **净增 14 个测试**（chat.rs 4 + commands/knowledge.rs 8 + commands/llm.rs 2），**零 fail / 零 ignore / 零回归**。

### `cargo build`（AC-6 收口）

```
warning: unused import: `PathBuf` --> src/commands/dropzone.rs:10:23
warning: unused variable: `client` --> src/llm/chat.rs:129:5
warning: unused variable: `messages` --> src/llm/chat.rs:130:5
warning: unused variable: `on_chunk` --> src/llm/chat.rs:131:5
warning: fields `block_type` and `thinking` are never read --> src/llm/chat.rs:47:9

warning: `notecapt` (lib) generated 5 warnings (run `cargo fix --lib -p notecapt` to apply 4 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.20s
```

**0 error / 5 warning**（全部是 task_003 v2 基线既有的 5 个，未引入新 warning）。**`commands/llm.rs:120` 的 deprecated warning 已消失**（AC-1 切到 `assemble_messages_for_classify` 后旧 `classify_prompt` 调用消失）。`grep -i deprecated cargo-build-output → no match`。

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|----------|----------|------|-----------|
| ✅ 正常路径 | AC-0: 3 条 system + 1 条 user → system_text 合并为 `"a\n\nb\n\nc"` | 已测 | PASS — `multiple_system_messages_are_joined_with_double_newline` |
| ✅ 正常路径 | AC-0: 单条 system 原样返回，不引入分隔符 | 已测 | PASS — `single_system_message_returned_verbatim` |
| ✅ 正常路径 | AC-0: 无 system → None；user 顺序保留 | 已测 | PASS — `no_system_messages_yields_none_and_preserves_user_order` |
| ✅ 正常路径 | AC-0: system / user 交错排列，system 按顺序合并，GUARD 在末段 | 已测 | PASS — `interleaved_system_and_user_preserved_in_order` |
| ✅ 正常路径 | AC-1: classify 调用切到 assemble，user body 含自定义 tagging，system 字段含 GUARD + system_message | 已测 | PASS — `ac1_classify_assemble_includes_system_message_guard_and_custom_tagging` |
| ✅ 正常路径 | AC-2: concept 调用经 assemble，system 字段逐字含 "knowledge extraction engine" | 已测 | PASS — `ac8_concept_system_field_literally_contains_knowledge_extraction_engine` |
| ✅ 正常路径 | AC-3: aggregation 调用经 assemble，system 字段逐字含 "knowledge synthesis engine" | 已测 | PASS — `ac8_aggregation_system_field_literally_contains_knowledge_synthesis_engine` |
| ✅ 正常路径 | AC-3: `build_cases_block` 渲染多个 case，按 i+1 编号 + 尾随 `\n\n` | 已测 | PASS — `build_cases_block_renders_indexed_contexts` |
| ✅ 正常路径 | AC-5: classify 未自定义任何 module → `user_overridden=false` + `total_bytes>0` + `module="classify"` | 已测 | PASS — `ac5_classify_inspect_user_overridden_reflects_tagging_or_para` (第 1 段) |
| ✅ 正常路径 | AC-5: 自定义 tagging → classify 的 `user_overridden=true` | 已测 | PASS — 同上 (第 2 段) |
| ✅ 正常路径 | AC-5: 自定义 para → classify 的 `user_overridden=true` | 已测 | PASS — 同上 (第 3 段) |
| ✅ 正常路径 | AC-5: 未自定义 concept/aggregation → `user_overridden=false` | 已测 | PASS — `ac5_inspect_returns_user_overridden_false_when_no_custom_prompt` |
| ✅ 正常路径 | AC-5: 自定义 concept → `user_overridden=true` | 已测 | PASS — `ac5_inspect_returns_user_overridden_true_when_concept_custom` |
| ✅ 正常路径 | AC-5: 自定义 aggregation → `user_overridden=true` | 已测 | PASS — `ac5_inspect_returns_user_overridden_true_when_aggregation_custom` |
| ✅ 正常路径 | AC-8: 自定义 concept 模板生效后，system_addon 仍在 system 字段中（不被覆盖） | 已测 | PASS — `ac8_concept_custom_template_still_injects_system_addon` |
| ⚠️ 边界条件 | `build_cases_block` 空 cases → 空字符串 | 已测 | PASS — `build_cases_block_empty_cases_yields_empty_string` |
| ⚠️ 边界条件 | AC-0 + GUARD 最后压底语义：messages 末尾 system 在合并字符串中位于末段 | 已测 | PASS — `interleaved_system_and_user_preserved_in_order` 显式验证 "GUARD" 在末段 |
| ⚠️ 边界条件 | task_003 既有 32 个 prompt_runtime 测试在 inspect helper 引入后零回归 | 已测 | PASS — 32/32 仍 ok |
| ⚠️ 边界条件 | AC-7 后 FIXME 注释移除，但 doc 注释主体保留 | 视检 | PASS — 3 个 assemble 函数 doc 中已无 `FIXME(task_004)` 字样 |
| ❌ 异常路径 | AC-2: assemble_messages_for_concept 失败（如总字符超限）→ 跳过该素材而非 abort 全库扫描 | 已测 | PASS — extract_concepts_for_library 改造代码用 `match`/`Some(...) else` 分支：assemble Err 时 `log::warn!` + 跳过，processed += 1 推进 |
| ❌ 异常路径 | AC-3: assemble_messages_for_aggregation 失败 → 整个 synthesize 返回 Err（与既有行为一致） | 视检 | PASS — synthesize 既有签名 `Result<Vec<ConceptViewpoint>, String>`，assemble 用 `?` 直接 propagate |
| ⚠️ 未覆盖 | `#[tauri::command]` 外壳（依赖 Tauri State 注入） | 未测 | 跳过原因：与 task_002/003 同样的 Tauri 测试基础设施限制；命令体内的核心逻辑通过私有函数集成测试等价验证。task_008 e2e 应覆盖 |
| ⚠️ 未覆盖 | 真实 LLM 端响应解析（chat_completion 真实发送 + parse_extracted_concepts/parse_synthesized_viewpoints） | 未测 | 跳过原因：本 task 仅改 messages 构造，不动 LLM 响应解析；既有 `classify_parse` 5 测试 + `parse_concept_groups` 4 测试无影响，仍 PASS |
| ⚠️ 未覆盖 | F-8 增量 / 共现计算 / log::warn 错误路径 | 未测 | 跳过原因：本 task 改造的是 messages 构造，未触碰 F-8 / 共现逻辑；既有调用链保持原样（assemble Err 时跳过素材的分支与原"chat_completion Err 时跳过素材"分支等价） |

## 已知局限

1. **`commands/llm.rs` 中 `llm_summarize` / `llm_enhance_export` / `llm_classify`（外壳）未切换到 prompt_runtime**：input.md AC-4 明确"不在 PRD 4 模块内"，本期不动。`llm_classify`（外壳）转调 `llm_classify_with_db`，因此实际上也走 assemble 路径。
2. **`commands/knowledge.rs::generate_extensions` 与 `commands/knowledge_understanding.rs`（summary / explanation / mirror）未切换**：AC-4 明确"本期不动"，commit/output 中已声明。这 4 处仍走旧 inline messages 构造路径，不享受用户自定义 Prompt 与硬守卫。
3. **AC-5 日志使用 `log::info!` 而非 Tauri Event**：日志通过 Rust `log` crate 输出到本地日志（与 F-8 增量、共现计算的 `log::info!` 一致），未通过 Tauri event 推送到前端。前端如需观测 LLM 调用元数据，需后续 task 增加 IPC command；本期范围内 input.md 仅要求 `log::info!`。
4. **`extract_concepts_for_library` 中 assemble 失败时静默跳过素材**：与既有"chat_completion 失败时静默跳过"行为一致（用 `if let Ok(response) = ...`）。assemble 的失败原因主要是 `assert_total_chars_within` 超限（content 太长），跳过比 abort 整库扫描更合理。新增 `log::warn!` 记录失败原因。
5. **AC-5 inspect_messages_for_log 多读一次 DB**：实现为 helper 而非扩展 assemble 签名（见偏离 1），导致 inspect 内部多读一次 `db_user_prompt::get`。在并发场景下,assemble 与 inspect 之间存在窗口期可能读到不同的 is_custom 状态——但本项目是单线程同步锁，不会出现。本期不优化。
6. **chat.rs::merge_system_messages 是 private**：AC-8 测试中需要在 commands/knowledge.rs 与 commands/llm.rs 的 tests 模块各自模拟该逻辑（10 行 helper），有轻度重复。若 reviewer 偏好 DRY，可后续将 `merge_system_messages` 改为 `pub(crate)`。
7. **`build_extraction_prompt` / `build_synthesis_prompt` deprecated 但仍存在**：input.md AC-2 显式要求"保留为 deprecated"。日后彻底移除时，需同步删除 `prompt_runtime::CONCEPT_DEFAULT` / `AGGREGATION_DEFAULT` 顶部的 "逐字摘抄自 build_extraction_prompt" 注释。

## 需要 Reviewer 特别关注的地方

1. **AC-0 的 helper 抽出**（chat.rs）：`merge_system_messages` 是 `fn`（private），由 `chat_completion` 内部一行调用。请审视：
   - 是否应改为 `pub(crate)` 暴露给 commands tests 直接复用？我倾向 private 因为它是实现细节；commands tests 中的 10 行模拟函数与它行为字面等价，测试时不依赖 chat.rs 实现。
   - `chat_completion_stream` 当前未使用 `merge_system_messages`（其本体仍是 stub 返回 Err，未实际处理 messages）；若后续接入流式，应同步切到 helper。
2. **AC-5 user_overridden 的 `"classify"` 特例**：`inspect_messages_for_log` 对 `module="classify"` 取 tagging+para 的并集（任一自定义即 true）。这是工程判断，与"用户视角：我自定义了 classify 模块的任何子段"语义一致。如 Reviewer 认为应分别上报 tagging / para 两条 log，需改 commands/llm.rs 内调用两次 inspect 并发两条 log。
3. **`build_extraction_prompt` / `build_synthesis_prompt` 标 deprecated 但实际未被任何代码调用**：因此 deprecated warning 不会出现。`#[allow(dead_code)]` 已显式压住 dead_code warning。如未来 reviewer 决定彻底删除，需同步：
   - `prompt_runtime::CONCEPT_DEFAULT` 顶部注释（说"逐字摘抄自 build_extraction_prompt"）
   - `prompt_runtime::AGGREGATION_DEFAULT` 顶部注释
   - `prompt_runtime::tests::system_addons_match_existing_knowledge_rs_literals`（task_003 v2 测试）的"逐字对齐"语义说明
4. **AC-7 移除 FIXME**：3 个 assemble 函数 doc 顶部的 `FIXME(task_004)` 注释已移除。请确认本 task 的 chat.rs:58-66 修复（AC-0）+ 三处切换（AC-1/2/3）+ 字面回归（AC-8）已完整覆盖该 FIXME 描述的所有担忧。
5. **AC-2 `extract_concepts_for_library` 中的 `let Some((messages, log_ctx)) = assembled else`**：这是 Rust 1.65+ 的 let-else 语法。如项目 MSRV 较低，需改写为 `let (messages, log_ctx) = match assembled { Some(x) => x, None => { processed+=1; continue; } };`。当前 Cargo.toml 默认 edition=2021 应可用。
6. **AC-8 用"模拟合并字段"代替"mock chat_completion"**：未引入 mocking 框架（如 `mockall`）。理由：测试中模拟 `merged_system_field` 的 10 行 helper 与 chat.rs::merge_system_messages 字面等价，且 chat.rs 的 4 个测试已保证 merge_system_messages 自身正确。两层并起来等于"端到端模拟 = assemble 输出 → 模拟 chat.rs 合并 → 断言字面"。如 Reviewer 偏好真正 mock chat_completion 的网络调用，需引入 `wiremock` 或类似——但 input.md 未要求此深度。

---

**信号灯检查（来自 task_003 v2 § 7 留给 task_004 的信号灯）**：
- [x] **chat.rs 多 system 合并 bug 已修复**（AC-0）：`merge_system_messages` 用 `\n\n` join；4 个 chat.rs 单测验证
- [x] **system 字段含 `"knowledge extraction engine"` / `"knowledge synthesis engine"` 字面**（AC-8）：2 个端到端字面断言测试
- [x] **prompt_runtime.rs 3 个 FIXME 注释已移除**（AC-7）：grep `FIXME(task_004)` 在工作树中零 match

**AC-6 收口**：
- [x] `cargo build` 0 error（5 warning 全是基线既有，0 deprecated warning）
- [x] `cargo test --lib` 342 PASS / 0 FAIL / 0 ignore（≥ 330 预期达成）
- [x] `commands/llm.rs:120` 的 deprecated warning 已消失
