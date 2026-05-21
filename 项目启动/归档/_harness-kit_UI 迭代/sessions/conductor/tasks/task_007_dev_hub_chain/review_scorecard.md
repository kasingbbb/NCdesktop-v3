# Review Scorecard — task_007_dev_hub_chain

## 审查思考过程

### 1. Task 意图

把 KnowledgeHubView 的 StepNav 从 4 个平级 chip 升级为「链条 + 计数」：
- step 间插 `›` chevron（aria-hidden，不进 tab 顺序）
- 每个 step 右侧渲染当前 store length；count===0 完全不渲染 span（符合 P-04 零数据零信号）
- `DEFAULT_HUB_STEP` 从 `assets` → `concepts`（落实 ADR-001：知识入口默认落"概念"而非"素材"心智）
- 父组件用 useMemo 聚合四个 store 长度后透传 counts，避免每次 render 创建新对象
- 链路：assetStore / knowledgeStore / libraryStore / knowledgeUnitsStore 四个 length selector

### 2. AC 逐条检查

| AC | 内容 | 结果 | 证据 |
|----|------|------|------|
| AC-1 | `DEFAULT_HUB_STEP === "concepts"` | ✅ | types.ts:12 |
| AC-2 | 空 hash 落 concepts | ✅ | useHubHashRoute.test.ts:26、85 用例 PASS |
| AC-3 | `#/knowledge` → `library` 不变 | ✅ | useHubHashRoute.test.ts:44-49 |
| AC-4 | `#/skills` → `skills` 不变 | ✅ | useHubHashRoute.test.ts:38-43 |
| AC-5 | StepNav 接受 counts prop（向后兼容 `Partial<Record<HubStep, number>>`） | ✅ | index.tsx:85 |
| AC-6 | 父组件 useMemo 聚合 | ✅ | index.tsx:58-66 依赖 `[assetCount, conceptCount, libraryCount, skillCount]` 完整 |
| AC-7 | 每个 step button 有 `data-step={step}` | ✅ | index.tsx:114 + KH-05 用例断言四项 |
| AC-8 | step 间 `<span aria-hidden="true">›</span>` | ✅ | index.tsx:101-109，color `var(--text-tertiary)`，font-size 12px |
| AC-9 | count > 0 时 `<span class="step-count">` mono | ✅ | index.tsx:124-134 用 `font-mono text-[11px] tabular-nums`, bg `var(--surface-tertiary)`, color `var(--text-tertiary)` |
| AC-10 | count===0 / undefined 不渲染（不是 display:none） | ✅ | index.tsx:124 `{n > 0 && ...}` 是条件渲染，DOM 中不存在；KH-04 用例 `querySelector(".step-count")` Null 证实 |
| AC-11 | active bg `var(--surface-tertiary)` + font-weight 600；inactive 400 | ✅ | index.tsx:117-121 |
| AC-12 | StepNav 容器 `py-[var(--space-3)]` + border-bottom 保留 | ✅ | index.tsx:93-94 |
| AC-13 | `role="tablist"` + `aria-selected` + chevron aria-hidden | ✅ | index.tsx:91、112-113、103 |
| AC-14 | 新增 KH-02/04/05 三个用例 + 默认 step 改 concepts | ✅ | KnowledgeHubView.test.tsx:99-124 |
| AC-15 | check + lint + test 全绿（baseline 锁内） | ✅ | output.md：26 fail / 245 pass，Lint 25 errors，TSC 通过 |

### 3. 关键发现

- **counts useMemo 依赖完整且正确**（4 个 length 全列入）。store 订阅严格只 select `.length`，符合 session_context §5 "禁止订阅整张表"。
- **count===0 真不渲染**：`{n > 0 && (...)}` 是 React 短路渲染，DOM 树中根本没有 step-count 节点（不是 `display:none`）。KH-04 用例验证。
- **chevron aria-hidden 落实正确**：`aria-hidden="true"` 加在 chevron `<span>` 上，本身不是 button 也不在 tablist 内的 role="tab" 节点（StepNav 用 `<span>` 作为外层，每个 step 是 `<button role="tab">`）。tablist 顺序不被破坏。
- **store 订阅纯净**：四个 selector 全部是 `useStore((s) => s.xxx.length)`，避免对象/数组引用造成的 deep equal 误差。

---

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 20% | 5 | 15 条 AC 全部满足；DEFAULT_HUB_STEP、counts、chevron、data-step、padding 一一对应 |
| 安全性 | 5% | 5 | 纯 UI 重排无新攻击面；无 `dangerouslySetInnerHTML`；输入边界不变 |
| 代码质量 | 20% | 4 | StepNav 内联 style 配合 Tailwind 略冗长但符合「全走 CSS var」约束；命名清晰；STEP_LABELS 常量分离；count 用 `n ?? 0` 防 undefined |
| 测试覆盖 | 20% | 4 | KH-01/02/04/05 + 旧 hash 迁移 + 默认值用例都覆盖到。**KH-03（counts 透传渲染数字 > 0 的正向用例）只做了反向断言，未直接验证 store 非空时正向数字渲染**。output 自承"间接验证"——MINOR 缺口 |
| 架构一致性 | 15% | 5 | 严格命中 ADR-001（concepts 默认）和 ADR-004（counts useMemo + 0 不显示）；useHubHashRoute 未动；不引新依赖 |
| 可维护性 | 10% | 5 | 注释指明 task_007 / ADR / KH-01~05；STEP_LABELS 易扩展；类型签名向后兼容（counts 可选） |
| UX 体感 | 10% | 5 | 完全契合 P-04 "零数据零信号"（count=0 整段不出现）；hover/active 仍由 CSS var 驱动；chevron 视觉提供链条感 |

**综合分**：5×0.20 + 5×0.05 + 4×0.20 + 4×0.20 + 5×0.15 + 5×0.10 + 5×0.10 = **4.60/5**

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

1. **缺正向 counts 渲染用例**
   - **代码位置**：`src/components/features/KnowledgeHubView/KnowledgeHubView.test.tsx`
   - **描述**：当前 KH-04 只测了 count===0 时 `.step-count` 不存在的"反向断言"；没有显式构造 store 非空 → 断言 `getByText("3")` 之类的"正向"行为。input.md AC-14 ③ "mock 四个 store 长度，断言 DOM 中数字渲染"严格说差一截。
   - **修复方向**：在 `KnowledgeHubView.test.tsx` 顶部 mock 中改用 `vi.mock` + setState 注入非空 store（参照 Sidebar.test.tsx 写法），新增 `it("KH-03: count > 0 时渲染 mono 数字 span")`
   - **验证标准**：用例 `expect(within(tab).getByText("3")).toBeInTheDocument()`、`expect(tab.querySelector(".step-count")?.textContent).toBe("3")` 均通过
   - **优先级**：可选；当前 useMemo 与渲染逻辑代码简单且类型保证，反向断言 + 视觉自测足够规避主要风险

2. **skill count 全局非 per-library**（已在 output 已知局限标注）
   - **描述**：`useKnowledgeUnitsStore((s) => s.units.length)` 取的是全局技能数，切换 library 时不变化。output.md 自承本期接受。
   - **修复方向**：未来若改为 per-library，需把 skillCount 的来源换成基于 `libraryId` 的 selector
   - **优先级**：v1.4 议程，不阻塞本 task

---

## 给 Dev 的修复指引

无需修复。本 task PASS，可进入下一 task。MINOR #1 列入 v1.3 收尾或 task_013 UX 审查时一并补；MINOR #2 v1.4 议程。

---

## 自检清单（Reviewer）

- [x] 逐条 AC 检查
- [x] 检查了 session_context §6 领域审查重点（零数据零信号、令牌、订阅 length 而非整张表均合规）
- [x] BLOCKER/MAJOR 无；MINOR 已给出具体位置与修复方向
- [x] 评分诚实（功能/架构/UX 都给到 5，代码质量与测试覆盖各扣 1 分有具体依据）
