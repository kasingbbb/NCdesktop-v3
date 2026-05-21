# Task 输入 — task_003_dev_db_asset_funcs

## 目标
在 `db/asset.rs` 新增 `find_markdown_derivative` / `update_markdown_derivative` / `set_derivative_version` 三个函数，让 `scheduler::write_derivative_md` 不再引用未定义符号。

## 前置条件
- 依赖 task：task_002（需要 `source_asset_id` / `derivative_version` 字段）
- 必须先存在：`Asset` 含新字段且迁移完成

## 验收标准（AC）
1. **AC-1**：`find_markdown_derivative(conn, root_asset_id) -> Result<Option<Asset>, String>`，SQL 条件 `source_asset_id = ?1 AND asset_type = 'markdown'`，按 `imported_at DESC LIMIT 1`。
2. **AC-2**：`update_markdown_derivative(conn, derived_asset_id, new_name, new_file_size, new_imported_at) -> Result<(), String>`，仅 UPDATE 这三列。
3. **AC-3**：`set_derivative_version(conn, asset_id, new_version) -> Result<(), String>`，参数化绑定。
4. **AC-4**：三个函数都有 rusqlite 单元测试（用内存库 + migration 完整跑一遍）。
5. **AC-5**：`cargo check` 在 `src-tauri/` 通过；`scheduler.rs:627/678/691-692` 编译错误清零。

## 技术约束
- 所有 SQL 使用 `params![]` 参数化绑定。
- 返回类型与既有风格一致：`Result<_, String>`，错误用 `map_err(|e| format!("..."))`。
- 不允许 `unwrap()`/`expect()`。
- 同一函数命名空间与 `db::asset::insert/get_by_id` 保持平级，不引入子模块。

## 参考文件
- `src-tauri/src/db/asset.rs`
- `src-tauri/src/extraction/scheduler.rs:627, 678, 691-692`（调用方）
- 架构方案 `task_001_architect/output.md` §七、§十一

## 预估影响范围
- 新建文件：无
- 修改文件：
  - `src-tauri/src/db/asset.rs`（新增 3 fn + 测试模块）

## Conductor 追加（M-1 跨 task 待办，来自 task_002 Reviewer 发现）
- task_002 审查时发现 `src/extraction/mod.rs:4` 当前是 `// pub mod scheduler;`（注释状态），导致 scheduler.rs 不参与编译。
- 本 task **不要求**取消该注释（取消后会暴露 task_004/008 未实现的引用，污染本 task 的 cargo check 输出）。
- 但本 task 完成后必须**单独跑一次** `grep -n "scheduler::" src-tauri/src/lib.rs src-tauri/src/extraction/mod.rs` 并在 output.md 报告：scheduler 模块当前在哪些地方被引用、是否还有其他注释屏蔽。这份信息将作为 task_008 取消注释前的前置依据。
