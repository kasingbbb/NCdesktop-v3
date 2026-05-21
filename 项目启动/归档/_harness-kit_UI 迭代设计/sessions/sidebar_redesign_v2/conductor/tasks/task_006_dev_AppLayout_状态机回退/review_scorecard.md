# Review Scorecard — task_006_dev_AppLayout_状态机回退

## 审查思考过程

1. **Task 意图复述**：在 AppLayout / ContentArea 实现「渲染层兜底」—— 当 `showLearningFeatures === false` 且 `section ∈ {today, calendar}` 时强制不挂载学习视图、调一次 `setSidebarSection('recent')`。**主路径** turnLearningOff 留给 task_007。本 task 不能反向写（show=true 时跳 today/calendar）。

2. **AC 检查**：逐条核对 task_006/input.md 的 AC-1~AC-N：
   - 兜底监听 `[showLearningFeatures, activeSidebarSection]` ✅
   - 触发条件正确（!show && section ∈ {today, calendar}）✅
   - 不反向写（show=true 不跳 today/calendar）✅
   - 不联动写两个依赖字段 ✅（task_003 派生 selector 已保证；本 task 无任何 settings 写入）
   - ContentArea 防御渲染 ✅
   - 未引入 store/SettingsPanel/turnLearningOff（严格守 task_007 scope）✅

3. **scope 冲突裁决**：Dev 报告标注与 Conductor prompt 冲突，按 input.md 决策。**Reviewer 裁定 Dev 正确**：
   - task_006/input.md 第 4 行明确「主路径在 task_007」
   - task_007/input.md 第 7 行明确「依赖 task_006，兜底已就位」+ 第 4 行明确「turnLearningOff 在本 task（007）」
   - Architect §11.3 拓扑：006 → 007（007 才是主路径）
   - 若本 task 也写 turnLearningOff，会与 task_007 重复且产生时序竞争
   - 结论：Conductor 在 dispatch 提示中提前要求主路径是 prompt 漂移，input.md 是契约真相

4. **关键发现**：
   - 兜底 effect 仅在 false → recent 一个方向上写，符合 PRD §11 底线 8（先跳路由→下一帧 toggle 的"接力"位）
   - 不触碰 turnLearningOff/On，不触碰 settingsStore，scope 收敛得当

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|---|---|---|---|
| 功能正确性 | 25% | 5 | 兜底逻辑闭环；6+5 新测试用例 + 3 原有 PASS |
| 用户体验 | 25% | 4.5 | 兜底保证不会出现 today/calendar 空白页；唯一遗憾是没有渐出动画（属 task_007 主路径范围） |
| 代码质量 | 15% | 4.5 | effect 注释清晰说明"接力"语义；ContentArea 防御一行到位 |
| 测试覆盖 | 15% | 4.5 | 兜底正反方向、不反向写、跨 task 回归都有覆盖 |
| 架构一致性 | 10% | 5 | 严格守 ADR-005 兜底/主路径分工；不越界 task_007 |
| 安全性 | 10% | 5 | 不删数据 ✓；DEV warn 门禁未引新；不引新库 |

**综合分：4.73/5**（加权）

## 总体判断

- [x] **PASS**
- [ ] FIX
- [ ] BLOCKER

## 问题列表

### BLOCKER
无

### MAJOR
无

### MINOR
1. **task_002 残留 TS6133** `_typeCheck unused`
   - 代码位置：`src/stores/uiStore.ts:47`
   - 修复方向：加 `// @ts-expect-error` 或改为类型层断言函数（不影响运行）
   - 验证标准：`pnpm exec tsc -b --noEmit` 零警告
   - **判定**：非本 task 引入，转 task_009/收尾处理

2. **快速连点 race 端到端验证**
   - Dev 在 output.md 已知局限 §2 主动披露
   - 修复方向：留待 task_009 端到端测试覆盖（task_007 turnLearningOff + task_006 兜底协同）
   - 验证标准：task_009 包含 "快速连点 show toggle 不出现空白页" 用例

## 跨 task 回归验证

```
pnpm test src/components/layout/__tests__/ src/stores/__tests__/ src/components/features/KnowledgeHubView/
Test Files  7 passed (7)
     Tests  85 passed (85)
```
task_002/003/004/005 全部 PASS，0 回归破坏。

## 修复指引
N/A（PASS）
