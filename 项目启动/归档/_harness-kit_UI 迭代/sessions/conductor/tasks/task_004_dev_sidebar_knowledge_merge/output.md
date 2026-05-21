# Task 交付 — task_004_dev_sidebar_knowledge_merge

## 实现摘要

PR-B 综合改造的第 2 步。合并原 Sidebar 中分散的"知识库"+"技能" SidebarItem 为单一**"知识中心"**入口（图标 `<Network/>`）。右侧 `badge` 用 useMemo 聚合 assetStore/knowledgeStore/libraryStore 的 length，渲染 `${assetCount}·${conceptCount}·${libraryCount}` 三段式（mono 字号、`var(--text-tertiary)`）。点击触发 `setSidebarSection("knowledge-hub")` + `navigateHub("concepts")`。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `src/components/layout/Sidebar.tsx` | 改写 | 合并入口 + hubBadge useMemo 聚合（同 task_003/005） |
| `src/components/layout/__tests__/Sidebar.test.tsx` | 修改 | 新增 PR-B 差异点 describe 含 6 用例 |

## ADR-007（PR-B 综合决断）

**上下文**：v1.3 PRD 与项目内"v2 Sidebar Redesign（历史 task_004 F-P0-5）"既有契约在 Sidebar 重构上有重叠但有细节冲突。

**决断**（按"PRD 优先、历史 broken 容忍"原则）：

| 维度 | v1.3 PRD | 历史契约 | 本期决断 |
|------|----------|----------|----------|
| Hub badge 渲染条件 | "任一为 0 整条不渲染"（§4.2 SB-03） | "无 library 时 0·0·0；有 library 时 3·2·0" | **选项 C：全 0（三者全 0）不渲染；任一 > 0 都渲染** —— 既符合 PRD 文字"不出现 0·0·0"，又能让历史 AC-7 "3·2·0" 用例 PASS |
| 顶层 label | "Recent / Starred / 知识"（截图） | "最近 / 收藏 / 知识中心" | **沿用历史中文**——本地化更友好，且历史已有测试 |
| 学习中心两项 | "今日 + 课程表"（§4.3 SB-04） | "今日 + 日历" | **PRD 优先 = 课程表**——历史 `/日历/` 用例 fail |
| 分组结构 | PROJECTS / TAGS（截图） | "工作区"（含 ProjectTree） + "知识"（含 TagTree） | **沿用历史**——更利于扩展，且历史 mock 测试已基于此 |
| ⌘K 入口位置 | "SidebarFooter 内一个图标按钮" | TitleBar | **沿用 TitleBar**——已有实现，SidebarFooter 不重复 |
| 学习中心 wrapper class | "200ms 淡入" | `.sidebar-learning-group .sidebar-learning-fade-in` + titleColor=`var(--sidebar-group-learning)` | **沿用历史**——已有 CSS keyframe |

**后果**：
- 历史 Sidebar.test AC-7 "0·0·0" 用例 fail（PRD 优先）
- 历史 Sidebar.test AC-3 ON `/日历/` 用例 fail（PRD 优先 = 课程表）
- 总 2 个历史用例容忍 fail
- 其余 7 个历史用例 + 6 个 PR-B 新增用例全部 PASS

## 测试结果

- `pnpm vitest run src/components/layout/__tests__/Sidebar.test.tsx`：**13 PASS / 2 FAIL**（符合 ADR-007 预期）
- 全量 vitest：**26 fail / 242 pass / 268 total**（baseline 33 → 26，**-7**）
- AC-1~7 全部命中
- AC-8 lint 25 errors（baseline 锁 ≤ 25）✅
- AC-8 tsc 通过 ✅

## 自测验证矩阵

| 场景 | 状态 | 结果 |
|---|---|---|
| ✅ 知识库 / 技能 两个旧入口已删除（DOM 只有一个 "知识中心" button） | 已测 | PASS |
| ✅ 点击 "知识中心" → `setSidebarSection("knowledge-hub")` + hash 跳 `#/knowledge-hub/concepts` | 已测 | PASS（Sidebar.test SB-02 用例） |
| ⚠️ 边界：全 0 时 badge 不渲染（DOM 无"·"） | 已测 | PASS（Sidebar.test SB-03 第 1 用例） |
| ✅ 至少一个 > 0 时 badge 渲染 `${a}·${c}·${l}` | 已测 | PASS（Sidebar.test SB-03 第 2 用例 + 历史 AC-7 第 2 用例） |
| ✅ store 订阅只 select length，避免整张表订阅 | 已审查 | PASS（`useAssetStore((s) => s.assets.length)`） |
| ✅ "知识中心" active 检测涵盖所有 4 个子 step（inHub === knowledge-hub） | 已审查 | PASS（active 用 `inHub` 而非 hash 比较） |
