# Task 输入 — task_006_dev_ui_entry_discovery

## 目标

在已有的概念详情页（ConceptDetail）中嵌入「深入理解」入口按钮（蓝色高亮）和一次性引导 Tooltip，实现 Feature Discovery，点击按钮后切换到深入理解页面容器。

---

## 前置条件

- 依赖 task：**task_005_dev_frontend_types_store**（knowledgeUnderstandingStore 和类型已就绪）
- 必须先存在的文件/接口：
  - `src/stores/knowledgeUnderstandingStore.ts`（task_005 产出）
  - `src/types/knowledge-understanding.types.ts`（task_005 产出）
  - `src/components/KnowledgeUnderstanding/index.ts`（task_005 产出）
  - 已有的 ConceptDetail 组件（需定位文件路径，阅读其结构，找到定义区域的渲染位置）

---

## 验收标准（Acceptance Criteria）

1. **AC-1**：`DeepUnderstandButton` 组件存在于 `src/components/KnowledgeUnderstanding/DeepUnderstandButton.tsx`，点击后通过 `knowledgeUnderstandingStore.setConceptId(conceptId)` 和页面导航/状态切换进入深入理解视图。

2. **AC-2**：按钮视觉规范满足：
   - 蓝色高亮样式（与页面其他灰色辅助按钮视觉上明显区分）
   - 文字为「深入理解」
   - 位置在概念详情页 Definition 区域右上角
   - 对所有已有概念的详情页均可见

3. **AC-3**：`FirstVisitTooltip` 组件存在于 `src/components/KnowledgeUnderstanding/FirstVisitTooltip.tsx`：
   - 第一次进入知识关联视图时显示，文字为「点击「深入理解」，让 AI 基于你的文档帮你真正理解这个概念」
   - 使用 `localStorage` 记录已展示状态（key 建议：`nc_knowledge_tooltip_shown`），确保后续进入不重复显示
   - Tooltip 指向「深入理解」按钮位置

4. **AC-4**：空状态引导文字：在 Definition 区域下方，当该概念还未触发过「深入理解」时（即 `knowledgeUnderstandingStore` 中无该概念的 summary/explanation 数据），显示弱引导文字「想深入理解这个概念？」，样式为灰色小字（不打断用户当前查找模式）。

5. **AC-5**：已有 ConceptDetail 页面的所有现有功能（定义展示、观点聚合、案例引用、知识拓展）不受影响，视觉和功能均与修改前一致（回归验证）。

6. **AC-6**：点击「深入理解」按钮后，页面切换到深入理解视图（该视图由 task_007 实现，本 task 只需要正确调用导航/状态切换，占位渲染一个 `KnowledgeUnderstandingPage` 组件的空壳或 Loading 状态即可）。

7. **AC-7**：深入理解视图有「← 返回概念列表」入口，点击后返回概念详情页（回到概念选中状态）。

---

## 技术约束

- **不修改 v2.1 已有功能逻辑**：对 ConceptDetail 组件的修改仅限于添加新元素（按钮、Tooltip、引导文字），不改变已有数据获取逻辑、渲染逻辑、样式
- **Tooltip 一次性显示机制**：使用 `localStorage` 实现（不需要数据库），简单可靠；不使用全局 Store 存储该状态（避免跨 Store 依赖问题）
- **Zustand 约束**：`DeepUnderstandButton` 只使用 `knowledgeUnderstandingStore`，不 import 其他 Store；概念详情数据（conceptId、conceptName 等）通过 props 传入
- **「返回」导航**：返回按钮触发 `knowledgeUnderstandingStore.resetForConcept` 或等价的状态清理，并回到概念列表/详情视图（具体导航方式与已有路由/页面切换机制一致）
- **样式**：遵循已有项目 CSS/Tailwind/样式方案（不引入新 CSS 框架）；蓝色按钮使用已有设计系统的 primary/accent 颜色 token
- **TypeScript 严格类型**：组件 props 必须有明确接口定义，无 `any`

**关键 UI 文字（不可修改）**：
- 按钮文字：`深入理解`
- Tooltip 文字：`点击「深入理解」，让 AI 基于你的文档帮你真正理解这个概念`
- 空状态引导：`想深入理解这个概念？`
- 返回按钮：`← 返回概念列表`

---

## 参考文件

- **PRD Feature Discovery 规范**：`sessions/知识概念迭代为知识/prd/knowledge_evolution_prd_v1.md` — 「功能 1：「深入理解」入口（Feature Discovery）」章节（按钮位置、样式、Tooltip、空状态规范）
- **PRD 页面布局线框图**：同文件 — 「四、深入理解页面整体布局」（ASCII 线框图，理解页面结构）
- **Store 类型**：`src/stores/knowledgeUnderstandingStore.ts`（task_005 产出）
- **已有 ConceptDetail 组件**：需 Dev 自行定位（搜索 `ConceptDetail`、`concept detail` 或 Definition 区域的关键字），阅读后确定修改位置

---

## 预估影响范围

**新建文件**：
- `src/components/KnowledgeUnderstanding/DeepUnderstandButton.tsx`（约 40-60 行）
- `src/components/KnowledgeUnderstanding/FirstVisitTooltip.tsx`（约 40-60 行）

**修改文件**：
- `src/components/KnowledgeUnderstanding/index.ts`：添加 `DeepUnderstandButton`、`FirstVisitTooltip` 的 export
- 已有 ConceptDetail 组件文件（待 Dev 定位）：添加 `<DeepUnderstandButton>`、`<FirstVisitTooltip>`、空状态引导文字
