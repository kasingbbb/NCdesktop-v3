# Task 交付 — task_007_dev_hub_chain

## 实现摘要

PR-C：KnowledgeHubView 链条化升级。
- `types.ts` 中 `DEFAULT_HUB_STEP` 从 `"assets"` 改为 `"concepts"`（ADR-001）
- `KnowledgeHubView/index.tsx` 父组件用 useMemo 聚合 assetStore / knowledgeStore / libraryStore / knowledgeUnitsStore 四个 store 的 length，作为 `counts` prop 传给 StepNav
- StepNav 升级：step 间渲染 `<span aria-hidden="true">›</span>` chevron 分隔符（color=var(--text-tertiary)）；每个 step button 加 `data-step={step}` 属性；count>0 时在 label 右侧渲染 mono 数字 span（class `step-count`，bg=var(--surface-tertiary)，font-size 11px）；count===0 仅渲染 label
- StepNav 容器 padding-y 由 var(--space-2) 调整到 var(--space-3)，保留底部 border
- `role="tablist"` + `aria-selected` 保留
- 同步更新 `KnowledgeHubView.test.tsx`（默认 step → concepts）+ `useHubHashRoute.test.ts`（5 个 fallback 用例 from `assets` → `concepts`）

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `src/components/features/KnowledgeHubView/types.ts` | 修改 | `DEFAULT_HUB_STEP = "concepts"` |
| `src/components/features/KnowledgeHubView/index.tsx` | 改写 | 父组件加四个 store selector + useMemo counts；StepNav 加 counts prop / chevron / count span / data-step / padding 调整 |
| `src/components/features/KnowledgeHubView/KnowledgeHubView.test.tsx` | 修改 | 用例 #1 改"默认渲染 concepts"；新增 3 用例（KH-02 chevron、KH-04 count===0、KH-05 data-step） |
| `src/components/features/KnowledgeHubView/useHubHashRoute.test.ts` | 修改 | 5 个 fallback 用例从 `assets` → `concepts`（反映 PRD DEFAULT_HUB_STEP 改动） |

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致
- [x] API 路径/命名与 Architect 方案一致（counts prop + data-step）
- [x] 数据模型与 Architect 方案一致（不动 store；用 selector 订阅 length）
- [x] 未引入计划外的新依赖
- 偏离说明：无

## 测试结果

- `pnpm vitest run src/components/features/KnowledgeHubView/`：**KnowledgeHubView.test 10/10 PASS + useHubHashRoute.test 15/15 PASS**
- 全量 vitest：**26 fail / 245 pass / 271 total**（baseline 锁 ≤ 26 ✅）
- Lint 25 errors ✅
- TSC 通过 ✅

净改善：useHubHashRoute.test 中 5 个之前 fail 的 fallback 用例现在 PASS。

## 自测验证矩阵

| 场景 | 状态 | 结果 |
|---|---|---|
| ✅ `DEFAULT_HUB_STEP === "concepts"` | 已测 | PASS |
| ✅ 访问 `#/knowledge-hub`（无 step） / 空 hash → 落 concepts | 已测 | PASS（useHubHashRoute.test 更新后用例） |
| ✅ 旧 hash `#/knowledge` → 仍迁移到 library（不变） | 已测 | PASS（migrateLegacyHash 用例） |
| ✅ 旧 hash `#/skills` → 仍迁移到 skills（不变） | 已测 | PASS（同上） |
| ✅ StepNav 接受 counts prop | 已测 | PASS（KH-04 用例） |
| ✅ chevron `›` aria-hidden 3 个 | 已测 | PASS（KH-02 用例） |
| ✅ count > 0 渲染 mono 数字 span | 间接 | 实现验证（counts useMemo + n>0 守卫） |
| ✅ count === 0 不渲染 step-count span | 已测 | PASS（KH-04 用例） |
| ✅ 每个 step button 有 data-step | 已测 | PASS（KH-05 用例） |
| ✅ role=tablist + aria-selected 保留 | 已测 | PASS（历史用例） |

## 已知局限

1. **Skill 计数来自 knowledgeUnitsStore.units.length**（全局，非 per-library 区分）：当用户切换 library 时 skill count 可能未精确反映当前 library 的技能数量。本期接受（PRD §5.2 只说"4 个 store 长度聚合"，未要求 cross-library 精确）。未来若需可扩展为 `useMemo` + 过滤逻辑

## 需要 Reviewer 特别关注的地方

- counts useMemo 依赖完整（4 个 length）
- chevron 真 aria-hidden，不破坏 tablist 顺序
- step-count span 在 count===0 时**真的不渲染**（不是 display:none）
