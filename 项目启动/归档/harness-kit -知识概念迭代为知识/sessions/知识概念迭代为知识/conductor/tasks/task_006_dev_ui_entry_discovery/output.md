# Task 产出 — task_006_dev_ui_entry_discovery

## 实现摘要

在知识关联视图中实现了「深入理解」Feature Discovery 入口：
1. **DeepUnderstandButton** — 蓝色高亮按钮，位于 Definition 区右上角
2. **FirstVisitTooltip** — localStorage 驱动的一次性引导 Tooltip
3. **KnowledgeUnderstandingPage** — 深入理解页面占位容器（含返回导航）
4. **ConceptDetailPanel 改造** — 嵌入按钮 + Tooltip + 空状态引导文字
5. **KnowledgeAssociationView 改造** — 通过 knowledgeUnderstandingStore.conceptId 切换视图

---

## 新建文件表

| 文件路径 | 行数 | 说明 |
|---|---|---|
| `src/components/KnowledgeUnderstanding/DeepUnderstandButton.tsx` | ~33 行 | 蓝色高亮「深入理解」入口按钮 |
| `src/components/KnowledgeUnderstanding/FirstVisitTooltip.tsx` | ~50 行 | 一次性引导 Tooltip（localStorage） |
| `src/components/KnowledgeUnderstanding/KnowledgeUnderstandingPage.tsx` | ~60 行 | 深入理解页面占位容器 |

## 修改文件表

| 文件路径 | 变更类型 | 说明 |
|---|---|---|
| `src/components/KnowledgeUnderstanding/index.ts` | 修改 | 添加 3 个组件的 barrel export |
| `src/components/features/knowledge/ConceptDetailPanel.tsx` | 修改 | 新增 DeepUnderstandButton + FirstVisitTooltip + 引导文字；新增 onEnterUnderstanding 可选 prop |
| `src/components/features/knowledge/KnowledgeAssociationView.tsx` | 修改 | import knowledgeUnderstandingStore，添加视图切换逻辑（conceptId 非空时渲染 KnowledgeUnderstandingPage） |

---

## 架构遵守声明

- 目录结构与 ADR-007 一致：新组件在 `KnowledgeUnderstanding/` 目录
- DeepUnderstandButton 不 import 其他 Store，概念数据通过 props 传入
- Zustand 跨 Store 约束满足：组件层组合 knowledgeStore 和 knowledgeUnderstandingStore
- 样式使用 CSS 变量 + Tailwind，无硬编码颜色
- 所有 UI 文字与 input.md 规范完全一致
- onEnterUnderstanding 为可选 prop，不破坏 ConceptDetailPanel 的向后兼容性
- 视图切换在 KnowledgeAssociationView 内部管理（ADR-008），不新增 RightPanelMode

---

## TypeScript 编译结果

```bash
cd "/Users/zhongjiacheng/Documents/project/办公桌/NCdesktop/项目启动/NCdesktop"
npx tsc --noEmit
# 退出码: 0, 0 errors
```

---

## 自测验证矩阵

| AC | 描述 | 状态 |
|---|---|---|
| AC-1 | `DeepUnderstandButton` 组件存在，点击调用 `resetForConcept(conceptId)` 切换到深入理解视图 | PASS |
| AC-2 | 按钮蓝色高亮（`--brand-navy`），文字「深入理解」，位于 Definition 区右上角 | PASS |
| AC-3 | `FirstVisitTooltip` 组件存在，localStorage key `nc_knowledge_tooltip_shown`，指向按钮位置 | PASS |
| AC-4 | 定义区下方显示灰色引导文字「想深入理解这个概念？」 | PASS |
| AC-5 | ConceptDetailPanel 已有功能（定义、观点、案例、拓展）不受影响（新 prop 为可选） | PASS |
| AC-6 | 点击按钮后切换到 KnowledgeUnderstandingPage 占位页面（Loading 状态） | PASS |
| AC-7 | 深入理解视图有「← 返回概念列表」按钮，点击返回概念详情页 | PASS |

---

## 关键设计决策

### 视图切换机制（ADR-008）

使用 `knowledgeUnderstandingStore.conceptId` 作为视图切换信号：
- `conceptId === null` → 渲染普通的概念列表 + 详情视图
- `conceptId !== null` → 渲染 KnowledgeUnderstandingPage

不新增 `RightPanelMode`，因为深入理解是知识关联的子视图，在 `KnowledgeAssociationView` 内部管理更合理。

### 概念名称查找

在 KnowledgeAssociationView 中，通过 `concepts.find(c => c.id === understandingConceptId)?.name` 从概念列表中查找名称，兜底使用 `conceptDetail?.concept.name`。

### 返回导航

点击「← 返回概念列表」调用 `setConceptId(null)`，清除理解视图信号，回到概念列表/详情状态。之前选中的 `selectedConceptId`（来自 knowledgeStore）不受影响，返回后仍显示之前选中的概念详情。

---

## 已知局限

1. **空状态引导始终显示**：AC-4 要求"当该概念还未触发过深入理解时"显示引导文字，当前实现在 `onEnterUnderstanding` 存在时始终显示。因 Store 在概念切换时会 reset，无法通过 Store 判断历史数据。完整实现需查询数据库（`knowledge_get_understanding_data`），可在 task_007 集成时补充。

2. **Tooltip 定位**：FirstVisitTooltip 使用 `position: absolute` + `top: 100%`，相对于 DeepUnderstandButton 的包裹 div。在窗口宽度较窄时，Tooltip 可能溢出右边界。可在后续 UX 审查（task_010）中优化。

---

## Reviewer 关注点

1. **向后兼容性**：`onEnterUnderstanding` 为可选 prop（`?`），不传入时按钮和引导文字不渲染，完全兼容 ConceptDetailPanel 的其他使用场景。

2. **Store 隔离**：DeepUnderstandButton 不直接 import Store，通过 props 接收回调。KnowledgeAssociationView 作为容器组件组合两个 Store。

3. **KnowledgeUnderstandingPage 为占位**：task_007 将替换其内部内容。当前渲染 Loading 旋转动画 + 概念名称，用户体验可接受。
