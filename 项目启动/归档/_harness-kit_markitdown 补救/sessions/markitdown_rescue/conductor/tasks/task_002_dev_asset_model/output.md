# Task 交付 — task_002_dev_asset_model

## 实现摘要

为 `Asset` 模型与 `assets` 表追加 `source_asset_id: Option<String>` 与 `derivative_version: i32` 两个字段，用于支撑后续 task 的 markdown 衍生件家族关系与转换轮次追踪（来自 Architect ADR-001 / ADR-002）。

核心设计决策：
1. **Asset 加 `#[derive(Default)]`**：使所有现有构造点可通过 `..Default::default()` 一键补齐新字段，无需逐字段填写默认值。验证过现有字段（`String` / `i64` / `bool` / `Option<String>`）均实现 `Default`。
2. **迁移 V5**：使用 `PRAGMA table_info(assets)` 守卫，仅在列不存在时 `ALTER TABLE ADD COLUMN`，幂等可重跑。封装了通用 helper `list_table_columns(conn, table)` 供未来其它迁移复用。
3. **字段位置**：紧跟 `is_starred` 之后追加，对齐 INSERT/SELECT/`from_row` 三处列序。
4. **关键意外发现**：`src/extraction/mod.rs:4` 中 `scheduler` 模块**当前是注释屏蔽**的（`// pub mod scheduler;`，原因是依赖未恢复的 db 函数）。因此 task_003/004 缺失的 `propagate_tags_to_derivative` / `find_markdown_derivative` / `update_markdown_derivative` / `set_derivative_version` **当前并不会产生编译错误**——它们将在 task_003/004 实现后随 scheduler 一同被取消注释。这意味着本 task 完成后 `cargo check` **全绿无残留错误**。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `src-tauri/src/models/asset.rs` | 修改 | 加 `#[derive(Default)]`；紧跟 `is_starred` 追加 `source_asset_id: Option<String>` 与 `derivative_version: i32`，均带 `#[serde(default)]` 与 doc 注释 |
| `src-tauri/src/db/migration.rs` | 修改 | 新增 `v5_asset_derivative_columns` + `list_table_columns` helper；`run_migrations` 加 `current_version < 5` 分支；PRAGMA 守卫 + `idx_assets_source_asset_id` |
| `src-tauri/src/db/asset.rs` | 修改 | `insert` SQL 加 2 列；`ASSET_SELECT` 常量加 2 列；`get_by_project_and_tag` 内联 SQL 加 2 列；`row_to_asset` 读 col 13/14 |
| `src-tauri/src/commands/dropzone.rs` | 修改 | 第 541 处构造点加 `..Default::default()` |
| `src-tauri/src/commands/asset.rs` | 修改 | 第 59 处构造点加 `..Default::default()` |
| `src-tauri/src/commands/sync.rs` | 修改 | 第 155 处构造点加 `..Default::default()` |

> 第 5 处构造点（`src-tauri/src/extraction/scheduler.rs:655`）已在 PM/历史改动中显式填写了两个字段（`source_asset_id: Some(...)`、`derivative_version: next_version`），且 scheduler 模块当前被注释屏蔽，**无需改动**。

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（仅修改既有文件，未新增 / 移动）
- [x] API 路径/命名与 Architect 方案一致（无新增 Tauri 命令）
- [x] 数据模型与 Architect 方案一致：字段类型 `Option<String>` / `i32`，SQL `TEXT DEFAULT NULL` / `INTEGER NOT NULL DEFAULT 0`，索引 `idx_assets_source_asset_id`（§五.1 / §五.3）
- [x] 未引入计划外的新依赖（`Cargo.toml` 未触碰）
- [x] 迁移仅向后兼容（仅 `ALTER TABLE ADD COLUMN` + `CREATE INDEX IF NOT EXISTS`，未 DROP/RENAME）
- 偏离说明：无

## 测试命令

```bash
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri
cargo check
cargo test --no-run
```

## 测试结果

`cargo check` 完整输出：

```
warning: unused variable: `client`
   --> src/llm/chat.rs:109:5
    |
109 |     client: &LLMClient,
    |     ^^^^^^ help: if this is intentional, prefix it with an underscore: `_client`
    |
    = note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default

warning: unused variable: `messages`
   --> src/llm/chat.rs:110:5
    |
110 |     messages: Vec<ChatMessage>,
    |     ^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_messages`

warning: unused variable: `on_chunk`
   --> src/llm/chat.rs:111:5
    |
111 |     on_chunk: F,
    |     ^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_on_chunk`

warning: fields `block_type` and `thinking` are never read
  --> src/llm/chat.rs:47:9
   |
45 | struct AnthropicContent {
   |        ---------------- fields in this struct
46 |     #[serde(rename = "type")]
47 |     pub block_type: String,
   |         ^^^^^^^^^^
48 |     pub text: Option<String>,
49 |     pub thinking: Option<String>,
   |         ^^^^^^^^
   |
   = note: `AnthropicContent` has a derived impl for the trait `Debug`, but this is intentionally ignored during dead code analysis
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: `notecapt` (lib) generated 4 warnings (run `cargo fix --lib -p notecapt` to apply 3 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.67s
```

**0 个 error，4 个 warning，均与本 task 无关（在 `src/llm/chat.rs`，是 pre-existing dead code）。`cargo check` 完全通过。**

`cargo test --no-run` 摘录（lib test 全绿；唯一失败的是 pre-existing 集成测试 `tests/workspace_folders_integration.rs`，与本 task 完全无关 —— 该测试引用了不存在的 `app_lib::utils::write_guard` / `count_folder_assets_impl` 等符号，属于另一条工作流的遗留）：

```
warning: `notecapt` (lib test) generated 4 warnings (4 duplicates)
warning: `notecapt` (lib) generated 4 warnings (run `cargo fix --lib -p notecapt` to apply 3 suggestions)
error[E0433]: failed to resolve: could not find `utils` in `app_lib`
  --> tests/workspace_folders_integration.rs:19:14
   |
19 | use app_lib::utils::write_guard::WorkspaceWriteGuard;
   |              ^^^^^ could not find `utils` in `app_lib`

error[E0432]: unresolved imports `app_lib::commands::workspace_folders::count_folder_assets_impl`, ...
  --> tests/workspace_folders_integration.rs:14:5

error[E0425]: cannot find function `validate_and_canonicalize` in module `app_lib::workspace`
   --> tests/workspace_folders_integration.rs:224:50

error: could not compile `notecapt` (test "workspace_folders_integration") due to 3 previous errors
```

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|----------|----------|------|-----------|
| ✅ 正常路径 — Asset 模型 | `Asset` 结构体含两个新字段且 `#[serde(default)]`、`Default` derive 生效 | 已测 | PASS — `cargo check` 通过；`..Default::default()` 在 3 处构造点编译通过证明 `Default` impl 完整 |
| ✅ 正常路径 — 迁移 V5 | V5 迁移函数被 `run_migrations` 在 `current_version < 5` 时调用 | 已测（静态） | PASS — 代码路径已加入 `run_migrations`，编译通过；运行期幂等性见下一行 |
| ⚠️ 迁移幂等性 | 在已含两列的 sqlite 上重跑 V5 不报 `duplicate column` | 已测（静态推理） | PASS — `PRAGMA table_info(assets)` 守卫确保 ALTER 仅在列缺失时执行；`CREATE INDEX IF NOT EXISTS`；`PRAGMA user_version=5` 幂等；运行期端到端验证留给后续集成测试或人工触发 |
| ✅ Default trait 可用性 | `Asset::default()` 可构造，所有字段默认值合规（空 String / 0 / false / None） | 已测 | PASS — 3 处构造点已用 `..Default::default()` 通过编译；含 `asset_type` 在内的 String 字段默认空串，由业务方在构造点显式填值（不依赖 default） |
| ✅ SELECT 列序对齐 | `ASSET_SELECT` 常量、`get_by_project_and_tag` 内联 SQL、`row_to_asset` 三处列序一致（13 = is_starred, 14 = source_asset_id, 15 = derivative_version → 索引 0..14） | 已测 | PASS — 三处均按 `id, project_id, asset_type, name, original_name, file_path, file_size, mime_type, captured_at, imported_at, source_type, source_data, is_starred, source_asset_id, derivative_version` 顺序排列；`row_to_asset` 用 `row.get(13)` / `row.get(14)` 读取 |
| ✅ 5 处构造点编译通过 | dropzone.rs:541 / asset.rs:59 / sync.rs:155 / scheduler.rs:655 / `from_row` 全部编译通过 | 已测 | PASS — `cargo check` 0 error。前三处用 `..Default::default()` 补齐；scheduler.rs:655 已显式填值（且当前模块被注释，不参与编译）；`from_row` 显式从 SQL row 取值 |
| ⚠️ 边界条件 — `derivative_version` 类型 | i32 与 SQLite INTEGER 转换无符号问题 | 未测 | rusqlite 默认将 INTEGER 双向映射 `i32`；无 unsigned，且默认值 0 落在 i32 正范围 |
| ❌ 异常路径 — 迁移失败回滚 | V5 执行到一半（如先成功 ADD COLUMN 后建索引失败）的恢复行为 | 未测 | 不在本 task scope；当前实现每个 ALTER 独立 execute_batch，部分失败将卡在中间版本（user_version 不会被推进到 5），下次启动重跑会从遗留状态继续，PRAGMA 守卫已保证不重复加列 |

## 已知局限

1. **未跑端到端迁移测试**：未实际打开一个 v4 库执行 V5 迁移再读字段。原因：项目无现成的迁移单测脚手架，且 AC-5 明确"`cargo check` 通过即可"。建议 task_003 实现 `db/asset.rs` 新函数时顺带加 `#[cfg(test)] mod tests` 覆盖整个 V1→V5 链路。
2. **`tests/workspace_folders_integration.rs` 集成测试编译失败**：完全是 pre-existing 问题，引用了不存在的符号，与本 task 无关。**不修复**（违反"只做你的 task"约束）。建议 Conductor 单开一个 housekeeping task。
3. **`derivative_version` 在 source 上的语义"双写"**：本 task 仅落字段；推进语义（什么时候 +1 / placeholder 不推进 / 真成功才推进）由 task_008 实现，本 task 不涉及。

## 需要 Reviewer 特别关注的地方

1. **`src-tauri/src/db/migration.rs` 的 PRAGMA 守卫实现**（`list_table_columns` + `v5_asset_derivative_columns`）：
   - PRAGMA 守卫的具体行：`v5_asset_derivative_columns` 函数内，先调用 `list_table_columns(conn, "assets")` 取列名集，再分别用 `if !existing_cols.iter().any(|c| c == "source_asset_id")` / `... "derivative_version"` 守卫两次 ALTER。
   - `CREATE INDEX IF NOT EXISTS` 与 `PRAGMA user_version=5` 放在守卫之外，每次都执行 —— 这是预期的，CREATE INDEX 本身幂等，user_version 重置也无副作用。
   - 请关注：如果担心 `PRAGMA user_version` 持久化时机，请确认本项目 SQLite 是否启用了 WAL 或事务包裹（本 task 沿用既有迁移函数的 `execute_batch` 模式，未改变事务边界）。

2. **`..Default::default()` 应用范围**：仅用于 3 处真实需要默认值的构造点（dropzone / asset / sync）。`scheduler.rs:655` 因显式填写新字段（且模块被注释），未改动。`db::asset::row_to_asset` 因从 SQL 显式取值，未改动。

3. **意外发现 — scheduler 模块被注释屏蔽**（`src/extraction/mod.rs:4`）：
   - 这意味着 input.md 与 Conductor 启动消息中"task_003/004 未实现导致 scheduler 残留编译错误"的假设**不成立**。当前 `cargo check` 完全干净。
   - 这同时意味着：task_003 实现 `db::asset` 三新函数 + task_004 实现 `db::tag` 两新函数后，**必须同步取消 `src/extraction/mod.rs:4` 的注释**，否则 scheduler 永远不会被编译/执行。建议在 task_003 或 task_008 的 input.md 中显式加入这一动作。

4. **`Asset` 加 `#[derive(Default)]` 的副作用面**：Asset 现在可被任何代码用 `Asset::default()` 构造出"空 asset"（id=空串、asset_type=空串等无效状态）。若有代码以"Asset 总是有效"为前提（如 `assert!(!asset.id.is_empty())`），可能在 default 路径下违反。已 grep 确认本仓内无此类 assertion，但 Reviewer 可双确认。
