# Task 交付 — task_002_dev_backend_data

## 实现摘要

落地"用户自定义 Prompt"功能的持久化与 IPC 基础设施。共完成 6 块工作：

1. **migration V15** — 新建 `user_custom_prompt` 表（5 列 + 1 索引），`run_migrations` dispatcher 推 `user_version` 至 15，DDL 全 `IF NOT EXISTS` 保幂等（R7 防御）；3 个单测覆盖 fresh / 二次跑 / 表已存在残缺路径。
2. **DB 数据访问层** `src-tauri/src/db/user_prompt.rs` — `UserPromptRow` 结构 + `get / upsert / delete / delete_all / list_all` 5 个函数，全 `rusqlite::params!` 参数化；8 个单测覆盖正常路径 / 空表 / 重复 upsert / 注入防御。
3. **Tauri command 层** `src-tauri/src/commands/user_prompt.rs` — `PromptInfo` 类型 + 4 个 `#[tauri::command]`：`list_user_prompts`（恒返 4 条按 `tagging / para / concept / aggregation` 顺序）、`get_user_prompt`、`save_user_prompt`（白名单 → `ensure_writable` → 16 KiB 字节校验 → `validate_placeholders_stub`）、`reset_user_prompt`（`None` 删全部，`Some(m)` 删单条）。`default_text / required_placeholders / max_bytes` 用占位值，task_003 联调时回填（input.md AC-3 明确）。9 个单测覆盖白名单 / 字节校验 / `assemble_prompt_info` / 4 组集成链路 / `ensure_writable` 守卫。
4. **`lib.rs` setup 修复（R5）** — 在 `app.manage(database)` 之后追加 `app.manage(crate::startup::AppMode::Normal)`，invoke_handler 追加 4 行。
5. **模块挂接** — `db/mod.rs` 加 `pub mod user_prompt;`；`commands/mod.rs` 加 `pub mod user_prompt;`；`lib.rs` 加 `pub mod startup;` —— 见下"偏离说明"。
6. **R5 衍生连带修复** — 由于 `startup` 模块此前是孤儿（lib.rs 未声明），第一次被 `commands::user_prompt` 引用就拖出 `db::repair / utils::nfc / utils::safe_rename / utils::ipc_error / unicode-normalization` 的全链式注册缺口。在 task 范围内做了最小连带挂接（仅 mod 声明 + 1 个 crate 依赖），未引入新逻辑。详见"偏离说明"。

## 核心设计决策

- **`is_custom` 二态**：DB 层 `upsert` 永远写 `is_custom=1`；"恢复默认"统一走 `delete`，无记录 = `user_text=None` = 回退到内置默认（与 ADR-001/ADR-002 一致）。这样运行时回退判断只需检查"记录是否存在"，避免引入第三态。
- **`builtin_version` MVP 行为**：upsert 时固定写 `"1.0"`，本 task 不读取（R3 预留）。`db/user_prompt.rs` 中 `BUILTIN_VERSION_MVP` 常量为单点修改入口。
- **`updated_at` 显式写 UTC RFC3339**：而非依赖 SQLite `datetime('now')`，避免跨平台时区/格式漂移。
- **`validate_placeholders_stub`**：本 task 故意保留 stub 为"恒 Ok"，并在测试中显式断言这一行为，让 task_003 实现真正校验时**测试会失败**，触发显式更新提醒（防止接入点被遗忘）。
- **command 层测试策略**：`#[tauri::command]` 外壳依赖 `State<T>`，cargo test --lib 环境无 Tauri App。我把白名单、字节校验、占位符 stub、`assemble_prompt_info` 全部抽成可测私有函数；4 个 integration 测试通过直接驱动 DB 层 + `assemble_prompt_info` 模拟命令主流程，覆盖 `save → get → reset → get` / `reset(None)` 删全部 / `list_user_prompts` 恒 4 条 / 单 module 保存其他保持 None。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `src-tauri/src/db/user_prompt.rs` | 新建 | 数据访问层：`UserPromptRow` + `get/upsert/delete/delete_all/list_all` + 8 单测 |
| `src-tauri/src/commands/user_prompt.rs` | 新建 | 命令层：`PromptInfo` + 4 `#[tauri::command]` + 9 单测 |
| `src-tauri/src/db/mod.rs` | 修改 | 追加 `pub mod user_prompt;` 与 `pub mod repair;`（详见偏离说明） |
| `src-tauri/src/commands/mod.rs` | 修改 | 追加 `pub mod user_prompt;` |
| `src-tauri/src/db/migration.rs` | 修改 | 追加 `v15_user_custom_prompt` 函数 + `run_migrations` dispatcher 分支；既有 `fresh_db_runs_all_migrations_to_v12` 测试名改为 `_to_v15` 并断言推到 15；既有 `v11_repairs_user_version_10_missing_conversion_meta` / `run_migrations_is_idempotent` 测试断言更新至 v15；新增 `v15_idempotent_with_existing_table` 测试 |
| `src-tauri/src/lib.rs` | 修改 | ① 追加 `pub mod startup;`（R5 衍生）② 在 `app.manage(database)` 之后追加 `app.manage(crate::startup::AppMode::Normal)` ③ invoke_handler 追加 4 行 |
| `src-tauri/src/utils/mod.rs` | 修改 | 追加 `pub mod nfc; pub mod safe_rename; pub mod ipc_error;`（R5 衍生，仅挂接既有孤儿文件） |
| `src-tauri/Cargo.toml` | 修改 | 追加 `unicode-normalization = "0.1"`（`utils/nfc.rs` 既有依赖；R5 衍生） |

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（`db/user_prompt.rs` + `commands/user_prompt.rs`）
- [x] API 路径/命名与 Architect 方案一致（4 个 command 名、`PromptInfo` 字段 camelCase、4 module 白名单、字节上限 16 KiB）
- [x] 数据模型与 Architect § 5.1 完全一致（表名 / 5 列名 / 类型 / 默认值 / 索引名 `idx_user_custom_prompt_is_custom`）
- [x] 未引入计划外的新依赖（**一个例外**：`unicode-normalization`，详见偏离说明）

### 偏离说明

**偏离 1（R5 衍生）：`pub mod startup;` 与一组孤儿模块挂接 + 1 个 crate 依赖**

- **触发原因**：`commands::user_prompt` 写命令必须 `use crate::startup::{ensure_writable, AppMode}`（与 `commands/categories.rs:9` / `commands/prompts.rs:10` 同范式）。但 `src/startup.rs` 在 lib.rs 中**从未被 `pub mod` 声明过**，整个 `startup.rs` 处于"文件存在但编译不可达"的孤儿状态。Architect § 0.7 #1 指出"此为既有缺口，须在 task_002 中一并修复"，input.md AC-4 也明确写 `crate::startup::AppMode::Normal`，前提是 `startup` 模块可达。
- **第一波连带**：`startup.rs` 顶部 `use crate::db::repair::{...}; use crate::utils::nfc::nfc_heal_workspace; use crate::utils::safe_rename::cleanup_pending_scan;` —— 这些子模块同样是已存在但 mod.rs 未声明的孤儿。
- **第二波连带**：`utils/safe_rename.rs` 顶部 `use crate::utils::ipc_error::IpcError;` —— `ipc_error.rs` 同样未声明。
- **第三波连带**：`utils/nfc.rs` 顶部 `use unicode_normalization::UnicodeNormalization;` —— `unicode-normalization` crate 通过 idna 等间接依赖已在 Cargo.lock 中（version 0.1.25），但 Cargo.toml `[dependencies]` 段从未显式声明，Rust 不允许直接 `use`。
- **采取的修复**：
  - `lib.rs` 加 `pub mod startup;`
  - `db/mod.rs` 加 `pub mod repair;`
  - `utils/mod.rs` 加 `pub mod nfc; pub mod safe_rename; pub mod ipc_error;`
  - `Cargo.toml` 加 `unicode-normalization = "0.1"`
- **任何挂接均不调用新逻辑**：`setup` 中只 `app.manage(AppMode::Normal)`，**不**接入 `startup::bootstrap` 流程（input.md 明确禁止）。挂接产生的副作用 = 这些原本不可达的模块及其测试现在被编译并跑了（285 全绿），不破坏任何行为。
- **Architect 是否预知**：Architect § 0.7 / R5 已识别 `AppMode` 注册缺口，但**未列举** `startup.rs` 自身的全链路依赖也都是孤儿状态 —— 这是 Architect 的盲点。我做出的工程判断是：既然 Architect 把"AppMode 注册"明确划入 task_002，则连带的最小挂接也属于 R5 修复语义。所有挂接保持"只加 mod 声明，不增加新逻辑"，将范围控制在最小。
- **Reviewer 决策点**：如认为 `unicode-normalization` 加入 Cargo.toml 超出 task 范围，可在 Reviewer 阶段提议把 `commands::user_prompt` 改为引用本地 minimal `AppMode` 内联类型；但代价是后续 `commands/prompts.rs`、`commands/categories.rs` 一旦也接入 invoke_handler 同样会触发这些挂接，最终仍要做一次。

**偏离 2（无）**：除上述外无其他偏离。本 task 范围内的 4 个白名单 module / 字节上限 / DDL / API 签名 / 错误中文化均与 Architect 方案逐项对齐。

## 测试命令

```bash
cd "/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri"
cargo test --lib user_prompt 2>&1 | tail -60
cargo test --lib migration 2>&1 | tail -60
cargo build 2>&1 | tail -40
cargo test --lib 2>&1 | tail -8
```

## 测试结果

### `cargo test --lib user_prompt`（AC-5 / AC-2 / AC-3）

```
warning: `notecapt` (lib test) generated 5 warnings (run `cargo fix --lib -p notecapt --tests` to apply 4 suggestions)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.21s
     Running unittests src/lib.rs (target/debug/deps/app_lib-3e3f381fe43829f8)

running 22 tests
test commands::user_prompt::tests::assemble_prompt_info_none_row_returns_defaults ... ok
test commands::user_prompt::tests::assemble_prompt_info_with_row_carries_user_text ... ok
test commands::user_prompt::tests::ensure_writable_blocks_readonly_mode_for_writes ... ok
test commands::user_prompt::tests::byte_len_under_limit_passes ... ok
test commands::user_prompt::tests::byte_len_over_limit_rejects_with_chinese_message ... ok
test commands::user_prompt::tests::byte_len_counts_bytes_not_chars ... ok
test commands::user_prompt::tests::validate_module_accepts_four_whitelist ... ok
test commands::user_prompt::tests::validate_module_rejects_unknown ... ok
test commands::user_prompt::tests::validate_placeholders_stub_always_ok_in_this_task ... ok
test db::user_prompt::tests::list_all_returns_empty_on_empty_table ... ok
test db::user_prompt::tests::delete_on_missing_row_is_noop ... ok
test commands::user_prompt::tests::integration_save_get_reset_get_roundtrip ... ok
test db::user_prompt::tests::list_all_returns_rows_sorted_by_module ... ok
test commands::user_prompt::tests::integration_save_then_list_includes_user_text_for_saved_module_only ... ok
test db::user_prompt::tests::get_returns_none_on_empty_table ... ok
test commands::user_prompt::tests::integration_list_returns_four_in_fixed_order_on_empty_db ... ok
test commands::user_prompt::tests::integration_reset_none_deletes_all_four_modules ... ok
test db::user_prompt::tests::delete_all_clears_table ... ok
test db::user_prompt::tests::delete_removes_row ... ok
test db::user_prompt::tests::params_protect_against_quote_injection ... ok
test db::user_prompt::tests::upsert_then_get_roundtrips ... ok
test db::user_prompt::tests::upsert_overwrites_existing_row ... ok

test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 263 filtered out; finished in 0.13s
```

### `cargo test --lib migration`（AC-5 / AC-1）

```
warning: `notecapt` (lib test) generated 5 warnings (run `cargo fix --lib -p notecapt --tests` to apply 4 suggestions)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.22s
     Running unittests src/lib.rs (target/debug/deps/app_lib-3e3f381fe43829f8)

running 11 tests
test db::migration::tests::v11_repairs_user_version_10_missing_conversion_meta ... ok
test db::migration::tests::v12_alter_is_idempotent_against_existing_column ... ok
test db::migration::tests::v15_idempotent_with_existing_table ... ok
test db::migration::tests::run_migrations_is_idempotent ... ok
test db::migration::tests::fresh_db_runs_all_migrations_to_v15 ... ok
test db::migration::tests::v14_does_not_overwrite_existing_failure_code ... ok
test db::migration::tests::v14_backfills_extracted_with_empty_content ... ok
test db::migration::tests::v14_only_touches_latest_row_per_asset ... ok
test db::migration::tests::v14_is_idempotent ... ok
test db::migration::tests::v14_keeps_null_when_content_present ... ok
test db::tests::open_runs_migrations ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 274 filtered out; finished in 0.12s
```

### `cargo build`（AC-6）

```
warning: unused import: `PathBuf`
  --> src/commands/dropzone.rs:10:23
warning: unused variable: `client` --> src/llm/chat.rs:109:5
warning: unused variable: `messages` --> src/llm/chat.rs:110:5
warning: unused variable: `on_chunk` --> src/llm/chat.rs:111:5
warning: fields `block_type` and `thinking` are never read --> src/llm/chat.rs:47:9

warning: `notecapt` (lib) generated 5 warnings (run `cargo fix --lib -p notecapt` to apply 4 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.19s
```

零 error；5 个 warning 均为既有代码（`dropzone.rs` / `llm/chat.rs`）的历史 warning，**与本 task 无关**（基线 `cargo build` 同样产出此 5 个 warning）。

### `cargo test --lib`（全表，AC-6 不回归）

```
test result: ok. 285 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.56s
```

注：未挂接 `startup` 前基线全表测试数低于 285（因为 `startup::tests` / `utils::safe_rename::tests` / `utils::nfc::tests` 等孤儿模块测试此前不被编译进 lib_test）。本次挂接后这些孤儿测试**额外**跑了一遍，全部 PASS —— 这是另一份证据：挂接没有破坏既有行为。

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|----------|----------|------|-----------|
| ✅ 正常路径 | fresh DB 跑 migration 推到 user_version=15 | 已测 | PASS — `fresh_db_runs_all_migrations_to_v15` |
| ✅ 正常路径 | DB 层 upsert → get 往返；is_custom=true / updated_at 非空 | 已测 | PASS — `upsert_then_get_roundtrips` |
| ✅ 正常路径 | command 层 save → get → reset → get 完整链路 | 已测 | PASS — `integration_save_get_reset_get_roundtrip` |
| ✅ 正常路径 | `list_user_prompts` 恒 4 条按 tagging/para/concept/aggregation 顺序 | 已测 | PASS — `integration_list_returns_four_in_fixed_order_on_empty_db` |
| ✅ 正常路径 | `reset_user_prompt(None)` 等价 `delete_all` | 已测 | PASS — `integration_reset_none_deletes_all_four_modules` |
| ✅ 正常路径 | 部分自定义场景 — 只 save concept 后 list，其他 module user_text 为 None | 已测 | PASS — `integration_save_then_list_includes_user_text_for_saved_module_only` |
| ✅ 正常路径 | `cargo build` 通过 + 全 285 测试不回归 | 已测 | PASS |
| ⚠️ 边界 | migration 幂等：连续两次 run_migrations 推到 15 不报错 | 已测 | PASS — `run_migrations_is_idempotent` |
| ⚠️ 边界 | R7 残缺：user_version=14 但 user_custom_prompt 表已存在 → v15 仍幂等 | 已测 | PASS — `v15_idempotent_with_existing_table` |
| ⚠️ 边界 | 模拟生产 user_version=10 路径，跑 V11..V15 推到 15 | 已测 | PASS — `v11_repairs_user_version_10_missing_conversion_meta` |
| ⚠️ 边界 | 空表 list_all 返回 `[]` | 已测 | PASS — `list_all_returns_empty_on_empty_table` |
| ⚠️ 边界 | 空表 get 返回 None | 已测 | PASS — `get_returns_none_on_empty_table` |
| ⚠️ 边界 | delete 不存在的 module 不报错（恢复默认幂等语义） | 已测 | PASS — `delete_on_missing_row_is_noop` |
| ⚠️ 边界 | upsert 覆盖已有行（同 module 第二次 save） | 已测 | PASS — `upsert_overwrites_existing_row` |
| ⚠️ 边界 | 字节恰好 = 16 KiB 通过；= 16 KiB+1 拒绝 | 已测 | PASS — `byte_len_under_limit_passes` + `byte_len_over_limit_rejects_with_chinese_message` |
| ⚠️ 边界 | UTF-8 多字节字符按字节计数（非字符数） | 已测 | PASS — `byte_len_counts_bytes_not_chars` |
| ⚠️ 边界 | list_all 按 module ASC 排序 | 已测 | PASS — `list_all_returns_rows_sorted_by_module` |
| ❌ 异常 | 非白名单 module（"classify" / "" / "Tagging" 大小写） | 已测 | PASS — `validate_module_rejects_unknown` 拒绝并返中文错 |
| ❌ 异常 | 字节超限错误消息为中文且含字节数 | 已测 | PASS — 错误含 "自定义 Prompt 过长" + 字节数 |
| ❌ 异常 | SQL 注入防御：prompt 中含 `'; DROP TABLE ...; --` | 已测 | PASS — `params_protect_against_quote_injection` 原样保存 + 表未被 DROP |
| ❌ 异常 | ReadOnly 模式下写命令被 `ensure_writable` 拒绝 | 已测 | PASS — `ensure_writable_blocks_readonly_mode_for_writes` |
| ⚠️ 接入点 | `validate_placeholders_stub` 任何输入恒 Ok | 已测 | PASS — `validate_placeholders_stub_always_ok_in_this_task`（task_003 实现真正校验时此测试会失败 = 故意脆弱） |
| ⚠️ 未覆盖 | `#[tauri::command]` 外壳本体 | 未测 | 跳过原因：Tauri State 注入只在 App 运行时生效；命令体逻辑通过 integration test 直接调用底层 DB + `assemble_prompt_info` 等价验证 |
| ⚠️ 未覆盖 | `save_user_prompt` 在真实 Tauri 进程中 panic 检查（R5） | 未测 | 跳过原因：cargo test --lib 不启动 Tauri；R5 修复正确性通过 `app.manage(AppMode::Normal)` 静态注册 + 编译期类型检查保证。task_008 e2e 会验证 |

## 已知局限

1. **`default_text / required_placeholders / max_bytes` 是占位值**（input.md AC-3 明确要求）：`default_text` 形如 `"[default for tagging]"`，`required_placeholders` 为空 vec，`max_bytes` 固定 16384。真正实现由 task_003（`llm/prompt_runtime.rs::default_for`）回填。前端 task_005~007 联调期间会看到占位字符串 —— 这是预期行为。
2. **`validate_placeholders_stub` 是 no-op**：与上一条配套，task_003 接入真实占位符规则后会让 `validate_placeholders_stub_always_ok_in_this_task` 测试失败，触发显式更新。
3. **`builtin_version` 字段写入但不读取**：R3 预留，本 task 与 task_003 都不消费；P2 内置 Prompt 升级提示功能才会用到。
4. **`#[tauri::command]` 外壳本身没有单测**：见自测矩阵"未覆盖"行。task_008 e2e 会通过真实 IPC 验证。
5. **挂接的孤儿模块（`startup` / `db::repair` / `utils::nfc` / `utils::safe_rename` / `utils::ipc_error`）现在是"可编译且测试通过"状态**，但 lib.rs 的 setup 仍未调用 `startup::bootstrap` —— task_002 范围内不接入。后续若 PR 想要启用完整 Degraded/ReadOnly 流程，需要另一个 task 把 `app.manage(AppMode::Normal)` 替换为 `app.manage(bootstrap(&db_path).mode)`。

## 需要 Reviewer 特别关注的地方

1. **R5 衍生的连带挂接（最关键）**：`lib.rs` 加 `pub mod startup;` 后必然链式挂接 `db::repair` / `utils::nfc` / `utils::safe_rename` / `utils::ipc_error` + `Cargo.toml` 加 `unicode-normalization`。请审视：
   - 是否同意"挂接孤儿模块属于 R5 修复语义"的工程判断？
   - `unicode-normalization` 加入 dependencies 是否可接受？（替代方案：让 `commands::user_prompt` 引用本地 inline 的 `AppMode` 拷贝；但若 `commands/categories.rs` 与 `commands/prompts.rs` 未来也接入 invoke_handler，挂接仍要做。）
   - 见"偏离说明"完整论证。
2. **`assemble_prompt_info` 中 `default_text / required_placeholders / max_bytes` 是占位值**：见 `commands/user_prompt.rs::default_text_placeholder_for / required_placeholders_placeholder_for`。task_003 必须替换这两个函数为真正实现 + 删除占位测试。
3. **`upsert_overwrites_existing_row` 测试中关于 `updated_at` 的注释**：MVP 不依赖 `updated_at` 严格递增，仅依赖"被覆盖"。如果后续 UX 需要"上次修改时间"展示，需要在 DB 层显式拒绝同秒内的覆盖或改为 nanosecond 精度。
4. **`validate_placeholders_stub` 的"故意脆弱"测试**：`validate_placeholders_stub_always_ok_in_this_task` 故意断言占位行为，task_003 改写时会失败 —— 这是接入点保护。Reviewer 请确认这个设计是否符合预期，或建议改为 `#[ignore]` 标记。
5. **`builtin_version = "1.0"` 硬编码**：`BUILTIN_VERSION_MVP` 在 `db/user_prompt.rs`。R3 启用时应改为运行时计算（与 `llm/prompts.rs` 内置常量版本同步）。
6. **migration V15 不写默认行**（ADR-002）：测试 `fresh_db_runs_all_migrations_to_v15` 显式断言 row_count=0。这与 R3 "用户已自定义不被升级覆盖"保持一致。
