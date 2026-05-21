# Task 输入 — task_003_left_pane_drag

## 目标

在 `AssetListView.tsx` 的左栏 rawAssets 卡片（list viewMode 和 grid viewMode 两处）上挂载 `{...makeDragProps(a.id)}`，使原件可以被拖拽到 macOS Finder 或其他外部应用。

## 前置条件

- 依赖 task：**task_002 必须 DONE**（`draggable: true` 已移除，否则 startDrag 仍无效）
- 必须先存在的文件/接口：
  - `src/components/features/AssetListView.tsx`
  - `src/hooks/useDragAssets.ts`（`makeDragProps` 接口不变）

## 验收标准（Acceptance Criteria）

1. **AC-1**：左栏 list 模式下拖拽一个 PDF 原件到桌面，桌面生成该 PDF 的物理副本。
2. **AC-2**：左栏 grid 模式下拖拽一个 PDF 原件到桌面，同样成功复制。
3. **AC-3**：拖拽左栏原件时，**不触发**其关联的 Markdown 转化文件的任何操作（只有原件 filePath 被传入 `startDrag`）。
4. **AC-4**：左栏 Cmd+Click 多选 3 个原件后拖拽，3 个文件均被复制到目标位置。

## 技术约束

- `AssetListView` 顶部已调用 `const { makeDragProps } = useDragAssets(selectedAssetIds, assets)`，**不需要新增 hook 调用**，直接使用现有的 `makeDragProps`。
- `useDragAssets` 的 `assets` 参数传入的是全量 `assets`，其中包含 rawAssets，`resolveFilePaths` 可正确解析其 `filePath`，**不需要修改 hook**。
- list viewMode 改动位置：`AssetListView.tsx` 约 L460 的左栏 list 卡片元素。
- grid viewMode 改动位置：`AssetListView.tsx` 约 L497 的左栏 grid 卡片元素。
- 两处卡片元素均添加 `{...makeDragProps(a.id)}`，形式与右栏一致。
- 不修改 `useDragAssets.ts` 或任何 store/Rust 代码。

## 参考文件

- `src/components/features/AssetListView.tsx`（L460 list 左栏卡片，L497 grid 左栏卡片）
- `src/hooks/useDragAssets.ts`（`makeDragProps` 接口确认）
- Architect output.md §2.2

## 预估影响范围

- 新建文件：无
- 修改文件：`src/components/features/AssetListView.tsx`（2 处 spread 添加，约 2 行改动）
