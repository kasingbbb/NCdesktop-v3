# Task 输入 — task_005_dev_sidebar_learning_center

## 目标

将 Sidebar 学生态下分散的 `Calendar` SidebarItem、`今日复习` SidebarItem、独立的 `TODAY` SidebarSection（含"今天没有课程"占位行）合并为一个**"学习中心"** SidebarSection，位于"知识"入口下方、ProjectTree 之前。分组仅在 `showLearningFeatures === true` 时渲染，含两项：
- `今日`（仅 `todayCount > 0` 时渲染，badge 显示 todayCount）
- `课程表`（总是渲染）

分组容器带 200ms 淡入动效。

## 前置条件

- 依赖 task：**task_004**（避免合并冲突）
- 必须先存在的文件/接口：
  - `src/components/layout/Sidebar.tsx`（task_004 修改后状态）
  - `src/stores/settingsStore.ts`（提供 `useEffectiveLearningSettings`）
  - 今日任务数数据源（可复用 TodayView 内部使用的 store/hook）

## 验收标准（Acceptance Criteria）

1. **AC-1**：当 `showLearningFeatures === false`，Sidebar 主体内不存在任何"学习"/"课程"/"今日"相关 DOM
2. **AC-2**：当 `showLearningFeatures === true`，渲染 "学习中心" SidebarSection（title="学习中心"），位置在"知识"入口和 ProjectTree 之间
3. **AC-3**："学习中心"分组**不再渲染**"今天没有课程"占位文字（原 src/components/layout/Sidebar.tsx:117-126 的 div 已删除）
4. **AC-4**：当 `todayCount > 0`，分组内渲染"今日" SidebarItem，右侧 badge 显示 todayCount；点击触发 `setSidebarSection("today")`
5. **AC-5**：当 `todayCount === 0`，"今日" SidebarItem 不渲染
6. **AC-6**："课程表" SidebarItem 总是渲染（学生态下）；点击触发 `setSidebarSection("calendar")`
7. **AC-7**：分组容器拥有 class `sidebar-learning-fade-in`（globals.css 如无该 keyframe 则在 task_012 统一补；本 task 至少先加 class，让 task_012 补 CSS）
8. **AC-8**：单测覆盖：① showLearningFeatures false/true 切换前后 DOM diff ② todayCount > 0 / === 0 两种渲染 ③ 点击 today / calendar 入口触发对应 setSidebarSection
9. **AC-9**：`pnpm check` + `pnpm lint` + `pnpm test` 全绿

## 技术约束

- **useEffectiveLearningSettings**：必须用此 hook 读取派生值，不直接读 settingsStore.showLearningFeatures
- **todayCount 数据源**：先 `git grep` 找 TodayView 使用的 store；若有现成 selector 直接复用；如无可见数据源，本期可暂用 `useTodayCount()` 占位（返回 0），交付中明确标注
- **delete 旧代码**：
  - 删除 Calendar SidebarItem（src/components/layout/Sidebar.tsx:81-88 区域）
  - 删除 今日复习 SidebarItem（src/components/layout/Sidebar.tsx:89-96 区域）
  - 删除独立的 TODAY SidebarSection（src/components/layout/Sidebar.tsx:117-126 区域）
- **不动 union 值**：`SidebarSection` 类型不变；仍用 "today" / "calendar"
- **不改 SidebarSection 组件 API**：仅消费 title 与 className

## 参考文件

- `src/components/layout/Sidebar.tsx:81-96`（待删的 Calendar / 今日复习 SidebarItem）
- `src/components/layout/Sidebar.tsx:117-126`（待删的 TODAY section + 占位）
- `src/components/layout/SidebarItem.tsx`（SidebarSection 组件定义）
- `src/components/features/today/TodayView.tsx`（todayCount 数据源参考）
- `product/prd/notecapt-v1.3-ui_prd_v1.md` §4.2 SB-04 + ADR-003

## 预估影响范围

- **修改文件**：
  - `src/components/layout/Sidebar.tsx`（删旧代码 + 新增"学习中心"分组）
  - `src/components/layout/__tests__/Sidebar.test.tsx`（扩展）

- **新建文件**：无

---

## Reviewer 重点关注项

- 确认 `showLearningFeatures === false` 时 DOM 真的不渲染分组（不是 `display:none` 隐藏）
- todayCount 数据源是否复用了 TodayView 的同款 hook（避免数据不一致）
- 200ms 淡入是否符合 PRD §9.1（fade-in duration 由 --duration-fast 200ms 控制，与 task_012 token 收敛对齐）
