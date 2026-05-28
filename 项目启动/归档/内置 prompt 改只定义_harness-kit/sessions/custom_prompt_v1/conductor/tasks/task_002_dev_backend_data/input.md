# Task 输入 — task_002_dev_backend_data

## 目标

在 NCdesktop 后端落地 `user_custom_prompt` 表 + DB 层 + 4 个 Tauri command + `AppMode` 前置注册修复，构成"用户自定义 Prompt"功能的持久化与 IPC 基础设施。

## 前置条件

- 依赖 task：无（流水线第一个 Dev task）
- 必须先存在的文件/接口：
  - `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/db/migration.rs`（V14 终止位置，需追加 V15）
  - `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/db/settings.rs`（连接获取 / params! 模式参考）
  - `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/startup.rs`（`AppMode` 定义 + `ensure_writable`）
  - `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/lib.rs`（setup / invoke_handler 注册位置）

## 验收标准（Acceptance Criteria）

1. **AC-1（migration V15）** — 新增 `fn v15_user_custom_prompt(conn)` 函数；`run_migrations` 推 `user_version` 到 15。Schema 严格按 Architect output.md § 5.1：
   - 表名 `user_custom_prompt`，列：`module TEXT PRIMARY KEY / prompt_text TEXT NOT NULL / is_custom INTEGER NOT NULL DEFAULT 0 / builtin_version TEXT NOT NULL DEFAULT '1.0' / updated_at TEXT NOT NULL DEFAULT (datetime('now'))`
   - 索引：`idx_user_custom_prompt_is_custom`
   - DDL 全部使用 `CREATE TABLE IF NOT EXISTS` / `CREATE INDEX IF NOT EXISTS`
   - 单测覆盖：`fresh_db_runs_all_migrations_to_v15` 推到 15；`run_migrations_is_idempotent` 升级到 15 仍幂等；空库连跑两次 migration 不报错

2. **AC-2（db/user_prompt.rs）** — 新建该文件并通过 `db/mod.rs` 暴露 `pub mod user_prompt;`。函数签名：
   ```rust
   pub struct UserPromptRow { pub module: String, pub prompt_text: String, pub is_custom: bool, pub builtin_version: String, pub updated_at: String }
   pub fn get(conn: &Connection, module: &str) -> Result<Option<UserPromptRow>, String>;
   pub fn upsert(conn: &Connection, module: &str, prompt_text: &str) -> Result<(), String>;  // is_custom=1, updated_at=now()
   pub fn delete(conn: &Connection, module: &str) -> Result<(), String>;
   pub fn delete_all(conn: &Connection) -> Result<(), String>;
   pub fn list_all(conn: &Connection) -> Result<Vec<UserPromptRow>, String>;
   ```
   全部使用 `params!` 参数化；单测覆盖 upsert/get/delete/list_all 正常路径与空表路径

3. **AC-3（commands/user_prompt.rs）** — 新建该文件并通过 `commands/mod.rs` 暴露 `pub mod user_prompt;`。包含 4 个 `#[tauri::command]`：
   - `list_user_prompts(database: State<Database>) -> Result<Vec<PromptInfo>, String>`：恒定返回 4 条记录（按 module 顺序：tagging / para / concept / aggregation），每条 `PromptInfo` 字段见 Architect output.md § 5.2，`default_text` 来源**暂时**返回占位字符串 `"[default for {module}]"`（真正实现在 task_003），`required_placeholders` 与 `max_bytes` 同样占位（`vec![]` 与 `16384`），上线在 task_003 联调时回填
   - `get_user_prompt(database, module: String)` -> `Result<PromptInfo, String>`
   - `save_user_prompt(database, mode: State<AppMode>, module: String, text: String)` -> `Result<(), String>`：必经 `validate_module` 白名单（4 module）+ `ensure_writable(mode.inner())` + 字节长度校验（>16 KiB 拒绝，返回中文错误）；占位符校验本期占位（实现于 task_003，本 task 仅留接入点 stub `validate_placeholders_stub`，返回 Ok）
   - `reset_user_prompt(database, mode: State<AppMode>, module: Option<String>)` -> `Result<(), String>`：`None` = `delete_all`，`Some(m)` = `delete`，必经 `ensure_writable`
   - 单测覆盖：white-list 拒绝未知 module；字节超限拒绝；`reset_user_prompt(None)` 删全部；`save → get → reset → get` 完整链路

4. **AC-4（lib.rs setup 修复 + 注册）** — 修改 `src-tauri/src/lib.rs`：
   - 在 `app.manage(database);` 之后追加 `app.manage(crate::startup::AppMode::Normal);`（详细理由见 Architect output.md § 0.7 / R5；MVP 不接入完整 bootstrap 流程，避免任务范围爆炸）
   - 在 `invoke_handler![]` 中追加 4 行：`commands::user_prompt::list_user_prompts, get_user_prompt, save_user_prompt, reset_user_prompt`
   - 不修改其他既有 invoke_handler 注册

5. **AC-5（Cargo test 全绿）** — `cd src-tauri && cargo test --lib user_prompt`（含 db 层与 command 层单测）必须全部 PASS；`cargo test --lib migration` 既有 4 个 v11~v14 测试不被破坏

6. **AC-6（不破坏既有调用）** — `cargo build` 通过；`cargo test --lib` 全表跑（不仅 `user_prompt`）必须 PASS，不允许引入回归

## 技术约束

- **代码规范**（继承 `session_context.md` § 5）：
  - 数据访问层走 `db/*.rs` 文件；command 层走 `commands/*.rs`；二者职责严格分离
  - 错误返回统一 `Result<T, String>`，错误消息全中文
  - 全部使用 `rusqlite::params!` 参数化 SQL，禁止字符串拼接
- **Architect 方案约束**（来自 output.md）：
  - 不修改 PR-4 半成品文件（`commands/prompts.rs` / `db/settings.rs` 中除调用 settings::get/set 外的部分）
  - 表名、列名、索引名严格按 § 5.1
  - 命名一律 `user_prompt` 前缀（避免与 PR-4 的 `prompt.override.*` 命名空间冲突，详见 ADR-005 / R6）
- **AppMode 注册**：仅追加 `app.manage(AppMode::Normal)`，不要试图引入完整 `startup::bootstrap` 流程（那是单独的、跨多任务的工作）。Architect 已确认此简化在 MVP 范围安全。

## 参考文件

**必读**：
- Architect output.md：`/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/内置 prompt 改只定义_harness-kit/sessions/custom_prompt_v1/conductor/tasks/task_001_architect/output.md`
  - § 0.4（migration 机制现状）
  - § 0.7（AppMode 未注册的现实缺口）
  - § 5（数据模型）
  - § 6.1（Tauri commands 签名）
  - § 9 R5 / R6 / R7

**代码参考（必读）**：
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/db/migration.rs:147-209` — V13/V14 写法（含幂等守卫 / `PRAGMA user_version` 推进 / 测试范式）
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/db/settings.rs` — DB 层 `get / set / get_all` 模式参考
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/commands/prompts.rs` — 参考（**不复用**），包括其 `validate_field` / `params!` / `#[tauri::command]` 范式
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/commands/categories.rs:9` — `use crate::startup::{ensure_writable, AppMode};` 引用范式

**测试参考**：
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/db/migration.rs:758-1049` — migration 测试范式（`Connection::open_in_memory` / `fresh_db_runs_all_migrations` / 幂等回归）

## 预估影响范围

- **新建文件**：
  - `src-tauri/src/db/user_prompt.rs`
  - `src-tauri/src/commands/user_prompt.rs`
- **修改文件**：
  - `src-tauri/src/db/mod.rs`（加 `pub mod user_prompt;`）
  - `src-tauri/src/commands/mod.rs`（加 `pub mod user_prompt;`）
  - `src-tauri/src/db/migration.rs`（追加 v15 函数 + dispatcher 入口 + 测试 fresh/idempotent 推到 15）
  - `src-tauri/src/lib.rs`（① `app.manage(AppMode::Normal)` ② invoke_handler 加 4 行）
- **预估变更**：~700 行（含测试 ~300 行）
