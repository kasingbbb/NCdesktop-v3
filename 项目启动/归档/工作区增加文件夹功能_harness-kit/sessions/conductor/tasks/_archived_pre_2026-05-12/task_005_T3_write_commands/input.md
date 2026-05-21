# Task 输入 — task_005_T3_write_commands

## 目标
实现 4 个写命令（`create_workspace_folder` / `rename_workspace_folder` / `delete_workspace_folder` / `move_asset_to_workspace_folder`），全部走 `IpcError` + `WorkspaceWriteGuard` + `validate_and_canonicalize` + ai_organized 入口拦截 + 同事务前缀替换 + EXDEV copy-first；产出 PRD §6.1 Rust 单测全套。

## 前置条件
- 依赖 task：task_003_T1_backend_utils（DONE）
- 必须先存在的文件/接口：
  - `IpcError` enum + `From<IpcError> for String`
  - `workspace::validate_and_canonicalize` / `validate_folder_name` / `resolve_relative_path`
  - `utils::safe_rename::safe_rename`
  - `utils::write_guard::WorkspaceWriteGuard`
  - `utils::nfc::nfc_normalize`
  - `db::asset::rename_path_prefix(tx, old_prefix, new_prefix)`（本 task 一并新增）
  - 既有 `commands/asset.rs::move_asset_to_workspace_folder`（旧版，本 task 替换 / 退役）

## 验收标准（Acceptance Criteria）
1. **AC-1（PRD §6.1 路径越界）**：`cargo test --manifest-path NCdesktop/src-tauri/Cargo.toml workspace_folders::path_escape` PASS — `../../etc`、`/etc/passwd`、symlink 指向 `/tmp` 三例均返 `E_PATH_ESCAPE`。
2. **AC-2（PRD §6.1 保留字）**：`cargo test workspace_folders::reserved_name` PASS — `create_workspace_folder(_, "organized")` 返 `E_NAME_RESERVED`；`rename_workspace_folder(_, "foo", "organized")` 同样返 `E_NAME_RESERVED`。
3. **AC-3（PRD §6.1 ai_organized 四类写）**：`cargo test workspace_folders::ai_organized_protected` PASS — 对 `relative_path="organized/x"` 调用 rename / delete / create 子目录 / move 入，全部返 `E_PROTECTED_KIND`，即使前端绕过直 invoke。
4. **AC-4（PRD §6.1 SQL 前缀边界）**：`cargo test workspace_folders::prefix_boundary` PASS — 构造 `100` 与 `100%off` 两根级目录，各放 1 个 asset；rename `100 → 200`，断言 `100%off` 下 asset.file_path 未变；`100` 目录下 asset 的 file_path 改为 `200/...`。
5. **AC-5（PRD §6.1 NFC）**：`cargo test workspace_folders::nfc_dup` PASS — 先 create NFD `"参考"`，再 create NFC `"参考"` 返 `E_NAME_DUP`。
6. **AC-6（删除走 trash + 复检）**：`cargo test workspace_folders::trash_recheck` PASS — mock trash 后 `path.exists()` 仍 true 时返 `E_TRASH_FAILED`；macOS 实际走 `trash::delete`；Win/Linux cfg 编译路径返 `E_PLATFORM_UNSUPPORTED`。
7. **AC-7（写通道锁）**：`cargo test workspace_folders::write_guard_serializes` PASS — 两个并发线程对同一 project_id 各调一次 rename，最终结果一致、无 race。
8. **AC-8（handler 入口判定）**：4 个命令首行均 `let _g = guard.lock_for(&project_id)?;` + kind 判定，单测覆盖 direct invoke 绕过 UI 仍被拒。
9. **AC-9（IpcError 序列化）**：所有错误返回值 `serde_json::from_str::<IpcError>(&string)` 可成功还原。
10. **AC-10（lib.rs 注册）**：4 个写命令注册到 `invoke_handler!`；`app.manage(WorkspaceWriteGuard::new())`；`cargo build` 成功。
11. **AC-11（既有 caller 调整）**：`AssetListView` / `AssetContextMenu` 等调用旧 `move_asset_to_workspace_folder(Vec<String>, ..)` 的位置改为循环单素材调用（或 wrapper 层批处理），`pnpm tsc --noEmit` PASS。

## 技术约束
- **路径越界**：所有写命令首行调 `validate_and_canonicalize(project_id, rel)`；canonicalize 后必须仍 `starts_with(workspace_root_canonical)`（PRD §4.1.1）。
- **ai_organized 双层拦**：handler 第一段从 `relative_path` 推 kind（`starts_with("organized/")` 视为 ai_organized；`__ROOT__` 视为 root_import），返 `E_PROTECTED_KIND` 不依赖 UI（ADR-007、底线 1）。
- **写通道锁**：4 命令首行取 `WorkspaceWriteGuard::lock_for(&project_id)`；同时**在 `import_drop_paths` 命令开头追加**同一锁（PRD 底线 7）。
- **同事务 SQL 前缀替换**：rename 与 move 均用 `db::asset::rename_path_prefix(tx, old, new)` 模板（ADR-006）：
  ```sql
  UPDATE assets
  SET file_path = :new_prefix || substr(file_path, length(:old_prefix)+1)
  WHERE file_path = :old_no_slash
     OR file_path LIKE :old_prefix || '/%' ESCAPE '\';
  ```
  `:old_prefix` 强制尾 `/`，`\ % _` 预转义；与物理 `safe_rename` 同一事务（PRD §4.2.1、底线 4）。
- **EXDEV**：rename / move 失败为 EXDEV 时走 `safe_rename` 的 copy-first 两阶段（copy→fsync→rename(tmp→final)→COMMIT→remove src）；COMMIT 后 remove 失败仅 log `cleanup_pending`（ADR-002、底线 5）。
- **删除**：
  - `confirm_non_empty=false` 且 count > 0 直接返 `E_FOLDER_DIRTY{old:0, now:count}`（前端必先 confirm）；
  - 事务内 recount `count_folder_assets`，与入参 `expected_count` 不一致返 `E_FOLDER_DIRTY{old:expected, now:actual}`；
  - 走 `trash::delete(&abs_path)` 后 `abs_path.exists()` 复检（PRD §4.2.3、底线 3）；
  - 残留扫描：每个 trashed asset 调用现有 `db::asset::delete_asset_semantics` 同步 DB（PRD §4.2.3）；
  - Win/Linux `#[cfg(not(target_os = "macos"))]` 返 `E_PLATFORM_UNSUPPORTED`（PRD §3 F3）。
- **命名校验**：后端权威调 `validate_folder_name(name)`；rename 时还要查同级是否已有 NFC 同名（`E_NAME_DUP`）；create 同样查重（ADR-008、底线 9）。
- **`__ROOT__` 防 DB 污染**：`db::asset` 所有 INSERT/UPDATE 入口加 `debug_assert!(!path.contains("__ROOT__"))`（ADR-004、底线 6）。
- **签名收敛**：`move_asset_to_workspace_folder` 收敛为 PRD §5.1 单素材签名 `(asset_id: String, target_relative_path: String) -> Result<Asset, IpcError>`；旧多素材入口删除或改 `#[deprecated]`（不破坏既有调用方编译，但调用方需改造）。
- 命令归属：4 写命令全部落 `commands/workspace_folders.rs`（与 PRD §5.1 一致）。
- 不顺手改无关代码；commit 中文 Conventional。
- **规模红线**：若实测 LOC 超过 1500，向 Conductor 报告拆 T3b。

## 参考文件
- 既有：
  - `NCdesktop/src-tauri/src/commands/workspace_folders.rs`（list / reveal / get_root 三命令保留）
  - `NCdesktop/src-tauri/src/commands/asset.rs:248-319`（旧 move 实现，替换）
  - `NCdesktop/src-tauri/src/db/asset.rs`（INSERT/UPDATE 入口加 debug_assert）
  - `NCdesktop/src-tauri/src/lib.rs:141-143`（既有 3 个 workspace 命令注册位置）
- 契约：`sessions/conductor/tasks/task_002_T0_contracts/contracts.md`
- 方案：output.md ADR-001/002/003/004/006/007/008、§API 设计、§数据模型

## 预估影响范围
- 新建文件：
  - 无（命令落既有 `workspace_folders.rs`；helper 落既有 `db::asset` 与 T1 utils）
- 修改文件：
  - `NCdesktop/src-tauri/src/commands/workspace_folders.rs`（+4 写命令）
  - `NCdesktop/src-tauri/src/commands/asset.rs`（旧 `move_asset_to_workspace_folder` 删除 / 退役）
  - `NCdesktop/src-tauri/src/commands/dropzone.rs`（`import_drop_paths` 加锁）
  - `NCdesktop/src-tauri/src/db/asset.rs`（追加 `rename_path_prefix` + debug_assert）
  - `NCdesktop/src-tauri/src/lib.rs`（注册 4 写命令；`app.manage(WorkspaceWriteGuard)`；移除旧 `move_asset_to_workspace_folder` 旧注册或重定位）
  - `NCdesktop/src/components/features/AssetListView.tsx`（调用方改造为单素材 IPC）
  - `NCdesktop/src/components/features/AssetContextMenu.tsx`（同上，如有调用）
