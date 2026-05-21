# Task 输入 — task_007_dev_ui_understanding_page

## 目标

实现深入理解页面主容器（KnowledgeUnderstandingPage）以及「你的文档怎么说」（SummarySection）和「理解框架」（ExplanationSection）两个区域，包含首屏加载、流式渲染、来源链接、「查看依据」功能和顶部透明度声明横幅。

---

## 前置条件

- 依赖 task：
  - **task_005_dev_frontend_types_store**（类型和 Store 就绪）
  - **task_006_dev_ui_entry_discovery**（入口按钮和页面切换机制已就绪）
- 必须先存在的文件/接口：
  - `src/stores/knowledgeUnderstandingStore.ts`
  - `src/types/knowledge-understanding.types.ts`
  - `src/components/KnowledgeUnderstanding/index.ts`
  - Tauri `invoke`、`listen` API（已有项目中必然存在，确认导入路径）

---

## 验收标准（Acceptance Criteria）

1. **AC-1**：`KnowledgeUnderstandingPage.tsx` 挂载时调用 `invoke('knowledge_get_understanding_data', { conceptId })` 获取缓存数据：
   - 若 `summary` 有缓存（`UnderstandingData.summary !== null`），直接渲染，不触发新 LLM 调用
   - 若 `summary` 无缓存，自动触发 `invoke('knowledge_generate_summary', { conceptId, forceRegenerate: false })`，并监听 `"knowledge:summary:chunk"` 事件实现流式渲染

2. **AC-2**：`ExplanationSection` 的触发策略：
   - 理解框架**不在页面挂载时自动触发**（用户首次打开深入理解页面只自动触发 summary）
   - ExplanationSection 初始显示一个「生成理解框架」按钮，用户点击后触发 `invoke('knowledge_generate_explanation', ...)`
   - 若 `explanation` 已有缓存，直接渲染缓存内容（不显示触发按钮）

3. **AC-3**：`TransparencyBanner.tsx` 存在，在页面顶部显示固定的透明度声明：「以下解释基于你的文档由 AI 生成，AI 可能有理解偏差——点击来源链接查看原文对照」；样式为黄色/橙色警示框，视觉上明显。

4. **AC-4**：`SummarySection` 渲染规范：
   - 标题「你的文档怎么说」
   - 流式渲染中显示骨架屏（StreamingStatus = 'streaming'）
   - 渲染完成显示摘要文本 + 每个来源的文档名标注
   - 「展开原文」按钮：点击后展开对应的原始引用段落（可使用折叠/展开 UI 交互）
   - 「重新生成」按钮：点击后调用 `invoke('knowledge_generate_summary', { forceRegenerate: true })`

5. **AC-5**：`ExplanationSection` 渲染规范（4个模块）：
   - 核心机制（mechanism）：文本 + 来源链接 + 「查看依据」按钮
   - 典型场景（typical_scenarios）：列表，每项有来源链接
   - 常见误区（common_misconceptions）：列表，若为空或 null 不渲染该区域（不显示空区块）
   - 一句话精华（essence_sentence）：文本，标注「根据你的文档总结」

6. **AC-6**：`ExplanationItem.tsx` 存在，渲染单条解释条目；`SourceEvidence.tsx` 存在，「查看依据」点击后展示对应原文段落（从 `concept_cases.excerpt` 或 `concept_viewpoints.summary` 中获取对应来源数据）。

7. **AC-7**：性能要求：
   - 首屏（有 summary 缓存时）：页面从打开到 summary 内容可见 ≤ 500ms
   - 流式渲染（无缓存时）：首字出现 ≤ 3s（依赖 LLM API，前端必须在收到第一个 chunk 后立即渲染）

8. **AC-8**：错误状态处理：
   - LLM 调用失败（`StreamingStatus = 'error'`）时显示友好错误提示「生成失败，请检查网络或重试」+ 重试按钮
   - 无来源文档时（`KnowledgeError::NoSourceDocuments`）显示「该概念暂无文档内容，无法生成解释」

9. **AC-9**：「重新生成」按钮在 `ExplanationSection` 右上角可见（对应 PRD 线框图中的 `[重新生成]` 位置）。

---

## 技术约束

- **流式监听生命周期**：Tauri Event listener 必须在组件 unmount 时取消监听（`useEffect` 返回 cleanup 函数调用 `unlisten()`），防止内存泄漏
- **Store 更新**：流式 chunk 追加通过 `knowledgeUnderstandingStore.appendSummaryChunk(chunk)` 更新，渲染时读取 `summaryStreamBuffer`；流结束（`is_final: true`）时将完整结果写入 `summary` 字段，清空 buffer
- **来源链接**：点击文档来源链接应跳转到原文（复用已有的文档跳转逻辑；若无法实现，至少显示文档名作为非交互文本，AC 中该行降为 MINOR）
- **`SourceEvidence` 的数据获取**：「查看依据」展示的原文段落来自已有的 `concept_cases.excerpt` 数据（通过已有的 conceptStore 或专门查询获取）；`SourceEvidence` 组件通过 props 接收原文数组，不自行 invoke Command
- **不引入新依赖**：不引入新的 React UI 组件库；使用项目已有的 CSS/Tailwind 和交互模式
- **TypeScript 严格类型**：所有组件 props 有明确接口，无 `any`
- **Zustand 约束**：组件通过 hook 使用 `knowledgeUnderstandingStore`，需要概念基础信息（名称、来源文档等）时通过 props 接收（由父级从其他 Store 读取后传入）

**关键 UI 文字（不可修改）**：
- 透明度声明：`以下解释基于你的文档由 AI 生成，AI 可能有理解偏差——点击来源链接查看原文对照`
- 摘要区域标题：`你的文档怎么说`
- 理解框架区域：`核心机制`、`典型场景`、`常见误区`、`一句话精华`
- 空状态（无文档）：`该概念暂无文档内容，无法生成解释`
- 精华标注：`（根据你的文档总结）`

---

## 参考文件

- **PRD 功能 2（文档整合摘要）**：`sessions/知识概念迭代为知识/prd/knowledge_evolution_prd_v1.md` — 「功能 2：文档整合摘要」章节
- **PRD 功能 3（理解框架）**：同文件 — 「功能 3：理解框架生成」章节（透明度要求、内容模块、LLM 策略）
- **PRD 页面布局线框图**：同文件 — 「四、深入理解页面整体布局」（对照 ASCII 线框图确认组件位置）
- **技术方案 API Command 1、2**：`sessions/知识概念迭代为知识/conductor/tasks/task_001_architect/output.md` — Command 1（generate_summary）和 Command 2（generate_explanation）的 Tauri Event 格式
- **Store 定义**：`src/stores/knowledgeUnderstandingStore.ts`
- **类型定义**：`src/types/knowledge-understanding.types.ts`

---

## 预估影响范围

**新建文件**：
- `src/components/KnowledgeUnderstanding/KnowledgeUnderstandingPage.tsx`（约 100-150 行）
- `src/components/KnowledgeUnderstanding/TransparencyBanner.tsx`（约 20-30 行）
- `src/components/KnowledgeUnderstanding/SummarySection.tsx`（约 80-120 行）
- `src/components/KnowledgeUnderstanding/ExplanationSection.tsx`（约 100-150 行）
- `src/components/KnowledgeUnderstanding/ExplanationItem.tsx`（约 40-60 行）
- `src/components/KnowledgeUnderstanding/SourceEvidence.tsx`（约 40-60 行）

**修改文件**：
- `src/components/KnowledgeUnderstanding/index.ts`：添加所有新组件的 export
