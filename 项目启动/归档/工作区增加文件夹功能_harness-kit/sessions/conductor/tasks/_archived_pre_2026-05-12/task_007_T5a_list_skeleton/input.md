# Task 输入 — task_007_T5a_list_skeleton

## 目标
新建 `WorkspaceFolderListView` 列表骨架（列头 + 数据行 + 选中态 + hover）、工具栏（3 按钮）与右键菜单（3 kind 形态），所有键盘 / 工具栏 / 菜单 handler 入口统一做 `kind === 'root'` 判定，**不接 inline 编辑、不接拖拽**（留 T5b / T6）。

## 前置条件
- 依赖 task：task_004_T2_frontend_ipc（DONE）
- 必须先存在的文件/接口：
  - `tauri-commands.ts` 5 个 wrapper（T2 产出）
  - `ipc-errors.ts` 文案表（T2 产出）
  - 既有 `WorkspaceFolderStrip` 与 `AssetListView` 集成方式

## 验收标准（Acceptance Criteria）
1. **AC-1**：`WorkspaceFolderListView` 渲染列表 = 列头一行（浅灰 + 1px 底线 + 13px 半粗，三列：名称 / 项目数 / 修改时间）+ 数据行（行高 ~24px，无斑马纹、无行分隔、无阴影；列宽固定）（PRD §3 列表视图）。
2. **AC-2**：每行图标 16px 文件夹；`ai_organized` 行右下叠 8px lucide `Sparkles` 角标（PRD §3）。
3. **AC-3**：选中态 = `var(--border-active)` 背景 + 文字反白；hover = `rgba(0,0,0,0.04)` / 深色 `rgba(255,255,255,0.06)`（PRD §3）。
4. **AC-4**：工具栏 36px 三按钮：`+ 新建文件夹`（永激活）、`重命名`（仅 `selection.kind === 'root'` 激活）、`移到废纸篓`（仅 `selection.kind === 'root'` 激活）。**面包屑 / 列显隐 / 视图切换 不做**（PRD §3 工具栏）。
5. **AC-5**：右键菜单 3 形态：
    - `root`：重命名 / 移到废纸篓 / — / 在 Finder 中显示
    - `ai_organized`：仅"在 Finder 中显示"可点 + 其余项灰显 + tooltip "AI 归类目录受保护"
    - `__ROOT__`：仅"在 Finder 中显示"（重命名/删除**不显示，非灰显**）
    - 空白处：新建文件夹
    （PRD §3 右键菜单）
6. **AC-6**：键盘 handler：`Enter` 触发 rename 入口（kind=root）；`⌘⌫` 触发 delete 入口（kind=root）；**每个 handler 首行 `if (selection.kind !== 'root') return;`**，不依赖 UI disabled（PRD §4.1.3 + ADR-007）。
7. **AC-7**：双击 root / __ROOT__ / ai_organized 行触发 `setWorkspaceFolderRelativePath`（筛选切换，与既有行为一致）。
8. **AC-8**：组件单测 `pnpm test WorkspaceFolderListView`：
    - 列表渲染含 3 kind 行（mock data）
    - 工具栏激活规则正确
    - 右键菜单 3 kind 形态正确（`__ROOT__` 隐藏 rename/delete 项）
    - `⌘⌫` 在选中 `ai_organized` 时无任何 invoke（验证 handler 入口判定）
    - 双击行调用 `setWorkspaceFolderRelativePath`
9. **AC-9**：`AssetListView` 中 `WorkspaceFolderStrip` 替换为 `WorkspaceFolderListView`；旧组件文件 `WorkspaceFolderStrip.tsx` 删除；`pnpm tsc --noEmit` PASS。

## 技术约束
- React 19 函数组件 + Zustand；不新增 store（PRD §5）。
- 组件路径 `src/components/features/`；子组件可放 `WorkspaceFolderListView/` 子目录。
- 项目数列：用前端 `assets` 聚合（`firstSegment` 算法；ADR-010），不每行 invoke `countFolderAssets`。删除 modal 取数才调 `countFolderAssets`（留给 T5b）。
- 修改时间列：从 `fs::metadata` 取不到（这里用 `WorkspaceFolderEntry` 不带 mtime），**本 task 先空白渲染 `--` 占位**，并在 ouput.md 中记录待 T5b/T6 决定是否扩 list 命令字段（**也可决定本期就空着**，PRD §3 未硬性要求 mtime 真实显示）。
- 不接 inline 编辑（留 T5b）。
- 不接拖拽（留 T6）。
- handler 入口判定**不依赖 UI disabled**（ADR-007 / 底线 1）。
- 文案中文（无 i18n）。
- 不顺手改无关代码；commit 中文 Conventional。

## 参考文件
- 既有：`NCdesktop/src/components/features/AssetListView.tsx:453-456`（旧 strip 挂载位置）、`NCdesktop/src/components/features/WorkspaceFolderStrip.tsx`（参考 props 形态，本 task 删除文件）、`NCdesktop/src/lib/workspace-folder-badges.ts`（如存在，沿用 kind badge 样式）
- 契约：`sessions/conductor/tasks/task_002_T0_contracts/contracts.md`
- 方案：output.md ADR-007/009/011（仅判定 + state，拖拽具体实现留 T6）、§目录结构、§系统架构

## 预估影响范围
- 新建文件：
  - `NCdesktop/src/components/features/WorkspaceFolderListView.tsx`
  - `NCdesktop/src/components/features/WorkspaceFolderListView/FolderListRow.tsx`
  - `NCdesktop/src/components/features/WorkspaceFolderListView/FolderListToolbar.tsx`
  - `NCdesktop/src/components/features/WorkspaceFolderListView/FolderContextMenu.tsx`
  - `NCdesktop/src/components/features/__tests__/WorkspaceFolderListView.test.tsx`
- 修改文件：
  - `NCdesktop/src/components/features/AssetListView.tsx`（替换 strip 为 listview；删除旧 import）
- 删除文件：
  - `NCdesktop/src/components/features/WorkspaceFolderStrip.tsx`
