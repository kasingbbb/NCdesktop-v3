# 技术方案 — 日历功能迭代

## 项目概述

为 NCdesktop 新增课程日历周视图功能。在侧边栏新增始终可见的 Calendar 导航项，ContentArea 中渲染时间网格周视图，支持周切换和课程卡片点击进入预习空间。同时优化 CourseSection 空状态和预习空间 Back 回退逻辑。后端不改动，纯前端迭代。

## 技术选型

无新增外部依赖。所有实现基于现有技术栈：React 19 + TypeScript + Zustand + Tailwind CSS 4 + Lucide React。

## Architecture Decision Records (ADR)

### ADR-001: 周视图时间网格使用纯 CSS Grid 实现
- **状态**：已接受
- **上下文**：时间网格需要在 7 列（周一到周日）× N 行（小时）的网格中精确定位课程卡片
- **决策**：使用 CSS Grid + absolute positioning 在网格格中放置课程卡片
- **被排除项**：Canvas 渲染（过度工程）、Table 布局（定位不灵活）
- **后果**：代码简洁，但课程重叠时卡片会叠加（MVP 可接受，P1 处理并列）

### ADR-002: 周导航状态存储在 calendarStore 而非 URL
- **状态**：已接受
- **上下文**：用户切换周时需要记住当前浏览的周
- **决策**：在 calendarStore 中新增 `activeWeekStart` 状态
- **被排除项**：URL 参数（单页应用，不需要深链接）、localStorage（违反宪章，持久化数据用 Tauri 后端）
- **后果**：应用刷新时回到当前周（可接受）

### ADR-003: coursePreviewReturnTo 使用对象而非 history stack
- **状态**：已接受
- **上下文**：Back 按钮需根据进入路径智能回退
- **决策**：在 uiStore 新增 `coursePreviewReturnTo` 对象，记录进入前的 section 和 weekStart
- **被排除项**：浏览器 history API（与 Tauri 单页应用不兼容）、全局 navigation stack（过度工程）
- **后果**：简单可靠，但只支持一级回退（当前需求足够）

## 系统架构

```
┌─ Sidebar ──────────────────┐  ┌─ ContentArea ─────────────────────────┐
│ Search                     │  │                                       │
│ Recent                     │  │  activeSidebarSection === "calendar"   │
│ Starred                    │  │  → CalendarWeekView                   │
│ ─── (新增) Calendar ───────│──│     ├── WeekHeader (导航栏)            │
│ ─── CourseSection ─────────│  │     └── TimeGrid (时间网格)            │
│   Today / Tomorrow / Week  │  │           └── EventCard (课程卡片)     │
│ ─── ProjectTree ───────────│  │                                       │
│ ─── TagTree ───────────────│  │  rightPanelMode === "course_preview"   │
└────────────────────────────┘  │  → CoursePreviewSpace (已有)           │
                                └───────────────────────────────────────┘
```

**状态流**：
```
点击 Calendar SidebarItem
  → setSidebarSection("calendar")
  → ContentArea 渲染 CalendarWeekView
  → CalendarWeekView 调用 fetchWeekEvents(activeWeekStart)

点击课程卡片（从周视图）
  → setCoursePreviewReturnTo({ section: "calendar", weekStart })
  → setActiveCourseEventId(eventId)
  → setRightPanelMode("course_preview")
  → ContentArea 渲染 CoursePreviewSpace

点击 Back（在 CoursePreviewSpace）
  → 读取 coursePreviewReturnTo
  → setSidebarSection(returnTo.section)
  → 如果有 weekStart，恢复 calendarStore.activeWeekStart
  → setActiveCourseEventId(null)
  → setRightPanelMode("inspector")
```

## 数据模型

无新增数据模型。复用现有 `CourseEvent` 类型和 `get_course_events` Tauri 命令。

## API 设计

无新增后端 API。复用 `getCourseEvents(libraryId, startAfter, endBefore)` 前端封装。

## 目录结构

```
src/
├── types/
│   └── ui.ts                          # 修改：SidebarSection += "calendar"
├── stores/
│   ├── uiStore.ts                     # 修改：+coursePreviewReturnTo
│   └── calendarStore.ts               # 修改：+activeWeekStart/fetchWeekEvents/navigateWeek
├── components/
│   ├── layout/
│   │   ├── Sidebar.tsx                # 修改：+Calendar SidebarItem
│   │   └── ContentArea.tsx            # 修改：+"calendar" 分区渲染
│   └── features/
│       ├── calendar/
│       │   ├── CourseSection.tsx       # 修改：空状态优化
│       │   ├── CalendarWeekView.tsx    # 新增：周视图容器
│       │   ├── WeekHeader.tsx         # 新增：周导航栏
│       │   ├── TimeGrid.tsx           # 新增：时间网格
│       │   └── EventCard.tsx          # 新增：课程卡片（周视图用）
│       └── preview/
│           └── CoursePreviewSpace.tsx  # 修改：Back 智能回退
```

## 风险登记表

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| 时区处理导致课程卡片位置偏移 | 中 | 中 | 复用现有 localDateKey 逻辑，统一使用 Date 对象本地时间 |
| 课程重叠时卡片遮挡 | 低 | 低 | MVP 允许叠加，P1 实现并列布局 |
| 窗口极窄时网格布局错乱 | 低 | 低 | 设定 min-width，窄窗口降级只显示课程代码 |

## Task 清单

- [ ] task_002_dev_types — SidebarSection 新增 "calendar" 枚举值
- [ ] task_003_dev_uistore — uiStore 新增 coursePreviewReturnTo 状态
- [ ] task_004_dev_calendarstore — calendarStore 新增周导航状态和方法
- [ ] task_005_dev_sidebar — Sidebar 新增 Calendar SidebarItem
- [ ] task_006_dev_coursesection — CourseSection 空状态优化
- [ ] task_007_dev_weekview — CalendarWeekView + WeekHeader 周视图组件
- [ ] task_008_dev_timegrid — TimeGrid + EventCard 时间网格组件
- [ ] task_009_dev_contentarea — ContentArea 添加 "calendar" 分区路由
- [ ] task_010_dev_backbutton — CoursePreviewSpace Back 智能回退

## Task 依赖拓扑

```
task_002 → task_003 → task_010
         ↘ task_005
task_004 → task_007 → task_008
                    ↘ task_009
task_006（独立）

可并行：task_002, task_004, task_006
可并行：task_003, task_005（完成 task_002 后）
可并行：task_008, task_009（完成 task_007 后）
```
