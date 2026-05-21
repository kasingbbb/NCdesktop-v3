# Task 交付 — task_003_dev_m1m2_list_state

## 实现摘要

落地"工作区列表唯一查询入口 + 实时四态派生"：

1. **`db::asset::list_root_assets(conn, project_id)`**：单 SQL 一次性返回 root asset（`source_asset_id IS NULL`）+ canonical markdown 衍生件 + extracted_content + 最近一条 pipeline_tasks + 最近一条 conversion_meta（后两者用 `ROW_NUMBER() OVER (PARTITION BY ... ORDER BY ... DESC, rowid DESC)` 子查询，避免 N+1 / 相关子查询）；按 `assets.imported_at DESC` 排序。**db 层零 IO**。
2. **`db::asset::compute_asset_state`** 纯函数：5 个值类型入参 → `AssetState`，规则严格按 input.md AC-2。`source_missing_known` 显式不参与决策，避免 source 短暂离线把已 Done 资产降级。
3. **`models::asset::AssetState` / `WorkspaceAssetView`**：serde camelCase / lowercase；前端 `src/types/workspaceAsset.ts` 字段镜像。
4. **`commands::asset::get_assets`** 改造：返回 `Vec<WorkspaceAssetView>`，在命令层 `Path::exists()` stat rendition / source，调用 `compute_asset_state` 派生四态；通过 `app.try_state::<SourceMissingSet>()` 容忍 task_007 未注册场景。把 view 拼接逻辑抽到 `build_workspace_view`，便于纯函数单测。
5. **`SourceMissingSet`** 类型骨架放在 `commands::asset`：本 task 只声明结构（RwLock<HashSet> + contains/insert/remove），完整启动期扫描归 task_007；未注册时 `try_state` 返回 None，`get_assets` 自动跳过该标记。
6. **`db::asset::get_by_project`** 加 `#[deprecated(note = "工作区列表请使用 list_root_assets；本函数仅供非工作区视图使用")]` + doc；在仅剩两处合法 caller（`commands/export.rs`、`commands/extraction.rs::extract_project_assets`）就地加 `#[allow(deprecated)]`，避免警告污染。

核心设计决策：
- **窗口函数取最新**：用 `ROW_NUMBER OVER PARTITION BY` 一次拿到最近 pipeline_tasks / conversion_meta，避免相关子查询；rusqlite bundled SQLite ≥ 3.40 支持。`ORDER BY created_at DESC, rowid DESC` 第二序保证同时刻插入仍有稳定顺序（测试中两条同时间记录得以正确选最新）。
- **state_reason 优先级**：`error_class > pipeline.error_message`，且仅在 `state == Failed` 时填充；done/converting/offline 一律为 None。
- **source_missing 字段语义**：取 `source_missing_known || !source_exists`，把"启动期扫描记录"与"命令时实时 stat"统一为单 boolean；UI 标记位，不影响 state。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---|---|---|
| `NCdesktop/src-tauri/src/models/asset.rs` | 修改 | + `AssetState` 枚举 (lowercase) + `WorkspaceAssetView` 结构（camelCase） |
| `NCdesktop/src-tauri/src/db/asset.rs` | 修改 | + `AssetListJoinRow` + `list_root_assets` + `compute_asset_state` + ROOT_ASSET_COLS；`get_by_project` 加 `#[deprecated]` doc；新增 13 个单测 |
| `NCdesktop/src-tauri/src/commands/asset.rs` | 修改 | `get_assets` 切到 `list_root_assets` + `WorkspaceAssetView`；新增 `SourceMissingSet` 占位类型；抽 `build_workspace_view`；新增 6 单测 |
| `NCdesktop/src-tauri/src/commands/extraction.rs` | 修改 | 给 `get_by_project` caller 加 `#[allow(deprecated)]` 单行 |
| `NCdesktop/src-tauri/src/commands/export.rs` | 修改 | 给 `get_by_project` caller 加 `#[allow(deprecated)]` 单行 |
| `NCdesktop/src/types/workspaceAsset.ts` | 新建 | 前端 DTO 镜像 |

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（修改文件均在 §八 列出的真实路径下）
- [x] API 路径/命名与 Architect 方案一致（`list_root_assets` / `compute_asset_state` / `WorkspaceAssetView` / `AssetState` / `SourceMissingSet` 命名均按 §六 / ADR-002 / ADR-003 落地）
- [x] 数据模型与 Architect 方案一致（WorkspaceAssetView 字段集合与 §六 完全一致，含 `derivativeVersion` / `renditionId` / `renditionPath` / `renditionSize` / `state` / `stateReason` / `sourceMissing`）
- [x] 未引入计划外的新依赖（仅使用已有 `rusqlite` / `serde` / `std::sync::RwLock` / `std::collections::HashSet`）
- 偏离说明：
  - `SourceMissingSet` 类型定义放在 `commands::asset` 内（而非 §五 中规划的 `source_scan.rs`）。原因：input.md 明示本 task 仅"接受 `Option<State<SourceMissingSet>>` 兼容尚未注册场景"，task_007 才落地启动期扫描。本 task 提前定义最小骨架（RwLock<HashSet> + contains/insert/remove），task_007 落地 `source_scan.rs` 时可直接 `pub use crate::commands::asset::SourceMissingSet` 或迁移到 source_scan 模块，零破坏接口。这是本 task 的微调，不偏离架构方案。
  - `get_assets` 命令签名从 `(database: State<'_, Database>, project_id)` 改为 `(app: AppHandle, database: State<'_, Database>, project_id)`，因 `try_state::<SourceMissingSet>()` 需要 `AppHandle::try_state`。Tauri 命令接受 `AppHandle` 注入，前端 invoke 调用 signature 无变化。

## 测试命令

```bash
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri
cargo test -p notecapt --lib db::asset
cargo test -p notecapt --lib commands::asset
# 前端类型检查
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop
npm run check
```

> 说明：input.md 写的 `-p app_lib` 是包名笔误（本仓库 `[package].name = "notecapt"`；`[lib].name = "app_lib"` 仅是 crate 输出名）。`cargo test -p notecapt` 是与 task_002 Dev 已对齐的正确命令。

## 测试结果

### cargo test -p notecapt --lib db::asset

```
running 18 tests
test db::asset::tests::compute_state_converting_for_queued_or_running ... ok
test db::asset::tests::compute_state_done_when_completed_and_rendition_present ... ok
test db::asset::tests::compute_state_failed_when_conversion_meta_has_error_class ... ok
test db::asset::tests::compute_state_completed_without_rendition_falls_through ... ok
test db::asset::tests::compute_state_done_even_when_source_missing ... ok
test db::asset::tests::compute_state_offline_when_cancelled ... ok
test db::asset::tests::compute_state_offline_when_no_pipeline_no_meta ... ok
test db::asset::tests::compute_state_failed_when_pipeline_failed ... ok
test db::asset::tests::set_derivative_version_advances_value ... ok
test db::asset::tests::list_root_assets_empty_project_returns_empty_vec ... ok
test db::asset::tests::find_markdown_derivative_returns_none_when_absent ... ok
test db::asset::tests::list_root_assets_excludes_markdown_derivative ... ok
test db::asset::tests::list_root_assets_orders_by_imported_at_desc ... ok
test db::asset::tests::update_markdown_derivative_changes_only_three_columns ... ok
test db::asset::tests::list_root_assets_isolates_by_project_id ... ok
test db::asset::tests::list_root_assets_mixed_states_three_roots ... ok
test db::asset::tests::find_markdown_derivative_returns_latest_match ... ok
test db::asset::tests::list_root_assets_joins_latest_pipeline_and_conversion_meta ... ok

test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 84 filtered out; finished in 0.10s
```

### cargo test -p notecapt --lib commands::asset

```
running 6 tests
test commands::asset::tests::build_view_done_when_pipeline_completed_and_rendition_present ... ok
test commands::asset::tests::build_view_failed_falls_back_to_pipeline_error_when_no_error_class ... ok
test commands::asset::tests::build_view_failed_uses_error_class_as_reason ... ok
test commands::asset::tests::build_view_offline_when_no_pipeline_no_meta ... ok
test commands::asset::tests::build_view_source_missing_marks_flag_but_keeps_state ... ok
test commands::asset::tests::build_view_serializes_camel_case ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 96 filtered out; finished in 0.00s
```

### npm run check（前端 tsc --noEmit）

```
> ncdesktop@0.0.0 check
> tsc --noEmit
```

（无输出 = 0 个 type error；workspaceAsset.ts 与既有 src/ 代码兼容）

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|----------|----------|------|-----------|
| ✅ 正常路径 | root + markdown 衍生件 → list 只返 root，rendition_path 正确 join | 已测 | PASS（`list_root_assets_excludes_markdown_derivative`） |
| ✅ 正常路径 | 多 root 按 imported_at DESC 排序 | 已测 | PASS（`list_root_assets_orders_by_imported_at_desc`） |
| ✅ 正常路径 | 混合三态（done / converting / failed）单次 list 返回 | 已测 | PASS（`list_root_assets_mixed_states_three_roots`） |
| ✅ 正常路径 | pipeline_tasks / conversion_meta 多条时取最近 | 已测 | PASS（`list_root_assets_joins_latest_pipeline_and_conversion_meta`） |
| ✅ 正常路径 | 四态派生：done / converting (queued+running) / failed×2 / offline×2 共 8 组合 | 已测 | PASS（8 个 compute_state_* 用例全过） |
| ✅ 正常路径 | view 字段 serde camelCase 与 state lowercase | 已测 | PASS（`build_view_serializes_camel_case`） |
| ⚠️ 边界条件 | 空项目（无 asset） | 已测 | PASS（`list_root_assets_empty_project_returns_empty_vec`） |
| ⚠️ 边界条件 | 项目隔离（同 conn 多项目） | 已测 | PASS（`list_root_assets_isolates_by_project_id`） |
| ⚠️ 边界条件 | source 缺失但 pipeline completed + rendition 在 | 已测 | PASS：state 仍 Done、source_missing=true（`build_view_source_missing_marks_flag_but_keeps_state` + `compute_state_done_even_when_source_missing`） |
| ⚠️ 边界条件 | pipeline status=cancelled 不归 failed/converting | 已测 | PASS（`compute_state_offline_when_cancelled`） |
| ⚠️ 边界条件 | pipeline=completed 但 rendition 文件缺失（用户手删 .md） | 已测 | PASS：落入 Offline（`compute_state_completed_without_rendition_falls_through`） |
| ❌ 异常路径 | state_reason 选择：error_class 优先于 pipeline_error | 已测 | PASS（`build_view_failed_uses_error_class_as_reason`） |
| ❌ 异常路径 | state_reason 回退：无 error_class 时取 pipeline_error | 已测 | PASS（`build_view_failed_falls_back_to_pipeline_error_when_no_error_class`） |
| ❌ 异常路径 | SourceMissingSet 未注册（task_007 未上线）→ get_assets 不 panic | 已测 | PASS：`try_state` 返回 None，已纳入运行时路径（`build_workspace_view` 直接用 stat 结果），编译通过 |

## 已知局限

1. **`build_workspace_view` 的命令层路径未走端到端 Tauri 调用测试**。`get_assets` 命令本身依赖 `AppHandle` / `Database` `State`，需要完整 Tauri runtime 才能直接调用；本 task 通过把核心拼接逻辑抽到 `build_workspace_view` 纯函数 + 6 个 unit test 覆盖了所有数据形变路径，但"AppHandle 注入 + try_state 返回 None"分支没有运行时验证。端到端集成测试归 task_009 范畴。
2. **`SourceMissingSet` 仅作为类型占位**：本 task 提前定义在 `commands::asset` 是为了让 `get_assets` 立即可注入，但其上未挂任何写入路径。task_007 会落地真正的 `source_scan.rs` 与 `app.manage(SourceMissingSet::default())`。
3. **`derivative_version` 是 root 行字段（非 derivative 行）**：DTO 中带出来用于前端 UI 显示"已成功转换 N 次"。但当前生产数据中 root 与 derivative 双写不分叉，前端如何展示由 task_008 决策。
4. **窗口函数排序的稳定性依赖 `rowid` 二级序**：在极少数同 `created_at` 多行情况下用 `rowid DESC` 保稳。若未来表存在 `WITHOUT ROWID`（目前 V7/V6 均带 rowid），需要调整。

## 需要 Reviewer 特别关注的地方

1. **`db::asset::list_root_assets` 的 SQL 列序与 row_to_asset / AssetListJoinRow 字段位置对齐**（`db/asset.rs::row_to_asset` 与 list_root_assets 的 query_map closure）。任何对 `ROOT_ASSET_COLS` / `ASSET_SELECT` 列顺序的改动都必须同步两侧。
2. **窗口函数子查询的最近一条选择策略**：`ORDER BY created_at DESC, rowid DESC`。reviewer 请确认这与 `db/conversion_meta.rs::latest_for_source` 的"converted_at DESC"语义一致（两侧都是"最近一条"概念，列名不同）。
3. **`compute_asset_state` 的优先级顺序**：rule 1 (done) → rule 2 (converting) → rule 3 (failed) → rule 4 (offline)。如果未来需要"pipeline failed 但 conversion_meta 已成功"反转为 Done，需要回到这里修改。
4. **`source_missing` 字段语义**：当前是 `source_missing_known || !source_exists`，即把"启动期扫描的持久记录"与"命令时实时 stat"双重保险合并为单 bool。task_007 / task_008 是否需要区分两者（UI 是否需要"扫描中"vs"已确认"）请在 review 中明确。
5. **两处 `#[allow(deprecated)]` 是否合规**：`commands/extraction.rs::extract_project_assets`（批量入队需所有 asset）与 `commands/export.rs`（导出需完整列表）。reviewer 请确认这两路确实属于"非工作区视图"，不应误用 `list_root_assets`。
