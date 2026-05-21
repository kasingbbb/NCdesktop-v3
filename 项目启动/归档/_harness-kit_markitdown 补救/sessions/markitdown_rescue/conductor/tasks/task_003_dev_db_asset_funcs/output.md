# Task 交付 — task_003_dev_db_asset_funcs

## 实现摘要

在 `db/asset.rs` 顶层（与 `insert`/`get_by_id` 平级）追加 3 个 pub 函数：

1. **`find_markdown_derivative(conn, root_asset_id) -> Result<Option<Asset>, String>`**：复用 task_002 落地的 `ASSET_SELECT` 常量与 `row_to_asset` 映射函数（`format!("{ASSET_SELECT} WHERE ...")`），WHERE 条件 `source_asset_id = ?1 AND asset_type = 'markdown'`，按 `imported_at DESC LIMIT 1`。同一 root asset 的 canonical markdown 衍生件理论上唯一（session_context §不可妥协底线 §2），但仍以 `LIMIT 1 + ORDER BY` 防御历史脏数据。
2. **`update_markdown_derivative(conn, derived_asset_id, new_name, new_file_size, new_imported_at)`**：仅 UPDATE `name` / `file_size` / `imported_at` 三列。**故意不动** `file_path`（canonical 路径稳定，ADR-006）与 `derivative_version`（由 #3 独立推进，保证 source/derivative 双写不分叉）。
3. **`set_derivative_version(conn, asset_id, new_version)`**：参数化 UPDATE 单列。scheduler 在 source 与 derivative 两侧分别调用（session_context §6 审查重点 line 50-51）。

附带 `#[cfg(test)] mod tests`：用 `Connection::open_in_memory()` + `run_migrations` 完整跑 V1→V5，覆盖 4 个用例。

### 核心设计决策

- **绝不在 SQL 里重写列序**：所有 SELECT 走 `ASSET_SELECT` + `row_to_asset`，与 `get_by_id` 同款。这避免了 task_002 报告中点名的 "三处列序对齐" 风险再次出现。
- **错误信息全部走 `map_err(|e| format!("..."))`**，符合 session_context §5 与既有风格。
- **零 unwrap/expect 在非测试代码**。测试代码内部使用 `unwrap()`/`expect()` 仅用于 fixture 准备失败的即时崩溃，符合 Rust 测试惯例（session_context §5 对 unwrap 的禁令限定在"非 main/测试代码"）。
- **测试 fixtures 显式插入 libraries + projects 父行**：尽管当前 SQLite 默认 `foreign_keys=OFF`，仍构造完整外键链，避免未来 PRAGMA 切换导致测试失效。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `src-tauri/src/db/asset.rs` | 修改 | 在 `get_by_id` 与 `update` 之间新增 3 个 pub fn（共约 50 行业务代码）；文件尾部新增 `#[cfg(test)] mod tests`（4 个 #[test]） |

未触碰前端 `src/`，未修改 `extraction/mod.rs:4` 的 scheduler 注释，未引入任何新依赖，未修改 `Cargo.toml`。

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（未新增/移动文件，未引入子模块）
- [x] API 路径/命名与 Architect 方案一致：三个函数名、签名、返回类型 100% 对齐 §七、§十一 与 input.md AC-1/2/3
- [x] 数据模型与 Architect 方案一致（沿用 task_002 落的 `source_asset_id` / `derivative_version` 列与索引 `idx_assets_source_asset_id`）
- [x] 未引入计划外的新依赖（仅 `use crate::db::migration::run_migrations`、`use crate::models::Asset`、`use rusqlite::Connection`，均为 test 模块内部 use，已存在）
- 偏离说明：无

## 测试命令

```bash
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri
cargo check 2>&1 | tail -50
cargo test --lib db::asset 2>&1 | tail -60
```

## 测试结果

### `cargo check`（关键部分）

```
    Checking notecapt v0.1.0 (/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri)
warning: unused variable: `client`
   --> src/llm/chat.rs:109:5
（以下 3 个 warning 全部位于 src/llm/chat.rs，与本 task 无关，task_002 已记录为 pre-existing）
warning: `notecapt` (lib) generated 4 warnings (run `cargo fix --lib -p notecapt` to apply 3 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.81s
```

**0 error，4 warning（均 pre-existing，与本 task 无关）。**

### `cargo test --lib db::asset`

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3.20s
     Running unittests src/lib.rs (target/debug/deps/app_lib-894db13dfb4c3843)

running 4 tests
test db::asset::tests::update_markdown_derivative_changes_only_three_columns ... ok
test db::asset::tests::set_derivative_version_advances_value ... ok
test db::asset::tests::find_markdown_derivative_returns_none_when_absent ... ok
test db::asset::tests::find_markdown_derivative_returns_latest_match ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 33 filtered out; finished in 0.04s
```

**4/4 通过。**

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|----------|----------|------|-----------|
| ✅ 正常路径 — find | 同一 root 有 2 个 markdown 衍生件（`imported_at` 不同）+ 噪声（其他 root 的 md、本 root 的 image），断言返回最新一条（`d_new`，imported_at=2025-01-05） | 已测 | PASS — `find_markdown_derivative_returns_latest_match` |
| ⚠️ 边界条件 — find 无结果 | root 存在但无衍生件、以及完全不存在的 root_id，均应返回 `Ok(None)` | 已测 | PASS — `find_markdown_derivative_returns_none_when_absent`（两条断言都过） |
| ✅ 正常路径 — update | 改 3 列后断言：`name`/`file_size`/`imported_at` 已变，`file_path`/`derivative_version`/`source_asset_id`/`asset_type` 不变 | 已测 | PASS — `update_markdown_derivative_changes_only_three_columns` |
| ✅ 正常路径 — set version | 从默认 0 推进到 7，断言 `derivative_version=7`、`name`/`file_path` 未变 | 已测 | PASS — `set_derivative_version_advances_value` |
| ⚠️ 边界条件 — update 不存在的 id | SQLite UPDATE 不存在行返回受影响行=0，不报错 | 未测 | scheduler 调用前已 `find_markdown_derivative` 拿到 Asset，不会传入无效 id；行为与 SQLite 默认契约一致，留 scheduler 集成测试覆盖 |
| ❌ 异常路径 — DB 锁 / IO 失败 | rusqlite 底层 IO 错误转 `map_err -> String` 透传 | 未测 | 内存库无法触发；信任 rusqlite 的错误传播 |
| ✅ 列序一致性 | `find_markdown_derivative` 通过复用 `ASSET_SELECT` + `row_to_asset` 自动对齐 13 = is_starred, 14 = source_asset_id, 15 = derivative_version 的列序 | 已测（结构性） | PASS — 不再写第二份 SELECT 列序，物理上不存在分叉 |

## 已知局限

1. **`update_markdown_derivative` 在 id 不存在时静默成功**：SQLite UPDATE 找不到行时返回 affected_rows=0 但不报错，本 fn 也不做 `affected_rows == 0` 检查。理由：scheduler 调用链总是先 `find_markdown_derivative` 拿到真实 Asset 再调 update，逻辑上不会传入野 id；如未来需要诊断"幽灵更新"，可加 `if affected == 0 { return Err(...) }` 检查。
2. **未覆盖并发写**：同一 root asset 的并发转换 → 两个 scheduler 同时 `find -> update` 竞态。这属于 ADR-001 / ADR-006 在更高层的并发控制范围（task_008 scheduler 锁机制），与本 task scope 无关。
3. **整数溢出**：`derivative_version: i32`，理论上 2^31 次转换溢出。实际不可达，未做保护。
4. **`cargo test` 的 33 个 filtered out**：是仓库其他模块的测试（task_002 提到过 `tests/workspace_folders_integration.rs` 因 pre-existing 引用错误编译失败），与本 task 无关。本次只跑 `db::asset` 路径，未跑全量。

## 需要 Reviewer 特别关注的地方

1. **`find_markdown_derivative` 复用 `ASSET_SELECT` 的方式**：使用 `format!("{ASSET_SELECT} WHERE source_asset_id = ?1 AND asset_type = 'markdown' ORDER BY imported_at DESC LIMIT 1")`。请确认这是项目接受的拼接方式（既有 `get_by_id` 与 `get_by_project` 都是这套模式）。WHERE 部分**不含**用户输入字符串拼接，参数走 `params![root_asset_id]`，无 SQL 注入面。

2. **`update_markdown_derivative` 故意只动 3 列**：未触 `file_path` 与 `derivative_version` 是设计而非疏忽。`file_path` 在 canonical 路径模型下稳定（ADR-006）；`derivative_version` 由 `set_derivative_version` 单独推进，scheduler 必须分别在 source/derivative 上调用以保持双写对齐。请验证 task_008 scheduler 在更新衍生件时确实是 "1) update_markdown_derivative + 2) set_derivative_version(derivative) + 3) set_derivative_version(source)" 的三步组合调用。

3. **测试 fixtures 内的 `expect()`**：测试模块内的 `Connection::open_in_memory().expect(...)` 与 `run_migrations(&conn).expect(...)` 是有意为之 —— 测试 setup 失败应立即崩溃。session_context §5 对 unwrap/expect 的禁令限定在非 main/测试代码，测试内允许。

4. **M-1 跨 task 调研结果（Conductor 在 input.md 追加的待办）**：

   ```bash
   $ grep -n "scheduler::" src-tauri/src/lib.rs src-tauri/src/extraction/mod.rs
   (无输出 — 两个文件中没有任何 `scheduler::` 调用形式的引用)
   ```

   补充更广范围的扫描结果：

   ```
   $ grep -n "pub mod\|// pub mod" src-tauri/src/extraction/mod.rs
   src-tauri/src/extraction/mod.rs:1:pub mod extractors;
   src-tauri/src/extraction/mod.rs:2:pub mod models;
   src-tauri/src/extraction/mod.rs:4:// pub mod scheduler;

   $ grep -n "scheduler" src-tauri/src/lib.rs
   (无输出)
   ```

   **结论**：scheduler 模块在整个仓库内**只有一处声明** —— `src-tauri/src/extraction/mod.rs:4`，当前以 `//` 注释屏蔽。**没有任何其他文件以 `scheduler::xxx` 形式调用它**，也没有第二处被注释的引用。这意味着 task_008 取消该单行注释即可完成"激活 scheduler 模块"动作，无需在其他文件做配套取消注释。该信息作为 task_008 的前置依据已记录在此。
