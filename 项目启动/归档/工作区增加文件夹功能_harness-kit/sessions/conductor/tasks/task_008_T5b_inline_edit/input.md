# Task 输入 — task_008_T5b_inline_edit

## 目标
在 `WorkspaceFolderListView` 上加 inline 编辑状态机（`mode: 'idle' | 'creating' | 'renaming'`）：F1 幽灵新建行（Enter 提交 / Esc 取消 / blur 提交、失败保留编辑态 + 红框、切走二次确认 modal「放弃新建『xxx』？」）与 F2 重命名（三入口：右键 / Enter / 工具栏；全选当前名；同步乐观提交 + `pendingRenameIds` selection 冻结；失败回退 + selection 自动回到该节点 + 红框）；并实现 F3 删除二次确认 modal（含 `expected_count` 文案）。

## 前置条件
- 依赖 task：task_007_T5a_list_skeleton
- 必须先存在的文件/接口：
  - T5a 全部组件骨架
  - T4 uiStore 5 字段（`editingFolderPath` / `pendingNewFolder` / `pendingRenameIds` / `dragOverPath` + setter）
  - T2 `createWorkspaceFolder` / `renameWorkspaceFolder` / `deleteWorkspaceFolder` wrapper + `validateFolderNameSync` + `errorMessages`

## 验收标准（Acceptance Criteria）

1. **AC-1 编辑状态机**：`WorkspaceFolderListView` 内部 `mode = 'idle' | 'creating' | 'renaming'` 推导自 `uiStore` 字段（`pendingNewFolder` → creating；`editingFolderPath !== null` → renaming；否则 idle）。三态互斥；同一时刻**至多一个**输入框处于编辑。
2. **AC-2 F1 幽灵新建**：点击工具栏「+ 新建文件夹」按钮（或 P1 `⌘⇧N` 快捷键，可选）→ `startCreating()` → 列表末尾插入幽灵行（不进 fs / 不发 IPC），输入框默认值 `未命名文件夹` 且**全选**进入 inline 编辑：
   - Enter / blur → `validateFolderNameSync` 即时校验失败保留编辑态 + 红框；通过 → `await createWorkspaceFolder(projectId, name)`：成功 → `cancelCreating()` + 触发列表刷新（`listProjectWorkspaceFolders`）+ `setWorkspaceFolderRelativePath(newRel)` 自动选中新行；失败（IpcError）→ 保留编辑态 + 行内 error toast（用 `renderIpcError`）+ 红框（用户可继续改名 / Esc 放弃）。
   - Esc → `cancelCreating()` 直接丢弃。
   - 编辑期间用户点击其他行 / 触发拖拽 → 弹二次确认 modal「放弃新建『xxx』？」；确认放弃 → `cancelCreating()` + 执行原 action；取消 → 不变。
3. **AC-3 F2 重命名**：三入口（右键菜单「重命名」、选中 root 行按 Enter、工具栏「重命名」按钮）→ 首行权限判定 `if (selection.kind !== 'root') return;` → `startRenaming(relativePath)` → 行内输入框全选当前名进入 inline 编辑：
   - Enter / blur → `validateFolderNameSync` → 通过 → 同步乐观替换名称（UI 立即变更）→ `await renameWorkspaceFolder(projectId, oldRel, newName)`：成功 → `finishRename(oldRel)` + 触发列表刷新 + 重新设置 `setWorkspaceFolderRelativePath(newRel)` 保持选中；失败（IpcError）→ 回滚名称 + `finishRename(oldRel)` + selection 自动回到该节点 + 行内 error + 红框。
   - Esc → 取消编辑（`finishRename(oldRel)`，不发 IPC，UI 名称不变）。
   - `pendingRenameIds` 非空时点击其他行**无响应**（selection 冻结）。
4. **AC-4 F3 删除二次确认**：选中 root 行 → ⌘⌫ / 右键「移到废纸篓」/ 工具栏「移到废纸篓」→ 首行权限判定 → 从前端聚合取 `N = firstSegmentCount(folder)` 作为 `expectedCount` → 弹 modal：
   - `N === 0` 文案：`「删除文件夹『xxx』？」`
   - `N > 0` 文案：`「该文件夹包含 N 个素材，一同移到废纸篓？」`
   - 确认 → `await deleteWorkspaceFolder(projectId, relativePath, confirmNonEmpty: N > 0, expectedCount: N)`：成功 → toast「已移到废纸篓」+ 列表刷新 + 清 selection 或回退到 `__ROOT__`；失败 IpcError code `E_FOLDER_DIRTY` → 用 `details.now` 重弹 modal「内容已变化（原 N，现 details.now），请重新确认？」用户重新点确认时 `expected_count` 用 `details.now`；其他 IpcError → toast `renderIpcError`。
5. **AC-5 组件单测**：`__tests__/WorkspaceFolderListView.test.tsx` 追加（或新建分文件）：
   - F1 Enter 提交 / Esc 取消 / blur 提交 三态
   - F1 失败保留编辑态 + 红框（mock `createWorkspaceFolder` reject `IpcError { code: 'E_NAME_DUP' }`）
   - F1 切走二次确认 modal（编辑期间点击其他行 → modal 出现）
   - F2 同步乐观（UI 立即变名，IPC 完成前）
   - F2 失败回滚（mock reject → 名称回到旧值 + selection 回到旧 path）
   - selection 冻结：`pendingRenameIds.has("x")` 时点击其他行无效
   - F3 二次确认含 "包含 N 个素材"（N 来自前端聚合，>0 与 ==0 两个分支）
   - F3 dirty 重弹（mock reject `IpcError { code: 'E_FOLDER_DIRTY', details: { old: 2, now: 5 } }` → modal 重弹含 "5"）
6. **AC-6 `pnpm test WorkspaceFolderListView` 全绿；`tsc --noEmit` 通过**。

## 技术约束
- ADR-007 / 底线 1：所有 handler 首行 `if (selection.kind !== 'root') return;`，不依赖 UI disable。
- ADR-008 / 底线 9：前端 `validateFolderNameSync` 仅作即时反馈；后端 `validate_folder_name` 是最终权威。
- ADR-009：5 字段 setter 必须通过 uiStore 暴露的 `startCreating` / `cancelCreating` / `startRenaming` / `finishRename`；不要绕过直接 set。
- ADR-010：F3 `expected_count` 取前端聚合（O(N) 一次），dirty 用 `details.now` 重弹；不要每行一次后端 invoke。
- 底线 6：UI 永远操作 `relativePath` 字符串（含 `__ROOT__`）；不要试图把 `__ROOT__` 翻译成空字符串发 IPC（后端 `resolve_relative_path` 单点负责）。
- 不顺手改无关代码；不在本 task 实现拖拽（留 T6）；不持久化 5 字段。

## 参考文件
- `sessions/conductor/tasks/task_001_architect/output.md` ADR-007 / 008 / 009 / 010
- `sessions/workspace_folder_mgmt/debate/session_001/debate_conclusions.md` §4（交互状态机）
- `product/prd/workspace_folder_mgmt_prd_v1.md` §3 P0 F1 / F2 / F3 行为表
- 既有代码：
  - T5a 三个子组件 `FolderListRow.tsx` / `FolderListToolbar.tsx` / `FolderContextMenu.tsx`
  - `NCdesktop/src/lib/folder-name-validate.ts`（即时校验）
  - `NCdesktop/src/lib/tauri-commands.ts`（`createWorkspaceFolder` / `renameWorkspaceFolder` / `deleteWorkspaceFolder`）
  - `NCdesktop/src/lib/ipc-errors.ts`（`renderIpcError` / `errorMessages`）
  - `NCdesktop/src/stores/uiStore.ts`（5 字段 + setter）

## 预估影响范围
- 新建文件：可选 — `WorkspaceFolderListView/InlineNameEditor.tsx`（封装受控输入框 + Enter/Esc/blur）；可选 — `WorkspaceFolderListView/DeleteConfirmModal.tsx`
- 修改文件：
  - `NCdesktop/src/components/features/WorkspaceFolderListView.tsx`（接入状态机 + handler）
  - `NCdesktop/src/components/features/WorkspaceFolderListView/FolderListRow.tsx`（条件渲染输入框 vs 文字）
  - `NCdesktop/src/components/features/WorkspaceFolderListView/FolderListToolbar.tsx`（接入 handler）
  - `NCdesktop/src/components/features/WorkspaceFolderListView/FolderContextMenu.tsx`（接入 handler）
  - `NCdesktop/src/components/features/__tests__/WorkspaceFolderListView.test.tsx`（追加用例）
