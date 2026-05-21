# Task 输入 — task_010_dev_empty_state

## 目标

建立全局空状态规范：
1. Audit `features/EmptyState.tsx`，确认 API 涵盖 `icon / title / hint / cta?`；不足则扩展
2. `TodayView.tsx` 顶部 0/0/0 计数栏在所有数字为 0 时**整行不渲染**
3. `TodayView.tsx` 去掉 emoji 🎉 与"恭喜/加油/今天没有 XXX"等感性文案，统一调用 `<EmptyState>`，文案降到中性陈述
4. 所有 step / list 空状态（包括 SkillsStep 空、ProjectTree 空、TagTree 空）统一调用 EmptyState

## 前置条件

- 依赖 task：**无**（与其他 P1 task 并行）
- 必须先存在的文件/接口：
  - `src/components/features/EmptyState.tsx`
  - `src/components/features/today/TodayView.tsx`

## 验收标准（Acceptance Criteria）

1. **AC-1**：`EmptyState` 组件签名（如需扩展）：`<EmptyState icon={ReactNode} title={string} hint?={string} cta?={ReactNode} />`
2. **AC-2**：TodayView 数据 `pendingCount + processingCount + doneCount === 0` 时，顶部计数栏（含三色数字 chip 的那一行）**整行不渲染**（不是 hidden）
3. **AC-3**：TodayView 全 0 数据下，主区域渲染 `<EmptyState icon={<Check/>} title="今日无待处理" hint="导入素材后这里会自动生成任务" />`（注意：不放 🎉，icon 用 lucide Check 或不放）
4. **AC-4**：grep `src/` 后，**🎉**、"恭喜"、"加油"、"今天没有"四个字面量在生产代码中（test 文件除外）出现次数为 0
5. **AC-5**：SkillsStep / ProjectTree / TagTree 等空状态统一调用 EmptyState（或确认已用，本期至少 SkillsStep 必须改）
6. **AC-6**：单测覆盖：① 全 0 时计数栏不渲染 ② EmptyState 在 TodayView 中按预期文案渲染 ③ DOM 不含 🎉 字符
7. **AC-7**：`pnpm check` + `pnpm lint` + `pnpm test` 全绿

## 技术约束

- **不引入新组件**：复用 `EmptyState`
- **图标**：用 lucide-react；若 PRD 文案"今日无待处理"觉得 `<Check/>` 不合，可不放 icon（icon 是可选）
- **文案规范**：所有空状态必须是"中性陈述"，单句，无感叹号、无 emoji
- **样式**：EmptyState 圆角 `var(--radius-md)`、padding `var(--space-9)`、center align、icon size 42px、title font-size 15px、hint font-size 12.5px、颜色用 `var(--text-tertiary)`
- **不动 EmptyState 现有 props 默认值**：仅在必要时新增可选 prop

## 参考文件

- `src/components/features/EmptyState.tsx`
- `src/components/features/today/TodayView.tsx`
- `src/components/features/KnowledgeHubView/steps/SkillsStep.tsx`
- `product/prd/notecapt-v1.3-ui_prd_v1.md` §7.2 ES-01 ~ ES-04

## 预估影响范围

- **修改文件**：
  - `src/components/features/EmptyState.tsx`（可能扩展 props）
  - `src/components/features/today/TodayView.tsx`
  - 可能：`src/components/features/KnowledgeHubView/steps/SkillsStep.tsx`
  - 可能：`src/components/features/ProjectTree.tsx` / `TagTree.tsx`（空状态统一）
  - `src/components/features/today/__tests__/TodayView.test.tsx`（新建或扩展）
  - 可能：`src/components/features/__tests__/EmptyState.test.tsx`（新建）

- **新建文件**：可能上述测试

---

## Reviewer 重点关注项

- 🎉 / 恭喜 / 加油 / "今天没有 XXX" 字面量全局清理是否彻底（grep src/ 应零命中，test 内允许）
- TodayView 计数栏不渲染**不是用 visibility: hidden**
- EmptyState API 扩展时未破坏现有调用者
- SkillsStep 空状态的 CTA 是否引用 EmptyState 的 cta 槽
