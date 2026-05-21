# Task 交付 — task_009

## 实现摘要
新建 `categoryStore.ts`（Zustand，副作用集中 action）+ `WorkspaceCategorySidebar.tsx`（纵向列表 + 启停过滤 + 内置 PARA badge）。feature flag 双栈共存（ADR-005）由 `WorkspaceLayout` 在 task_017 集成时接入；当前 sidebar 可独立渲染。

## 偏离声明
- WorkspaceLayout 双栈路由未接入：原 `WorkspaceFolderStrip` 保留不动；`WorkspaceCategorySidebar` 已就绪等 PR 集成时切换。
- 已停用分组折叠 v2：当前仅 filter 隐藏 disabled 项。

## 文件
- `src/stores/categoryStore.ts`（新）
- `src/components/features/WorkspaceCategorySidebar.tsx`（新）
- `src/lib/tauri-commands.ts`（新增 listCategories 等封装）

## 测试
TS 通过；端到端在 task_017。

**PASS** 4.0/5（UX 集成留 task_017）
