# Task 输入 — task_009_dev_pr3_sidebar

## 目标
F8 `WorkspaceCategorySidebar`（基于 `WorkspaceFolderStrip` 升级为纵向）；新建 `categoryStore`；与 `WorkspaceLayout` 配合走 feature flag `workspace_view_v2` 双栈共存（ADR-005）。

## 前置条件
- 依赖 task：task_008（命令就绪）+ task_012 的 `list_categories` **优化建议**：list_categories 提前到 PR-3 起始；本 task 与 task_012 共享同一 store，但 task_012 完整 CRUD 后做
- 必须先存在的文件/接口：`list_workspace_assets`、`list_categories` 命令骨架

## 验收标准（AC）
1. 新建 `src/stores/categoryStore.ts`：`categories`、`activeCategorySlug`、`fetch()`、`setActive(slug)`；副作用集中 action
2. 新建 `src/components/features/WorkspaceCategorySidebar.tsx`：纵向列表，每项显示 icon + label + 计数（从 list_workspace_assets 概要返回）
3. 已停用分类折叠分组（默认收起）
4. 内置 PARA 项标识 builtin badge
5. `WorkspaceLayout` 按 `featureFlags.workspace_view_v2` 路由：on→Sidebar，off→Strip
6. 老 Strip 不删，保留至 flag 全量后清理
7. 视觉与现有"紫色毛玻璃"基线一致

## 技术约束
- TS 严格模式
- Zustand 副作用集中 action
- 复用 ~60% Strip 逻辑（取数函数提取至 store）

## 参考文件
- `src/components/features/WorkspaceFolderStrip.tsx`
- `src/stores/uiStore.ts`（featureFlags）
- task_001 output.md ADR-005

## 预估影响范围
- 新建：`categoryStore.ts`（~120）、`WorkspaceCategorySidebar.tsx`（~200）
- 修改：`WorkspaceLayout.tsx`、`uiStore.ts`（featureFlags）

## Reviewer 重点关注
- flag 切换时的状态清理
- 既有 Strip 路径回归测试
- 计数数据源与 list 命令同步
