# Debate 结论 — 日历功能迭代

> Session: notecapt 迭代 / debate / session_001
> 完成时间: 2026-04-11
> 辩题: 如何迭代 NCdesktop 日历功能，使用户在「当天无课」场景下仍能看到日历入口，并通过日历视图浏览和预习未来的每一堂课？

---

## 四层共识总结

### Layer 1 — 问题定义

- **核心问题**：入口可见性（日历入口不应因无课消失）+ 日历浏览能力（需要周视图浏览未来课程）
- **功能定义**：课程日历（Course Calendar），非通用日历，展示 ICS 导入的周期性课程事件
- **核心场景**：S1 周末预习（P0）、S2 无课日浏览（P0）、S4 当日预习（P0）；S3 学期规划降为 P2
- **导航架构**：侧边栏 Calendar SidebarItem（始终可见） + 可折叠课程列表（快速路径）
- **系统边界**：不做事件编辑、通知、月视图、系统日历同步

### Layer 2 — 理想态

- **周视图**：时间网格，Mon-Sun 列，时间行，课程卡片按比例缩放
- **侧边栏**：Calendar 作为标准 SidebarItem（与 Search/Recent/Starred 一致），新增 SidebarSection "calendar"
- **课程重叠**：并列布局，最多 3 列（P1）
- **空状态**：未导入→引导导入；当周无课→提示切换周

### Layer 3 — 差距分析

- **P0 Gap**：侧边栏入口、SidebarSection 扩展、周视图 4 个子组件、Store 状态扩展、CourseSection 空状态修复、Back 行为调整
- **后端无需改动**：get_course_events 已支持时间范围查询
- **主要风险**：时间网格渲染（低）、时区处理（中）、新导航理解成本（中）

### Layer 4 — 策略

- **MVP 9 个 Task**：类型扩展 → Store 扩展 → UI 组件 → 路由集成 → 交互调整
- **回溯校验通过**：MVP 完全覆盖 Layer 1 核心问题，无遗漏，无 scope creep

---

## 关键决策记录

| 决策 | 来源 | 理由 |
|------|------|------|
| 课程日历而非通用日历 | Reviewer 挑战 → Proposer 接受 | 数据源为 ICS 课程表，非通用事件 |
| Calendar 作为 SidebarItem | Reviewer 挑战 → Proposer 调整 | 保持交互一致性 |
| 两条路径并存 | Proposer 提出 → Reviewer 验证 | 快速路径（侧边栏课程列表）+ 浏览路径（日历周视图） |
| S3 学期规划降为 P2 | Reviewer 挑战 → Proposer 接受 | 只有周视图无法支撑学期浏览 |
| MVP 不含独立 EventLayout | Reviewer 挑战 → Proposer 接受 | MVP 不处理重叠，布局逻辑内联 |
| Back 返回取决于进入路径 | Reviewer 挑战 → Proposer 设计 | 避免用户从 Projects 视图进入预习后被跳到 Calendar |

---

## 争议与搁置项

无未解决的核心争议。所有搁置项均已标注优先级：
- 课程卡片颜色系统（P1）
- 课程重叠并列布局（P1）
- 空状态引导页面（P1）
- 月视图（P2）
