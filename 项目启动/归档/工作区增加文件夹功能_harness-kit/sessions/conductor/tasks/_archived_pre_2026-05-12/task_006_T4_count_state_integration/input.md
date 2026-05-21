# Task 输入 — task_006_T4_count_state_integration

## 目标
实现 `count_folder_assets` 命令、`uiStore` 新增 5 字段及其 setter，并完成 2 个 Rust 集成测试 `test_rename_db_path_sync` 与 `test_round_trip_root_to_folder_to_root`。

## 前置条件
- 依赖 task：task_003_T1_backend_utils（DONE）；可与 task_005_T3 部分并行，但 2 个集成测试**依赖 T3 的 rename/move 命令**已实现，因此本 task 启动时间不早于 T3。
- 必须先存在的文件/接口：
  - T1 工具层（validate / write_guard / nfc）
  - T3 4 写命令已实现（用于 round-trip 集成测试）
  - 既有 `uiStore.ts` 中 `workspaceFolderRelativePath`

## 验收标准（Acceptance Criteria）
1. **AC-1**：`cargo test --manifest-path NCdesktop/src-tauri/Cargo.toml --test workspace_folder_integration test_rename_db_path_sync` PASS — 构造一个工作区含 `参考/` 目录 + 3 个 asset（DB 中 file_path = `参考/a.png` 等），rename 为 `参考资料`，断言：(a) 物理目录改名；(b) DB 中受影响行数 = 3 = 物理子树文件数；(c) 同级别 `参考N` 类目录文件 file_path 未变。
2. **AC-2**：`cargo test --manifest-path NCdesktop/src-tauri/Cargo.toml --test workspace_folder_integration test_round_trip_root_to_folder_to_root` PASS — 一个 asset 从 `__ROOT__`（DB `file_path = "a.png"`）→ move 到 `参考` 文件夹（DB → `参考/a.png`，物理 → `参考/a.png`）→ move 回 `__ROOT__`（DB → `a.png`，物理 → 根目录 `a.png`），断言物理位置与 DB `file_path` 双向一致，且 `file_path` 永不含 `__ROOT__`。
3. **AC-3**：`count_folder_assets(project_id, "__ROOT__")` 返根级无 `/` 的 asset 数；`count_folder_assets(_, "参考资料")` 返 `LIKE '参考资料/%' ESCAPE '\'` + 等值匹配的 asset 数；单测覆盖 `100%off` 边界（与 T3 AC-4 算法一致）。
4. **AC-4**：`uiStore.ts` 新增 5 字段 + setter，按 PRD §5.2：
    - `editingFolderPath: string | null`
    - `pendingNewFolder: boolean`
    - `pendingRenameIds: Set<string>`
    - `dragOverPath: string | null`
    - （`workspaceFolderRelativePath` 已存在）
   每个 setter 单测：`pnpm test uiStore` PASS。
5. **AC-5**：5 新字段**禁止**进入 `partialize` 白名单（仍仅持久化 `activeSidebarSection`、`todayLastTab`）；单测验证刷新后 5 字段重置为初始值。
6. **AC-6**：`count_folder_assets` 注册到 `invoke_handler!`；前端 wrapper `countFolderAssets` 已在 T2 加好，本 task 验证 e2e 调用链通畅（`pnpm tsc --noEmit` PASS）。
7. **AC-7**：集成测试套件作为单独 binary `tests/workspace_folder_integration.rs`；走真实 SQLite + tempfile 工作区目录；测试间不串扰。

## 技术约束
- `count_folder_assets` 不下钻（MVP 非递归，Debate §5）。
- count SQL 与 T3 rename 同算法（共享 helper 或保证一致；ADR-010）。
- uiStore 5 字段全部为**会话内瞬态**，不可 persist（PRD §5.2 + 现有 partialize 约定）。
- `pendingRenameIds: Set<string>` 的 setter 必须返回新 `Set` 实例（Zustand 浅比较，否则不触发 re-render）。
- 集成测试用 `tempfile::TempDir` 模拟工作区根，**禁止操作真实 `~/Downloads/NoteCaptWorkPlace`**；可通过 env override `WORKSPACE_ROOT_OVERRIDE` 或重构 workspace.rs 注入 root（小心不要破坏既有 prod 行为）。
- 不顺手改无关代码；commit 中文 Conventional。

## 参考文件
- 既有：`NCdesktop/src/stores/uiStore.ts:120-280`、`NCdesktop/src-tauri/src/workspace.rs`、`NCdesktop/src-tauri/src/db/asset.rs`、`NCdesktop/src-tauri/tests/`（如目录不存在则新建）
- 契约：`sessions/conductor/tasks/task_002_T0_contracts/contracts.md`
- 方案：output.md ADR-006/010、§数据模型、§API 设计

## 预估影响范围
- 新建文件：
  - `NCdesktop/src-tauri/tests/workspace_folder_integration.rs`
- 修改文件：
  - `NCdesktop/src-tauri/src/commands/workspace_folders.rs`（+`count_folder_assets`）
  - `NCdesktop/src-tauri/src/lib.rs`（注册 `count_folder_assets`）
  - `NCdesktop/src/stores/uiStore.ts`（+5 字段 + setter；不进 partialize）
  - `NCdesktop/src/stores/__tests__/uiStore.test.ts`（追加 5 字段单测；如文件不存在则新建）
