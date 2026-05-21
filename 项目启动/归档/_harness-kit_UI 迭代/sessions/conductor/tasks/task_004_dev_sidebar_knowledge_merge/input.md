# Task 输入 — task_004_dev_sidebar_knowledge_merge

## 目标

将 Sidebar 中的"知识库"+ "技能"两个 SidebarItem 合并为单一**"知识"**入口（图标 `<Network/>`）。点击 → `setSidebarSection("knowledge-hub")` + `navigateHub("concepts")`。右侧渲染 hub badge `素材·概念·库`（mono 字号 11px，`var(--text-tertiary)` 颜色，`·` 分隔），但**任一计数为 0 时整条 badge 不渲染**。

## 前置条件

- 依赖 task：**task_003**（避免合并冲突）
- 必须先存在的文件/接口：
  - `src/components/layout/Sidebar.tsx`（task_003 修改后状态）
  - `src/stores/assetStore.ts`、`src/stores/knowledgeStore.ts`、`src/stores/knowledgeUnitsStore.ts` 等（用于读 length）

## 验收标准（Acceptance Criteria）

1. **AC-1**：Sidebar 主导航中只存在**一个**与 KnowledgeHub 相关的入口，label 为 "知识"
2. **AC-2**：原"知识库"和"技能" SidebarItem 均已从 DOM 中删除
3. **AC-3**：点击"知识"入口后，`useUIStore.getState().activeSidebarSection === "knowledge-hub"` 且 `window.location.hash === "#/knowledge-hub/concepts"`
4. **AC-4**："知识"入口的 `active` 状态在 `activeSidebarSection === "knowledge-hub"` 且 hash 以 `#/knowledge-hub/` 开头时为 true（包括 concepts / library / skills / assets 任一）
5. **AC-5**：当 `assetCount > 0 && conceptCount > 0 && libraryCount > 0`（三者都 > 0）时，"知识"入口右侧渲染 badge 文本 `${assetCount}·${conceptCount}·${libraryCount}`
6. **AC-6**：当三者任一为 0 时，**整条 badge 不渲染**（DOM 无 badge 容器）
7. **AC-7**：单测覆盖：① 合并后只有一个知识入口 ② 三计数都 >0 时 badge 显示 ③ 任一为 0 时不显示 ④ 点击触发正确的 store 和 hash 变更
8. **AC-8**：`pnpm check` + `pnpm lint` + `pnpm test` 全绿

## 技术约束

- **store 订阅**：用 `useStore(s => s.items.length)` 选择器订阅长度，**不订阅整张表**（性能约束，session_context §6 第 5 条）
- **counts useMemo**：在 Sidebar 组件内用 `useMemo` 把三个数字组合为 `hubBadge?: string`；只在三者都 > 0 时返回非 undefined
- **navigateHub 目标**：必须是 `"concepts"`（ADR-001）
- **不要新增 SidebarSection union 值**：复用 `"knowledge-hub"`
- **SidebarItem 接受 badge prop**：如 SidebarItem 当前无 badge 槽，需为它加 optional `badge?: ReactNode` prop（不改其他行为）
- **样式**：badge 容器 `font-family: var(--font-mono)`、`font-size: 11px`、`color: var(--text-tertiary)`、与 label 间距 `var(--space-2)`

## 参考文件

- `src/components/layout/Sidebar.tsx:97-114`（现有"知识库"和"技能"两个 SidebarItem）
- `src/components/layout/Sidebar.tsx:15-21`（现有 navigateHub helper）
- `src/components/layout/SidebarItem.tsx`（看是否已有 badge slot）
- `src/stores/assetStore.ts`、`src/stores/knowledgeStore.ts`、`src/stores/knowledgeUnitsStore.ts`（数据源）
- `product/prd/notecapt-v1.3-ui_prd_v1.md` §4.2 SB-02, SB-03 + ADR-001

## 预估影响范围

- **修改文件**：
  - `src/components/layout/Sidebar.tsx`（合并 + badge 计算）
  - 可能：`src/components/layout/SidebarItem.tsx`（加 badge 槽 prop）
  - 新增：`src/components/layout/__tests__/Sidebar.test.tsx`（如 task_003 已建则扩展）

- **新建文件**：可能上述 Sidebar.test.tsx

---

## Reviewer 重点关注项

- Sidebar 是否真的只 select 长度（不订阅整张 store）
- badge "任一为 0 整条不渲染" 的实现是否在 useMemo 中做（不在 JSX 里做条件链）
- "知识"入口的 active 检测涵盖所有 4 个子 step（不只 library）
- SidebarItem badge prop 的 a11y：badge 文本对屏幕阅读器友好（建议 `aria-label="120 素材，47 概念，12 知识库"`）
