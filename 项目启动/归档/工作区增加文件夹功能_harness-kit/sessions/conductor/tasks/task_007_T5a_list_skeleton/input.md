# Task 输入 — task_007_T5a_list_skeleton

## 目标
新建 `WorkspaceFolderListView` 组件骨架替换既有 `WorkspaceFolderStrip`：表格式列表渲染（列：名称 / 项目数 / 修改时间）+ 选中态 + 工具栏 3 按钮（新建 / 重命名 / 移到废纸篓）+ 右键菜单（root / ai_organized / __ROOT__ 三 kind 形态）+ 键盘 handler 入口判定（Enter / ⌘⌫ 仅 root 触发）；本 task **不**实现 inline 编辑与拖拽（留 T5b / T6）。

## 前置条件
- 依赖 task：task_004_T2_frontend_ipc（5 wrapper + `errorMessages` + 类型已就位）
- 必须先存在的文件/接口：
  - T2 全部产出（`tauri-commands.ts` 5 wrapper、`ipc-errors.ts`、`folder-name-validate.ts`、`src/types/workspace.ts` 类型完整）
  - `NCdesktop/src/stores/uiStore.ts` 已含 `workspaceFolderRelativePath`（T4 5 字段可不依赖；本 task 仅消费 selection 字段）
  - `NCdesktop/src/components/features/AssetListView.tsx`（既有 `workspaceFolders` 状态、`listProjectWorkspaceFolders` 加载）
  - `NCdesktop/src/components/features/WorkspaceFolderStrip.tsx`（待替换）

## 验收标准（Acceptance Criteria）

1. **AC-1 组件骨架**：新建 `src/components/features/WorkspaceFolderListView.tsx`（主入口）+ 子组件目录 `src/components/features/WorkspaceFolderListView/`：
   - `FolderListRow.tsx`：单行渲染（图标 + 名称 + 项目数 + 修改时间；`ai_organized` 行图标右下 `Sparkles` 8px 角标；选中行 `var(--border-active)` 背景 + 文字反白；hover `rgba(0,0,0,0.04)` / 深色 `rgba(255,255,255,0.06)`；行高 ~24px；无斑马纹/无分隔线/无阴影）
   - `FolderListToolbar.tsx`：36px 高 3 按钮（`+ 新建文件夹` 永激活、`重命名` 仅 `kind === 'root'` 激活、`移到废纸篓` 仅 `kind === 'root'` 激活）
   - `FolderContextMenu.tsx`：3 kind 形态 — `root`：重命名 / 移到废纸篓 / 在 Finder 中显示；`ai_organized`：仅"在 Finder 中显示"+ 其余项灰显 + tooltip "AI 归类目录受保护"；`__ROOT__`：仅"在 Finder 中显示"（**重命名/删除不显示**，非灰显）；空白处：新建文件夹
2. **AC-2 列表头与数据行**：列头 13px 半粗 + 浅灰背景 + 1px 底线；3 列分别为「名称（弹性宽 + 16px 文件夹图标）」「项目数（右对齐）」「修改时间 `MM/DD HH:mm`（右对齐）」；列宽固定（本期不做拖宽 - PRD §3 P2 明示）。
3. **AC-3 项目数前端聚合**：从 `assetStore.assets` 按 `firstSegment(file_path)` 聚合（ADR-010）；`__ROOT__` 行 = 不含 `/` 的文件数；其余行 = `firstSegment === relativePath` 的文件数。算法必须与后端 `count_folder_assets` 等价（T6 集成验证）。
4. **AC-4 选中态**：单击行触发 `setWorkspaceFolderRelativePath(row.relativePath)`；双击行触发筛选切换（PRD §3 + verifications 4）。键盘 Up/Down 可导航选中行（无 a11y 要求，仅基本键盘可用）。
5. **AC-5 handler 入口统一权限判定**（ADR-007 / 底线 1）：所有写动作 handler（重命名按钮、移到废纸篓按钮、Enter 键、⌘⌫ 键、右键菜单项）首行：
   ```ts
   if (selection.kind !== 'root') return; // 不依赖 UI disabled
   ```
   `⌘⌫` 键盘事件绑定在列表区域 keydown；`Enter` 键当选中行 kind === 'root' 时**仅准备进入编辑**（实际进入留 T5b，本 task 占位即可，handler 已存在且通过权限判定）。
6. **AC-6 替换 `WorkspaceFolderStrip`**：在 `AssetListView.tsx` 中用 `WorkspaceFolderListView` 替换既有 `WorkspaceFolderStrip` import 与 JSX 位置；删除 `src/components/features/WorkspaceFolderStrip.tsx` 文件。保留既有 `workspaceFolders` 状态加载逻辑（`listProjectWorkspaceFolders`）。
7. **AC-7 组件单测**：`src/components/features/__tests__/WorkspaceFolderListView.test.tsx` 覆盖：
   - 列表渲染 3 类 kind 行
   - 工具栏「重命名」「移到废纸篓」按钮在选中 `root` 行时激活，选中 `ai_organized` / `__ROOT__` 时 disabled
   - 右键 `root` 行：菜单显示 重命名 / 移到废纸篓 / 在 Finder 中显示
   - 右键 `ai_organized` 行：仅"在 Finder 中显示"可点，其余灰显（且尝试点击灰显项不触发 handler）
   - 右键 `__ROOT__` 行：仅"在 Finder 中显示"显示（不含重命名 / 删除条目）
   - direct invoke 防御：模拟 `⌘⌫` 在选中 `ai_organized` 时不触发 `deleteWorkspaceFolder` wrapper（handler 首行返回）
   - 双击行触发 `setWorkspaceFolderRelativePath`
8. **AC-8 `pnpm test WorkspaceFolderListView` 全绿；`tsc --noEmit` 通过**。

## 技术约束
- session_context §5：React 19 函数组件、Zustand；新组件放 `src/components/features/`；**不**新增 store。
- 底线 1：ai_organized 前端 disable；handler 入口判定不依赖 disable。
- 底线 10：错误（如 reveal 失败）用 `errorMessages[code]` 渲染；不直接展示后端 `message`。
- ADR-007：所有 handler 首行 `if (selection.kind !== 'root') return;`；权限判定与 UI disable 完全解耦。
- ADR-011：本 task **不**做拖拽（留 T6）；行根 `draggable={false}`。
- 视觉规范：选中行 `var(--border-active)` 背景 + 反白；hover 4%/6% alpha；ai_organized 角标 `Sparkles` 8px；行高 ~24px；列宽固定；**不做**像素级 Finder 复刻、列宽拖、列排序（PRD §3 P2）。
- 不顺手改无关代码；删除 `WorkspaceFolderStrip.tsx` 后确保无其他 import 残留。

## 参考文件
- `sessions/conductor/tasks/task_001_architect/output.md` ADR-007 / 009 / 010 / 011；§系统架构 Frontend 树
- `sessions/conductor/tasks/task_002_T0_contracts/contracts.md` §B（命令签名）/ §D（文案）
- 既有代码：
  - `NCdesktop/src/components/features/WorkspaceFolderStrip.tsx`（参照既有 props 与样式 token；将被替换）
  - `NCdesktop/src/components/features/AssetListView.tsx`（替换点 + workspaceFolders 加载逻辑）
  - `NCdesktop/src/components/features/AssetContextMenu.tsx`（右键菜单 UI 风格参考）
  - `NCdesktop/src/lib/tauri-commands.ts`（5 wrapper、`revealProjectWorkspaceFolder` 既有）
  - `NCdesktop/src/lib/ipc-errors.ts`（`renderIpcError` / `errorMessages`）
  - `NCdesktop/src/stores/uiStore.ts`（`workspaceFolderRelativePath` selector）

## 预估影响范围
- 新建文件：
  - `NCdesktop/src/components/features/WorkspaceFolderListView.tsx`
  - `NCdesktop/src/components/features/WorkspaceFolderListView/FolderListRow.tsx`
  - `NCdesktop/src/components/features/WorkspaceFolderListView/FolderListToolbar.tsx`
  - `NCdesktop/src/components/features/WorkspaceFolderListView/FolderContextMenu.tsx`
  - `NCdesktop/src/components/features/__tests__/WorkspaceFolderListView.test.tsx`
- 修改文件：
  - `NCdesktop/src/components/features/AssetListView.tsx`（替换 `WorkspaceFolderStrip` 使用点）
- 删除文件：
  - `NCdesktop/src/components/features/WorkspaceFolderStrip.tsx`
