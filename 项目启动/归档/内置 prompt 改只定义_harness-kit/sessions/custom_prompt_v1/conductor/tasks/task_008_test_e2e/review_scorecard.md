# Review Scorecard — task_008_test_e2e

## 审查思考过程

### 1. Task 意图

为"用户自定义 Prompt 功能"端到端流水线（task_002~007）落地集成测试与手动验收清单，覆盖六类核心场景（正常注入 / 占位符 / 字节超限保存层 + 调用前层 / R1 对抗式 / 一键恢复 / 4 module 独立 / R3 兼容），并形成手动 e2e checklist 供 PM 验收 mock 不可达场景（真实 LLM 行为 + 跨重启持久化）。

### 2. AC 检查结果（逐条）

- ✅ **AC-1（Rust 端到端集成测试）** —— `src-tauri/tests/user_prompt_e2e.rs`（768 行，20 测试）全绿。覆盖 input.md 列出的全部 6 子场景（正常路径 / fallback / 占位符校验 / 字节超限 / R1 对抗 / 一键恢复），加 4 module 独立 + R3 兼容 + whitespace fallback + list_all 共 8 大场景类。每个测试有真实断言（不是占位）：
  - 正常路径：3 个 module 各一个（classify / concept / aggregation）+ fallback，断言 messages[2] 含用户文本字面、默认 TAGGING_DEFAULT 标志字面已被替换、`messages.last() == *_OUTPUT_GUARD`。
  - 字节超限保存层：`e2e_save_over_16kib_byte_is_rejected_with_chinese_message`（user_prompt_e2e.rs:297-316）含 16 KiB+1 拒绝 + DB 未更新 + 恰好 16 KiB 通过的**双向边界断言**。
  - 字节超限调用前层：classify / concept 两个 module 各一个，验证 `MAX_TOTAL_PROMPT_CHARS+1` 触发"LLM 请求过长"中文错误。
  - R1 对抗式：详见关键发现 1。
  - 占位符校验：concept 缺 `{content}` + aggregation 缺 `{concept_name}` + tagging/para 无强制占位符，前两者断言 DB 未更新。
  - 一键恢复：reset(Some(m)) 单条 + reset(None) 全部，后者 for 循环验证 4 module 全部回 `default_for(m)`。
  - 4 module 独立：双重测试（保存 tagging 不污染 concept/agg/para + 4 module 各自 marker 不串扰）。
  - R3 兼容：`UPDATE user_custom_prompt SET builtin_version='1.1'` 模拟升级，验证 is_custom=1 行用户文本不变，is_custom=0 行回退默认。
- ⚠️ **AC-2（前端集成测试 `userPromptFlow.integration.test.tsx`）** —— **Dev 已偏离，未实现**。Dev 论证：task_006（20 store 测试）+ task_007（23 panel 测试）已覆盖 AC-2 列出的 7 个 UI 行为；SettingsPanel.test.tsx 已 mock PromptCustomizationPanel 并验证 Tab 切换。**Reviewer 评估**：论证基本成立，task_007 panel 测试确实是"渲染 4 条折叠 + 占位符警告 + saveButton disabled + saveUserPrompt 调用参数 + resetUserPrompt 单条/全部" 的等价覆盖；多写一份 SettingsPanel 包裹层的 vitest 集成本质是"双重 mock"（mock invoke + 真实 store + 真实 panel + 真实 SettingsPanel），增量价值仅"SettingsPanel Tab 切换后 panel 能挂载"——SettingsPanel.test.tsx 已断言。该偏离属于工程判断合理，但**严格按 input.md 字面要求**确实少了一份独立文件。归为 MINOR。
- ✅ **AC-3（手动 e2e checklist）** —— `manual_e2e_checklist.md`（190 行）覆盖 A~L 共 12 段、33 项必勾 + 2 项可选，涉及入口 / 编辑保存 / 占位符 / 字节超限 / **真实 LLM 路径**（E）/ **R1 对抗式真实 LLM**（F）/ 单条恢复 / 全部恢复 / 跨重启持久化 / 4 module 独立 / UI 错误反馈 / 空白文本。每项有期望结果。末尾附问题记录表 + 验收总结 + 6 条快速排错指引（覆盖每段可能 FAIL 的诊断路径）。
- ✅ **AC-4（CI 全绿）** —— 实跑确认：
  - `cargo test --test user_prompt_e2e`：20/20 PASS（0.20s）
  - `cargo test --lib`：342/342 PASS（与 task_004 基线一致零回归）
  - `cargo build`：0 error，5 warning 均 task_004 基线既有
- ✅ **AC-5（覆盖率自检）** —— output.md 覆盖矩阵齐全（14 个场景类 × 后端 / 前端 / 手动三栏）。

### 3. 关键发现

1. **R1 对抗式测试是本期最高质量增量** —— `e2e_adversarial_prompt_does_not_override_output_guard`（user_prompt_e2e.rs:363-418）不止 `messages.last() == GUARD` 浅断言，还经测试侧 `simulate_merge_system_messages`（user_prompt_e2e.rs:81-97）合并系统消息后断言 `merged_system.ends_with(CLASSIFY_OUTPUT_GUARD)` + 含"不可被覆盖"字面 + 含"不要使用 markdown 代码块"行为约束。我**逐行比对** `chat.rs::merge_system_messages`（chat.rs:63-79）：行为完全字面等价（push 顺序 / `"\n\n"` join / 空集合 None / 非 system 保序）。这是 ADR-003 Layer A + task_004 AC-0 修复的"端到端真正抵达 LLM"双层守卫。R1 守卫从"我们在 messages 里 push 了 GUARD"升级到"GUARD 实际作为字符串末段送出"，质量超越 AC 要求。
2. **生产代码零修改严格执行** —— git status 确认本期 Dev 只新增 `tests/user_prompt_e2e.rs`（May 15 mtime），未触及 task_002~007 任何产物。
3. **`tests/workspace_folders_integration.rs` baseline 破损与本期完全无关** —— 文件 mtime 是 May 12（早于本期 May 15）；4 个 unresolved import（`count_folder_assets_impl` / `move_asset_to_workspace_folder_impl` / `rename_workspace_folder_impl` / `write_guard` / `validate_and_canonicalize` 已不存在）是其他特性 PR 删除函数后忘清理的孤儿测试。**不属于 task_008 范围**。

---

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | 20 测试全绿 + 实跑确认；AC-1 全部 6 子场景实现且**字节超限有双向边界断言**；R3 升级模拟 + whitespace fallback 是超出 AC 的附加价值 |
| 安全性 | 20% | 5 | R1 对抗式双层断言（messages 层 + 合并层）是 ADR-003 + task_004 AC-0 的"实抵 LLM"守卫验证；占位符校验 + 字节超限拒绝同步断言 DB 未更新（防 partial write） |
| 代码质量 | 15% | 5 | 测试函数命名表意（`e2e_<场景>_<期望>`）；helper 注释明示"与 chat.rs 字面等价 / 等价性由 task_004 单测保证"；模块分段清晰（场景 1~8 + 附加），无重复 |
| 测试覆盖 | 10% | 5 | AC-5 矩阵 14 场景 × 3 通道覆盖完整；超出 AC：4 module 独立双重测试 / R3 双侧（is_custom=1 不变 + is_custom=0 仍走默认）/ whitespace fallback |
| 架构一致性 | 10% | 5 | 全部走 `app_lib::*` pub 接口（无 private 旁路），目录结构与 `workspace_unified_md_integration.rs` 同级；in-memory 范式与 task_002/003 单测一致 |
| 可维护性 | 20% | 4 | 测试命名 `e2e_` 前缀利于 `cargo test e2e_` 精准过滤；唯一可维护性扣分项是 `simulate_merge_system_messages` 与 `chat.rs::merge_system_messages` 字面等价但**未做自动等价检查**——chat.rs 未来改动时测试侧不会自动同步（Dev 已在已知局限 #5 提示，建议未来把 `merge_system_messages` 升 `pub(crate)`，DRY 后三处合并） |

**综合分加权计算**：
- 5×0.25 + 5×0.20 + 5×0.15 + 5×0.10 + 5×0.10 + 4×0.20 = 1.25 + 1.00 + 0.75 + 0.50 + 0.50 + 0.80 = **4.80 / 5**

---

## 总体判断

- [x] **PASS**

理由：
- 综合分 4.80/5（远超 PASS 阈 3.5）
- 0 BLOCKER / 0 MAJOR / 2 MINOR
- 6 个 AC 中 5 个完全满足、1 个（AC-2）以等价覆盖方式偏离但有充分论证
- 20 测试全绿 + 342 lib 零回归 + 0 build error
- R1 对抗式 + 字节超限两项关键场景均超出 AC 要求（双层断言 + 双向边界）

---

## 问题列表

### BLOCKER（必须修复，否则不可能 PASS）

无。

### MAJOR（强烈建议修复）

无。

### MINOR（可选）

1. **AC-2 偏离：未新增 `src/__tests__/userPromptFlow.integration.test.tsx`**
   - **代码位置**：（input.md AC-2 要求新建该文件）
   - **现状**：task_006/007 已等价覆盖 AC-2 列出的 7 个 UI 行为；SettingsPanel.test.tsx 已 mock PromptCustomizationPanel 并断言 Tab 切换挂载
   - **影响**：极低 —— Dev 论证成立，重写一份 SettingsPanel 包裹层 vitest 集成本质是"mock invoke + 真实 store + 真实 panel + 真实 SettingsPanel" 的双重 mock，增量价值仅"Tab 切换可挂载"且已被其他测试覆盖
   - **修复建议（可选）**：若强求与 input.md 字面对齐，可补 ~50-80 行 vitest（mock 4 invoke → 渲染 SettingsPanel → 切 Tab "prompt" → 走 user flow → 断言 saveUserPrompt / resetUserPrompt 调用）。如不补，建议在 Conductor 决策日志记录"AC-2 以 task_006+007 等价覆盖替代，本期通过"
   - **本 Reviewer 立场**：**接受偏离**，不阻断 PASS

2. **`simulate_merge_system_messages` 字面等价缺自动同步检查**
   - **代码位置**：`tests/user_prompt_e2e.rs:81-97` vs `src/llm/chat.rs:63-79`
   - **现状**：两者字面等价，但 chat.rs 改动时测试侧不会自动同步
   - **修复建议（可选）**：在后续 task 把 `merge_system_messages` 升为 `pub(crate)`，三处合并使用同一函数。本 task 范围内已合理（Dev 已在已知局限 #5 标注）
   - **本 Reviewer 立场**：不阻断 PASS

### 给 Conductor 的额外提示（非 task_008 范围）

`tests/workspace_folders_integration.rs` baseline 破损（4 个 unresolved import）：
- **确认与 task_008 无关**：文件 mtime May 12 < 本期 May 15；Dev `git stash` 至干净状态后破损依旧
- **处置建议**：开**独立 task**修复（**不在 task_008 范围内强求**）。临时方案是 CI 改为分目标跑（`cargo test --test user_prompt_e2e --test workspace_unified_md_integration --test live_api`）；正式方案是删/改 `workspace_folders_integration.rs` 中已不存在的函数调用。**本 task 不要求修复**

---

## 给 Dev 的修复指引

**不适用（PASS 无需修复）**。

如 Conductor 决策接受 AC-2 偏离，本 task 直接进入 task_009（如有）。如 Conductor 决策要求严格满足 AC-2，请单独开 task_008b 补 vitest 集成测试，约 50-80 行；Dev 已在 output.md 表态可补，本 Reviewer 立场是不必要。
