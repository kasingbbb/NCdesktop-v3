# Task 输入 — task_008_dev_inspector_tabs

## 目标

将 `Inspector.tsx` 的 TABS 数组从 `[inspector, timeline-flow, knowledge_association]` 重排为 `[inspector, knowledge_association, timeline-flow]`。让"详情"+"知识关联"两个静态信息 tab 相邻，"时间流"动态 tab 沉到末位。

## 前置条件

- 依赖 task：**无**（与其他 P1 task 并行）
- 必须先存在的文件/接口：
  - `src/components/layout/Inspector.tsx`
  - `src/stores/uiStore.ts`（`rightPanelMode` 字段）

## 验收标准（Acceptance Criteria）

1. **AC-1**：Inspector 顶部 segmented control 渲染顺序为：详情 → 知识关联 → 时间流
2. **AC-2**：点击任一 tab，`useUIStore.getState().rightPanelMode` 正确切换到对应值
3. **AC-3**：默认初始 tab 仍为"详情"（`rightPanelMode === "inspector"`）
4. **AC-4**：若用户上次停在 `"timeline-flow"`，rehydrate 后仍保留——但 `rightPanelMode` **未被持久化**（已确认 uiStore.partialize 当前不含此字段），所以本 AC 自动满足
5. **AC-5**：切换 tab 时无视觉闪烁（panel 内容平滑切换）
6. **AC-6**：`role="tablist"` + `aria-selected` 保留；键盘左右箭头能切换 tab（如现有实现已支持，确认未被破坏）
7. **AC-7**：单测覆盖：① TABS 顺序 ② 点击切换正确 ③ aria 属性正确
8. **AC-8**：`pnpm check` + `pnpm lint` + `pnpm test` 全绿

## 技术约束

- **不引入新 TABS 类型**：复用现有 `RightPanelMode` union（`"inspector" | "timeline-flow" | "knowledge_association"`）
- **不动 store 持久化**：rightPanelMode 不进 partialize（保持现状）
- **TABS 数组**：在 Inspector.tsx 顶部声明 `const TABS: readonly RightPanelMode[] = ["inspector", "knowledge_association", "timeline-flow"] as const;`
- **样式与现有 SegmentedControl 一致**：不引入新样式

## 参考文件

- `src/components/layout/Inspector.tsx`（当前实现）
- `src/types/index.ts`（RightPanelMode 定义）
- `product/prd/notecapt-v1.3-ui_prd_v1.md` §6 IN-01, IN-02

## 预估影响范围

- **修改文件**：
  - `src/components/layout/Inspector.tsx`（仅改 TABS 顺序常量）
  - 可能：`src/components/layout/__tests__/Inspector.test.tsx`（新建或扩展）

- **新建文件**：可能上述测试文件

---

## Reviewer 重点关注项

- 确认重排后 InspectorDetails / InspectorAI 等子组件仍正确根据 `rightPanelMode` 渲染
- 键盘 a11y 不被破坏
- 用户视觉断点切换流畅
