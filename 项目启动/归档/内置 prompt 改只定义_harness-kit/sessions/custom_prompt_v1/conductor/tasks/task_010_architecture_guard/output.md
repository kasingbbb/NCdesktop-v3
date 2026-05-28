# Architecture Guard Report — custom_prompt_v1（终审）

## 扫描信息

- **扫描时间**：2026-05-15
- **源码路径**：`/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/`
- **原始架构方案**：`sessions/custom_prompt_v1/conductor/tasks/task_001_architect/output.md`
- **已完成 Task 数**：9（task_001 Architect / task_002~007 Dev × 6 / task_007_round2 二轮 / task_008 e2e / task_009 UX）
- **复杂度**：L（强制 Architecture Guard 终审）
- **实跑验证**：
  - `cargo test --lib` → **342 passed / 0 failed**（3.04s）
  - `cargo test --test user_prompt_e2e` → **20 passed / 0 failed**（0.21s）
  - `pnpm tsc --noEmit` → **0 error**（空输出 / exit 0）
  - `cargo build` 隐含通过（lib test 跑得起即编译成功；0 warning）

---

## 架构原则回顾（从 Architect output.md 提取）

> 5 条核心架构原则（每条对应一条 ADR，决定本期不可妥协的形状）：

1. **【ADR-001】内置 fallback 永远兜底**：内置 Prompt 是 Rust 源码常量；`runtime_prompt_for` 在 `is_custom=0`（或空白文本）时回退到 `default_for(module)`；内置升级 = 改源码常量，不覆盖已存在的用户自定义。
2. **【ADR-002】SQLite 独立表 + 4-module 白名单**：新建 `user_custom_prompt(module PK / prompt_text / is_custom / builtin_version / updated_at)`；module 限定为 `tagging / para / concept / aggregation`；migration V15 全 `IF NOT EXISTS`，不写默认行；`builtin_version` 字段为 R3 升级提示预留（MVP 不读取，仅写入）。
3. **【ADR-003】三层防御输出格式守卫**：Layer A `output_format_addon`（GUARD system 永远最后压底，用户 prompt 绕不过）+ Layer B `validate_required_placeholders`（保存时静态校验）+ Layer C 既有 parser 容错。
4. **【ADR-004】双层字节/字符校验**：保存时 `MAX_USER_PROMPT_BYTES = 16 KiB`（byte_len_check）+ 调用前 `MAX_TOTAL_PROMPT_CHARS = 64 KiB 字符`（assert_total_chars_within）。
5. **【ADR-005】独立 store / 命名前缀隔离**：新建 `userPromptStore.ts` + `PromptCustomizationPanel.tsx`；后端 `commands::user_prompt` / `db::user_prompt` / 前端 `userPrompt*`；与 PR-4 半成品（`commands/prompts.rs` / `promptStore.ts` / `PromptEditor.tsx`）零交叉、零修改。

---

## 1. ADR 落地一致性

| ADR | 验证清单 | 结果 |
|---|---|---|
| **ADR-001** 内置 fallback | ① `runtime_prompt_for` `is_custom=0`/空白 → `default_for` （`llm/prompt_runtime.rs:225-232`）✓<br/>② 升级源码字面值后已自定义不被覆盖（user_custom_prompt 表与源码常量解耦）✓<br/>③ `default_for(module)` 唯一对外入口（`prompt_runtime.rs:166-174`）✓ | **✓** |
| **ADR-002** SQLite 表 | ① migration V15 落地（`db/migration.rs:53-93`）✓<br/>② 表结构含 `builtin_version`，与 § 5.1 完全一致 ✓<br/>③ migration 测试推到 15（`fresh_db_runs_all_migrations_to_v15` + `run_migrations_is_idempotent` + `v15_idempotent_with_existing_table`）✓ | **✓** |
| **ADR-003** 输出守卫三层防御 | ① 三个 `*_OUTPUT_GUARD` 常量字面值与 § 4.2 一致（`prompt_runtime.rs:120-132`）✓<br/>② messages 中 GUARD 永远 `messages.last()`（3 个 assemble 函数 + e2e 断言）✓<br/>③ Layer B `validate_required_placeholders` 在 `save_user_prompt` 中调用（`commands/user_prompt.rs:151`）✓<br/>④ R1 对抗式 prompt e2e PASS（`e2e_adversarial_prompt_does_not_override_output_guard`）✓ | **✓** |
| **ADR-004** 双层字节校验 | ① `MAX_USER_PROMPT_BYTES = 16 * 1024`（`prompt_runtime.rs:155`）✓<br/>② `MAX_TOTAL_PROMPT_CHARS = 64 * 1024`（`prompt_runtime.rs:159`）✓<br/>③ 保存校验在 `save_user_prompt`（`commands/user_prompt.rs:150` → `validate_byte_len` → `byte_len_check`）✓<br/>④ 调用前校验在每个 `assemble_messages_for_*` 末尾（`prompt_runtime.rs:413/454/497`）✓ | **✓** |
| **ADR-005** 独立 store / 命名隔离 | ① 未修改 `stores/promptStore.ts`（main..HEAD diff 为空）✓<br/>② 未修改 `components/settings/PromptEditor.tsx`（同上）✓<br/>③ 未修改 `commands/prompts.rs`（同上）✓<br/>④ 新建 `userPromptStore.ts` + `PromptCustomizationPanel.tsx` ✓ | **✓** |

---

## 2. 目录结构一致性（vs Architect § 7）

| 类别 | 文件 | 状态 |
|---|---|---|
| 新建（后端） | `src-tauri/src/commands/user_prompt.rs` | ✓ |
| 新建（后端） | `src-tauri/src/db/user_prompt.rs` | ✓ |
| 新建（后端） | `src-tauri/src/llm/prompt_runtime.rs` | ✓ |
| 新建（测试） | `src-tauri/tests/user_prompt_e2e.rs` | ✓（task_008，§ 7 未列出，但 task_008 input.md 显式约定） |
| 新建（前端） | `src/types/user-prompt.ts` | ✓ |
| 新建（前端） | `src/stores/userPromptStore.ts` | ✓ |
| 新建（前端） | `src/components/settings/PromptCustomizationPanel.tsx` | ✓ |
| 修改 | `src-tauri/src/lib.rs`（setup + invoke_handler） | ✓（行 61 `app.manage(AppMode::Normal)`；行 257-260 注册 4 command） |
| 修改 | `src-tauri/src/commands/mod.rs` | ✓（`pub mod user_prompt`） |
| 修改 | `src-tauri/src/db/mod.rs` | ✓（`pub mod user_prompt` + `pub mod repair` 同期挂接） |
| 修改 | `src-tauri/src/db/migration.rs`（V15） | ✓ |
| 修改 | `src-tauri/src/llm/mod.rs` | ✓（`pub mod prompt_runtime`） |
| 修改 | `src-tauri/src/llm/prompts.rs`（classify 拆段 + deprecated wrapper） | ✓ |
| 修改 | `src-tauri/src/llm/chat.rs`（merge_system_messages） | ✓（task_004 AC-0） |
| 修改 | `src-tauri/src/commands/llm.rs`（assemble_messages_for_classify） | ✓ |
| 修改 | `src-tauri/src/commands/knowledge.rs`（assemble_messages_for_{concept,aggregation}） | ✓ |
| 修改 | `src-tauri/src/utils/mod.rs` | ✓（lib.rs 行 17 挂接） |
| 修改 | `src-tauri/Cargo.toml`（unicode-normalization 显式声明） | ✓（Cargo.toml:54） |
| 修改 | `src/lib/tauri-commands.ts`（4 函数） | ✓（行 802-839） |
| 修改 | `src/components/features/SettingsPanel.tsx`（Tab + dirty 守卫） | ✓（含 AC-8 `confirmIfPromptDirty`） |

**计划外文件**：无（§ 7 之外仅 task_008 与各 `__tests__/` 测试文件，task_008 input.md 已约定，可接受）。

---

## 3. 契约一致性

### 3.1 PromptInfo 9 字段前后端 1:1

| 字段（camelCase） | 后端 `commands::user_prompt::PromptInfo` | 前端 `types/user-prompt.ts::PromptInfo` |
|---|---|---|
| `module` | `String`（行 36） | `PromptModule`（行 39） |
| `displayTitle` | `String`（行 37） | `string`（行 40） |
| `defaultText` | `String`（行 38） | `string`（行 41） |
| `userText` | `Option<String>`（行 39） | `string \| null`（行 42） |
| `isCustom` | `bool`（行 40） | `boolean`（行 43） |
| `builtinVersion` | `String`（行 41） | `string`（行 44） |
| `updatedAt` | `Option<String>`（行 42） | `string \| null`（行 45） |
| `requiredPlaceholders` | `Vec<String>`（行 43） | `string[]`（行 46） |
| `maxBytes` | `usize`（行 44） | `number`（行 47） |

`#[serde(rename_all = "camelCase")]` 已在后端结构体上声明（行 34），与前端字面 1:1 命中。

### 3.2 4 个 Tauri command 三点命名一致

| Command | `lib.rs invoke_handler!` | `commands::user_prompt::*` | `tauri-commands.ts` |
|---|---|---|---|
| `list_user_prompts` | 行 257 ✓ | 行 108 ✓ | 行 814 ✓ |
| `get_user_prompt` | 行 258 ✓ | 行 128 ✓ | 行 819 ✓ |
| `save_user_prompt` | 行 259 ✓ | 行 142 ✓ | 行 828 ✓ |
| `reset_user_prompt` | 行 260 ✓ | 行 160 ✓ | 行 837 ✓ |

### 3.3 PromptModule 字面量三点严格一致

- 后端 `commands/user_prompt.rs::MODULES`（行 27）：`["tagging", "para", "concept", "aggregation"]`
- 后端 `prompt_runtime.rs::default_for / display_title / required_placeholders / output_format_addon` match 分支（行 167-211）：4 个完全一致 ✓
- 前端 `types/user-prompt.ts::PromptModule`（行 22）联合 + `PROMPT_MODULES`（行 55） + `PROMPT_MODULE_TITLES`（行 60-65）：4 个完全一致 ✓

**round2 store error 升级**（`{ module, message } | null`）属内部 surface 变更，前后端 IPC 契约未变。✓

---

## 4. 风险闭环（R1~R9）

| ID | 验证 | 结果 |
|----|------|------|
| **R1** 对抗式 prompt | ADR-003 三层防御全部到位；e2e `e2e_adversarial_prompt_does_not_override_output_guard` PASS；merge_system_messages 把 GUARD 拼到合并后 system 字段末尾（chat.rs:63 + 4 个 chat tests） | **✓** |
| **R2** token 超限 | 保存 16 KiB（`commands/user_prompt.rs:150`）+ 调用前 64 KiB 字符（3 个 assemble 函数末尾）双层校验；e2e 双场景全 PASS | **✓** |
| **R3** 版本落后 | `builtin_version` 字段在表中存在（migration.rs:80）；写入固定 `"1.0"`（`db/user_prompt.rs:30`）；MVP 不读取（接受未使用预留）；e2e `e2e_builtin_version_bump_preserves_user_custom_text` PASS | **✓** |
| **R4** PRD 4 ↔ 后端 3 映射 | UI 抽象 4 module；后端 classify 调用合并 tagging+para（`assemble_messages_for_classify` 同时读两段）；R4 文案方案 B 落地（PromptCustomizationPanel.tsx:34-37 + AC-4 副标题"与...共用同一次分类调用，两者同时生效"） | **✓** |
| **R5** AppMode 未注册 | `lib.rs:61 app.manage(crate::startup::AppMode::Normal)` 在 setup 中明确注册；写命令 `save_user_prompt / reset_user_prompt` 均经 `ensure_writable(mode.inner())` 守卫 | **✓** |
| **R6** PR-4 半成品零污染 | `git log main..HEAD` 对 `commands/prompts.rs / promptStore.ts / PromptEditor.tsx` 三文件**空输出**（零 commit 触碰）；本期新代码 grep `promptStore\|PromptEditor` 仅命中"注释中说明不复用"4 处，零交叉引用 | **✓** |
| **R7** migration V15 残留 schema | V15 全 `CREATE TABLE/INDEX IF NOT EXISTS`；测试覆盖 `fresh_db_runs_all_migrations_to_v15` + `run_migrations_is_idempotent` + `v15_idempotent_with_existing_table` 三路径 PASS | **✓** |
| **R8** classify_prompt 签名兼容 | 旧 `classify_prompt(content)` 保留为 `#[deprecated]` wrapper（prompts.rs:32-38）转调 `classify_prompt_v2(content, TAGGING_DEFAULT, PARA_DEFAULT)`；`classify_prompt_v2_with_defaults_matches_legacy_wrapper` 测试守护字符串等价；grep 无非 wrapper 自身的 `classify_prompt(` 调用残留 → **0 deprecated warning** | **✓** |
| **R9** dry-run 缺失 | Architect § 9 明示 MVP 不实现；task_009 UX 评审未要求新增；占位符静态校验（`validate_required_placeholders`）作为最低保障 PASS | **✓** |

**R1~R9 全闭环。**

---

## 5. 发现

### BLOCKER（架构级问题）

**无。**

### WARNING（架构偏移，建议在验收前修复）

**无。**

### INFO（信息性发现，不影响继续）

1. **`builtin_version` MVP 未读取（已知预留）**
   - 位置：`db/user_prompt.rs:30`（写固定 `"1.0"`）；前端 `PromptInfo.builtinVersion` 字段虽透传但 UI 未消费
   - 偏离的架构原则：ADR-002 明示"MVP 不使用，仅写入"，**符合设计**
   - 影响：无；R3 升级提示属 P2 范围；Architect § 12 第 4 条已记录为待 Conductor/PM 复核点
   - 建议：保留现状，未来 R3 真正启用时（builtin 升版本号）配合前端"内置已更新"提示一并加

2. **PR-4 半成品仍是孤儿代码**
   - 位置：`commands/prompts.rs` + `stores/promptStore.ts` + `components/settings/PromptEditor.tsx`
   - 偏离的架构原则：无（ADR-005 显式选择新建独立 store，PR-4 不复用）
   - 影响：开发者看仓库会发现两套"prompt-ish"代码并存可能困惑；但本期严守命名隔离，零交叉引用
   - 建议：Architect § 12 第 2 条已记录；由 Conductor 在本期完成后单开清理 task 删除

3. **UI 字节上限文案中硬编码 "16 KB"**
   - 位置：`PromptCustomizationPanel.tsx:242, 391`（默认值 16384 与文案 "已超过 16 KB 上限"）
   - 偏离的架构原则：配置管理一致性轻微偏移；后端阈值通过 `PromptInfo.maxBytes` 字段下发，前端却又写死降级值
   - 影响：MVP 范围零风险（后端 16 KiB 与 UI 文案一致）；若未来阈值改变，需同步两处
   - 建议：后续可把 UI 文案改为 `${(maxBytes / 1024).toFixed(0)} KB`，使用后端下发值；不阻塞本期

4. **chat.rs `chat_completion_stream` 仍是占位 stub**
   - 位置：`llm/chat.rs:127-138`
   - 偏离的架构原则：无（既有遗留，不在本期范围）
   - 影响：本期不涉及流式
   - 建议：与本期独立的后续 task 处理

---

## 6. 架构一致性矩阵

| 维度 | 评分(1-5) | 关键发现 |
|------|-----------|----------|
| 模块边界完整性 | **5** | `commands` ↔ `db` ↔ `llm/prompt_runtime` 严格分层；3 个 LLM 调用方零绕过；前端 store/UI 不直接 invoke |
| 接口一致性 | **5** | 4 Tauri command + PromptInfo 9 字段在 invoke_handler / commands / tauri-commands 三处字面对齐；零冗余 API |
| 数据流完整性 | **5** | save 路径（white-list → ensure_writable → byte → placeholder → upsert）+ 调用路径（runtime_prompt_for → assemble → GUARD 压底 → chars 校验 → chat_completion → merge_system）端到端连贯；错误中文一直透传到 UI 红色横条 |
| 配置管理 | **4** | 阈值常量唯一定义在 `prompt_runtime.rs:155/159`；其他位置全部引用；UI 仅文案处轻微硬编码（INFO-3） |
| 错误处理一致性 | **5** | 后端统一 `Result<T, String>` + 中文错误；前端 store error 升级为归属对象 `{module, message}`；chat.rs system 合并修复让多 system 不再丢失 |
| 依赖治理 | **5** | `unicode-normalization` 显式声明（Cargo.toml:54）；零新增前端依赖；deprecated `classify_prompt` 仅作 v2 wrapper 保留，无外部调用残留（0 warning） |

---

## 架构健康评分：**4.83 / 5**

> 评分依据：6 维度评分（5+5+5+4+5+5 = 29 / 6）。本期严格遵循 5 条 ADR，9 项风险全闭环，3 项 INFO 均属"设计中已记录的预期偏离"或"非本期范围的轻微优化空间"，不影响验收。

---

## 给 Conductor 的建议

**继续到 ACCEPTANCE（可视为 DONE）。**

- 0 BLOCKER / 0 WARNING / 4 INFO
- 实跑：cargo lib 342/342 + e2e 20/20 + tsc 0 error，全绿
- R1~R9 全闭环；ADR-001~005 全落地；PR-4 半成品零污染
- 唯一遗留：Architect § 12 第 2/4 条（PR-4 清理 + builtin_version 启用）为 PM/Conductor 在本期完成后的独立决策，**不阻塞**本期入 ACCEPTANCE

---

## 7. 已知遗留（移交 Conductor 决策）

| # | 来源 | 内容 | 当前处置 | 建议下一步 |
|---|---|---|---|---|
| 1 | Architect § 12.1 + R4 | PRD 4 module ↔ 后端 3 调用链映射 | 已落地（R4 方案 B 副标题；e2e 4 module 独立覆盖；后端 classify 合并 tagging+para） | 无需进一步动作；UI 文案已让用户感知 |
| 2 | Architect § 12.2 + ADR-005 | PR-4 半成品（`commands/prompts.rs` / `promptStore.ts` / `PromptEditor.tsx`）是孤儿 | 本期严守命名隔离零修改 | Conductor 单开清理 task 删除；非本期 |
| 3 | Architect § 12.3 + R9 | dry-run 验证 | MVP 不实现，占位符静态校验作为最低保障 PASS | task_009 UX 评审未要求；保持现状 |
| 4 | Architect § 12.4 + ADR-002 / R3 | `builtin_version` 字段是否值得保留 | MVP 已写入但不读取（INFO-1） | 保留；未来真正启用 R3 时配合 UI "内置已更新" 提示一并加，避免 P2 二次 migration |
| 5 | INFO-3 | UI 16 KB 文案硬编码 | MVP 后端值与 UI 文案一致 | 后续可改为 `${maxBytes/1024} KB` 由后端下发；不阻塞 |
