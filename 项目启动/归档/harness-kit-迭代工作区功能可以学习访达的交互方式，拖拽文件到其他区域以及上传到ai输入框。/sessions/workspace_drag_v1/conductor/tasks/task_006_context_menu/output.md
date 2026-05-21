# Task Output — task_006_context_menu

## 实现摘要

新建了 `AssetContextMenu.tsx` 右键菜单组件，并将其集成到 `AssetListView.tsx` 中。实现了左右栏卡片（列表视图和网格视图）的右键菜单，含"移到文件夹 ▶"二级子菜单、"在 Finder 中显示"和"删除"三项操作，支持对选中集合的批量移动。

## 修改/新建的文件表格

| 操作 | 文件路径 | 变更摘要 |
|------|----------|----------|
| 新建 | `src/components/features/AssetContextMenu.tsx` | 右键菜单组件，含二级子菜单、Finder 显示、删除确认 |
| 修改 | `src/components/features/AssetListView.tsx` | 添加 import、contextMenu state、4 处 onContextMenu handler、底部渲染 AssetContextMenu |

## 架构遵守声明

- 未修改任何 store（未新增 slice 或 action）
- 未引入新的 CSS 框架或第三方组件库
- 删除操作使用 `window.confirm` 确认对话框
- 错误处理使用 `console.error` 占位（无现有 Toast 系统可直接调用）
- 组件使用 `position: fixed`，`z-index: 1000/1001`
- 样式使用 CSS Variables：`var(--bg-primary)`、`var(--border-primary)`、`var(--text-primary)`、`var(--surface-secondary)` 等
- 通过 `useEffect` 监听 `mousedown` 事件（点击外部关闭）和 `keydown` 事件（Esc 关闭）

## tsc 输出

```
$ npx tsc --noEmit
（无输出，无错误）
```

退出码：0，无新增 TypeScript 错误。

## 自测验证矩阵

| 验收标准 | 满足状态 | 说明 |
|----------|----------|------|
| AC-1：`AssetContextMenu.tsx` 文件存在且导出组件 | ✅ 满足 | 文件已创建，`export function AssetContextMenu` |
| AC-2：左右栏卡片均有 `onContextMenu` handler | ✅ 满足 | 左栏列表/网格、右栏列表/网格共 4 处均已添加 |
| AC-3："移到文件夹 ▶" hover 后展开二级子菜单 | ✅ 满足 | hover 触发 `setSubMenuOpen(true)`，展示 allFolders 列表（含根目录"根目录 /"） |
| AC-4：点击子文件夹后调用 `moveAssetToWorkspaceFolder`，触发 `onMoved` | ✅ 满足 | `handleMoveToFolder` 调用命令后依次执行 `onMoved()` 和 `onClose()` |
| AC-5：点击菜单外区域可关闭菜单 | ✅ 满足 | `useEffect` 监听 `mousedown`，点击外部调用 `onClose()` |
| AC-6：`npx tsc --noEmit` 无新增错误 | ✅ 满足 | 编译通过，无输出 |

## 补充说明

- `targetIds` 逻辑：若右键的资产在 `selectedAssetIds` 中，则操作整个选中集合；否则只操作单个资产（AC-5 规格）。
- 当前文件夹的灰显通过 `isCurrent(folder)` 函数判断，通过分析 `currentFilePath` 是否包含对应 `relativePath` 路径片段实现。
- 根目录项（`relativePath: "__ROOT__"`）总是排在子菜单第一项。
- 子菜单位置会根据视口边界自动调整（左侧或右侧弹出）。
