# Task 输入 — task_009_T6_drag_drop

## 目标
F4 拖拽落地（单素材从右栏 AssetListView 卡片拖到 `WorkspaceFolderListView` 的 `root` / `__ROOT__` 行；ai_organized 拦截 + toast；编辑行禁止 + toast；drop 高亮 inset 2px `var(--accent-emphasis)`）；并产出 2 个集成测试 `test_exdev_two_phase` 与 `test_delete_dirty_recount`；最后产 PR 截图 + 10s GIF。

## 前置条件
- 依赖 task：task_005_T3_write_commands、task_006_T4_count_state_integration、task_008_T5b_inline_edit
- 必须先存在的文件/接口：
  - T3 `move_asset_to_workspace_folder` + `delete_workspace_folder` + `safe_rename` EXDEV 测试注入
  - T4 集成测试基础设施（`workspace_folders_integration.rs` 已存在）+ uiStore `dragOverPath` setter
  - T5b 全套（inline 编辑 + `editingFolderPath` 字段已生效）

## 验收标准（Acceptance Criteria）

1. **AC-1 drag source**：在 `AssetListView` 右栏素材卡片上加 `draggable={true}` + `onDragStart`：通过 `e.dataTransfer.setData('application/x-ncdesk-asset-id', asset.id)` 携带 asset id；不污染既有素材卡片 UI；单素材，**多素材本期不做**（PRD §3 P2）。
2. **AC-2 drop target — `root` / `__ROOT__` 双向合法**：`FolderListRow` 行根 `onDragEnter / onDragOver / onDragLeave / onDrop`：
   - 使用 `useRef<number>(0)` 计数器避免子元素冒泡抖动：enter ++、leave --，归零才 `setDragOverPath(null)`。
   - `onDragOver` `e.preventDefault()` 才能 drop；行视觉高亮 = `boxShadow: 'inset 0 0 0 2px var(--accent-emphasis)'`（深浅色均可见 R9），**不**做整行反色。
   - `onDrop`：取 dataTransfer 的 asset id → 调 `moveAssetToWorkspaceFolder(assetId, row.relativePath)`（`__ROOT__` 直接作为入参，后端 `resolve_relative_path` 单点归一）。
3. **AC-3 ai_organized 拦截**：drop 到 `kind === 'ai_organized'` 行 → `preventDefault` 不 dispatch + `addNotification` toast「AI 归类目录受保护，不可手动移入」；**不发 IPC**（前端拦死）。
4. **AC-4 编辑行禁止**：drop 到 `editingFolderPath === row.relativePath` 的行 → 禁止图标（`cursor: not-allowed`）+ toast「目标正在编辑中」；**不发 IPC**。
5. **AC-5 selection 不变**：drop 完成后**不改变** `workspaceFolderRelativePath`（用户保持当前筛选）。
6. **AC-6 集成测试 `test_exdev_two_phase`**（`src-tauri/tests/workspace_folders_integration.rs` 追加）：
   - 注入 `safe_rename::test_inject::SIMULATE_EXDEV_FOR` 模拟 EXDEV
   - 调 `move_asset_to_workspace_folder_impl(db, guard, asset_id, target_rel)` → 断言：
     1. 物理 dst 完整存在
     2. 物理 src **未被立即删除**（直到 caller 在 commit 后调 `remove_src_after_commit`）
     3. DB `file_path` 已更新（事务已 commit）
     4. 测试结尾调 `remove_src_after_commit` 后 src 被清
7. **AC-7 集成测试 `test_delete_dirty_recount`**：
   - 创建 root 文件夹 `tmp/`，先 insert 2 个 asset → 前端取 `expected_count = 2`
   - **并发**在 confirm 与 invoke 之间往该目录塞入第 3 个 asset（直接 INSERT，绕过命令）
   - 调 `delete_workspace_folder_impl(db, guard, "p1", "tmp", true, expected_count=2)` → 断言 IpcError `code === E_FOLDER_DIRTY`、`details.old === 2`、`details.now === 3`
   - 文件夹**未被**移到回收站（事务回滚）
8. **AC-8 前端 drop 单测**：`__tests__/WorkspaceFolderListView.test.tsx` 追加：
   - drop 到 ai_organized 行 → 不触发 `moveAssetToWorkspaceFolder` wrapper + toast 出现
   - drop 到 `editingFolderPath` 行 → 不触发 wrapper + toast 出现
   - drop 到 `__ROOT__` 行 → 触发 wrapper，入参 `targetRelativePath === "__ROOT__"`
   - drop 到 `root` 行 → 触发 wrapper，入参 `targetRelativePath === folder.relativePath`
   - dragenter 计数器：连续 enter 子元素再 leave 不应清除高亮，归零才清
9. **AC-9 全测试绿 + PR 物料**：
   - `cargo test --manifest-path NCdesktop/src-tauri/Cargo.toml` 全绿（含 T3 单测 + T4 + T6 集成测试）
   - `pnpm test` 全绿（含 T2 / T4 uiStore / T5a / T5b / T6 单测）
   - 手动验收按 PRD §6.4 / §验收 1-6 全过
   - PR 描述附**新列表截图**（浅色 + 深色各一张，drop 高亮可见）+ **10 秒 GIF**（演示「新建 → 重命名 → 拖入素材 → 删除」四连）
   - Commit 用中文 Conventional Commits（如 `feat(workspace): F4 拖拽与集成测试 + GIF`）

## 技术约束
- ADR-011：仅用 HTML5 DnD；**禁止**接 `tauri://drag-drop`；dragenter 计数器避免抖动；drop 高亮用 `var(--accent-emphasis)` 2px inset。
- ADR-007 / 底线 1：drop 到 ai_organized / 编辑行 → 前端拦死，**不发 IPC**。
- ADR-002 / 底线 5：EXDEV copy-first；测试用 `test_inject` 验证两阶段顺序。
- ADR-006 / 底线 4：rename / move 在同事务；T6 不应再触动 SQL，沿用 T3 实现即可。
- ADR-012 / 底线 3：删除走 `trash::delete` + 复检；本期不补强删除逻辑。
- 底线 6：drop 入参可为 `"__ROOT__"`；后端 `resolve_relative_path` 单点归一；前端**不**翻译。
- session_context §6 审查重点：drop 高亮限 2px 内描边（不整行反色）；深色模式可见；编辑行禁止图标。
- 不顺手改无关代码；多素材拖拽不做。

## 参考文件
- `sessions/conductor/tasks/task_001_architect/output.md` ADR-002 / 007 / 011 / 012
- `sessions/workspace_folder_mgmt/debate/session_001/debate_conclusions.md` §6 R1 / R7 / R9
- `product/prd/workspace_folder_mgmt_prd_v1.md` §3 P0 F4 / §6 测试要求 / §8 PR Ready Checklist
- 既有代码：
  - `NCdesktop/src-tauri/src/utils/safe_rename.rs::test_inject`（EXDEV 模拟）
  - `NCdesktop/src-tauri/src/commands/workspace_folders.rs::move_asset_to_workspace_folder_impl` / `delete_workspace_folder_impl`
  - `NCdesktop/src-tauri/tests/workspace_folders_integration.rs`（T4 已建立的测试 harness）
  - `NCdesktop/src/components/features/AssetListView.tsx`（drag source 改造点）
  - `NCdesktop/src/components/features/WorkspaceFolderListView/FolderListRow.tsx`（drop target）
  - `NCdesktop/src/lib/tauri-commands.ts::moveAssetToWorkspaceFolder`
  - `NCdesktop/src/stores/uiStore.ts`（`dragOverPath` / `editingFolderPath`）

## 预估影响范围
- 新建文件：无（仅追加集成测试与单测）
- 修改文件：
  - `NCdesktop/src/components/features/AssetListView.tsx`（卡片加 `draggable` + `onDragStart`）
  - `NCdesktop/src/components/features/WorkspaceFolderListView/FolderListRow.tsx`（drop target 处理 + dragenter 计数器 + 高亮）
  - `NCdesktop/src/components/features/__tests__/WorkspaceFolderListView.test.tsx`（追加 drop 用例）
  - `NCdesktop/src-tauri/tests/workspace_folders_integration.rs`（追加 `test_exdev_two_phase` + `test_delete_dirty_recount`）
- 交付物：
  - PR 描述截图 ×2（浅 + 深色）
  - PR 描述 10s GIF
