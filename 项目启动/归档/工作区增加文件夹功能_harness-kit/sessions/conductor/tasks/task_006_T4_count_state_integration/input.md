# Task 输入 — task_006_T4_count_state_integration

## 目标
实现 `count_folder_assets` read 命令（不取写锁）；在 `src/stores/uiStore.ts` 追加 5 个工作区文件夹瞬态字段 + setter（不进 partialize 白名单）；产出 2 个集成测试 `test_rename_db_path_sync` 与 `test_round_trip_root_to_folder_to_root`。

## 前置条件
- 依赖 task：task_003_T1_backend_utils（`validate_and_canonicalize` / `IpcError` 已就位）
- 必须先存在的文件/接口：
  - T1 全部产出
  - `NCdesktop/src/stores/uiStore.ts`（既有 `workspaceFolderRelativePath`）
  - `NCdesktop/src-tauri/src/db/asset.rs::rename_path_prefix`（注：若 T3 尚未实现，则在本 task 集成测试运行前必须等 T3 完成；Conductor 应保证 T4 集成测试在 T1+T3 双依赖 ready 后再跑）
- 本 task 可**与 T3 并行实现**（count 命令 + uiStore 字段不依赖 T3）；集成测试需 T3 完工后运行。

## 验收标准（Acceptance Criteria）

1. **AC-1 `count_folder_assets` 实现**：在 `src-tauri/src/commands/workspace_folders.rs` 实现 `count_folder_assets_impl(db, project_id, relative_path) -> Result<u32, IpcError>` + `#[tauri::command] count_folder_assets`。
   - **不取写通道锁**（read 命令；底线 7 除外项）
   - `validate_and_canonicalize` 仍要做（拒 `..` / 绝对 / symlink 越界）
   - `__ROOT__` 语义 = 根级**裸文件**（不下钻 organized 子树）：`file_path LIKE root/% AND NOT LIKE root/%/%`，元字符转义同 ADR-006
   - 子目录 = 子树（含等值匹配）：`file_path = abs OR file_path LIKE abs/% ESCAPE '\'`
   - 算法必须与前端聚合（ADR-010 `firstSegment`）等价
   - 注册到 `invoke_handler!`
2. **AC-2 uiStore 字段**：`src/stores/uiStore.ts` 追加（PRD §5.2）：
   ```ts
   editingFolderPath: string | null
   pendingNewFolder: boolean
   pendingRenameIds: Set<string>
   dragOverPath: string | null
   ```
   配套 setter：`startCreating()` / `cancelCreating()` / `startRenaming(path)` / `finishRename(path)` / `setDragOverPath(path)`。**5 字段全部禁止**加入 `partialize` 白名单（瞬态）。`pendingRenameIds` setter 必须返新 `Set` 实例（zustand 浅比较）。
3. **AC-3 集成测试 `test_rename_db_path_sync`**：
   - 文件：`NCdesktop/src-tauri/tests/workspace_folders_integration.rs`
   - 场景：建项目 → 在工作区根创建 `参考` 目录 → 写入 3 个 asset 入 DB（路径 `参考/a.pdf` 等）→ 调 `rename_workspace_folder` 将 `参考 → 参考资料` → 断言 DB 内 3 个 asset 的 `file_path` 前缀全部从 `参考/` 变为 `参考资料/`，且**受影响行数 = 物理子树文件数**（无遗漏、无误伤）。
4. **AC-4 集成测试 `test_round_trip_root_to_folder_to_root`**：
   - 场景：根目录有 `a.pdf`（DB 中 `file_path = "a.pdf"`）→ 调 `move_asset_to_workspace_folder(a_id, "参考")` → 断言物理在 `参考/a.pdf`、DB `file_path = "参考/a.pdf"`（不含 `__ROOT__`） → 再调 `move_asset_to_workspace_folder(a_id, "__ROOT__")` → 断言物理回根、DB `file_path = "a.pdf"`（裸文件名）；全程 `assets.file_path` 永不含 `__ROOT__` 字面量（验证 ADR-004）。
5. **AC-5 uiStore 单测**：`NCdesktop/src/stores/__tests__/uiStore.test.ts` 追加（或新建）覆盖：(a) `startRenaming("x")` 后 `pendingRenameIds.has("x") === true` 且 `editingFolderPath === "x"`；(b) `finishRename("x")` 后 `pendingRenameIds.has("x") === false` 且 `editingFolderPath === null`；(c) `pendingRenameIds` 是新 Set 实例（zustand 浅比较）；(d) `partialize` 序列化的快照不含本期 5 字段（瞬态）。
6. **AC-6 `cargo test --manifest-path NCdesktop/src-tauri/Cargo.toml --test workspace_folders_integration test_rename_db_path_sync` 与 `test_round_trip_root_to_folder_to_root` 全绿；`pnpm test uiStore` 全绿**。

## 技术约束
- 底线 7：count 是 read 命令，**不取写通道锁**。
- 底线 6：`__ROOT__` 在 count 语义里特殊处理（仅根级裸文件，不下钻 `organized/`）；DB 内 `file_path` 永不含 `__ROOT__`。
- 底线 10：所有错误用 `IpcError`。
- ADR-010：前端聚合与后端 count 必须算法等价（`__ROOT__` = 不含 `/` 的直属文件；其余 = `LIKE folder/%` + 等值匹配）。
- ADR-009：5 字段进入 `UIStore` interface 但**禁止**进入 `partialize` 白名单；`pendingRenameIds` setter 必须返新 Set。
- 不顺手改无关代码。

## 参考文件
- `sessions/conductor/tasks/task_002_T0_contracts/contracts.md` §B / §C
- `sessions/conductor/tasks/task_001_architect/output.md` ADR-004 / 009 / 010
- 既有代码：
  - `NCdesktop/src-tauri/src/commands/workspace_folders.rs`（在末尾追加 count）
  - `NCdesktop/src-tauri/src/lib.rs`（`invoke_handler!` 注册 count）
  - `NCdesktop/src/stores/uiStore.ts`（追加 5 字段；现有 `workspaceFolderRelativePath` 已存在）
  - `NCdesktop/src-tauri/tests/` （新建 `workspace_folders_integration.rs`）

## 预估影响范围
- 新建文件：
  - `NCdesktop/src-tauri/tests/workspace_folders_integration.rs`
  - `NCdesktop/src/stores/__tests__/uiStore.test.ts`（或在现有 test 文件中追加用例）
- 修改文件：
  - `NCdesktop/src-tauri/src/commands/workspace_folders.rs`（+ count_folder_assets）
  - `NCdesktop/src-tauri/src/lib.rs`（+ count_folder_assets 注册）
  - `NCdesktop/src/stores/uiStore.ts`（+ 5 字段 + 5 setter；不进 partialize）
