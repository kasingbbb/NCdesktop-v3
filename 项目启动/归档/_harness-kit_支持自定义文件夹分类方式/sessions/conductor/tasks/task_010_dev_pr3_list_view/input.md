# Task 输入 — task_010_dev_pr3_list_view

## 目标
F9 `FolderListView`（react-virtuoso）+ `FolderIconView` v1（mime → 内置 SVG）；列：图标 / 名称 / 分类 / 标签 / 大小 / 修改时间，按 mtime 倒序。

## 前置条件
- 依赖 task：task_008
- 必须先存在的文件/接口：`list_workspace_assets`、`assetStore`

## 验收标准（AC）
1. `FolderListView`：虚拟滚动（react-virtuoso）；1k 文件首屏 < 300ms（性能 AC）
2. 列宽可调（持久化到 `settings`）；列序固定 v1
3. `FolderIconView`：网格布局；mime → 图标映射（PDF/图/视/音/Office/文本/未知 共 7 类）
4. 视图切换按钮（list ⇄ icon）持久化用户偏好
5. 双击文件：列表选中并预览（v1 沿用现有预览面板）
6. 双击目录：进入子目录（与 task_011 面包屑联动）
7. 空目录态由 task_011 接管

## 技术约束
- 虚拟滚动 itemHeight 自适应（list 行 v1 固定 36px）
- 图标视图无缩略图（v2 F18）

## 参考文件
- task_001 output.md §目录结构
- 现有 `assetStore`、预览面板组件
- `package.json` 中 react-virtuoso 版本

## 预估影响范围
- 新建：`FolderListView.tsx`（~250）、`FolderIconView.tsx`（~150）、`mimeIconMap.ts`（~50）
- 修改：`WorkspaceLayout.tsx` 接入

## Reviewer 重点关注
- 1k 文件首屏性能实测（CPU & memory）
- 虚拟滚动的滚动还原（路由切回保持位置）
- 图标视图 grid 在窄屏的换行
