# Review Scorecard — task_005_dev_sidebar_learning_center

## 审查思考过程

1. **Task 意图**：删除原 Sidebar 中分散的 Calendar / 今日复习 SidebarItem 与独立的 TODAY SidebarSection（含"今天没有课程"占位行）；在"知识"入口下方、ProjectTree 之前新增 `<SidebarSection title="学习中心">`，仅在 `showLearningFeatures === true` 时条件渲染；含「今日」+「课程表」两项；wrapper 加 `sidebar-learning-group sidebar-learning-fade-in` 实现 200ms 淡入。（ADR-003 / SB-04）

2. **AC 检查结果**：
   - AC-1 `showLearningFeatures === false` 时 Sidebar 无任何"学习/课程/今日"相关 DOM：✅（`Sidebar.tsx:100` 用 `{showLearningFeatures && ...}` 条件渲染，整块树不挂载，非 display:none；Sidebar.test "AC-2 默认态「日历/今日/学习中心」均不在 DOM 中" PASS）
   - AC-2 `showLearningFeatures === true` 渲染"学习中心" SidebarSection，位置在"知识"和 ProjectTree 之间：✅（`Sidebar.tsx:99-120`，紧接知识中心 SidebarItem，在工作区 ProjectTree section 之前）
   - AC-3 "今天没有课程"占位文字已删：✅（`Sidebar.test` "SB-04：不再渲染 '今天没有课程' 占位文案" PASS）
   - AC-4 `todayCount > 0` 时渲染"今日" SidebarItem 带 badge：⚠️ **未实现** — output.md 显式说明"未引入 todayCount 数据源"。当前"今日"项总是渲染、且无 badge。这是 input.md AC-4/AC-5 的部分降级。
   - AC-5 `todayCount === 0` 时"今日" SidebarItem 不渲染：⚠️ **未实现**（同上）— 当前学生态总是渲染"今日"按钮。注意：这违反 PRD §9.1 / P-04"零数据零信号" 的字面要求（学习中心模式下若无任务仍渲染一行"今日"），但 PM 在 progress.md 中已将此延后至 v1.4。
   - AC-6 "课程表" SidebarItem 总是渲染（学生态下），点击触发 `setSidebarSection("calendar")`：✅（`Sidebar.tsx:112-117`）
   - AC-7 分组容器有 class `sidebar-learning-fade-in`：✅（`Sidebar.tsx:101`；globals.css:361-363 已有 200ms keyframe）
   - AC-8 单测覆盖 ① ON/OFF 切换 DOM diff ② todayCount > 0/= 0 ③ 点击 today/calendar：①PASS（Sidebar.test AC-2 + AC-3 ON 反向） ②**未覆盖**（因数据源未引入） ③ "今日"点击未单独测、"课程表"点击未单独测（但 SB-04 用例验证两个按钮存在）
   - AC-9 全量测试 + lint + tsc：✅（vitest 26 fail / baseline ≤ 26；lint 25 / 锁 ≤ 25；tsc 通过）

3. **关键发现**：
   - **状态门控正确**：用 `useEffectiveLearningSettings()` 派生而非直接读 settings.showLearningFeatures（符合 session_context §6 第 1 条领域审查重点 "任何'学习'相关 UI 是否用 useEffectiveLearningSettings()"）。✅
   - **条件渲染而非 display:none**：`{showLearningFeatures && (...)}` 整块不挂载；Sidebar.test 用 `container.querySelector(".sidebar-learning-group") === null` 验证。符合 P-04 与 ADR-003。✅
   - **wrapper class + titleColor 一应俱全**：`sidebar-learning-group sidebar-learning-fade-in` + `titleColor="var(--sidebar-group-learning)"` 都到位，globals.css 已有 200ms keyframe（值 `--duration-fast` 对齐 task_012）。✅
   - **AC-4/AC-5 todayCount 数据未连接**：output.md "关于 input.md 中'今日 badge 仅在 todayCount > 0 时渲染'"段落坦诚标注未实现，且转嫁到 task_010；progress.md "v1.3 最终交付总结"也明确"今日 badge 数字" 延后到 v1.4。此为已知降级、PM 已知。
   - **PRD §9.1 风险**：当前学生态进入应用、当日无课程时仍渲染一行"今日"按钮，按 P-04 字面应不渲染。但学生态本身就是有"课程表"的，"今日"作为入口而非状态指示渲染也算合理；只是 badge 数字延后。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 20% | 3 | AC-1/2/3/6/7/9 满足；AC-4/AC-5 todayCount badge 未实现（部分降级），AC-8 ② 数据源未连导致测试缺失。已知降级 PM 同意，但仍是 input.md 字面 AC 偏离 |
| 安全性 | 5% | 5 | 纯条件渲染 + 派生 hook，无新数据流 |
| 代码质量 | 20% | 5 | 状态门控走 `useEffectiveLearningSettings`；wrapper class 与 titleColor 完备；删除旧代码彻底（占位文案、Calendar/今日复习 SidebarItem 全清） |
| 测试覆盖 | 20% | 3 | ON/OFF 切换 DOM diff 完备；wrapper class 与 titleColor 覆盖；缺 ①"今日"点击触发 `setSidebarSection("today")` 单测 ②"课程表"点击触发 `setSidebarSection("calendar")` 单测 ③todayCount 分支整体缺位（合 input.md AC-8 ②） |
| 架构一致性 | 15% | 5 | 完全符合 ADR-003：删 Calendar/今日复习/TODAY 三处、合并到"学习中心"；不动 SidebarSection API；不引新 union 值 |
| 可维护性 | 10% | 4 | 注释"仅学生态"语义清晰；扣分：todayCount 接入点没在代码留 TODO 标记，后续 task_010 接入者可能漏看 |
| UX 体感 | 10% | 4 | 学习中心分组语义化、淡入动效到位；扣分：学生态首启当日无课时仍显"今日"一行，与 P-04"零数据零信号"略有摩擦（已记 v1.4 跟进） |

**综合分**：(3·0.20)+(5·0.05)+(5·0.20)+(3·0.20)+(5·0.15)+(4·0.10)+(4·0.10) = 0.60+0.25+1.00+0.60+0.75+0.40+0.40 = **4.00/5**（加权）

## 总体判断

- [x] **PASS**

## 问题列表

### BLOCKER
（无）

### MAJOR
（无 — AC-4/5 已知降级在 ADR-007 / progress.md 延后清单内，不计 MAJOR）

### MINOR
1. **"今日" / "课程表"点击交互未单测**
   - 代码位置：`src/components/layout/__tests__/Sidebar.test.tsx` "Sidebar — v1.3 PR-B 差异点" describe
   - 现象：当前用例只验证两个按钮存在，未验证点击触发 `setSidebarSection("today" / "calendar")`
   - 建议：补两个 it：分别 click "今日" / "课程表" 后断言 `useUIStore.getState().activeSidebarSection` 等于 "today" / "calendar"
   - 验证标准：用例 PASS；点击 setter 真实路径打通
2. **todayCount 接入点缺 TODO 标记**
   - 代码位置：`src/components/layout/Sidebar.tsx:106-111`
   - 现象：当前"今日"项无 badge 与条件渲染；后续 task_010 接入者可能漏看 output.md 中的"取舍"段落
   - 建议：在 SidebarItem 上方加注释 `// TODO(v1.4): 接入 todayCount，>0 时渲染 badge 数字；=0 时整条不渲染 (PRD §9.1 P-04)`
   - 验证标准：注释包含 v1.4 标识 + PRD 引用
3. **P-04 字面摩擦提示**
   - 代码位置：`src/components/layout/Sidebar.tsx:106-111`
   - 现象：学生态首启当日无课程时仍渲染"今日"一行（PRD §9.1 "无任务时不出现"字面）
   - 建议：v1.4 task_010 同步处理 — 当前不阻塞 PR-B 合并

## 给 Dev 的修复指引

（PASS，无需修复；MINOR 1/2 建议在 task_010 一并处理）
