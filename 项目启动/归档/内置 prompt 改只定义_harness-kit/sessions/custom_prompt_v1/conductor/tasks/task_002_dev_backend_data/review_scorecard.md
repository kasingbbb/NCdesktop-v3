# Review Scorecard — task_002_dev_backend_data

## 审查思考过程

### 1. Task 意图（一句话复述）
为 NCdesktop 后端落地"用户自定义 Prompt"功能的持久化与 IPC 基础设施：migration V15 + DB 层（5 函数）+ 命令层（4 个 `#[tauri::command]`）+ AppMode 前置注册修复，让 task_003 起后续 task 有可用的数据底座与 IPC 接入点。

### 2. AC 检查结果（逐条）

| AC | 状态 | 证据 |
|----|-----|------|
| **AC-1** migration V15 + 表 / 索引 / DDL `IF NOT EXISTS` + 单测 | ✅ | `db/migration.rs:53-93`：dispatcher 分支 + `v15_user_custom_prompt` 函数；DDL 完全匹配 Architect § 5.1（5 列 + 索引名 `idx_user_custom_prompt_is_custom`）。测试：`fresh_db_runs_all_migrations_to_v15`（断言推到 15 + 5 列 + 索引 + row_count=0）、`run_migrations_is_idempotent`（连跑两次推到 15）、`v15_idempotent_with_existing_table`（R7 残缺路径）三个测试全绿 |
| **AC-2** `db/user_prompt.rs` 5 函数 + `params!` + 单测 | ✅ | `db/user_prompt.rs:45-114`：`get / upsert / delete / delete_all / list_all` 全部 `params!` 参数化（无字符串拼接）；签名与 input.md 完全一致；`UserPromptRow` 5 字段对齐 Architect § 5.2；8 个单测覆盖正常路径、空表、覆盖、引号注入防御 |
| **AC-3** `commands/user_prompt.rs` 4 命令 + 守卫顺序 + 占位值 | ✅ | `commands/user_prompt.rs:137-205`：4 个 `#[tauri::command]` 签名完全匹配 Architect § 6.1；`save_user_prompt` 守卫顺序严格为 `validate_module → ensure_writable → validate_byte_len → validate_placeholders_stub → upsert`（行 177-183）；`reset_user_prompt(None)` = `delete_all`，`Some(m)` = `delete`；`PromptInfo.default_text/required_placeholders/max_bytes` 用占位值（行 67-74、131），与 input.md 显式约定一致；9 个单测覆盖白名单、字节、stub、4 集成链路、`ensure_writable` |
| **AC-4** lib.rs setup 修复 + 注册 | ✅ | `lib.rs:61` `app.manage(crate::startup::AppMode::Normal)` 紧随 `app.manage(database)`；`lib.rs:257-260` invoke_handler 追加 4 行；其他既有 invoke_handler 注册未动 |
| **AC-5** `cargo test --lib user_prompt` / `migration` 全绿 | ✅ | 22 个 user_prompt 测试 + 11 个 migration 测试全 PASS；既有 v11/v12/v14 测试断言被更新到 v15 + 新增 v15 幂等测试，未删既有逻辑覆盖 |
| **AC-6** `cargo build` / `cargo test --lib`（全表）不回归 | ✅ | 我本地复现：`cargo build` 0 error 5 warning（全为既有代码 `dropzone.rs` / `llm/chat.rs`，与本 task 无关）；`cargo test --lib` `test result: ok. 285 passed; 0 failed`（最末输出已贴） |

### 3. 关键发现

1. **Dev 的偏离说明 R5 衍生连带挂接是必要的工程权衡，可接受**（详见安全性/架构一致性维度）。在不接入 bootstrap 的最小范围内，挂接的孤儿模块 `db::repair / utils::nfc / utils::safe_rename / utils::ipc_error` 全部被编译并跑测试，285 全绿证明无回归。`unicode-normalization` 已在 Cargo.lock 0.1.25（由 idna 间接），新增显式依赖无引入新版本风险。
2. **DB 层与命令层职责严格分离**（白名单校验只在命令层、错误消息全中文、`params!` 全覆盖），与现有 `commands/categories.rs` / `db/settings.rs` 范式高度一致。
3. **测试设计有"接入点护栏"**：`validate_placeholders_stub_always_ok_in_this_task` 与 `default_text_placeholder_for` 故意脆弱，task_003 接入真实实现时会强制让相关测试失败（防止接入点被遗忘）—— 这是符合 ADR-003 三层防御契约的好工程实践。

---

## 评分

> **权重来源**：`session_context.md § 4`（功能正确性 25% / 安全性 20% / 代码质量 15% / 测试覆盖 10% / 架构一致性 10% / 可维护性 20%）。

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | **5** | AC-1~AC-6 全部满足；285 测试全绿；DB schema 与 Architect § 5.1 字符级一致；命令层守卫顺序与 input.md 字面一致；正常路径 / 边界 / 异常路径 22 项自测矩阵全 PASS |
| 安全性 | 20% | **5** | 所有 SQL 走 `params!`（含 `params_protect_against_quote_injection` 显式测试 `'; DROP TABLE...; --` payload）；4 module 白名单严格闭集（命令层强制 + 大小写敏感测试 `Tagging` 也拒绝）；写命令必经 `ensure_writable`（有显式 `ensure_writable_blocks_readonly_mode_for_writes` 测试）；16 KiB 字节上限（按字节非字符）；DDL `IF NOT EXISTS` 全覆盖（R7）；隐私上不离机（全部存本机 SQLite）。Prompt 注入硬约束（ADR-003 Layer A）落在 task_003，但本 task 已为接入点留好命令面（占位符 stub 与 max_bytes 上线即生效），未给 task_003 留架构债 |
| 代码质量 | 15% | **5** | 命名一致（`user_prompt` 前缀全程，避免 PR-4 `prompt.override.*` 冲突 R6）；常量 `BUILTIN_VERSION_MVP / MAX_USER_PROMPT_BYTES / MODULES` 集中且带注释；`row_to_user_prompt` 抽出复用；`assemble_prompt_info` 单职责（DB 行 + module → DTO）；中文错误消息一致；`#[serde(rename_all = "camelCase")]` 与前端契约对齐；文档注释（`//!` 模块级 + `///` 函数级）覆盖每个公共面 |
| 测试覆盖 | 10% | **5** | 22 个新增测试 + 3 个 migration 测试覆盖：fresh / 残缺 / 幂等（V15 三路径），DB 5 函数 × 正常 / 空表 / 重写 / 引号注入，命令层白名单 / 字节 / stub / 4 集成链路 / `ensure_writable` 三态。Dev 显式标注"未覆盖项"（`#[tauri::command]` 外壳 + 真实 Tauri 进程 panic 检查）并说明替代验证策略（task_008 e2e），不掩盖局限 |
| 架构一致性 | 10% | **4** | 数据模型 / API 命名 / 目录结构与 Architect 完全一致；唯一偏离是 Dev 偏离说明的"挂接 startup + 4 孤儿模块 + 1 个 crate 依赖"。**这一偏离是合规的**：Architect § 0.7 明确把 `AppMode` 注册划入 task_002，但未识别 `startup.rs` 自身已是孤儿（Architect 的盲点）。Dev 的工程判断「挂接属于 R5 修复语义、保持最小、不增加新逻辑」站得住脚。扣 1 分原因：偏离链向上传播到 `Cargo.toml`，属于跨技术栈影响，最好让 Architect 在 task_010 architecture_guard 时复核确认 |
| 可维护性 | 20% | **5** | `BUILTIN_VERSION_MVP` 单点常量（R3 启用时一处改）；`default_text_placeholder_for` / `required_placeholders_placeholder_for` 故意命名为 `_placeholder_for` 让 task_003 接入时一搜即定；migration V15 注释明确指出 R3 / R7 / ADR-002 落点；偏离说明 60 行详尽到位（PR / Architect / Conductor 任何一方未来回看都能复现决策上下文）；占位符 stub 测试故意脆弱以触发接入提醒；所有新模块带 `//!` 模块级文档说明上下游 |

**综合分计算**：

```
0.25 × 5 + 0.20 × 5 + 0.15 × 5 + 0.10 × 5 + 0.10 × 4 + 0.20 × 5
= 1.25 + 1.00 + 0.75 + 0.50 + 0.40 + 1.00
= 4.90 / 5
```

**综合分：4.9/5**

---

## 总体判断

- [x] **PASS**
- [ ] FIX
- [ ] BLOCKER

---

## 问题列表

### BLOCKER（必须修复，否则不可能 PASS）

**无**

### MAJOR（强烈建议修复）

**无**

### MINOR（可选）

1. **`reset_user_prompt` 中 `Some(m)` 分支的 `validate_module` 顺序**
   - **观察**：当前顺序为 `ensure_writable → validate_module`（`commands/user_prompt.rs:194-200`）；从快速失败角度，把 `validate_module` 放在 `ensure_writable` 之前可以让"未知 module"在 ReadOnly 模式下也得到精确错误（而不是被 ReadOnly 兜底吞掉）。
   - **影响**：纯锦上添花。当前实现在功能上完全正确，且 input.md AC-3 对 reset 仅要求"必经 `ensure_writable`"，对白名单与 `ensure_writable` 的相对顺序无强约束。
   - **是否要改**：不强求。task_003 / task_004 在 review 时若觉得对齐 `save_user_prompt` 顺序更一致，可顺手调一行。

2. **`upsert_overwrites_existing_row` 测试没有断言 `updated_at` 严格递增**
   - **观察**：测试代码注释（`db/user_prompt.rs:162-163`）已明确解释为何不依赖 `updated_at` 严格递增。这是 Dev 自己已经在 output.md 第 3 点标注的"需要 Reviewer 关注"项。
   - **是否要改**：MVP 不需要。若后续 UX 引入"上次修改时间"展示，可在 P1/P2 改为 nanosecond 精度或引入"同 module 同秒内拒绝二次 upsert"守卫。

3. **`PromptInfo` 字段 `display_title` 仅在 `assemble_prompt_info` 时通过 `display_title_for` 查表，未来本地化（i18n）会有耦合**
   - **观察**：当前硬编码中文标题（`commands/user_prompt.rs:55-63`）。MVP 中可接受。
   - **是否要改**：MVP 不需要。task_009 UX 评审若决定走 i18n，应在 task_007 前端把"标题来源"重新定位（i.e. 前端字符串表而非后端常量）。这超出 task_002 范围。

---

## 给 Dev 的修复指引

**判定为 PASS，不需要修复。**

按 Reviewer prompt § 判断标准：综合分 4.9/5 ≥ 3.5，无 BLOCKER，无 MAJOR。MINOR 3 项均为"可选"或"未来风险预留"，task_002 范围内已交付完整且可立即支撑 task_003 / task_005 启动。

**Conductor 注意事项**（不属于修复，但建议在状态转移时记录）：
- Dev 偏离 1（挂接 startup + 4 孤儿模块 + `unicode-normalization` 加入 dependencies）属合规偏离，建议在 task_010 architecture_guard 复核时验证「孤儿模块挂接后是否引入了非预期的运行时副作用」（应为零，但作为 L 复杂度强制复核项）。
- task_003 接入 `llm/prompt_runtime.rs` 时，必须：
  1. 替换 `default_text_placeholder_for` / `required_placeholders_placeholder_for` 为真实实现
  2. 替换 `validate_placeholders_stub` 为真实占位符校验
  3. 删除或更新 `validate_placeholders_stub_always_ok_in_this_task` 测试（Dev 故意脆弱，会失败）
  4. 删除或更新 `assemble_prompt_info_none_row_returns_defaults` / `assemble_prompt_info_with_row_carries_user_text` 中关于 `[default for tagging]` / 空 vec 的断言
