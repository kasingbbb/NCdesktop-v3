# Task 交付 — task_004_dev_m3_asset_id_commands

## 实现摘要

落地 ADR-007 "命令链 asset_id 化"的 rename 路径与底层解算辅助：

1. **`db::asset::resolve_asset_pair(conn, asset_id) -> (Asset /* root */, Option<Asset> /* derivative */)`**：
   - 先 `get_by_id`；若结果 `source_asset_id IS NULL` → 自身即 root，derivative 用
     `find_markdown_derivative` 取最新；
   - 若结果 `source_asset_id IS NOT NULL` → 自身是 derivative，按 `source_asset_id`
     反查 root，再用 root 调 `find_markdown_derivative`（结果应包含传入 asset_id）；
   - 任一阶段查不到 → `Err("素材不存在")`。

2. **`commands::asset::rename_asset(app, database, asset_id, new_display_name) -> WorkspaceAssetView`**：
   - `validate_display_name`：trim 后非空 + UTF-8 长度 ≤ 200 字节；
   - 调 `resolve_asset_pair` 拿到 (root, Option<derivative>)；
   - 双写 `assets.name`：root 写用户原始输入；derivative 写 `derivative_name_from_root`
     的结果（**sanitize_stem 清洗 → 切掉最后一个 `.` 后的扩展 → 拼 `.md`**）；
   - **不动磁盘文件名 / file_path / file_size / derivative_version**（PRD 硬约束 §4，
     ADR-006）；
   - 复用 `list_root_assets` 过滤出 root.id，复用 `build_workspace_view` 拼最新视图
     返回，避免在 db/ 新增"单 root 查询"重复 SQL（保持 ADR-002 单查询入口）；
   - 命令层入口附加 `SourceMissingSet`（task_007 注册前为空，行为兼容）。

3. **`derivative_name_from_root`** 设计要点：先 `sanitize_stem` 整体清洗（把 `/`
   等路径分隔符替换为 `_`），再用 `rfind('.')` 切尾，**不**用 `Path::file_stem`
   —— 否则 `"a/b.pdf"` 会被当作路径解析为 `b`，丢失前缀。`.env` 这种首位即 `.`
   的输入也得到合理回退（整串作为 stem）。

4. **lib.rs**：在原有 `commands::asset::update_asset` 之后单行追加
   `commands::asset::rename_asset`，不重排既有命令顺序。

5. **前端**：`tauri-commands.ts` 新增 `renameAsset(assetId, newDisplayName)`
   wrapper 返回 `WorkspaceAssetView`；旧 `updateAsset` 保留并标注 `@deprecated`
   引导后续 caller 迁移。`assetStore.ts` 同步加 `renameAsset(assetId, newDisplayName)`
   action，就地 patch `assets[].name` 而非整列表 refetch。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---|---|---|
| `NCdesktop/src-tauri/src/db/asset.rs` | 修改 | + `resolve_asset_pair` + 4 个单测（root 输入 / derivative 输入 / 无 derivative / 缺失） |
| `NCdesktop/src-tauri/src/commands/asset.rs` | 修改 | + `DISPLAY_NAME_MAX_BYTES` 常量 + `validate_display_name` + `derivative_name_from_root` + `rename_asset_inner`（私有，可测）+ `rename_asset`（Tauri 命令）+ 7 个单测 |
| `NCdesktop/src-tauri/src/lib.rs` | 修改 | `invoke_handler!` 中 `update_asset` 之后追加 `rename_asset` |
| `NCdesktop/src/lib/tauri-commands.ts` | 修改 | `updateAsset` 加 `@deprecated` JSDoc；新增 `renameAsset` wrapper（返回 `WorkspaceAssetView`） |
| `NCdesktop/src/stores/assetStore.ts` | 修改 | 加 `renameAsset(assetId, newDisplayName)` action；`updateAsset` 加 `@deprecated` 注释 |

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（修改集中在既有 `db::asset` / `commands::asset` / `lib.rs`，无新建文件）
- [x] API 路径/命名与 Architect 方案一致（`resolve_asset_pair`、`rename_asset`、`WorkspaceAssetView` 命名与 ADR-007 / §六 完全一致）
- [x] 数据模型与 Architect 方案一致（rename 返回 `WorkspaceAssetView`，仅触动 `assets.name`，不动 `file_path` / `derivative_version` / `file_size` / `imported_at`）
- [x] 未引入计划外的新依赖（仅使用既有 `rusqlite` / `crate::utils::safe_name::sanitize_stem`）
- 偏离说明：
  - `rename_asset` Tauri 命令签名为 `(app: AppHandle, database, asset_id, new_display_name)` —— 与 task_003 的 `get_assets` 一致地接入 `AppHandle` 以读 `SourceMissingSet`，前端 invoke 参数不变。
  - 复用 `update_markdown_derivative(conn, id, new_name, file_size, imported_at)` 完成 derivative 双写，但 `file_size` 与 `imported_at` 传原值（即"只改 name"语义）。未新增"只改 name"专用 db 函数，避免接口面膨胀。
  - 引入私有 `rename_asset_inner`：把核心逻辑提到不依赖 `State` 的形态，让单测能直接构造 `Connection` 注入 stat 值跑过。这是测试可达性微调，不偏离架构。

## 测试命令

```bash
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri
cargo test -p notecapt --lib db::asset
cargo test -p notecapt --lib commands::asset
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop
npm run check
```

> 注：input.md 中的 `cargo test -p app_lib` 是包名笔误（`[package].name = "notecapt"`，`[lib].name = "app_lib"` 仅是 crate 输出名），已沿用 task_002 / task_003 已对齐的 `-p notecapt`。

## 测试结果

### cargo test -p notecapt --lib db::asset

```
running 22 tests
test db::asset::tests::compute_state_completed_without_rendition_falls_through ... ok
test db::asset::tests::compute_state_done_even_when_source_missing ... ok
test db::asset::tests::compute_state_converting_for_queued_or_running ... ok
test db::asset::tests::compute_state_failed_when_conversion_meta_has_error_class ... ok
test db::asset::tests::compute_state_done_when_completed_and_rendition_present ... ok
test db::asset::tests::compute_state_failed_when_pipeline_failed ... ok
test db::asset::tests::compute_state_offline_when_cancelled ... ok
test db::asset::tests::compute_state_offline_when_no_pipeline_no_meta ... ok
test db::asset::tests::list_root_assets_empty_project_returns_empty_vec ... ok
test db::asset::tests::resolve_asset_pair_returns_err_when_asset_missing ... ok
test db::asset::tests::list_root_assets_excludes_markdown_derivative ... ok
test db::asset::tests::list_root_assets_isolates_by_project_id ... ok
test db::asset::tests::find_markdown_derivative_returns_latest_match ... ok
test db::asset::tests::list_root_assets_orders_by_imported_at_desc ... ok
test db::asset::tests::find_markdown_derivative_returns_none_when_absent ... ok
test db::asset::tests::resolve_asset_pair_resolves_via_derivative_id ... ok
test db::asset::tests::list_root_assets_mixed_states_three_roots ... ok
test db::asset::tests::list_root_assets_joins_latest_pipeline_and_conversion_meta ... ok
test db::asset::tests::resolve_asset_pair_returns_root_and_derivative_when_input_is_root ... ok
test db::asset::tests::resolve_asset_pair_root_without_derivative_returns_none ... ok
test db::asset::tests::update_markdown_derivative_changes_only_three_columns ... ok
test db::asset::tests::set_derivative_version_advances_value ... ok

test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 91 filtered out; finished in 0.12s
```

### cargo test -p notecapt --lib commands::asset

```
running 13 tests
test commands::asset::tests::build_view_done_when_pipeline_completed_and_rendition_present ... ok
test commands::asset::tests::build_view_failed_uses_error_class_as_reason ... ok
test commands::asset::tests::build_view_source_missing_marks_flag_but_keeps_state ... ok
test commands::asset::tests::build_view_failed_falls_back_to_pipeline_error_when_no_error_class ... ok
test commands::asset::tests::build_view_offline_when_no_pipeline_no_meta ... ok
test commands::asset::tests::build_view_serializes_camel_case ... ok
test commands::asset::tests::rename_rejects_when_asset_missing ... ok
test commands::asset::tests::rename_rejects_empty_after_trim ... ok
test commands::asset::tests::rename_without_derivative_only_writes_root ... ok
test commands::asset::tests::rename_via_derivative_id_resolves_to_root ... ok
test commands::asset::tests::rename_double_writes_root_and_derivative ... ok
test commands::asset::tests::rename_derivative_name_uses_sanitize_stem ... ok
test commands::asset::tests::rename_rejects_over_200_bytes ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 100 filtered out; finished in 0.07s
```

### npm run check（前端 tsc --noEmit）

```
> ncdesktop@0.0.0 check
> tsc --noEmit
```

（无输出 = 0 个 type error）

## AC 实现状态

| AC | 状态 | 验证位置 |
|---|---|---|
| AC-1 `resolve_asset_pair` | ✅ | `db::asset::resolve_asset_pair` + 4 单测 |
| AC-2 `rename_asset` 双写 + 校验 + 不动磁盘 | ✅ | `commands::asset::rename_asset` + `rename_asset_inner` + 单测 `rename_double_writes_root_and_derivative` 验证 file_path / canonical 路径不变 |
| AC-3 rename(root_id, "新名.pdf") → derivative.name="新名.md" | ✅ | `rename_double_writes_root_and_derivative` |
| AC-4 rename(derivative_id) 解算回 root 后双写 | ✅ | `rename_via_derivative_id_resolves_to_root` |
| AC-5 旧 `update_asset` 保留 + 前端 rename 切到 `renameAsset` | ✅ | tauri-commands.ts 加 `renameAsset` + `updateAsset` 加 `@deprecated`；assetStore 加 `renameAsset` action |
| AC-6 `lib.rs` 注册 `rename_asset` | ✅ | lib.rs `invoke_handler!` 中 `update_asset` 后单行追加 |
| AC-7 cargo test 全过 + ≥ 4 新单测 | ✅ | db::asset +4（resolve_asset_pair 系列）；commands::asset +7（rename 系列），共 +11 |

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|---|---|---|---|
| ✅ 正常路径 | rename(root_id, "新名.pdf") 双写 root + derivative | 已测 | PASS（`rename_double_writes_root_and_derivative`） |
| ✅ 正常路径 | rename(derivative_id) 反查 root 后双写 | 已测 | PASS（`rename_via_derivative_id_resolves_to_root`） |
| ✅ 正常路径 | rename 不动 file_path / canonical 路径 | 已测 | PASS（同上两测均断言 file_path 保持原值） |
| ✅ 正常路径 | resolve_asset_pair 输入 root.id | 已测 | PASS（`resolve_asset_pair_returns_root_and_derivative_when_input_is_root`） |
| ✅ 正常路径 | resolve_asset_pair 输入 derivative.id 反解 | 已测 | PASS（`resolve_asset_pair_resolves_via_derivative_id`） |
| ⚠️ 边界条件 | rename 无 derivative：只写 root | 已测 | PASS（`rename_without_derivative_only_writes_root`） |
| ⚠️ 边界条件 | resolve_asset_pair 无 derivative：返回 (root, None) | 已测 | PASS（`resolve_asset_pair_root_without_derivative_returns_none`） |
| ⚠️ 边界条件 | rename 名称含 `/` 等非法字符 → derivative 走 sanitize_stem 清洗 | 已测 | PASS（`rename_derivative_name_uses_sanitize_stem`：root.name 保 `a/b.pdf`，derivative.name = `a_b.md`） |
| ⚠️ 边界条件 | rename 200 字节边界值通过 | 已测 | PASS（`rename_rejects_over_200_bytes` 末段） |
| ❌ 异常路径 | rename 空字符串 / 全空白 | 已测 | PASS：返回 `"新名称不能为空"`（`rename_rejects_empty_after_trim`），原 name 未改 |
| ❌ 异常路径 | rename UTF-8 ≥ 201 字节 | 已测 | PASS：返回 `"新名称超长（请控制在 200 字节内）"`（`rename_rejects_over_200_bytes`） |
| ❌ 异常路径 | rename 不存在的 asset_id | 已测 | PASS：返回 `"素材不存在"`（`rename_rejects_when_asset_missing`） |
| ❌ 异常路径 | resolve_asset_pair 不存在 | 已测 | PASS（`resolve_asset_pair_returns_err_when_asset_missing`） |

## 已知局限

1. **`rename_asset_inner` 的视图重建复用 `list_root_assets` 查整项目**：rename 是低频单次操作，多扫一次项目列表代价可接受；如果未来项目内 asset 数量极大（> 万级），可考虑在 db 加 `load_root_view_by_id`，但当前没必要（保持 ADR-002 单一查询入口）。
2. **`rename_asset` Tauri 命令本体未走端到端测试**：依赖 `AppHandle` / `State<Database>` 注入，无法在 unit test 直接构造；本 task 把核心逻辑提到 `rename_asset_inner`（接受 `&Connection` + 注入的 stat 值），用 7 个 unit test 覆盖所有数据形变；命令外壳仅做 lock 获取 + SourceMissingSet 叠加，逻辑近乎平凡。
3. **前端 UI 暂无 rename 入口在调用**：`AssetListView.tsx` 有"已重命名"角标但目前没有真正调用 `updateAsset` / `renameAsset` 的 UI 路径（grep 结果仅 store 内引用）。本 task 已让 `assetStore.renameAsset` 就位，UI rename 浮层接线归后续 task。
4. **`update_asset` 仍保留**：AC-5 明示保留，供 `is_starred` / 整行 Asset 同名兼容场景使用；JSDoc 已标 `@deprecated`，引导 rename 调用者迁移。

## 需要 Reviewer 特别关注的地方

1. **`derivative_name_from_root` 不使用 `Path::file_stem`**：见实现摘要 §3。Path 会把 `/` 当路径分隔符切走前缀；本实现先 `sanitize_stem` 把 `/` 替换为 `_`，再 `rfind('.')` 切尾。请确认这一选择满足 ADR-007 / PRD §4.4 对 display_name → derivative.name 的映射预期。
2. **`update_markdown_derivative` 的复用方式**：rename 调用 `update_markdown_derivative(conn, id, new_name, d.file_size, &d.imported_at)`，传 `file_size` / `imported_at` 原值。这沿用了 task_003 已有的 db 接口而未新增"只改 name"专函，避免接口膨胀。请确认这种"传原值不变"的语义在阅读上不会被误以为是改值。
3. **`rename_asset` 命令返回 `WorkspaceAssetView`**：前端 store 就地 patch `assets[].name`。若未来 rename 需要级联触发其他视图（搜索 / 知识中枢）刷新，需要在 store 这一侧再补 emit；当前 task 范畴只到 workspace 列表。
4. **AC-5 的"删除 rename 场景入口"如何理解**：grep 显示当前没有 UI 组件真正调 `updateAsset`，只有 `assetStore.updateAsset` 一处 store action。本 task 选择：保留 `updateAsset` wrapper（带 `@deprecated`），新增 `renameAsset` wrapper + store action 作为 rename 唯一入口。如果 reviewer 期望直接删除 `updateAsset` IPC wrapper，请明示——目前考虑到 `is_starred` 等同名兼容路径可能存在历史依赖，未做强删。
