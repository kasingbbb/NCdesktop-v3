# Review Scorecard — task_004_dev_sidebar_knowledge_merge

## 审查思考过程

1. **Task 意图**：将 Sidebar 中的「知识库 + 技能」两个 SidebarItem 合并为单一「知识」入口（label 取「知识中心」沿用历史中文），点击 → `setSidebarSection("knowledge-hub") + navigateHub("concepts")`（ADR-001）；右侧 hub badge `素材·概念·库` 由三个 store 的 `.length` 聚合，符合 PRD §4.2 SB-03"零数据零信号"。ADR-007 决断细化为"全 0 不渲染、任一 > 0 都渲染"。

2. **AC 检查结果**：
   - AC-1 Sidebar 只有一个 KnowledgeHub 入口（label "知识中心"）：✅（`Sidebar.tsx:88-97` 单一 `<Network/>` SidebarItem）
   - AC-2 旧"知识库"和"技能" SidebarItem 已删：✅（grep 验证）
   - AC-3 点击触发 `setSidebarSection("knowledge-hub")` + hash 跳 `#/knowledge-hub/concepts`：✅（`Sidebar.tsx:93-96`；Sidebar.test SB-02 用例 PASS）
   - AC-4 active 状态在 `activeSidebarSection === "knowledge-hub"` 时为 true（涵盖所有 4 个 step）：✅（`const inHub = activeSidebarSection === "knowledge-hub"`，不耦合 hash 子串，覆盖所有子 step）
   - AC-5 三者都 > 0 时 badge 渲染 `${a}·${c}·${l}`：✅（Sidebar.test "至少一个 >0 时渲染 '3·2·0'" PASS）
   - AC-6 任一为 0 时整条 badge 不渲染：⚠️ **ADR-007 显式偏离** — 改为"全 0（三者全 0）才不渲染、任一 > 0 都渲染"。该偏离已在 ADR-007 中记录上下文（与历史"3·2·0"用例兼容），并由 PM 同意。结果：input.md 字面 AC-6 与新行为不一致；历史 AC-7 "0·0·0" 用例 fail（在容忍清单内）。
   - AC-7 单测覆盖 ①合并后只有一个入口 ②都 >0 显示 ③任一为 0 不显示 ④点击触发 store/hash：①PASS ②PASS（"3·2·0"）③**只测了"全 0"分支，未测"两个 >0、一个 = 0"的中间分支** ④PASS。第 ③ 项是 AC 覆盖空白。
   - AC-8 全量 pnpm check/lint/test：✅（vitest 26 fail / baseline 锁 ≤ 26；lint 25 / 锁 ≤ 25；tsc 通过）

3. **关键发现**：
   - **store 订阅严格 select length**：`useAssetStore((s) => s.assets.length)` / `useKnowledgeStore((s) => s.concepts.length)` / `useLibraryStore((s) => s.libraries.length)` — 完全符合 session_context §6 第 5 条"禁止订阅整张表"。✅
   - **hubBadge 用 useMemo 聚合**：`Sidebar.tsx:37-40` 在 useMemo 中做全 0 判断，不在 JSX 写条件链。符合 input.md 技术约束。✅
   - **SidebarItem badge prop a11y**：当前 badge 在 SidebarItem 内仅渲染为 `<span class="...">{badge}</span>`，**未带 aria-label 解释三段含义**（input.md Reviewer 关注项建议"`aria-label="120 素材，47 概念，12 知识库"`"）— 该建议未实现，是 MINOR 项。
   - **ADR-007 偏离影响**：input.md AC-6 字面要求与代码不符，但 ADR-007 在 progress.md 关键决策中显式记录，并被 PM 列在容忍清单内。合理。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 20% | 4 | AC-1/2/3/4/5/8 满足；AC-6 因 ADR-007 偏离（PM 已同意）；AC-7 ③缺"两个 >0、一个 = 0"中间分支测试 |
| 安全性 | 5% | 5 | 纯前端读 store length，无新数据流；hash 写入用 `pushState` 不 reload；零 XSS 风险 |
| 代码质量 | 20% | 5 | hubBadge useMemo 边界清晰；`inHub` 派生用一行常量；store selector 严格 length；无行内 hex |
| 测试覆盖 | 20% | 4 | 6 个 PR-B 用例覆盖核心路径 + ADR-007 偏离行为；缺 ①badge 边界"两 >0 一 = 0"中间分支 ②active 在 hash 切换到 library/skills/assets 子 step 时仍 true（AC-4 未直接测） |
| 架构一致性 | 15% | 5 | navigateHub 目标按 ADR-001 取 `"concepts"`；不引入 SidebarSection 新 union 值；store 订阅模式符合 §6；ADR-007 显式记录偏离原因 |
| 可维护性 | 10% | 4 | useMemo deps 完整；命名"hubBadge"语义清晰；扣分：SidebarItem badge 缺 a11y aria-label（屏幕阅读器读到 "3·2·0" 不知何意） |
| UX 体感 | 10% | 5 | 单一"知识"入口对应北极星"工作区 × 知识链条"；badge 三段式简洁；零数据零信号满足 |

**综合分**：(4·0.20)+(5·0.05)+(5·0.20)+(4·0.20)+(5·0.15)+(4·0.10)+(5·0.10) = 0.80+0.25+1.00+0.80+0.75+0.40+0.50 = **4.50/5**（加权）

## 总体判断

- [x] **PASS**

## 问题列表

### BLOCKER
（无）

### MAJOR
（无）

### MINOR
1. **SidebarItem badge 缺 a11y aria-label**
   - 代码位置：`src/components/layout/SidebarItem.tsx:20-27`
   - 现象：badge 仅渲染纯文本 `3·2·0`，屏幕阅读器读出"三点二点零"无法理解三段含义
   - 建议：在按钮上加 `aria-describedby` 或在 SidebarItem 接受 `badgeAriaLabel?: string` slot，Sidebar.tsx 处传入 `${assetCount} 素材, ${conceptCount} 概念, ${libraryCount} 库`
   - 验证标准：renderer 包 `<button aria-describedby="...">` 关联到隐藏文本节点；testing-library 用 `getByRole("button", { description: /素材.+概念.+库/ })` 可命中
2. **AC-6 中间分支测试缺失**
   - 代码位置：`src/components/layout/__tests__/Sidebar.test.tsx:174-199`
   - 现象：只测了"全 0 不渲染"与"全 >0 渲染"，缺"两个 >0、一个 = 0"（如 assets=3 / concepts=0 / library=0 → 应渲染 `3·0·0`，按 ADR-007 当前实现）
   - 建议：补一个用例，固化 ADR-007"任一 > 0 都渲染"的行为契约，避免后续若有人想回滚到"任一为 0 不渲染"时无测试托底
   - 验证标准：用例命名如"SB-03：assets=3 / concepts=0 / library=0 → badge='3·0·0' 仍渲染"且 PASS
3. **navigateHub 签名残留 `"skills"`**
   - 代码位置：`src/components/layout/Sidebar.tsx:19`
   - 现象：union 包含已被合并的 `"skills"`，本组件内无引用
   - 建议：v1.4 清理（不属本 task scope）

## 给 Dev 的修复指引

（PASS，无需修复；MINOR 留作 v1.4 跟进）
