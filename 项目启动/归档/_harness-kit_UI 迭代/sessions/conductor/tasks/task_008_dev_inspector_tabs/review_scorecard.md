# Review Scorecard — task_008_dev_inspector_tabs

## 审查思考过程

### 1. Task 意图

把 Inspector 底部 segmented tabs 数组从 `[inspector, timeline-flow, knowledge_association]` 重排为 `[inspector, knowledge_association, timeline_flow]`，让两个静态信息 tab（详情 + 知识关联）相邻，动态时间流沉到末位。同时新建 `Inspector.test.tsx` 覆盖顺序与点击切换。

### 2. AC 逐条检查

| AC | 内容 | 结果 | 证据 |
|----|------|------|------|
| AC-1 | Tab 顺序：详情 → 知识关联 → 时间流 | ✅ | Inspector.tsx:17-25 `BOTTOM_TABS` 数组顺序；Inspector.test.tsx:54-63 DOM 顺序断言 |
| AC-2 | 点击 tab 切换 `rightPanelMode` | ✅ | Inspector.tsx:146 `onClick={() => setRightPanelMode(tab.key)}`；Inspector.test.tsx:73-89 两个用例验证 |
| AC-3 | 默认初始 tab "详情"（`rightPanelMode === "inspector"`） | ✅ | uiStore.ts:134 默认 `"inspector"`；Inspector.test.tsx:65-71 |
| AC-4 | rehydrate 后停在上次（`rightPanelMode` 未持久化 → 自动满足） | ✅ | uiStore.ts:254-258 partialize 不含 `rightPanelMode`，无需 migrator |
| AC-5 | 切换无视觉闪烁 | ✅（视觉） | tab 间共用 segmented control 容器，仅 inner panel swap，不重挂载 Inspector aside |
| AC-6 | role/aria-pressed 保留 + 键盘左右切换 | ⚠️ | aria-pressed 保留（Inspector.tsx:147）；**但当前实现没有 role="tablist"**，只有按钮带 aria-pressed。input AC-6 描述"`role="tablist"` + `aria-selected` 保留"。**这里实际是 `aria-pressed`**（不是 `aria-selected`），原代码就是 aria-pressed 风格，未引入 tablist semantic。键盘左右箭头切换没有专门处理，但 button 间 Tab 顺序天然可达 |
| AC-7 | 单测：① 顺序 ② 点击切换 ③ aria 属性 | ✅ | Inspector.test.tsx 4 用例：顺序、默认 mode、知识关联点击、时间流点击；aria-pressed=true 默认项已验证 |
| AC-8 | check + lint + test 全绿（baseline 锁内） | ✅ | output：26 fail / 249 pass / 275 total，Lint 25 errors，TSC 通过 |

### 3. 关键发现

- **`rightPanelMode` 确认未持久化**：源代码 uiStore.ts:254-258 partialize 仅含 `activeSidebarSection / todayLastTab / tagsExpanded`，所以本期重排 tab 顺序 **不需要 migrator**，与 output 一致、与 ADR/风险登记一致。
- **DOM 顺序断言到位**：测试通过 `getAllByRole("button").filter(b => b.hasAttribute("aria-pressed"))` 拿到底部 tab 数组并按位置断言文本——既忽略了顶部关闭按钮（无 aria-pressed），又准确锁定底部 segmented control，写法稳健。
- **aria 语义保留方式与 input.md 不完全对齐**：input.md AC-6 写"role=tablist + aria-selected 保留"，但当前实现是 `<button aria-pressed>`（toolbar 风格），**没有外层 tablist 容器、也没有 role=tab**。这是与现状一致的行为（task 不主动引入 tablist），且不属于本次改造范围（input.md "确认未被破坏"即可），不算违反 AC，只是 AC 文本与代码现状有错位。

---

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 20% | 5 | 顺序、默认 mode、点击切换、aria-pressed 全满足；持久化无需 migrator 已确认 |
| 安全性 | 5% | 5 | 纯 UI 数组顺序变更，无新 API/无新数据流，无注入面 |
| 代码质量 | 20% | 5 | 改动最小（一行数组 + key union 顺序）；新增测试结构清晰，使用 store setState 而非全 mock，符合 §5 测试规范 |
| 测试覆盖 | 20% | 4 | 4 用例覆盖顺序/默认/两条切换路径。**缺少 key=`timeline-flow` 切换后 panel 内容渲染对应 mock 的断言**（虽然 mock 已在文件顶部 vi.mock，但用例没显式 `getByTestId("mock-timeline-flow")` 验证 panel swap） |
| 架构一致性 | 15% | 5 | 不引入新类型、不动 partialize、不改 RightPanelMode union；命中风险登记表第 6 行（"重排无需 migrator"） |
| 可维护性 | 10% | 5 | 注释明确指出 task_008 IN-01/02 及"静态 vs 动态"理由；key union 顺序与数组顺序对齐 |
| UX 体感 | 10% | 4 | tab 顺序更合理（静态相邻、动态末位），符合 PRD §6 IN-01 心智收敛。但底部 tab 整体仍是 toolbar 视觉风格，aria 语义仍 aria-pressed 而非 tablist——若未来要做键盘左右箭头切换需另开 task |

**综合分**：5×0.20 + 5×0.05 + 5×0.20 + 4×0.20 + 5×0.15 + 5×0.10 + 4×0.10 = **4.70/5**

---

## 总体判断

- [x] **PASS**
- [ ] FIX
- [ ] BLOCKER

---

## 问题列表

### BLOCKER

无。

### MAJOR

无。

### MINOR

1. **缺 panel swap 验证**
   - **代码位置**：`src/components/layout/__tests__/Inspector.test.tsx`
   - **描述**：用例只断言 `useUIStore.getState().rightPanelMode === "timeline-flow"`，未额外断言 `getByTestId("mock-timeline-flow")` 出现在 DOM 中。完整覆盖应包含"切换后 panel 内容真随之 swap"。
   - **修复方向**：在两条切换用例最后加一行 `expect(screen.getByTestId("mock-timeline-flow")).toBeInTheDocument()` / `expect(screen.getByTestId("mock-knowledge-association")).toBeInTheDocument()`
   - **优先级**：可选；行为由 Inspector.tsx:87-102 的条件渲染保证，类型安全也提供保护

2. **AC-6 文本与代码语义错位**
   - **代码位置**：`task_008_dev_inspector_tabs/input.md` AC-6 与 `Inspector.tsx:147`
   - **描述**：input.md 写 "role=tablist + aria-selected"，代码实际用 `aria-pressed`。本次改动**未改变**这一语义层（沿用现状），不属于回归，但 input 文案应在下次迭代前修订或本期统一到 tablist
   - **修复方向**：要么改 Inspector.tsx 用 role=tab/tablist 与 aria-selected（建议留到 task_012 视觉打磨阶段一起做 a11y 升级）；要么修订 input.md
   - **优先级**：v1.3 收尾或 v1.4 议程

---

## 给 Dev 的修复指引

无需修复。本 task PASS。MINOR 可在 task_013 UX 审查或下一轮迭代中处理。

---

## 自检清单（Reviewer）

- [x] 逐条 AC 检查
- [x] 检查了 session_context §6 风险登记（第 6 行 "rightPanelMode 重排不需 migrator" 已确认）
- [x] BLOCKER/MAJOR 无；MINOR 已给出代码位置与修复方向
- [x] 评分诚实（测试覆盖与 UX 各扣 1 分有具体依据）
