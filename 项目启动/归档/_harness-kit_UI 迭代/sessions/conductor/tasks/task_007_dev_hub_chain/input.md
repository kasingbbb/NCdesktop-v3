# Task 输入 — task_007_dev_hub_chain

## 目标

将 `KnowledgeHubView/StepNav` 从平级 4 chip 升级为"链条 + 计数"：步骤间插 `›` chevron 分隔符（aria-hidden），每个 step 右侧显示当前计数（count > 0 时才显示数字）。同时把 `DEFAULT_HUB_STEP` 从 `assets` 改为 `concepts`。父组件 useMemo 聚合四个 store 长度后透传 `counts` prop 给 StepNav。

## 前置条件

- 依赖 task：**无**（与 task_002~006 并行）
- 必须先存在的文件/接口：
  - `src/components/features/KnowledgeHubView/index.tsx`
  - `src/components/features/KnowledgeHubView/types.ts`
  - `src/components/features/KnowledgeHubView/useHubHashRoute.ts`（不动）
  - 四个数据源 store（assetStore、knowledgeStore、knowledgeUnitsStore、skillsStore 或对应文件）

## 验收标准（Acceptance Criteria）

1. **AC-1**：`types.ts` 中 `DEFAULT_HUB_STEP === "concepts"`
2. **AC-2**：访问 `#/knowledge-hub`（无 step）或空 hash，落到 `concepts` step（依赖 `DEFAULT_HUB_STEP`，useHubHashRoute 不动）
3. **AC-3**：访问 `#/knowledge` 旧 hash 仍迁移到 `library`（migrateLegacyHash 不动 — 加单测验证）
4. **AC-4**：访问 `#/skills` 旧 hash 仍迁移到 `skills`（同上）
5. **AC-5**：StepNav 接受新 prop `counts?: Partial<Record<HubStep, number>>`，签名向后兼容
6. **AC-6**：父组件 `KnowledgeHubView` 用 useMemo 聚合四个 store 的 length 后透传 counts
7. **AC-7**：每个 step 在 DOM 上有 `data-step={step}` 属性（便于 e2e selector）
8. **AC-8**：每个 step 之间渲染 `<span aria-hidden="true">›</span>`，颜色 `var(--text-tertiary)`，字号 12px
9. **AC-9**：每个 step 的 count > 0 时，在 label 右侧渲染 `<span class="step-count">{n}</span>`（font-family mono，font-size 11px，背景 `var(--surface-tertiary)`，颜色 `var(--text-tertiary)`，padding `1px 6px`，border-radius 8px）
10. **AC-10**：count === 0 或 undefined 时，**不渲染**任何 count span
11. **AC-11**：active step 的样式：背景 `var(--surface-tertiary)` + `font-weight: 600`；inactive 字重 400，hover 时 bg 变 `var(--surface-secondary)`
12. **AC-12**：StepNav 容器 padding-y 由 `var(--space-2)` 改为 `var(--space-3)`；border-bottom 保留
13. **AC-13**：`role="tablist"` + `aria-selected` 保持；chevron sep 不参与 tab 顺序（已 aria-hidden）
14. **AC-14**：扩展 KnowledgeHubView.test 新增用例：① 默认 step 为 concepts ② migrateLegacyHash "#/knowledge" → library 不变 ③ counts 透传给 StepNav（mock 四个 store 长度，断言 DOM 中数字渲染）④ count === 0 不渲染数字
15. **AC-15**：`pnpm check` + `pnpm lint` + `pnpm test` 全绿

## 技术约束

- **不重写 useHubHashRoute**：仅改 types.ts 中的 DEFAULT_HUB_STEP 常量
- **不改 migrateLegacyHash 函数体**：仅加单测确认其行为
- **counts useMemo 守护**：父组件中 counts 必须 useMemo 依赖四个 length，避免每次 render 创建新对象
- **store 订阅**：用 `useStore(s => s.items.length)` 选择器订阅长度，**不订阅整张表**
- **不引新依赖**
- **样式全走 CSS var**：禁止行内 hex

## 参考文件

- `src/components/features/KnowledgeHubView/index.tsx`（现有 StepNav 实现，34-92 行）
- `src/components/features/KnowledgeHubView/types.ts`（HUB_STEPS / DEFAULT_HUB_STEP）
- `src/components/features/KnowledgeHubView/useHubHashRoute.ts`（不动，但要 grep 看 migrateLegacyHash 的导出）
- `src/components/features/KnowledgeHubView/KnowledgeHubView.test.tsx`（已存在，扩展）
- `product/prd/notecapt-v1.3-ui_prd_v1.md` §5 + ADR-001/004

## 预估影响范围

- **修改文件**：
  - `src/components/features/KnowledgeHubView/types.ts`（改 1 行常量）
  - `src/components/features/KnowledgeHubView/index.tsx`（StepNav 重写 + 父组件聚合 counts）
  - `src/components/features/KnowledgeHubView/KnowledgeHubView.test.tsx`（扩展）

- **新建文件**：无

---

## Reviewer 重点关注项

- counts useMemo 依赖数组是否完整（四个 length）
- chevron sep 是否真 `aria-hidden`，不破坏 tablist 顺序
- count === 0 真的不渲染 span，而不是渲染但 `display:none`（影响 a11y）
- 父组件订阅是否只 select length，避免 re-render
- `pushState + popstate` 现有逻辑不被破坏（前进后退仍可用）
