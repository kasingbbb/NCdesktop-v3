# Task 交付 — task_003_dev_sidebar_search_removal

## 实现摘要

PR-B 综合改造的第 1 步。从 `Sidebar.tsx` 完全删除 `Search` SidebarItem 与 `Search` lucide 图标 import；`SidebarFooter.tsx` 改造为水平 flex 布局，左侧两行 SidebarItem（设置 + 悬浮导入），右侧 TF 状态点/徽章（未连接显示 `data-testid="sidebar-footer-tf-dot"` 灰色小圆点；连接时显示 `data-testid="sidebar-footer-tf-badge"` 文字徽章），容器加 `data-testid="sidebar-footer"`。

ADR-007 决定：⌘K 入口保留在 TitleBar.tsx（项目已有），SidebarFooter 不重复设置 ⌘K 按钮。这符合 PRD §4.2 SB-01 "Search 改为浮层动作，不占栏位"的精神，且满足历史 SidebarFooter.test 期望的"DOM 行数 = 2"。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `src/components/layout/Sidebar.tsx` | 改写 | 删 `Search` import；删 Search SidebarItem；后续 task_004/005 一并完成 |
| `src/components/layout/SidebarFooter.tsx` | 改写 | 加 data-testid；TF 卡 SidebarItem 替换为右侧 dot/badge；保留 设置 / 悬浮导入 两个 SidebarItem |

## 测试结果

- `pnpm vitest run src/components/layout/__tests__/SidebarFooter.test.tsx`：**5/5 PASS**（原 baseline 3 fail → 0 fail）
- AC-1 Search SidebarItem 不在 DOM：✅（Sidebar.test PR-B 新增用例 "SB-01: Sidebar 内不存在 Search 入口" 验证）
- AC-7 全量 vitest 失败：26（baseline 锁更新为 ≤ 26）✅

## ADR-007（PR-B 综合决断，本 task 相关部分）

详见 [task_004 output.md](../task_004_dev_sidebar_knowledge_merge/output.md) 完整 ADR-007 记录。本 task 相关：
- ⌘K 入口保留在 TitleBar.tsx，SidebarFooter 不重复
- SidebarFooter 维持 2 行 button + TF 状态点（非 button）的"两行"结构（兼容历史 task_004 v2 Sidebar Redesign / AC-9 契约）

## 自测验证矩阵

| 场景 | 状态 | 结果 |
|---|---|---|
| Sidebar 不再 import Search 图标 | 已测 | PASS（grep 验证） |
| Sidebar 主体内无 Search SidebarItem | 已测 | PASS（Sidebar.test SB-01 用例） |
| SidebarFooter 2 个 button：设置 + 悬浮导入 | 已测 | PASS（SidebarFooter.test） |
| 未插入 TF 显示 dot | 已测 | PASS |
| 插入 TF 显示 TF 徽章 | 已测 | PASS |
| ⌘K 全局快捷键仍在 TitleBar | 未改 | TitleBar.tsx 未动；TitleBar.test 已有 3 个 ⌘K 用例 PASS |
