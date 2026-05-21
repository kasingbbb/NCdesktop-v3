# Task 输入 — task_003_dev_m1m2_list_state

## 目标
实现工作区列表唯一查询入口 `db::asset::list_root_assets` 与四态派生 `db::asset::compute_asset_state`，并把 `commands::asset::get_assets` 切流到新 DTO `WorkspaceAssetView`。

## 前置条件
- 依赖 task：task_002（确保 import 路径下 asset 落地形态稳定）
- 必须先存在的文件/接口（均已存在）：
  - `src-tauri/src/db/asset.rs::find_markdown_derivative`
  - `src-tauri/src/db/extraction.rs::PipelineTaskRow / ExtractedContentRow`
  - `src-tauri/src/db/conversion_meta.rs::latest_for_source`
  - V5/V6/V7/V8 migration（已落地）

## 验收标准（AC）
1. **AC-1**：新增 `db::asset::list_root_assets(conn, project_id) -> Vec<(Asset, AssetListJoinRow)>`，SQL 满足 ADR-002：
   - `WHERE assets.source_asset_id IS NULL AND project_id = ?`
   - LEFT JOIN derivative（`asset_type='markdown'`）取 `rendition_id / rendition_path / rendition_size`
   - LEFT JOIN `extracted_content`（per asset_id 一行，V8 已有 `idx_extracted_content_asset` UNIQUE）
   - 最近一条 `pipeline_tasks`（按 created_at DESC）
   - 最近一条 `conversion_meta`（按 converted_at DESC）
   - ORDER BY `assets.imported_at DESC`
2. **AC-2**：新增纯函数 `db::asset::compute_asset_state(pipeline_status: Option<&str>, latest_error_class: Option<&str>, rendition_exists: bool, source_exists: bool, source_missing_known: bool) -> AssetState`：
   - `rendition_exists && pipeline_status == Some("completed")` → `Done`
   - `pipeline_status` ∈ {`queued`, `running`} → `Converting`
   - `pipeline_status == Some("failed")` 或 最近一条 `conversion_meta.error_class.is_some()` → `Failed`
   - 其余（含 `pipeline_status` 为 None） → `Offline`
   - 单测覆盖 8 个组合（含 source-missing 不改变状态）
3. **AC-3**：新增 `models::asset::WorkspaceAssetView` 与 `AssetState` 枚举（serde camelCase / lowercase）；在前端 mirror `src/types/workspaceAsset.ts`。
4. **AC-4**：改造 `commands::asset::get_assets`：返回 `Vec<WorkspaceAssetView>`；在命令端做 `Path::exists()` stat（**不**在 db/ 内做 IO）；从 `app.state::<SourceMissingSet>()` 注入 source-missing 标记（task_007 提供，但本 task 接受 `Option<State<SourceMissingSet>>` 兼容尚未注册场景）。
5. **AC-5**：`db::asset::get_by_project` 加 `#[deprecated(note = "工作区列表请使用 list_root_assets")]`，并在 doc 注释中明示。**不**移除该函数（其他视图仍在用）。
6. **AC-6**：`cargo test -p app_lib --lib db::asset` 与 `cargo test -p app_lib --lib commands::asset` 全部通过；新增至少 6 个单测（list_root_assets：纯 derivative 不出现、空项目、混合态、conversion_meta join、compute_asset_state 8 种组合中至少 6 种）。

## 技术约束
- 不在 commands/ 中拼 SQL（保留 list_root_assets 在 db/）。
- compute_asset_state 必须是纯函数，不依赖 AppHandle / Connection / IO，便于单测。
- 不引入新依赖（rusqlite 已支持 ROW_NUMBER OVER 窗口函数；SQLite 3.25+）。
- DTO 字段必须与 task_001_architect §六 数据模型一致。

## 参考文件
- `src-tauri/src/db/asset.rs`（已存在 `find_markdown_derivative`、`ASSET_SELECT`）
- `src-tauri/src/db/migration.rs`（核对 V5/V6/V7/V8 列名）
- `src-tauri/src/db/conversion_meta.rs::latest_for_source`
- `src-tauri/src/db/extraction.rs::get_extracted_content`
- `task_001_architect/output.md` §ADR-002 / §ADR-003 / §六 数据模型

## 预估影响范围
- 新建文件：`src/types/workspaceAsset.ts`
- 修改文件：
  - `src-tauri/src/db/asset.rs`（+ list_root_assets / + compute_asset_state / + AssetListJoinRow / deprecate get_by_project）
  - `src-tauri/src/models/asset.rs`（+ WorkspaceAssetView / + AssetState）
  - `src-tauri/src/commands/asset.rs`（get_assets 改造）
- 估算变更：~600 行（含 ~200 行测试）
