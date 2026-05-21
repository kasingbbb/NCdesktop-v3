# NCdesktop 日历功能迭代 PRD v1.0

> **产品**：NCdesktop（NoteCapt Desktop）
> **版本**：v2.2 — 课程日历迭代
> **状态**：Debate 完成，待 Conductor 接收
> **日期**：2026-04-11
> **来源**：Debate Session notecapt 迭代 / session_001

---

## 1. 项目概述

### 1.1 背景

NCdesktop v2.1 引入了课程日历功能，支持通过 ICS 文件/URL 导入课程表，在侧边栏显示 Today/Tomorrow/ThisWeek 课程分组，并提供 AI 课程预习空间。

当前存在一个关键体验缺陷：**当用户在无课日（周末、假期、休息日）使用应用时，侧边栏的课程分区完全不渲染（`return null`），导致日历功能的入口彻底消失。** 用户无法主动进入日历浏览未来课程，也无法提前预习。

### 1.2 目标

本次迭代解决两个层次的问题：

1. **入口可见性（Availability）**：日历入口必须始终可见，不因「当天无课」而消失
2. **日历浏览能力（Browsability）**：用户需要一个课程日历视图来浏览未来任意周的课程

### 1.3 功能定义

这是一个**课程日历（Course Calendar）**——展示从 ICS 导入的周期性课程事件在时间槽上的分布，而非通用日历。不处理个人日程、日记、TODO 等非课程数据。

---

## 2. 用户定义与核心场景

### 2.1 目标用户

**大学生 / 研究生**，使用 NoteCapt 管理课程笔记和知识资产。他们通过学校教务系统导出 ICS 课表，导入 NCdesktop 进行课程管理和预习。

### 2.2 核心场景

| ID | 场景 | 描述 | 优先级 | 频率 |
|----|------|------|--------|------|
| S1 | 周末预习 | 周末想提前预习下周课程，通过日历找到目标课程，进入预习空间 | P0 | 每周 1-2 次 |
| S2 | 无课日浏览 | 当天没课，但想看看本周或下周还有什么课需要准备 | P0 | 每天可能 |
| S4 | 当日预习 | 课前 30 分钟，在侧边栏快速找到今天的课并进入预习 | P0 | 每个上课日 |
| S3 | 学期规划 | 学期初浏览整个学期的课程分布 | P2 | 学期初 1-2 次 |

### 2.3 场景用户旅程

**S1 周末预习**：
```
用户打开 NCdesktop → 点击侧边栏「Calendar」→ ContentArea 显示课程日历周视图
→ 切换到下一周 → 找到周二上午的「高等数学」→ 点击课程卡片
→ 进入 CoursePreviewSpace → AI 生成预习指南 → 记录预习笔记
→ 点击「← 课程日历」返回周视图 → 继续浏览其他课程
```

**S2 无课日浏览**：
```
用户打开 NCdesktop → 侧边栏「Calendar」始终可见
→ 点击进入周视图 → 浏览本周剩余课程 → 确认后关闭
```

**S4 当日预习**（快速路径）：
```
用户打开 NCdesktop → 侧边栏 Today 分区显示今日 3 门课
→ 直接点击「09:00 高数」→ 进入预习空间
→ 点击 Back → 返回之前的视图
```

---

## 3. 功能需求

### 3.1 F1 — 侧边栏 Calendar 入口（P0）

**描述**：新增「Calendar」导航项，作为标准 SidebarItem，始终可见。

**详细需求**：
- 在侧边栏 Search / Recent / Starred 下方，分割线之上，新增一个带日历图标的「Calendar」导航项
- 行为与 Search / Recent / Starred 一致：点击后 `setSidebarSection("calendar")`，ContentArea 切换为课程日历视图
- 选中时有 active 高亮（使用现有 SidebarItem 的 active 样式）
- **始终可见**，不依赖是否有课程事件
- 图标：使用 `lucide-react` 的 `CalendarDays` 图标

**验收标准**：
- AC-1：应用启动后，侧边栏始终显示 Calendar 导航项
- AC-2：点击 Calendar 后，ContentArea 切换为课程日历视图
- AC-3：Calendar 选中时有与其他 SidebarItem 一致的高亮样式

### 3.2 F2 — 课程日历周视图（P0）

**描述**：在 ContentArea 中渲染时间网格周视图，展示一周的课程分布。

**详细需求**：

**导航栏（WeekHeader）**：
- 左右箭头按钮切换周（上一周 / 下一周）
- 中间显示当前周的日期范围（如「4月13日 - 4月19日」）
- 「今天」按钮快速跳回当前周
- 今天所在的列以微妙背景色高亮标记

**时间网格（TimeGrid）**：
- 左侧时间列：显示小时标记（8:00, 9:00, ... 22:00）
- 顶部星期行：一 / 二 / 三 / 四 / 五 / 六 / 日，每列显示具体日期
- 主网格区域：以小时为单位的网格线
- 时间范围：默认显示 8:00-22:00（覆盖大学课程的常见时间段）

**课程卡片（EventCard）**：
- 位置：根据 startTime 和 endTime 精确定位在时间网格中（分钟级精度）
- 高度：按课程时长比例缩放
- 内容：显示课程代码（如 MATH101）或课程名称；次要信息显示教室
- 交互：点击进入该课程的 CoursePreviewSpace
- 悬浮：Tooltip 显示完整课程信息（名称、教师、教室、时间）

**验收标准**：
- AC-1：周视图正确显示指定周的所有课程事件
- AC-2：课程卡片位置和高度与时间信息准确对应
- AC-3：可以通过左右箭头切换到任意周
- AC-4：点击「今天」按钮回到当前周
- AC-5：点击课程卡片进入 CoursePreviewSpace
- AC-6：周视图加载时间 < 200ms

### 3.3 F3 — CourseSection 空状态优化（P0）

**描述**：修改侧边栏 CourseSection 组件，在无课日不再完全隐藏。

**详细需求**：
- 移除 `CourseSection.tsx` 中三组为空时 `return null` 的逻辑
- 当三个分组（Today/Tomorrow/ThisWeek）全部为空时：
  - 仍然显示 Today 分组标题
  - 在 Today 分组下显示一行提示文字：「今天没有课程」
- 有课时行为保持不变（展示 Today/Tomorrow/ThisWeek 分组课程列表）

**验收标准**：
- AC-1：无课日时，侧边栏仍显示 Today 分组标题和「今天没有课程」提示
- AC-2：有课日的行为与当前完全一致

### 3.4 F4 — 导航回退优化（P0）

**描述**：CoursePreviewSpace 的 Back 按钮行为根据进入路径智能回退。

**详细需求**：
- 在 `uiStore` 中新增 `coursePreviewReturnTo` 状态，记录进入预习前的上下文
- 从日历周视图点击课程进入预习时：
  - `coursePreviewReturnTo = { section: "calendar", weekStart: activeWeekStart }`
  - Back 按钮文案：「← 课程日历」
  - 点击 Back：返回日历周视图，恢复之前浏览的周数
- 从侧边栏快速路径点击课程进入预习时：
  - `coursePreviewReturnTo = { section: 当前activeSidebarSection }`
  - Back 按钮文案：「← 返回」
  - 点击 Back：恢复到之前的 activeSidebarSection + 关闭 coursePreview

**验收标准**：
- AC-1：从日历周视图进入预习后，Back 返回日历周视图，且保持之前浏览的周
- AC-2：从侧边栏快速路径进入预习后，Back 返回之前的视图
- AC-3：Back 按钮文案根据进入路径动态变化

### 3.5 F5 — Store 状态扩展（P0）

**描述**：扩展 calendarStore 和 uiStore 支持周导航。

**calendarStore 新增**：
- `activeWeekStart: string | null`：当前周视图显示的周一日期（ISO 字符串）
- `setActiveWeekStart(date: string): void`：设置当前周
- `fetchWeekEvents(weekStart: string): Promise<void>`：加载指定周的课程事件（调用已有的 `fetchEvents`，传入周一到周日的范围）
- `navigateWeek(offset: number): void`：向前/向后切换 N 周

**uiStore 新增**：
- `coursePreviewReturnTo: { section: SidebarSection; weekStart?: string } | null`：预习空间返回目标
- `setCoursePreviewReturnTo(target: ... | null): void`

**types/ui.ts 扩展**：
- `SidebarSection` 类型新增 `"calendar"` 枚举值

**验收标准**：
- AC-1：切换周时正确加载对应周的课程数据
- AC-2：SidebarSection 支持 "calendar" 值
- AC-3：coursePreviewReturnTo 正确记录和恢复状态

---

## 4. 非功能需求

### 4.1 性能

| 指标 | 目标值 |
|------|--------|
| 周视图首次加载 | < 200ms |
| 切换周 | < 100ms |
| 课程卡片点击响应 | < 50ms |
| 内存增量 | < 5MB（周视图组件） |

### 4.2 可用性

- 周视图在窗口宽度 >= 700px 时正常显示
- 窄窗口下（< 700px）课程卡片只显示课程代码，省略教室信息
- 所有交互元素符合最小点击目标 44x44px

### 4.3 一致性

- 所有新增 UI 遵循 Liquid Glass 设计系统（CSS 变量、圆角、阴影）
- 新增 SidebarItem 与现有 SidebarItem 交互行为完全一致
- 颜色、字体、间距使用 CSS 自定义属性

---

## 5. 技术约束

### 5.1 来自项目宪章的约束

- 前端：TypeScript 严格模式，React 函数式组件 + Hooks
- 状态管理：Zustand，禁止 prop drilling 超过 2 层
- 样式：Tailwind CSS 4 + CSS 变量
- Tauri IPC：命令使用 snake_case，返回 Result<T, E>

### 5.2 来自 Debate 的约束

- **后端不改动**：`get_course_events` 已支持时间范围查询，本次迭代只修改前端
- **交互一致性**：Calendar SidebarItem 必须与 Search/Recent/Starred 行为一致，不引入新交互模式
- **不处理课程重叠**：MVP 阶段课程如果时间重叠，卡片会叠加显示（不做并列布局），P1 解决
- **不做独立 EventLayout**：MVP 阶段卡片位置计算内联到 EventCard 组件，P1 再抽出

---

## 6. 分期计划

### P0 — MVP（本次迭代）

| Task | 描述 | 工作量 | 依赖 |
|------|------|--------|------|
| T1 | `types/ui.ts`：SidebarSection 新增 "calendar" | S | 无 |
| T2 | `uiStore.ts`：新增 coursePreviewReturnTo 状态 + setCoursePreviewReturnTo | S | T1 |
| T3 | `calendarStore.ts`：新增 activeWeekStart / setActiveWeekStart / fetchWeekEvents / navigateWeek | S | 无 |
| T4 | `Sidebar.tsx`：新增 Calendar SidebarItem（CalendarDays 图标） | S | T1 |
| T5 | `CourseSection.tsx`：移除空状态 return null，无课日显示 Today + 提示 | S | 无 |
| T6 | `calendar/CalendarWeekView.tsx` + `calendar/WeekHeader.tsx`：周视图容器 + 导航栏 | M | T3 |
| T7 | `calendar/TimeGrid.tsx` + `calendar/EventCard.tsx`：时间网格 + 课程卡片（含内联位置计算） | L | T6 |
| T8 | `ContentArea.tsx`：添加 "calendar" 分区判断，渲染 CalendarWeekView | S | T6 |
| T9 | `CoursePreviewSpace.tsx`：Back 按钮基于 coursePreviewReturnTo 动态回退 | S | T2 |

**依赖关系**：
```
T1 ──→ T2 ──→ T9
 │              
 └──→ T4
      
T3 ──→ T6 ──→ T7
              │
              └──→ T8

T5（独立）
```

### P1 — 体验增强（下一迭代）

- 课程卡片颜色系统：按课程代码哈希自动分配颜色（从品牌色板派生）
- 课程重叠并列布局：抽出 `eventLayout.ts`，支持最多 3 列并列
- 空状态引导页面：未导入课表→引导导入；当周无课→提示切换周 + 回到本周按钮
- 响应式布局优化：窄窗口下降级显示

### P2 — 功能拓展（远期）

- 月视图 / 学期概览（支撑 S3 学期规划场景）
- 日历提醒 / 通知
- 系统日历（Apple Calendar）集成

---

## 7. 文件影响范围

### 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `src/types/ui.ts` | 修改 | SidebarSection 新增 "calendar" |
| `src/stores/uiStore.ts` | 修改 | 新增 coursePreviewReturnTo 状态和 setter |
| `src/stores/calendarStore.ts` | 修改 | 新增 activeWeekStart / fetchWeekEvents / navigateWeek |
| `src/components/layout/Sidebar.tsx` | 修改 | 新增 Calendar SidebarItem |
| `src/components/layout/ContentArea.tsx` | 修改 | 添加 "calendar" 分区路由 |
| `src/components/features/calendar/CourseSection.tsx` | 修改 | 移除空状态 return null |
| `src/components/features/preview/CoursePreviewSpace.tsx` | 修改 | Back 行为基于 coursePreviewReturnTo |

### 新增的文件

| 文件路径 | 说明 |
|---------|------|
| `src/components/features/calendar/CalendarWeekView.tsx` | 周视图容器组件 |
| `src/components/features/calendar/WeekHeader.tsx` | 周导航栏组件 |
| `src/components/features/calendar/TimeGrid.tsx` | 时间网格组件 |
| `src/components/features/calendar/EventCard.tsx` | 课程卡片组件 |

### 不修改的文件

- `src-tauri/` 下所有 Rust 文件（后端无需改动）
- `src/lib/tauri-commands.ts`（复用已有的 `getCourseEvents` 命令封装）

---

## Conductor 桥接摘要

### 核心功能清单（带优先级）

| 功能 | 优先级 | 核心用户场景 | 来自 Debate 的关键约束 |
|------|--------|-------------|----------------------|
| Calendar SidebarItem 始终可见 | P0 | S2 无课日浏览 | 必须与 Search/Recent/Starred 交互一致（L2 Round 5 共识） |
| 课程日历周视图（CalendarWeekView） | P0 | S1 周末预习 / S2 无课日浏览 | 时间网格形式，不做月视图（L4 scope 裁剪） |
| CourseSection 空状态优化 | P0 | S2 无课日浏览 | 无课日保留 Today 分区 + 提示文字（L1 Round 3 共识） |
| 导航回退优化（coursePreviewReturnTo） | P0 | S1/S4 预习流程 | Back 返回取决于进入路径（L3 Round 7 共识） |
| Store 状态扩展（周导航） | P0 | S1 周末预习 | 后端不改动，只调前端 Store（L3 Round 7 确认） |
| ContentArea 日历路由 | P0 | 全部场景 | 新增 SidebarSection "calendar"（L2 Round 5 共识） |
| 课程卡片颜色系统 | P1 | 视觉体验 | 颜色按课程代码哈希分配（L2 共识） |
| 课程重叠并列布局 | P1 | 排课密集用户 | 最多 3 列并列（L2 共识），MVP 先简单堆叠 |
| 空状态引导页面 | P1 | 新用户 onboarding | 两种场景：未导入 / 当周无课（L2 共识） |
| 月视图 / 学期概览 | P2 | S3 学期规划 | S3 降级为 P2（L1 Round 2 共识） |

### 不可妥协的技术底线

1. **交互一致性**：Calendar SidebarItem 必须与现有 SidebarItem 行为完全一致，不引入新的交互模式
2. **后端不改动**：本次迭代不修改任何 Rust / Tauri 后端代码
3. **性能底线**：周视图加载 < 200ms，切换周 < 100ms
4. **Liquid Glass 一致性**：所有新增 UI 组件使用 CSS 变量，遵循设计系统

### 已识别的高风险项

| 风险 | 来源（Debate 哪一轮） | 当前状态 | 缓解策略 |
|------|---------------------|----------|----------|
| 时间网格渲染复杂度 | Layer 2 Round 4 | 已解决 | 组件拆分为 WeekHeader/TimeGrid/EventCard 三层 |
| 时区处理可能出错 | Layer 3 Round 6 | 已搁置 | 复用现有 localDateKey 逻辑，统一使用 Date 对象 |
| 新导航理解成本 | Layer 3 Round 6 | 已搁置 | P1 考虑首次使用 onboarding 提示 |
| 课程重叠显示 | Layer 2 Round 5 | 已搁置 | MVP 不处理重叠，P1 实现并列布局 |

### MVP 边界声明

- **做什么**：
  - Calendar SidebarItem（始终可见）
  - 课程日历周视图（时间网格 + 周导航）
  - CourseSection 空状态优化
  - Back 智能回退（基于进入路径）
  - calendarStore + uiStore 状态扩展

- **不做什么**：
  - 月视图/年视图/学期概览（P2，需要 S3 场景验证）
  - 课程事件编辑/创建（数据源为 ICS 导入，不在 NCdesktop 中编辑）
  - 日历提醒/通知（P2，需要系统集成）
  - 系统日历（Apple Calendar）同步（P2，复杂度高，价值待验证）
  - 课程卡片颜色系统（P1，不阻塞核心功能）
  - 课程重叠并列布局（P1，MVP 先堆叠显示）
  - 空状态引导页面（P1，MVP 先用简单文案）
  - 后端改动（已有 API 满足需求）

### Debate 中未达成共识的争议

无。所有核心争议均在四层 Debate 中解决，无遗留分歧。
