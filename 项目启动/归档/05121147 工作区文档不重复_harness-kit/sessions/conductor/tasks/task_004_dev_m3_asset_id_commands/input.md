# Task 输入 — task_004_dev_m3_asset_id_commands

## 目标
让 rename / delete / outbound / 重试等命令的语义全部以 `asset_id` 为唯一目标。本 task 集中交付 `rename_asset` 与底层 `resolve_asset_pair`，标签同步走现有 `propagate_tags_to_derivative`。

## 前置条件
- 依赖 task：task_003（提供 WorkspaceAssetView DTO，rename 命令返回类型用它）
- 必须先存在的文件/接口：
  - `src-tauri/src/db/asset.rs::find_markdown_derivative`、`get_by_id`、`update`
  - `src-tauri/src/db/tag.rs::propagate_tags_to_derivative`（已存在）
  - `src-tauri/src/utils/safe_name.rs::sanitize_stem`

## 验收标准（AC）
1. **AC-1**：新增 `db::asset::resolve_asset_pair(conn, asset_id) -> Result<(Asset /* root */, Option<Asset> /* derivative */), String>`：
   - 若传入 asset_id 是 root（`source_asset_id IS NULL`），derivative 用 `find_markdown_derivative`。
   - 若传入 asset_id 是 derivative，先按 `source_asset_id` 反查 root，再用 root 调 `find_markdown_derivative`（结果应包含传入的 asset_id）。
   - 若都查不到 → `Err("素材不存在")`。
2. **AC-2**：新增命令 `commands::asset::rename_asset(database, asset_id, new_display_name) -> WorkspaceAssetView`：
   - 校验 `new_display_name` trim 后非空，UTF-8 长度 ≤ 200 字节（与 PRD §4.4 一致）。
   - 双写 root.name 与 derivative.name；derivative.name 用 `sanitize_stem(new_stem_from_root_name)` + `.md` 后缀。
   - 不改 file_path / 磁盘文件名（display_name 仅活在 DB，PRD 硬约束 §4）。
   - 返回最新的 WorkspaceAssetView（复用 task_003 的查询路径，或自行拼一次 LEFT JOIN）。
3. **AC-3**：rename 单测 `commands::asset::tests::rename_double_writes_root_and_derivative`：导入 root + derivative，调用 `rename_asset(root_id, "新名.pdf")`，断言两行 name 都更新且 derivative.name = "新名.md"（去除原扩展，附加 .md）。
4. **AC-4**：rename 接受 derivative.id 也能正确解算回 root 后双写（覆盖 ADR-007 解算辅助）。
5. **AC-5**：`commands::asset::update_asset`（旧命令）保留但不再用于工作区 rename；在 `tauri-commands.ts` wrapper 中删除 `updateAsset` 在 rename 场景的调用入口（前端改走 `renameAsset`）。
6. **AC-6**：在 `lib.rs` `invoke_handler!` 注册 `commands::asset::rename_asset`。
7. **AC-7**：`cargo test -p app_lib --lib commands::asset` 与 `db::asset` 全部通过；至少 4 个新增单测。

## 技术约束
- 命令禁止接受 file_path（ADR-007）。
- 不引入新 sanitize 实现：复用 `crate::utils::safe_name::sanitize_stem`。
- display_name 校验失败统一中文错误："新名称不能为空" / "新名称超长（请控制在 200 字节内）"。
- 不动磁盘文件（PRD 硬约束）。

## 参考文件
- `src-tauri/src/db/asset.rs`（既有 update / get_by_id / find_markdown_derivative）
- `src-tauri/src/db/tag.rs`（既有 propagate_tags_to_derivative）
- `src-tauri/src/utils/safe_name.rs`
- `task_001_architect/output.md` §ADR-007

## 预估影响范围
- 新建文件：无
- 修改文件：
  - `src-tauri/src/db/asset.rs`（+ resolve_asset_pair）
  - `src-tauri/src/commands/asset.rs`（+ rename_asset）
  - `src-tauri/src/lib.rs`（注册命令）
  - `src/lib/tauri-commands.ts`（+ renameAsset wrapper）
- 估算变更：~350 行（含 ~150 行测试）
