# Task 输入 — task_002_dev_asset_model

## 目标
为 `Asset` 模型与 `assets` 表补齐 `source_asset_id` 和 `derivative_version` 字段，让 `cargo check` 恢复绿。

## 前置条件
- 依赖 task：无
- 必须先存在：`src-tauri/src/models/asset.rs`、`src-tauri/src/db/migration.rs`

## 验收标准（AC）
1. **AC-1**：`Asset` 结构体含 `source_asset_id: Option<String>` 与 `derivative_version: i32`，均带 `#[serde(default)]`。
2. **AC-2**：迁移函数追加 V{N+1}：用 `PRAGMA table_info(assets)` 守卫后执行 `ALTER TABLE assets ADD COLUMN ...`，`source_asset_id` 默认 `NULL`，`derivative_version` 默认 `0`。
3. **AC-3**：迁移幂等——在已经包含这两列的库上重跑不报错。
4. **AC-4**：建立 `idx_assets_source_asset_id` 索引（IF NOT EXISTS）。
5. **AC-5**：`cargo check` 在 `项目启动/NCdesktop/src-tauri/` 通过；剩余编译错误数量为 0（标签/db_asset 函数缺失会在 task_003/004 解决，本 task 完成不要求 `cargo build` 全绿，但 model/migration 这块不能再有错）。

## 技术约束
- 字段顺序：紧跟现有 `is_starred` 之后追加，避免打乱 `Asset::from_row` 中的列序。
- `from_row`（如有）必须按 SQL SELECT 顺序同步更新；现有 `db::asset::insert/update/get_*` 必须新增 SELECT/INSERT 这两列。
- 不允许移除 `#[serde(rename_all = "camelCase")]`。
- 迁移仅向后兼容，禁止 `DROP`/`RENAME`。

## 参考文件
- `src-tauri/src/models/asset.rs`
- `src-tauri/src/db/migration.rs`
- `src-tauri/src/db/asset.rs:5-220`（全部 fn 都要更新 INSERT/SELECT 列表）
- `src-tauri/src/extraction/scheduler.rs:610, 635, 670, 691-692`（使用方）
- 架构方案 `task_001_architect/output.md` §五.1、§五.3、§十一 R1/R5

## 预估影响范围
- 新建文件：无
- 修改文件：
  - `src-tauri/src/models/asset.rs`
  - `src-tauri/src/db/migration.rs`
  - `src-tauri/src/db/asset.rs`（INSERT/SELECT 列表）
  - 全仓 `Asset { ... }` 字面量构造点（grep 后逐处补齐两个字段；建议利用 `..Default::default()` 减负，需在 `Asset` 上加 `#[derive(Default)]`）
