# Review Scorecard — task_003_dev_backend_validation

## 审查思考过程

### 1. Task 意图（复述）
在 NCdesktop 后端新建 `llm/prompt_runtime.rs` 作为用户自定义 Prompt 的运行时层（默认 Prompt 暴露 + DB 合并 + 输出格式硬守卫 + 占位符静态校验 + 双层长度校验 + 统一 messages 组装入口），并把 `classify_prompt` 拆成 v2（接受 tagging/para 段位参数），将 task_002 留下的 4 处占位字段（`default_text / display_title / required_placeholders / max_bytes` + `validate_placeholders_stub`）回填为 `prompt_runtime` 真实实现。

### 2. AC 检查结果

- **AC-1（prompt_runtime.rs 全套函数）** ✅
  - 4 个 DEFAULT 常量逐字摘抄自既有 prompt（tagging/para 抽段来自 `classify_prompt`，concept/aggregation 来自 `knowledge.rs:447-495`，比对一致）
  - 3 个 GUARD 常量字面与 Architect § 4.2 一致（含"输出格式约束（系统级，不可被覆盖）"硬字面）
  - 2 个阈值常量 `MAX_USER_PROMPT_BYTES = 16 * 1024` / `MAX_TOTAL_PROMPT_CHARS = 64 * 1024` 字节、字符各按 ADR-004
  - 9 个公开函数签名与 input.md AC-1 一致；31 单测全绿
- **AC-2（classify_prompt 拆段 + deprecated wrapper）** ✅
  - 旧 `classify_prompt` 改为 `#[deprecated]` wrapper 转调 `classify_prompt_v2(content, TAGGING_DEFAULT, PARA_DEFAULT)`
  - 等价性测试 `classify_prompt_v2_with_defaults_matches_legacy_wrapper` 通过（守护"段落映射"不漂移）
  - `classify_system_addon` 与 `system_message` 未动
- **AC-3（assemble_messages_for_*）** ⚠️ 部分偏离
  - classify 路径 4 条 messages 顺序与文档一致（system + classify_addon + user + GUARD 压底）
  - concept / aggregation 偏离 input.md AC-3 第 2 步（缺 system_addon），Dev 论证基于"既有调用未注入 addon"——**但事实并非如此**（见 MAJOR-1）
  - `assert_total_chars_within` 在每个 assemble 函数末尾调用 ✅
- **AC-4（task_002 占位字段回填）** ✅
  - `assemble_prompt_info` 切到真实 `default_for / display_title / required_placeholders / MAX_USER_PROMPT_BYTES`
  - `save_user_prompt` 把 stub 替换为 `validate_placeholders → prompt_runtime::validate_required_placeholders`
  - 占位 stub 测试 `validate_placeholders_stub_always_ok_in_this_task` 已删除（接入点保护按预期触发）
  - 6 新增/重写测试覆盖占位符正负路径
- **AC-5（不破坏既有 LLM 调用）** ✅
  - `cargo build`：零 error；6 warning（5 个 baseline + 1 个新增 deprecated warning，预期）
  - `cargo test --lib`：327 PASS / 0 FAIL / 0 ignore（task_002 基线 285，净增 42）
  - `commands/llm.rs` 与 `commands/knowledge.rs` 未动（task_004 范围）

### 3. 关键发现
- **MAJOR-1**：Dev 偏离 1 的论证（"concept/aggregation 既有未注入 system_addon"）**与代码实际不符**——`commands/knowledge.rs:147` 与 `:276` 各自有 `"You are a knowledge extraction/synthesis engine..."` system_addon。Dev 的处置（assemble 函数中跳过 addon 第 2 步）会让 task_004 改造时**丢失既有 LLM 行为**——这恰好违反 Dev 自己宣称的"逐字摘抄不改 LLM 行为"原则。
- **MAJOR-2**：既有 `llm/chat.rs:58-66` 把多条 system message 用 `system_text = Some(msg.content)` 循环**覆盖**（不是合并/append），最终只发送**最后一条 system** 到 Anthropic。task_003 的 `assemble_messages_for_classify` 产出 4 条 messages，其中 messages[0]（system_message）与 messages[1]（classify_system_addon）会被 chat_completion **丢弃**。task_003 测试 `assemble_messages_for_classify_default_path_uses_builtin_segments` 验证了 messages 内容含 system_message + classify_system_addon，但**这两条在下游会被静默丢弃**——测试给出虚假信心。这是 chat.rs 既有 bug，非 task_003 直接责任，但 task_003 的 assemble 函数设计依赖 messages 数组多 system 都生效，task_004 改造调用方时需要解决。

---

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 4 | AC-1/2/4/5 全部满足；AC-3 偏离（concept/aggregation 缺 addon）有合理化叙事但事实依据错误。32 + 9 测试全绿。 |
| 安全性 | 20% | 4 | R1 三层防御：Layer A（GUARD 最后压底）落地正确；Layer B（占位符校验）concept/aggregation 必含 placeholder 拒绝缺失值；Layer C（既有 parser 用 `unwrap_or` 而非 `unwrap`，无 panic 风险）。R2 双层字节/字符校验阈值正确（16 KiB byte / 64 KiB chars）。**扣 1 分**：assemble 出的 messages 数组在 chat.rs 中被部分丢弃，导致 system_message + classify_system_addon 无效化——虽然 GUARD 仍 100% wins（最后一条），但"system 累加生效"的假设破灭。 |
| 代码质量 | 15% | 5 | 文档注释充分，模块分节清晰；DEFAULT 常量逐字摘抄自既有 prompt（已比对 `knowledge.rs:447-495` 与 `classify_prompt` 字面一致）；`classify_prompt_v2_with_defaults_matches_legacy_wrapper` 等价性测试是回归守护的最佳实践。 |
| 测试覆盖 | 10% | 5 | 31 + 9 + 4 测试覆盖：4 module × default_for/display_title/required_placeholders/output_format_addon、byte_len_check（含 UTF-8 多字节边界）、assert_total_chars_within（含 = 上限 与 + 1 拒绝）、runtime_prompt_for（空白当作未自定义边界）、assemble 3 函数（默认 + 自定义 + 占位符替换 + GUARD 压底）。占位符校验正负路径齐备。 |
| 架构一致性 | 10% | 4 | 目录、命名、签名、字面常量、阈值与 Architect output.md 一致。**扣 1 分**：assemble_messages_for_concept/aggregation 跳过 AC-3 第 2 步 system_addon，且论证依据事实错误（详见 MAJOR-1）。 |
| 可维护性 | 20% | 5 | 单点更新入口（byte_len_check / display_title / default_for / required_placeholders 全集中在 prompt_runtime.rs）；`validate_byte_len → prompt_runtime::byte_len_check` 转调保持阈值同步；deprecated 信号灯设计让 task_004 自动定位迁移点；R3 builtin_version 预留字段为升级兼容性留好接入点。 |

**综合分**：0.25×4 + 0.20×4 + 0.15×5 + 0.10×5 + 0.10×4 + 0.20×5 = **4.45/5**

## 总体判断

- [x] **FIX**
- [ ] PASS
- [ ] BLOCKER

**理由**：2 个 MAJOR 中，MAJOR-1 是 Dev 本期可修复（要么按 input.md AC-3 抄入既有 addon 字面值，要么修改偏离说明的事实描述），MAJOR-2 是既有 chat.rs bug 需要标注但非本期范围。无 BLOCKER（没有安全洞、没有架构严重偏离、核心功能均可运行），但 MAJOR-1 涉及"事实层错误论证"，必须在 PASS 前澄清。

---

## 问题列表

### BLOCKER（必须修复，否则不可能 PASS）
（无）

### MAJOR（强烈建议修复）

1. **问题**：偏离 1（concept/aggregation 跳过 system_addon）的论证基于错误事实
   - **代码位置**：
     - 偏离论证：`sessions/custom_prompt_v1/conductor/tasks/task_003_dev_backend_validation/output.md:50`
     - 反例 1：`src-tauri/src/commands/knowledge.rs:147`（`"You are a knowledge extraction engine. Given a student's academic document, extract key concepts with precision. Return only valid JSON array."`）
     - 反例 2：`src-tauri/src/commands/knowledge.rs:276`（`"You are a knowledge synthesis engine. Help students see how the same concept appears across different courses and contexts. Return only valid JSON array."`）
     - 受影响函数：`src-tauri/src/llm/prompt_runtime.rs:347-374` (`assemble_messages_for_concept`)、`:379-407` (`assemble_messages_for_aggregation`)
   - **修复方向**：从下列两条任选一条（**只需选一条**）：
     - 选项 A（推荐）：把 knowledge.rs:147 与 :276 的两段既有 addon 抄入 `prompt_runtime.rs` 作为常量（如 `CONCEPT_SYSTEM_ADDON / AGGREGATION_SYSTEM_ADDON`），并在 `assemble_messages_for_concept` 与 `_aggregation` 中按 input.md AC-3 第 2 步加入 messages（system_message 之后 / user 之前）。这样 task_004 改造既有调用方切到 assemble 时，**LLM 行为不变**（input.md 字面要求），既符合 AC-3 又符合 Dev 自己宣称的"逐字摘抄"原则。
     - 选项 B：修改偏离说明文字，承认"既有 knowledge.rs 有 system_addon 但 Dev 主动选择不复刻"。**此时必须**给出"为何不复刻 LLM 行为"的真实理由，并显式标记"task_004 改造调用方时需评估这两段 addon 是否丢弃"。如选 B，需要在 output.md "已知局限"加一条对应说明，让 task_004 接到信号灯。
   - **验证标准**：
     - 选项 A：`assemble_messages_for_concept/aggregation` 测试断言 `messages.len() == 4` 而非当前 3；新增 1 个测试断言 `messages[1].content` 含 `"knowledge extraction engine"` / `"knowledge synthesis engine"` 字面。
     - 选项 B：output.md "偏离 1" 文字更新为符合事实的论证；新增 "已知局限" 条目"task_004 需评估是否保留 knowledge.rs:147/276 的 system_addon"。

2. **问题**：`assemble_messages_for_*` 多条 system message 在 `chat.rs::chat_completion` 中只有最后一条生效（既有 bug，task_003 未感知）
   - **代码位置**：
     - 既有 bug：`src-tauri/src/llm/chat.rs:58-66`（`let mut system_text = None; for msg in messages { if msg.role == "system" { system_text = Some(msg.content); }}` —— 循环里每次**覆盖**而不是 append/合并；最终 `system_text` 只是最后一条 system 的 content）
     - 受影响函数：`src-tauri/src/llm/prompt_runtime.rs:312-341` (`assemble_messages_for_classify` 假定 4 条 messages 都生效)、`:347-407` (concept/aggregation 同样)
     - 给出虚假信心的测试：`src-tauri/src/llm/prompt_runtime.rs:628-650` (`assemble_messages_for_classify_default_path_uses_builtin_segments` 断言 messages[0]/[1] 内容，但下游不会发送)
   - **修复方向**：本 bug 不在 task_003 范围内（chat.rs 不动是 task_003 的隐含约束），但 task_003 应**显式记录**风险到 output.md 的"已知局限"或"需要 Reviewer 特别关注"。**最小修复**：在 output.md 加一条已知局限："`assemble_messages_for_*` 假定 messages 中所有 system 条目都生效，但 `chat.rs:58-66` 当前只保留最后一条 system 内容；这意味着 GUARD 必然 wins（R1 反而是巧合性 100% 生效），但 system_message + classify_system_addon 在 task_004 切到 assemble 后**会失效**。task_004 必须修复 chat.rs 的 system 合并逻辑（用 `\n\n` join 多条），或在 assemble 中预先把多条 system 拼为一条。"
   - **验证标准**：
     - output.md 新增已知局限条目，明确描述 chat.rs 的 system 覆盖行为对 assemble 函数的影响
     - 给 task_004 input.md 起草时留出"修复 chat.rs system 合并 OR 在 assemble 中预 join system"的明确选项（task_003 不实际改 chat.rs；这是给后续 task 的信号）
     - **可选**：把 task_003 的 assemble 函数测试加一行 `// FIXME: task_004 应修复 chat.rs system 合并，否则 messages[0]/[1] 实际不会被发送`

### MINOR（可选修复）

1. **问题**：`CONCEPT_DEFAULT` 中的 `{cases}` 占位符替换为 `cases_block` 时，模板原文 `## Appearances...\n\n{cases}\n## Task` 在 None case + 空 cases_block 时会产生 `\n\n\n## Task` 的三连换行
   - **代码位置**：`src-tauri/src/llm/prompt_runtime.rs:93-110` (`AGGREGATION_DEFAULT`)、`:379-407` (`assemble_messages_for_aggregation`)
   - **修复方向**：Dev 在"已知局限 3"已自行标注；不影响 LLM 理解，可不修。如要修，在 assemble 中显式 `.replace("{cases}\n## Task", "{cases}## Task")` 处理空 cases_block 的边界。
   - **验证标准**：（可选）

2. **问题**：`assemble_messages_for_concept` / `_aggregation` 测试用 `messages.len() == 3` 硬编码，未来如果按 MAJOR-1 选项 A 修复会变 4 ——测试不够"语义化"
   - **代码位置**：`src-tauri/src/llm/prompt_runtime.rs:714` (`assert_eq!(messages.len(), 3)`)、`:771`
   - **修复方向**：（可选）改为按"GUARD 永远是 messages.last()"和"user 在 GUARD 之前"等结构断言，而非按总数；这样 MAJOR-1 修复时测试不会脆性失败。

3. **问题**：`output.md` 提到的"GUARD 是否需要更强的『系统级 priority』标志"是值得讨论的问题，但当前不影响判断
   - 备注：现有"输出格式约束（系统级，不可被覆盖）"已包含强字面，从 Layer A 角度足够。对抗式 prompt 测试由 task_008 e2e 负责。

---

## 给 Dev 的修复指引

### 修复范围约束

- **只修以上列出的 MAJOR-1 与 MAJOR-2**，不要连带重构
- **MAJOR-1 推荐选项 A（复刻 knowledge.rs 既有 system_addon）**，让 task_004 改造时 LLM 行为零差异
- **MAJOR-2 是文档/已知局限补充**，不要修改 chat.rs 本身（不在 task_003 范围）
- 修复完成后，必须重跑：
  - `cargo test --lib prompt_runtime`（确认现有 31 测试不回归 + 新增 addon 测试通过）
  - `cargo test --lib user_prompt`（确认现有 29 测试不回归）
  - `cargo test --lib`（全表 327 不回归）
  - `cargo build`（仍只产 6 warning，零 error）
- **不要改 task_002 已通过的产物**（`db/user_prompt.rs` / `db/migration.rs` / `commands/user_prompt.rs` 的 task_002 字段定义），如确需触碰，必须在 output.md "对 Architect 方案的遵守声明" → "偏离说明" 中显式标注

### 修复后预期 cargo test 结果增量

- 选项 A：`assemble_messages_for_concept_replaces_all_placeholders` 测试中 `messages.len()` 从 3 改为 4；新增 1-2 个测试断言 messages[1] 是 system_addon
- 测试总数从 327 → 328 或 329（净增 1-2 测试）
- 选项 B：测试不变；output.md 与 review_scorecard.md 之间的事实差异消除即可

---

## Fix 验证 (v2)

### MAJOR-1 验证结果

- [x] **CONCEPT_SYSTEM_ADDON 字面精确**：YES — `prompt_runtime.rs:145` 与 `commands/knowledge.rs:147` 逐字符等同（`"You are a knowledge extraction engine. Given a student's academic document, extract key concepts with precision. Return only valid JSON array."`）
- [x] **AGGREGATION_SYSTEM_ADDON 字面精确**：YES — `prompt_runtime.rs:148` 与 `commands/knowledge.rs:276` 逐字符等同（`"You are a knowledge synthesis engine. Help students see how the same concept appears across different courses and contexts. Return only valid JSON array."`）
- [x] **assemble_messages_for_concept 注入 messages[1]**：YES — `prompt_runtime.rs:392-395` 把 `CONCEPT_SYSTEM_ADDON` 插入 system_message 之后 / user 之前 / GUARD 之前
- [x] **assemble_messages_for_aggregation 注入 messages[1]**：YES — `prompt_runtime.rs:440-443` 同上模式注入 `AGGREGATION_SYSTEM_ADDON`
- [x] **测试断言 len==4**：YES — `assemble_messages_for_concept_replaces_all_placeholders`（`prompt_runtime.rs:780`）与 `assemble_messages_for_aggregation_replaces_placeholders_and_handles_none_definition`（`:849`）均断言 `messages.len() == 4`
- [x] **测试断言字面**：YES — `assemble_messages_for_concept_replaces_all_placeholders` 断言 `messages[1].content.contains("knowledge extraction engine")` + `messages[1].content == CONCEPT_SYSTEM_ADDON`（`:785-789`）；aggregation 测试同模式断言 `"knowledge synthesis engine"`（`:852-856`）；**额外**：新增 `system_addons_match_existing_knowledge_rs_literals` 测试（`:550-560`）固化两段 addon 字面值，防止后续 knowledge.rs 漂移。
- [x] **GUARD 仍是 messages.last()**：YES — 3 个 assemble 函数全部以 GUARD 结尾，`messages.last().unwrap().content == *_OUTPUT_GUARD` 在 3 个对应测试中均有断言

### MAJOR-2 验证结果

- [x] **已知局限明确描述 chat.rs 系统覆盖**：YES — output.md 第 330-335 行"已知局限 6"详述 chat.rs:58-66 的 `Some(msg.content.clone())` 覆盖循环、对 3 个 assemble 的影响、"GUARD 100% 生效是巧合性"的判断、task_004 必须处理的明确范围
- [x] **task_004 信号灯就位**：YES — output.md 第 348-367 行"留给 task_004 的信号灯（MAJOR-2 / v2 新增）"列出三项必处理事项（修 chat.rs 多 system 合并 / 加 AC 断言 system 字段含字面 / 迁移后移除 FIXME），含示例修复代码
- [x] **FIXME 注释（可选）**：YES — `prompt_runtime.rs:329-332`、`:373-376`、`:420-423` 三个 assemble 函数 doc 注释末尾各有 `FIXME(task_004): chat.rs:58-66 ...` 注释（中文，建议两种修复路径）

### 回归验证

- **prompt_runtime 测试**：**32/32 PASS**（v1 基线 31，净增 1 个 `system_addons_match_existing_knowledge_rs_literals`）
- **全表 cargo test --lib**：**328/328 PASS**（v1 基线 327，净增 1；0 fail / 0 ignore）
- **cargo build error**：**0**（warning 仍为 6 个：5 个 baseline + 1 个预期 deprecated，与 v1 完全一致；无新增）

### 范围合规

- [x] **仅改 prompt_runtime.rs**：YES — 唯一改动的 .rs 文件是 `src-tauri/src/llm/prompt_runtime.rs`（output.md 也作为产物更新）；未触碰 mod.rs / prompts.rs / commands/user_prompt.rs（这些是 v1 已通过的部分）。
- [x] **未触碰 chat.rs / knowledge.rs / task_002 产物**：YES — `chat.rs` / `commands/knowledge.rs` / `commands/llm.rs` / `db/user_prompt.rs` / `db/migration.rs` 均无改动；MAJOR-2 修复严格按"文档 + FIXME 注释"路径（非代码修复），完全符合 task_003 范围约束。

### 更新后综合分（重新加权）

| 维度 | 权重 | v1 分 | v2 分 | 变化原因 |
|------|------|------|------|---------|
| 功能正确性 | 25% | 4 | 5 | MAJOR-1 完成：concept / aggregation 的 system_addon 已按选项 A 注入，AC-3 第 2 步完整落地，messages.len() 升 3→4 且字面等同 knowledge.rs：task_004 切换时 LLM 行为零差异。 |
| 安全性 | 20% | 4 | 5 | MAJOR-2 信号灯就位：output.md 明确标注 chat.rs:58-66 system 覆盖 bug + 3 处 FIXME 注释 + task_004 修复路径；"GUARD 100% wins 是巧合"的事实被显式记录，不再是隐患。R1 三层防御稳固。 |
| 代码质量 | 15% | 5 | 5 | 字面常量逐字摘抄到位；新增 1 个固化测试 `system_addons_match_existing_knowledge_rs_literals` 防止 knowledge.rs 字面漂移；FIXME 注释明确指出修复路径（两种方案可选）。 |
| 测试覆盖 | 10% | 5 | 5 | 31→32 测试净增 1 个固化测试；4 个 concept/aggregation assemble 测试均更新 len==4 + 字面断言。回归 0 fail / 0 ignore。 |
| 架构一致性 | 10% | 4 | 5 | AC-3 第 2 步（concept/aggregation 注入各自 system_addon）已落实；与 Architect 方案的 messages 顺序约束完全一致；偏离 1 已消除，v1 评分卡里的"扣 1 分"原因不复存在。 |
| 可维护性 | 20% | 5 | 5 | 信号灯设计完整：deprecated wrapper（task_004 自动发现迁移点）+ 3 处 FIXME(task_004)（task_004 内部修复点）+ output.md "已知局限" + "task_004 input.md 建议变更"——多层冗余信号灯让 task_004 不可能漏处理 chat.rs system 合并。 |

**综合分（v2）**：0.25×5 + 0.20×5 + 0.15×5 + 0.10×5 + 0.10×5 + 0.20×5 = **5.00/5**

**最终判断**：**PASS**

理由：
- MAJOR-1 按 Reviewer 推荐的"选项 A"完整落地（字面常量精确、注入位置正确、测试断言充分），任务 004 切换调用方时 LLM 行为可保证零差异。
- MAJOR-2 严格按 Reviewer 指引以"文档 + FIXME 注释"形式记录信号灯（不实改 chat.rs，符合 task_003 范围约束）；task_004 input.md 起草所需的"修复 chat.rs system 合并 OR assemble 预 join"两条选项均已在 output.md 第 348-367 行就位。
- 范围合规度 100%：本轮 Fix 唯一动了 `prompt_runtime.rs` 一个文件 + output.md 文档，未越界。
- 回归验证全绿：prompt_runtime 32/32、全表 328/328、cargo build 0 error / 0 新增 warning。
- 无任何遗留 MAJOR / BLOCKER；v1 的 MINOR（cases_block 三连换行、测试硬编码 len、GUARD 字面 priority 标志）属于可选 / 后续 task 范围，不阻塞本期 PASS。

**task_004 可以启动**：output.md 信号灯就位，task_004 Conductor 在起草 input.md 时直接照搬第 348-367 行的三项即可。
