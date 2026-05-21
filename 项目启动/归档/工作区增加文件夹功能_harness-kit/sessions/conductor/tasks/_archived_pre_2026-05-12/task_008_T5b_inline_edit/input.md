# Task 输入 — task_008_T5b_inline_edit

## 目标
在 T5a 列表骨架上实现 inline 编辑状态机 `mode: 'idle' | 'creating' | 'renaming'`：F1 幽灵行 / F2 重命名三态（Enter/Esc/blur）/ 同步乐观 + `pendingRenameIds` selection 冻结 / 失败保留编辑态 + 切走二次确认 modal / 删除二次确认 + `E_FOLDER_DIRTY` 重弹。

## 前置条件
- 依赖 task：task_007_T5a_list_skeleton（DONE）
- 必须先存在的文件/接口：
  - `WorkspaceFolderListView` 骨架 + 工具栏 + 右键菜单 + 键盘 handler 入口
  - `uiStore` 5 字段 setter（来自 T4，可能并行落地；本 task 启动前需 T4 的 store 字段 PR 已合）
  - `createWorkspaceFolder` / `renameWorkspaceFolder` / `deleteWorkspaceFolder` / `countFolderAssets` 5 wrapper
  - `folder-name-validate.ts` 前端即时校验函数（本 task 一并新增）

## 验收标准（Acceptance Criteria）
1. **AC-1 F2 三态 Enter/Esc/blur**：`pnpm test WorkspaceFolderListView` 中 inline 用例覆盖：(a) Enter 提交 → invoke `renameWorkspaceFolder`，乐观替换名称；(b) Esc → 不发 IPC，回到 idle；(c) blur 同 Enter 语义（同步乐观提交）。
2. **AC-2 F2 selection 冻结**：rename pending 时点击其他行无响应；`pendingRenameIds` 非空时 selection setter 拒绝；单测验证 IPC resolve 前其他行点击 noop。
3. **AC-3 F2 失败保留**：IPC 返 `E_NAME_DUP` / `E_NAME_INVALID` 时 → 红框 + 行内 inline error 显示 `errorMessages[code](details)`，**保留编辑态**，selection 自动回该行（PRD §3 F2 / Debate §4）。
4. **AC-4 F1 幽灵行**：工具栏 `+ 新建文件夹` 触发 → 列表末尾插入空白可编辑行（默认 `未命名文件夹` 全选）→ Enter 提交 invoke `createWorkspaceFolder`；成功后该行选中 + `pendingNewFolder=false`；失败保留编辑态 + 红框（PRD §3 F1）。
5. **AC-5 F1 切走二次确认**：幽灵行存在时点击其他行 / 切换 listview 外的视图 → 弹 modal「放弃新建『xxx』？」，确认放弃 = 丢弃幽灵行，取消 = 维持编辑态；单测覆盖（PRD §3 F1 / Debate §4）。
6. **AC-6 F3 二次确认**：触发 delete → 调 `countFolderAssets` 获取 N → 弹确认 modal，文案 `"该文件夹包含 N 个素材，一同移到废纸篓？"`（N=0 时文案为「确认移到废纸篓？」也可）；确认后 invoke `deleteWorkspaceFolder(_, _, true, N)`；若返 `E_FOLDER_DIRTY{old, now}` → 用 `now` 重新弹 modal 一次（PRD §3 F3 / §4.2.3 / Debate §5）。
7. **AC-7 ai_organized 灰显**：选中 `ai_organized` 时工具栏 rename / 移到废纸篓 不激活（继承 T5a），且**所有键盘 / 菜单 handler 首行 `if (selection.kind !== 'root') return;` 不依赖 UI**（再次验证 ADR-007）。
8. **AC-8 IpcError 文案**：所有错误用 `errorMessages[code](details)` 渲染；后端 `message` 不展示；单测覆盖 `E_FOLDER_DIRTY` 用 `details.now` 渲染。
9. **AC-9 前端即时校验**：`folder-name-validate.ts` 提供同步函数，Enter 时本地命中违规即不发 IPC + 红框 + tooltip；后端为最终权威。
10. **AC-10**：`pnpm test` 全绿（不引入新失败）。

## 技术约束
- 状态机 `mode` 用 `useReducer` 或 `useState` 实现，**不进 uiStore**（仅 listview 内部状态）；幽灵行/编辑路径/冻结集进 uiStore（PRD §5.2、ADR-009）。
- **同步乐观**：Enter/blur 立即在本地替换名称 + 把节点加入 `pendingRenameIds`；IPC resolve 后移出冻结集；失败把名字回退 + 显示 inline error（Debate §4）。
- **drop 编辑互斥**：本 task 不接 drop，但状态机必须暴露 `isEditing(row)` 给 T6 使用。
- **二次确认 modal**：使用既有 `activeModal` 机制（uiStore），不引入新 modal 框架。
- **`pendingNewFolder` 与 `editingFolderPath` 切走判定**：切走 = (a) 点击其他列表行；(b) 点击列表外区域；(c) 切换 sidebar；(d) F1 期间未输入任何字符则视为直接取消，不弹 confirm。
- **键盘**：组件容器 `tabIndex={-1}`；keydown 在容器级处理；与全局 `⌘⇧N`（P1，本期不做）不冲突。
- **handler 入口判定**：所有写动作 handler 首行 `if (selection.kind !== 'root') return;`（ADR-007 / 底线 1）。
- 不顺手改无关代码；commit 中文 Conventional。

## 参考文件
- 既有：T5a 产出（`WorkspaceFolderListView.tsx` + 子组件 + tests）、`NCdesktop/src/stores/uiStore.ts`（T4 已加 5 字段）
- 契约：`sessions/conductor/tasks/task_002_T0_contracts/contracts.md`（错误码 + 文案表）
- 方案：output.md ADR-007/008/009/010、§安全考量

## 预估影响范围
- 新建文件：
  - `NCdesktop/src/lib/folder-name-validate.ts`
  - `NCdesktop/src/lib/__tests__/folder-name-validate.test.ts`
- 修改文件：
  - `NCdesktop/src/components/features/WorkspaceFolderListView.tsx`（加状态机 + inline 编辑逻辑）
  - `NCdesktop/src/components/features/WorkspaceFolderListView/FolderListRow.tsx`（加 controlled input + Enter/Esc/blur）
  - `NCdesktop/src/components/features/WorkspaceFolderListView/FolderListToolbar.tsx`（F1 入口接幽灵行）
  - `NCdesktop/src/components/features/__tests__/WorkspaceFolderListView.test.tsx`（追加 inline / 幽灵行 / 二次确认 / dirty 重弹用例）
