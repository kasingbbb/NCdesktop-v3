# Review Scorecard — task_010_dev_empty_state

## 审查思考过程

1. **Task 意图**：建立全局空状态规范——TodayView 顶部 0/0/0 计数栏整行不渲染、去掉 🎉 / "今天没有 XXX" / 恭喜 / 加油等感性文案、统一调用 EmptyState（ES-01 ~ ES-04）。

2. **AC 检查结果**：
   - AC-1（EmptyState 扩展 `cta` 槽）：❌ **未实现**。output.md 已明确"未扩展 props"。EmptyState.tsx 仍是 hardcode `Welcome to NoteCapt` + `New Project` 按钮，未做通用化。
   - AC-2（TodayView stats-row 全 0 整行不渲染，不是 hidden）：✅ TodayView.tsx:213 用 `{(stats.total > 0 || stats.validated > 0 || stats.mastered > 0) && <div ...>}` 条件渲染，符合"不渲染"语义。
   - AC-3（TodayView 空状态调用 EmptyState + Check icon + 中性文案）：⚠ **部分**——文案改成"今日无待处理 / 导入素材后这里会自动生成任务"✅，但**没有调用 EmptyState 组件**，而是内联 `<div className="tdv-empty">` + `<BookOpen>` icon，且 icon 是 BookOpen 不是 Check（input.md 允许"不放 icon 也可"，本身不违规）。
   - AC-4（grep `🎉/恭喜/加油/今天没有` production 代码 0 命中）：❌ **未达成**。grep 结果显示：
     - `src/components/features/skills/SkillsView.tsx:277` "技能已验证 🎉"
     - `src/components/features/skills/SkillChallengePanel.tsx:223` "技能验证通过 🎉"
     - `src/components/features/calendar/CourseSection.tsx:85` "今天没有课程"
     - TodayView.tsx 已清理 ✅
   - AC-5（SkillsStep / ProjectTree / TagTree 空状态统一调用 EmptyState）：❌ **未实现**。output.md 明确"未做（ES-04 跨文件改造）"。
   - AC-6（单测覆盖：全 0 不渲染 / EmptyState 文案 / DOM 不含 🎉）：❌ **未实现**。output.md "未新增 TodayView.test"（advisory）。
   - AC-7（pnpm check / lint / test 全绿）：⚠ 在 baseline 锁内（26 fail / 25 lint errors / TSC 通过），符合"baseline 锁"语义但非真"全绿"。

3. **关键发现**：
   - **AC-4 全局清洁度不达标是最关键问题**：input.md 明确要求 production code 4 字面量 0 命中，实际 3 处命中（2 处 🎉 + 1 处"今天没有课程"），违反 PRD §7.2 ES-03 "去除感性文案"硬规则。这不是"延后到 v1.4"——这是同一 task scope 内必须解决的字面量审计，且改起来都是单点改文案，不涉及组件解构。
   - **AC-1 / AC-5 / AC-6 整体延后**：output.md 自陈"EmptyState 未扩展 / 跨文件未统一 / 测试未新增"。3 个 AC 全部未实现，约占 task scope 50%。
   - TodayView 本体改造（AC-2 / AC-3 部分）做得干净，data-testid 加得规范，条件渲染严格"不渲染"非 hidden。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 20% | 2 | AC-1/4/5/6 未达成（4 条 AC ❌，3 条 AC ✅/⚠）。AC-4 字面量清洁是硬规则。 |
| 安全性 | 5% | 5 | 纯文案 + 条件渲染，无安全风险。 |
| 代码质量 | 20% | 4 | TodayView 修改干净；条件渲染规范；data-testid 加得到位；只是范围严重不足。 |
| 测试覆盖 | 20% | 1 | 未新增 TodayView.test，AC-6 未达成。output.md 自陈延后理由（Tauri commands mock 体量）但仍是范围内交付物。 |
| 架构一致性 | 15% | 4 | 没引入外圈重构；遵循令牌沿用；条件渲染走 React 标准模式。 |
| 可维护性 | 10% | 3 | TodayView 本体可维护；但 SkillsView / SkillChallengePanel 残留 🎉 留给下一次审计，技术债显化。 |
| UX 体感 | 10% | 2 | "否决项"——PRD §9.1 与 §7.2 ES-03 明确"去 🎉/恭喜/加油"，全局仍有 3 处命中，UX 体感未通过北极星。 |

**综合分**：(2*0.20) + (5*0.05) + (4*0.20) + (1*0.20) + (4*0.15) + (3*0.10) + (2*0.10) = 0.40 + 0.25 + 0.80 + 0.20 + 0.60 + 0.30 + 0.20 = **2.75/5**

## 总体判断

- [x] **FIX**（综合 2.75 < 3.5；2 个 MAJOR + 否决项 UX 体感未过；但所有问题可在 1 轮内集中修复，未触发 BLOCKER 阈值——非安全/非核心运行问题）

## 问题列表

### BLOCKER（必须修复）

无（虽 UX 体感是"否决项"，但本期是迭代 task，FIX 修复后即可恢复 PASS 候选）。

### MAJOR（强烈建议修复）

1. **问题**：感性文案/emoji 全局字面量清洁未达成（AC-4 硬规则违反）
   - **代码位置**：
     - `src/components/features/skills/SkillsView.tsx:277` "技能已验证 🎉"
     - `src/components/features/skills/SkillChallengePanel.tsx:223` "技能验证通过 🎉"
     - `src/components/features/calendar/CourseSection.tsx:85` "今天没有课程"
   - **修复方向**：
     - SkillsView 277：删除 🎉，文案改"技能已验证"或保留 lucide Check icon
     - SkillChallengePanel 223：同上
     - CourseSection 85：按 PRD §9.1 "TODAY 整组在无课时不渲染"——把"今天没有课程"占位整段删除（条件渲染不进 DOM），或改成中性"今日无课程安排"（如确有渲染必要）
   - **验证标准**：`grep -rn '🎉\|恭喜\|加油\|今天没有' src/ --include='*.tsx' --include='*.ts' | grep -v '\.test\.' | grep -v '__tests__'` 0 命中

2. **问题**：AC-6 单测覆盖完全缺失（TodayView.test 未新增）
   - **代码位置**：`src/components/features/today/__tests__/TodayView.test.tsx`（不存在）
   - **修复方向**：新建 TodayView.test.tsx，最少 3 个用例：
     - ① stats.total/validated/mastered 全 0 时 `screen.queryByTestId('tdv-stats-row')` 为 null
     - ② prioritized 为空时 `screen.queryByTestId('tdv-empty')` 存在且 textContent 含"今日无待处理"
     - ③ 渲染后 `container.innerHTML.includes('🎉')` 为 false
   - 关于 Tauri commands mock 体量大的延后理由：tauri-commands.ts 只需 `vi.mock('../../../lib/tauri-commands', () => ({ kuGetList: vi.fn().mockResolvedValue([]), kuGetDueForReview: vi.fn().mockResolvedValue([]) }))`——10 行内的 mock，本身不构成延后理由
   - **验证标准**：`pnpm test TodayView.test` 3 用例 PASS

### MINOR（可选）

1. AC-1 EmptyState 扩展为通用 `<EmptyState icon title hint? cta? />` 与 AC-5 SkillsStep/ProjectTree/TagTree 统一调用——本期延后到下一轮 task 可接受，但应在 progress.md 显式记录技术债。
2. TodayView 空状态 icon 用 BookOpen 而非 input.md 建议的 Check——input.md 明确允许"icon 可不放"，本身不违规，但 BookOpen 与"今日无待处理"语义弱挂钩（"还没有书可读"），可考虑直接不放 icon 或改 lucide `Inbox` / `CheckCheck`。
3. TodayView.tsx:273 DailyReviewPanel 内嵌 `<div className="tdv-empty">` 是占位 Panel，自身文案"今日复习 / 复习清单将在后续版本中开放"——文案中性 ✅，但与 task_010 "统一调用 EmptyState"目标方向相反，建议 AC-5 修复时一并处理。

## 给 Dev 的修复指引

### 修复范围约束

- **只修以上列出的 MAJOR 1 + 2**，不要连带重构
- **MINOR 不强制**：但 MINOR 1 应在交付时注明技术债状态
- 修复完成后必须重跑 `pnpm test` 验证新增的 TodayView.test 通过 + 整体 baseline 锁不破
- 修复 MAJOR 1 时注意：CourseSection.tsx 涉及学习模式 UI，需确保删除"今天没有课程"占位不会破坏其他测试（Sidebar.test.tsx:211 已有该字面量的反向断言"应不渲染"，符合 PRD §9.1 心智）
