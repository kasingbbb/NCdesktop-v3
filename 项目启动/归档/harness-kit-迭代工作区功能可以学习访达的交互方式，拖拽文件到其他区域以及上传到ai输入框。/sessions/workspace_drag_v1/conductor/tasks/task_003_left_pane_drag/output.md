# Task Output — task_003_left_pane_drag

**Status: DONE**

## 实现摘要

在 `AssetListView.tsx` 左栏 rawAssets 的两处卡片元素上添加了 `{...makeDragProps(a.id)}`，使左栏原件支持拖拽到 macOS Finder 或其他外部应用。改动共 2 行（每处各 1 行 spread），无新增 import 或 hook 调用。

## 修改的文件

| 文件 | 改动位置 | 改动内容 |
|------|----------|----------|
| `src/components/features/AssetListView.tsx` | L476（list viewMode 左栏卡片 `<div>`） | 添加 `{...makeDragProps(a.id)}` |
| `src/components/features/AssetListView.tsx` | L526（grid viewMode 左栏卡片 `<button>`） | 添加 `{...makeDragProps(a.id)}` |

## 架构遵守声明

- `useDragAssets.ts` **未被修改**（零改动）
- 未引入任何新 import 语句；`makeDragProps` 来自文件顶部已有的 `const { makeDragProps } = useDragAssets(selectedAssetIds, assets)`
- 未修改任何 store 或 Rust 文件
- 改动形式与右栏已有的 `{...makeDragProps(a.id)}`（L638）完全一致

## 自测验证矩阵

| 验收标准 | 满足？ | 说明 |
|----------|--------|------|
| AC-1: 左栏 list viewMode 卡片有 `{...makeDragProps(a.id)}` | YES | L476 已添加 |
| AC-2: 左栏 grid viewMode 卡片有 `{...makeDragProps(a.id)}` | YES | L526 已添加 |
| AC-3: `useDragAssets.ts` 未被修改 | YES | 文件未触碰 |
| AC-4: 没有引入新的 import 语句 | YES | makeDragProps 已通过现有 hook 可用 |
