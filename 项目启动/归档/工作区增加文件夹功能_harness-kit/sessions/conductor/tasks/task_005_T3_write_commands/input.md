# Task 输入 — task_005_T3_write_commands

## 目标
在 `commands/workspace_folders.rs` 实现 4 个写命令（`create_workspace_folder` / `rename_workspace_folder` / `delete_workspace_folder` / `move_asset_to_workspace_folder`），全部走 `WorkspaceWriteGuard` + `validate_and_canonicalize` + kind 拦截 + 同事务前缀替换；注册到 `invoke_handler!`；产出 PRD §6.1 全套 Rust 单测；并把旧 `commands::asset::move_asset_to_workspace_folder`（多素材）退役。

## 前置条件
- 依赖 task：task_003_T1_backend_utils（IpcError / write_guard / safe_rename / nfc / validate_and_canonicalize 已就位）
- 必须先存在的文件/接口：
  - `NCdesktop/src-tauri/src/utils/ipc_error.rs`
  - `NCdesktop/src-tauri/src/utils/write_guard.rs::WorkspaceWriteGuard`
  - `NCdesktop/src-tauri/src/utils/safe_rename.rs::{safe_rename, RenameOutcome, remove_src_after_commit}`
  - `NCdesktop/src-tauri/src/utils/nfc.rs::{nfc_normalize, nfc_eq}`
  - `NCdesktop/src-tauri/src/workspace.rs::{validate_and_canonicalize, validate_folder_name, resolve_relative_path}`
- 本 task 可**与 T4 并行**（共依 T1）。

## 验收标准（Acceptance Criteria）

1. **AC-1 4 写命令实现**：在 `src-tauri/src/commands/workspace_folders.rs` 实现：
   - `create_workspace_folder(project_id, name) -> Result<WorkspaceFolderEntry, IpcError>`：首行 `guard.lock_for(&project_id)` → `validate_folder_name(name)` → `nfc_normalize(name)` → `validate_and_canonicalize(project_id, nfc_name)` → 同级 NFC 查重 → `fs::create_dir(abs)` → 返新 `WorkspaceFolderEntry { kind: "root" }`。仅允许根级创建。
   - `rename_workspace_folder(project_id, relative_path, new_name) -> Result<WorkspaceFolderEntry, IpcError>`：首行 guard → `kind_from_relative_path(rel)`（`organized/` 前缀 → `ai_organized` 返 `E_PROTECTED_KIND`；`__ROOT__` → `root_import` 返 `E_PROTECTED_KIND`） → `validate_and_canonicalize` 旧路径 → `validate_folder_name(new_name)` → 同级 NFC 查重（排除 self-equal no-op） → `tx = conn.unchecked_transaction()` → `safe_rename(old, new)` → `db::asset::rename_path_prefix(tx, old_prefix, new_prefix)` → `tx.commit()` → 若 `RenameOutcome::CrossDevice` 则 `remove_src_after_commit`。
   - `delete_workspace_folder(project_id, relative_path, confirm_non_empty, expected_count) -> Result<DeleteReport, IpcError>`：首行 guard → kind 拦（仅 `root`） → `validate_and_canonicalize` → 平台保护（`cfg!(not(target_os = "macos"))` → `E_PLATFORM_UNSUPPORTED { feature: "trash", platform }`） → `tx = conn.unchecked_transaction()` → 事务内 `count_assets_under_prefix` 重 count → `if !confirm_non_empty && recount > 0 → E_FOLDER_DIRTY{0, recount}`；`if recount != expected_count → E_FOLDER_DIRTY{expected, recount}` → 事务内 `delete_assets_under_prefix` → `trash::delete(abs)` 通过 `TrashAdapter` 抽象 → `abs.exists()` 复检失败 → `E_TRASH_FAILED` → `tx.commit()` → 返 `DeleteReport { trashed: recount }`。
   - `move_asset_to_workspace_folder(asset_id, target_relative_path) -> Result<Asset, IpcError>`：先 `db::asset::get_by_id` 拿 `project_id` → guard → kind 拦（target 拒 `ai_organized`；`__ROOT__` 允许 = 双向合法） → `validate_and_canonicalize(target)` → 目标目录不存在则 `create_dir_all` → 计算 `unique_path(target.join(file_name))` 避免覆盖 → `safe_rename(src, dst)` → `tx.execute("UPDATE assets SET name=?, file_path=? WHERE id=?")` 同事务 → COMMIT → 失败回滚物理 rename（best effort） → 若 `CrossDevice` 则 `remove_src_after_commit` → 返更新后 `Asset`。
2. **AC-2 `rename_path_prefix` helper**：在 `src-tauri/src/db/asset.rs` 实现 `rename_path_prefix(tx, old_prefix, new_prefix) -> rusqlite::Result<usize>`，SQL 模板严格遵守 ADR-006：
   ```sql
   UPDATE assets
   SET file_path = :new_prefix || substr(file_path, length(:old_prefix)+1)
   WHERE file_path = :old_no_slash
      OR file_path LIKE :old_prefix || '/%' ESCAPE '\';
   ```
   Rust 侧 `escape_like_prefix(s)` 对 `\ % _` 按序转义；`:old_prefix` 强制带尾 `/`。
3. **AC-3 invoke_handler 注册 + 旧命令退役**：`src-tauri/src/lib.rs` `invoke_handler!` 注册 4 新命令；同时**注销**旧 `commands::asset::move_asset_to_workspace_folder`（多素材签名）。如有调用方（`AssetListView` / `AssetContextMenu` 等）使用旧多素材命令，改为循环调用单素材新命令（NEW-R13）。`debug_assert!(!path.contains("__ROOT__"))` 加在 `db::asset::insert` / `update` 入口（ADR-004）。
4. **AC-4 PRD §6.1 单测全套**（`cargo test --manifest-path NCdesktop/src-tauri/Cargo.toml workspace_folders` 全绿）：
   - 路径越界：`../../etc` / `/etc/passwd` / symlink → `E_PATH_ESCAPE`
   - 保留字 `organized` create + rename → `E_NAME_RESERVED`
   - `ai_organized` 四类写（rename / delete / create 子目录 / move 入）→ `E_PROTECTED_KIND`
   - SQL 前缀边界：`100` 与 `100%off` 同级，rename `100→200`，`100%off` 子树 asset 未变
   - NFC 查重：磁盘上有 NFD `cafe\u{0301}`，再 create NFC `café` → `E_NAME_DUP`
   - trash 复检：注入 `LyingTrash` stub（声称成功但实际未删）→ `E_TRASH_FAILED`
   - 写通道串行：同 project 两线程并发 rename，断言串行（max concurrent = 1）
   - direct invoke 绕过：rename `organized/x` → `E_PROTECTED_KIND`；序列化 JSON 可 round-trip 还原 `code` + `details`
5. **AC-5 不打断既有功能**：`list_project_workspace_folders` / `reveal_project_workspace_folder` / `get_project_workspace_root` 保持现有 `Result<T, String>` 签名不动。
6. **AC-6 `cargo test --manifest-path NCdesktop/src-tauri/Cargo.toml` 全绿**（含 T1 单测）。

## 技术约束
- 底线 1：ai_organized 前后端各拦一次；后端 handler 入口判定不依赖前端。
- 底线 2：所有写命令首行三件套 = guard + kind 判定 + validate_and_canonicalize。
- 底线 3：`fs::remove_dir_all` 禁用；删除必经 `trash::delete` + `path.exists()` 复检。
- 底线 4：rename / move / delete 的 DB 改动必须在同一 SQL 事务内（`unchecked_transaction()`）。
- 底线 5：跨卷 EXDEV 走 `safe_rename` copy-first；caller 在 `tx.commit()` 后调 `remove_src_after_commit`。
- 底线 6：`assets` INSERT/UPDATE 必须 `debug_assert!(!path.contains("__ROOT__"))`。
- 底线 7：5 命令首行均取 `WorkspaceWriteGuard::lock_for(&project_id)`。
- 底线 9：命名校验调 `validate_folder_name`，不要在命令内再写校验。
- 底线 10：所有错误必须用 `IpcError` 工厂，不要返裸 String。
- 不顺手改无关代码。

## 参考文件
- `sessions/conductor/tasks/task_002_T0_contracts/contracts.md` §B / §C
- `sessions/conductor/tasks/task_001_architect/output.md` ADR-002 / 003 / 004 / 006 / 007 / 008 / 012、API 设计、安全考量
- `sessions/workspace_folder_mgmt/debate/session_001/debate_conclusions.md` §6 R1-R6
- 既有代码：
  - `NCdesktop/src-tauri/src/commands/workspace_folders.rs`（既有 read 命令保留；4 写命令在末尾追加）
  - `NCdesktop/src-tauri/src/commands/asset.rs`（旧 `move_asset_to_workspace_folder` 退役）
  - `NCdesktop/src-tauri/src/db/asset.rs`（追加 `rename_path_prefix`；INSERT/UPDATE 入口加 debug_assert）
  - `NCdesktop/src-tauri/src/lib.rs`（`invoke_handler!`）
  - `NCdesktop/src/components/features/AssetListView.tsx` / `AssetContextMenu.tsx`（旧 move 调用方迁移）

## 预估影响范围
- 新建文件：无
- 修改文件：
  - `NCdesktop/src-tauri/src/commands/workspace_folders.rs`（+ 4 写命令 + 辅助函数 + 单测）
  - `NCdesktop/src-tauri/src/commands/asset.rs`（旧 move 退役）
  - `NCdesktop/src-tauri/src/db/asset.rs`（+ rename_path_prefix + debug_assert）
  - `NCdesktop/src-tauri/src/lib.rs`（+ 4 新命令注册；注销旧 move）
  - `NCdesktop/src/components/features/AssetListView.tsx` / `AssetContextMenu.tsx`（若有旧多素材 move 调用方，循环改写）
