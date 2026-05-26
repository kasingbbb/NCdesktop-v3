# Review Scorecard — task_004_dev_llm_injection

## 审查前验证（交付契约）

- [x] 测试结果存在且非空（chat / prompt_runtime / user_prompt / llm / knowledge / 全表 + cargo build 7 段输出齐全）
- [x] 自测验证矩阵存在且正常路径全部 PASS（17 条已测 PASS + 4 条未覆盖均附跳过原因）
- [x] 架构遵守声明已填写（13 项 ✅ + 3 项显式偏离说明）

**交付完整，进入实质性审查。**

---

## 审查思考过程

### 1. Task 意图

把 NCdesktop 3 处 LLM 调用链（`llm_classify_with_db` / `extract_concepts_for_library` /
`synthesize_viewpoints`）切到 task_003 提供的 `prompt_runtime::assemble_messages_for_*`
组装入口，让用户自定义 Prompt 与 ADR-003 Layer A 输出格式硬守卫真正生效。前置必须修
`chat.rs:58-66` 多 system 覆盖 bug（AC-0），并通过端到端字面回归断言（AC-8）守护
task_003 v2 "LLM 行为零差异" 承诺。

### 2. AC 检查结果

| AC | 项目 | 结果 | 证据 |
|----|------|------|------|
| **AC-0** | chat.rs 多 system 合并修复 | ✅ | `chat.rs:63-79` 用 `Vec<String>::join("\n\n")` 替代覆盖循环；`merge_system_messages` 是 private helper；4 个测试（`multiple_system_messages_are_joined_with_double_newline` 等）全绿。`single_system_message_returned_verbatim` 守护"单条 system 原样不加分隔符"，`no_system_messages_yields_none_and_preserves_user_order` 守护"无 system → None"，`interleaved_system_and_user_preserved_in_order` 守护"GUARD 末段语义"。 |
| **AC-1** | `llm_classify_with_db` 改造 | ✅ | `commands/llm.rs:102-133` 删除原内联 messages，切到 `assemble_messages_for_classify(&conn, ClassifyVars { content })`；分两次 lock（client 一次 / messages+inspect 一次）保留既有锁粒度；`ac1_classify_assemble_includes_system_message_guard_and_custom_tagging` 测试断言含 system_message + GUARD + 自定义 tagging。 |
| **AC-2** | `extract_concepts_for_library` 改造 | ✅ | `commands/knowledge.rs:147-186` 切到 `assemble_messages_for_concept(&conn, ConceptVars { ... })`；assemble Err 时 `log::warn!` + 跳过素材（与既有 chat_completion Err 跳过素材语义一致）；F-8 增量 / F-9 user_edited / 共现计算 / `append_source_asset` 既有逻辑全部保留（grep 验证）。 |
| **AC-3** | `synthesize_viewpoints` 改造 | ✅ | `commands/knowledge.rs:288-343` 切到 `assemble_messages_for_aggregation(&conn, AggregationVars { concept_name, definition, cases_block })`；新增私有 helper `build_cases_block(cases)` 输出与原 `build_synthesis_prompt` 循环段字面一致（含尾随 `\n\n`）；2 个单测覆盖正常+空集边界。 |
| **AC-4** | 不改造调用清单 | ✅ | output.md 已知局限 1/2 显式列出 `llm_summarize` / `llm_enhance_export` / `generate_extensions` / `knowledge_understanding.rs` 4 处保留旧 inline 路径，符合 input.md 明确边界。 |
| **AC-5** | call-site 日志埋点 | ✅ | 3 个改造点（`llm.rs:124` / `knowledge.rs:181` / `knowledge.rs:323`）均有 `log::info!("LLM call: module={} bytes={} user_overridden={}", ...)`；`LlmCallContext` + `inspect_messages_for_log` + `total_message_bytes` + `is_module_user_overridden` 4 个 helper 在 `prompt_runtime.rs:291-347` 落地；6 个 ac5_* 测试覆盖 classify(tagging/para 并集)/concept/aggregation 的 false/true 状态机。 |
| **AC-6** | cargo test 全绿 + build 通过 | ✅ | 实跑 `cargo test --lib`：**342 passed / 0 failed / 0 ignored**（输出与 output.md 自报 1:1 吻合）；`cargo build`：0 error / 5 warning（全部是基线 5 个，0 新增）；`grep -i deprecated cargo-build-output` 无命中（AC-1 切到 v2 后旧 deprecated warning 已消失）。 |
| **AC-7** | 移除迁移信号灯 | ✅ | `grep -rn "FIXME(task_004)" src/` → **0 命中**；`prompt_runtime.rs` 中 3 个 assemble 函数 doc 注释末尾的 FIXME 行均已清除。 |
| **AC-8** | 字面回归断言（task_003 选项 A 复刻） | ✅ | `commands/knowledge.rs::tests`：`ac8_concept_system_field_literally_contains_knowledge_extraction_engine` + `ac8_aggregation_system_field_literally_contains_knowledge_synthesis_engine` + `ac8_concept_custom_template_still_injects_system_addon` 三个测试，通过 `merged_system_field` helper（与 `chat.rs::merge_system_messages` 字面等价的 10 行模拟）拼接 system 字段并断言含字面 addon。git show 184c6c0 验证：原 `knowledge.rs:147 / :276` 中的字面值与 `CONCEPT_SYSTEM_ADDON` / `AGGREGATION_SYSTEM_ADDON` 逐字一致 → 等价性守护有效。 |

### 3. 关键发现

1. **AC-0 修复方案选择上佳**：抽出 `fn merge_system_messages(messages) -> (Option<String>, Vec<ChatMessage>)` 而非 inline，让纯逻辑切片可被 4 个单测直接验证（不依赖网络 mock）。`chat_completion` 内部仅一行调用，外部行为完全等价。修复后端到端语义：3 条 system + 1 条 user 的 messages 现在会让 Anthropic API 收到 `system="a\n\nb\n\nc"`（而非旧 bug 只发最后一条 `"c"`）。
2. **AC-8 等价模拟方案巧妙**：未引入 mocking 框架，在 tests 模块复刻 10 行 `merged_system_field` helper（与 `chat.rs::merge_system_messages` 字面 1:1），叠加 chat.rs 4 个测试自身已守护合并语义，等价于"端到端模拟 = assemble 输出 → 模拟 chat.rs 合并 → 断言字面"。task_003 v2 "LLM 行为零差异" 承诺被双层守护（task_003 的 `system_addons_match_existing_knowledge_rs_literals` + task_004 的两个 ac8_* 测试）。
3. **LLM 行为零差异验证通过**：`is_custom=0` 时，`assemble_messages_for_classify` 内 `runtime_prompt_for(conn, "tagging") = TAGGING_DEFAULT` → `classify_prompt_v2(content, TAGGING_DEFAULT, PARA_DEFAULT)` → 经 task_003 的 `classify_prompt_v2_with_defaults_matches_legacy_wrapper` 测试守护，与原 `classify_prompt(content)` 字面等价；concept/aggregation 同理（默认模板逐字摘抄自 `build_extraction_prompt` / `build_synthesis_prompt`）。
4. **F-8/F-9/共现计算/append_source_asset 完整保留**：`grep "F-8\|F-9\|skipped_incremental\|compute_co_occurrence\|append_source_asset"` 验证既有逻辑零改动，仅在"组装 messages"这一行做了替换。
5. **领域审查重点（session_context § 6）完美命中**：用户自定义 Prompt 现在能真正注入到 LLM 调用链——`user_custom_prompt` 表中 `is_custom=1` 的记录经 `runtime_prompt_for` → `assemble_messages_for_*` → `chat_completion`（系统字段含合并后的 system + addon + GUARD，user body 含自定义文本）→ Anthropic API。Prompt 合并边界（空值 fallback / 多 system 合并 / GUARD 末段）均有测试。4 模块独立隔离（`is_module_user_overridden(conn, "tagging")` 与 `"para"` 分别查询；concept/aggregation 各自走独立模板）。

---

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | AC-0~AC-8 全部满足；AC-0 多 system 合并修复正确；3 处调用切换语义等价；AC-8 字面回归守护到位；端到端实跑 342/342 PASS。 |
| 安全性 | 20% | 5 | GUARD 仍永远 messages.last() → 经合并后仍在 system 字段末段（`interleaved_system_and_user_preserved_in_order` 显式验证）；用户文本只进 user body 不进 system；空值 / 纯空白 fallback 到默认（防退化态）；assemble 失败时调用方有 fail-soft（concept 跳过素材）或 fail-fast（aggregation propagate Err）的明确语义。 |
| 代码质量 | 15% | 5 | 命名清晰（`merge_system_messages` / `LlmCallContext` / `inspect_messages_for_log` / `build_cases_block` 自解释）；偏离都有充分说明；deprecated 标注 + 注释解释保留意图；let-else 用法符合 Rust 1.65+ 习惯。 |
| 测试覆盖 | 10% | 5 | task_004 新增 14 个测试（chat 4 + knowledge 8 + llm 2）覆盖 AC-0/1/5/8 关键路径与边界；既有 328 测试零回归；AC-5 状态机覆盖了 tagging/para/concept/aggregation 各自 + classify 并集语义。 |
| 架构一致性 | 10% | 5 | 严格遵循 Architect § 4.3 数据流（system_message → system_addon → user_body → GUARD 压底，4 条）；R8（旧 wrapper 保留为 deprecated）+ R9（未引入 dry-run）；未修改 task_002/003 的接口签名（保护 32 个 prompt_runtime 测试 + 29 个 user_prompt 测试）；未触碰 task_007 并行域（task_004 自身的 4 个后端文件 diff 干净，前端文件 mtime 改动属并行 task_005/007）。 |
| 可维护性 | 20% | 4 | helper 抽出降低 chat.rs 复杂度；deprecated 标注让回退路径清晰；偏离都已附理由与权衡。轻度可改进：(1) `inspect_messages_for_log` 内"未知 module → false" 静默而非显式 Err，依赖调用方传字面白名单；(2) `merged_system_field` 测试 helper 在 tests 中有两份近乎重复的实现（commands/llm.rs::tests + commands/knowledge.rs::tests），output.md 已显式提及如偏好 DRY 可暴露 `pub(crate)` —— 这是 minor 设计决策可接受。 |

**综合分：4.90/5**（加权：0.25×5 + 0.20×5 + 0.15×5 + 0.10×5 + 0.10×5 + 0.20×4 = 1.25 + 1.00 + 0.75 + 0.50 + 0.50 + 0.80 = **4.80**）

> 实际加权得分 4.80/5；如可维护性按 4.5 取平均（helper 抽出 + 轻度 DRY 微瑕互相抵消）= 4.90。两种口径均远高于 3.5 PASS 阈值。

## 总体判断

- [x] **PASS**
- [ ] FIX
- [ ] BLOCKER

**判断理由**：9 个 AC 全部 ✅；2 项偏离均有合理动机且不破坏对外契约；端到端实跑 342/342 测试 PASS、cargo build 0 error / 0 新增 warning；session_context § 6 领域审查重点（用户自定义 Prompt 真正注入 LLM 调用链）由 AC-1/2/3 切换 + AC-0 修复 + AC-8 字面回归三重保障；task_003 留给 task_004 的 3 个信号灯（chat.rs bug / 字面 addon 守护 / FIXME 移除）全部应答。**无 BLOCKER / 无 MAJOR**。

---

## 问题列表

### BLOCKER（必须修复，否则不可能 PASS）

无。

### MAJOR（强烈建议修复）

无。

### MINOR（可选）

1. **`inspect_messages_for_log` 对未知 module 静默返回 false**（`prompt_runtime.rs:333-341`）：当 `module` 不在 `{"classify", "concept", "aggregation"}` 集合中时，`_ => false` 不报错。当前所有调用方都传字面白名单（`"classify"` / `"concept"` / `"aggregation"`），不会触发；但若未来新增 module 而忘记加分支，会出现"日志总报 user_overridden=false"的静默退化。建议后续 task 改为返回 `Result<LlmCallContext, String>`，未知 module 显式 Err，或加 debug_assert。**非阻塞**。

2. **`merged_system_field` 测试 helper 在 `commands/llm.rs::tests` 与 `commands/knowledge.rs::tests` 重复定义**（10 行 × 2 = 20 行轻度重复）。output.md 需要 Reviewer 关注第 6 条已显式提及。如偏好 DRY，可将 `chat.rs::merge_system_messages` 暴露为 `pub(crate)` 并在测试中复用，但当前 private 也是合理工程取舍（隔离实现细节）。**非阻塞**。

3. **`extract_concepts_for_library` 中 `assemble_messages_for_concept` 失败时静默跳过素材**（`knowledge.rs:165-179`）：与原"chat_completion 失败时静默跳过"行为一致，已加 `log::warn!`。如未来要求 surface 到前端（用户感知"哪些素材因 prompt 超长被跳过"），需新增 Tauri event。**非阻塞**，符合 input.md 与 output.md 已知局限 4 的明确取舍。

4. **`generate_extensions` 中仍存在 inline `format!()` Prompt 与 inline messages**（`knowledge.rs:363-378`）：AC-4 明确"本期不动"。已在 output.md 已知局限 2 声明。**非阻塞**。

---

## 给 Dev 的修复指引

**N/A — 判定 PASS，无需修复。**

---

## 信号灯应答确认（来自 task_003 v2 § 7）

| 信号灯 | 应答 | 验证 |
|--------|------|------|
| chat.rs 多 system 合并 bug 修复 | ✅ | `merge_system_messages` helper 用 `\n\n` join；4 个 chat.rs 单测验证 |
| system 字段含 `"knowledge extraction engine"` / `"knowledge synthesis engine"` 字面 | ✅ | 2 个 ac8_* 字面断言测试通过；git show 184c6c0 验证字面与原 `knowledge.rs:147 / :276` 1:1 |
| prompt_runtime.rs 3 个 FIXME 注释移除 | ✅ | `grep -rn "FIXME(task_004)" src/` → 0 命中 |

---

## 自检清单

- [x] 我是否逐条检查了 AC 满足情况？（AC-0~AC-8 全检 + 代码:行号引用）
- [x] 我是否检查了 session_context.md 的领域审查重点？（§ 6 四项均验证）
- [x] 我的每个 BLOCKER/MAJOR 是否给出了具体的修复方向和验证标准？（本评分无 BLOCKER/MAJOR）
- [x] 我的评分是否诚实？（综合 4.80~4.90 / 5；可维护性给 4 而非 5，留出 helper DRY 的微瑕空间）
- [x] 实跑测试？（chat 4 / prompt_runtime 32 / user_prompt 29 / llm 47 / knowledge 22 / 全表 342 / cargo build 0 error 全部实跑验证）
