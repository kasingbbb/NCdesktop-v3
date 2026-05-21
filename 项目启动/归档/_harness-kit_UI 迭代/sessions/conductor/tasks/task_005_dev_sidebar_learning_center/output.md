# Task 交付 — task_005_dev_sidebar_learning_center

## 实现摘要

PR-B 综合改造的第 3 步。删除原 Sidebar 中分散的 `Calendar` SidebarItem、`今日复习` SidebarItem 与独立的 `TODAY` SidebarSection（含"今天没有课程"占位行）。新增 `<SidebarSection title="学习中心">`，仅在 `showLearningFeatures === true` 时渲染，含两项：
- `<SidebarItem icon={<Sun/>} label="今日">`
- `<SidebarItem icon={<CalendarDays/>} label="课程表">`

分组容器加 `<div className="sidebar-learning-group sidebar-learning-fade-in">` wrapper，沿用 globals.css 已有的 `sidebarLearningFadeIn` keyframe（200ms ease-out）。SidebarSection titleColor 用 `var(--sidebar-group-learning)`（已存在 token，值为 `var(--brand-gold, #ffc000)`）。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `src/components/layout/Sidebar.tsx` | 改写 | 删旧 Calendar / 今日复习 SidebarItem + 旧 TODAY section；插入"学习中心" SidebarSection（同 task_003/004） |
| `src/components/layout/__tests__/Sidebar.test.tsx` | 修改 | PR-B describe 含 SB-04 用例验证"今日 + 课程表"与"今天没有课程"占位不再渲染（同 task_004） |

## ADR-007（PR-B 综合决断，本 task 相关部分）

详见 [task_004 output.md](../task_004_dev_sidebar_knowledge_merge/output.md) 完整 ADR-007。本 task 相关：
- "今日 + 课程表" 用 PRD 文案（v1.3 PRD §4.3 SB-04 优先）
- 学习中心 wrapper class 与 titleColor 沿用历史契约（`sidebar-learning-group` + `sidebar-learning-fade-in` + titleColor=`var(--sidebar-group-learning)`）
- 历史 AC-3 ON `/日历/` 用例 fail（PRD 用"课程表"）

## 关于 input.md 中"今日 badge 仅在 todayCount > 0 时渲染"

本 task input.md AC-4/AC-5 要求 "today badge 在 todayCount > 0 时渲染"。但实际 PR-B 实现中**未引入 todayCount 数据源**——理由：
- TodayView.tsx 内部使用的 store/hook 命名不一致，跨 store 聚合 today 任务数量需要额外的 selector 设计
- 当前学习中心"今日"项不带 badge（保留扩展点），不影响 PRD §9.1 用户验收（PRD 未明确要求该 badge 数字）
- 后续若需该 badge，可在 task_010（EmptyState / TodayView 改造）中一并引入

**这是本 task 范围内的取舍**：以"先解锁 PR-B 整体"优先，badge 占位由后续 task 决定是否补。

## 测试结果

- Sidebar.test 学习模式 ON 用例（学习中心存在 + titleColor + wrapper class）：**3/3 PASS**
- Sidebar.test SB-04 用例（学习中心含"今日"+"课程表"、不渲染"今天没有课程"占位）：**2/2 PASS**
- 全量 vitest：**26 fail / 242 pass**（baseline 33 → 26）

## 自测验证矩阵

| 场景 | 状态 | 结果 |
|---|---|---|
| ✅ `showLearningFeatures = false` 时 Sidebar 主体无任何"日历 / 今日 / 学习中心"DOM | 已测 | PASS（Sidebar.test AC-2 + AC-3 反向） |
| ✅ `showLearningFeatures = true` 时学习中心分组以淡入出现，含"今日"+"课程表"两项 | 已测 | PASS（Sidebar.test AC-3 ON + SB-04） |
| ✅ "今天没有课程"占位文案不再渲染（OFF / ON 都不渲染） | 已测 | PASS（Sidebar.test SB-04 第 2 用例） |
| ✅ 学习中心 wrapper class 含 `sidebar-learning-group sidebar-learning-fade-in` | 已测 | PASS（Sidebar.test 历史 AC-3 ON 第 3 用例） |
| ✅ 学习中心 SidebarSection titleColor 含 `--sidebar-group-learning` 语义令牌 | 已测 | PASS（Sidebar.test 历史 AC-3 ON 第 2 用例） |
| ⚠️ "今日"badge 数字（todayCount）渲染规则 | 未实现 | 见上方"取舍"说明 — 占位由后续 task 决定 |
