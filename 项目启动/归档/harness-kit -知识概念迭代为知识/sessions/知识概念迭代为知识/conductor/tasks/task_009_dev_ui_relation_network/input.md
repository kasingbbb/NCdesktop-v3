# Task 输入 — task_009_dev_ui_relation_network

## 目标

实现概念关系网络区域（RelationNetworkSection），以卡片列表形式展示与当前概念共现的关联概念，并将 v2.1 的 upstream/downstream 关系集成进来，点击关联概念可导航到该概念详情页。

---

## 前置条件

- 依赖 task：
  - **task_004_dev_rust_co_occurrence**（`concept_relations` 表已有数据，`knowledge_get_relations` Command 已就绪）
  - **task_007_dev_ui_understanding_page**（KnowledgeUnderstandingPage 主容器已就绪，RelationNetworkSection 将嵌入其中）
- 必须先存在的文件/接口：
  - `src/stores/knowledgeUnderstandingStore.ts`（含 `relations` 字段和 `setRelations` action）
  - `src/types/knowledge-understanding.types.ts`（含 `ConceptRelationResult` 接口）
  - `src/components/KnowledgeUnderstanding/KnowledgeUnderstandingPage.tsx`
  - 已有的概念导航机制（阅读已有代码确认如何导航到指定概念详情页）

---

## 验收标准（Acceptance Criteria）

1. **AC-1**：`RelationNetworkSection.tsx` 存在，在 `KnowledgeUnderstandingPage` 挂载时调用 `invoke('knowledge_get_relations', { conceptId, limit: 8 })` 获取关联概念列表，存入 `knowledgeUnderstandingStore.relations`。

2. **AC-2**：卡片列表渲染规范：
   - 区域标题：「在你的知识库里，这个概念连接了：」
   - 每张卡片显示：
     - 关联概念名称（粗体，可点击）
     - 关系描述文字（见下方类型对应规则）
   - 按 `co_occurrence_count` 降序排列（已由 Rust Command 排序，前端直接渲染）
   - 最多显示 8 条

3. **AC-3**：关系类型对应的 UI 文字（严格遵守，不可使用「紧密相关」「深层联系」等词）：
   - `co_occurrence`：显示「一起出现在 [文档名1]、[文档名2]」（多个文档时用顿号连接，最多显示 2 个文档名，超过显示「等 N 个文档」）
   - `upstream`：显示「前置知识」（带不同颜色标签区分）
   - `downstream`：显示「应用方向」（带不同颜色标签区分）

4. **AC-4**：点击关联概念名称，导航到该概念的详情页（使用已有的概念导航机制；具体实现方式由 Dev 参考已有代码确定）。

5. **AC-5**：空状态（`relations` 为空数组时）：显示「暂时还没发现相关概念。随着你导入更多文档，关联会逐渐丰富。」，灰色小字样式。

6. **AC-6**：性能要求：关系网络区域从页面挂载到内容可见 ≤ 100ms（纯数据库查询，无 LLM 调用，应远优于此目标）。

7. **AC-7**：`RelationNetworkSection` 嵌入 `KnowledgeUnderstandingPage` 中，位于 UserNotesSection 下方（页面最底部，按 PRD 线框图）。

8. **AC-8**：`upstream`/`downstream` 类型的卡片与 `co_occurrence` 类型视觉上有区分（颜色标签或边框颜色不同），便于用户区分关系类型。

---

## 技术约束

- **无 LLM 调用**：`RelationNetworkSection` 的数据完全来自 `knowledge_get_relations` Command（SQLite 查询），不触发任何 LLM 调用
- **UI 措辞底线**：文字中只能说「一起出现在」，严禁「紧密相关」「深层联系」「强关联」等词——这是 PRD 和 Debate 的明确共识
- **导航机制**：点击关联概念应复用已有的导航逻辑（不自行发明新的路由方式）；若已有代码通过 `conceptId` 参数导航，则直接使用 `related_concept_id` 触发；同时应调用 `knowledgeUnderstandingStore.resetForConcept(relatedConceptId)` 切换到新概念
- **Zustand 约束**：组件通过 `knowledgeUnderstandingStore.relations` 读取数据，通过 `setRelations` 写入；不 import 其他 Store
- **错误状态**：若 `knowledge_get_relations` 调用失败，在区域内显示「加载失败，请刷新」，不 crash 整个页面
- **TypeScript 严格类型**：`ConceptRelationResult` 中的 `source_asset_names` 为 `string[]`，渲染时注意处理空数组

**关键 UI 文字（不可修改）**：
- 区域标题：`在你的知识库里，这个概念连接了：`
- 共现描述前缀：`一起出现在`
- 前置知识标签：`前置知识`
- 应用方向标签：`应用方向`
- 空状态：`暂时还没发现相关概念。随着你导入更多文档，关联会逐渐丰富。`

---

## 参考文件

- **PRD 功能 5（概念关系网络）**：`sessions/知识概念迭代为知识/prd/knowledge_evolution_prd_v1.md` — 「功能 5：概念关系网络（共现版）」章节（UI 规范、透明度声明、v2.1 upstream/downstream 集成）
- **PRD 页面线框图**：同文件 — 「四、深入理解页面整体布局」（关系网络在页面底部的位置）
- **技术方案 Command 7**：`sessions/知识概念迭代为知识/conductor/tasks/task_001_architect/output.md` — `knowledge_get_relations` 签名（`ConceptRelationResult` 结构）
- **Store 和类型**：`src/stores/knowledgeUnderstandingStore.ts`、`src/types/knowledge-understanding.types.ts`
- **已有概念导航逻辑**：需 Dev 自行定位（搜索概念列表点击处理逻辑，确认导航到概念详情的方式）

---

## 预估影响范围

**新建文件**：
- `src/components/KnowledgeUnderstanding/RelationNetworkSection.tsx`（约 80-120 行）

**修改文件**：
- `src/components/KnowledgeUnderstanding/KnowledgeUnderstandingPage.tsx`：在 UserNotesSection 下方嵌入 `<RelationNetworkSection>`
- `src/components/KnowledgeUnderstanding/index.ts`：添加 `RelationNetworkSection` 的 export
