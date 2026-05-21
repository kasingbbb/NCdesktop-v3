# Task 输入 — task_008_dev_ui_user_notes_mirror

## 目标

实现「用你自己的话解释这个概念」区域（UserNotesSection），包含带自动保存的自由文本输入框、「给我一个出发点」草稿生成，以及「和 AI 核对一下」触发的 AI 镜子反馈展示（MirrorFeedbackDisplay），措辞严格遵循探索式原则。

---

## 前置条件

- 依赖 task：**task_007_dev_ui_understanding_page**（KnowledgeUnderstandingPage 主容器已就绪，UserNotesSection 将嵌入其中）
- 必须先存在的文件/接口：
  - `src/stores/knowledgeUnderstandingStore.ts`（含 `userNote`、`mirrorFeedback`、`mirrorStatus` 等字段）
  - `src/types/knowledge-understanding.types.ts`（含 `UserNoteResult`、`MirrorFeedbackResult` 接口）
  - `src/components/KnowledgeUnderstanding/KnowledgeUnderstandingPage.tsx`（task_007 产出，本 task 在其中嵌入 UserNotesSection）

---

## 验收标准（Acceptance Criteria）

1. **AC-1**：`UserNotesSection.tsx` 存在，包含：
   - 标题文字「用你自己的话解释这个概念」
   - 自由文本 `<textarea>`（多行，可调整高度）
   - 初始值：若 `knowledgeUnderstandingStore.userNote.user_explanation` 非空，预填充该值

2. **AC-2**：自动保存功能：
   - 用户停止输入 **1000ms** 后自动触发 `invoke('knowledge_save_user_note', { conceptId, userExplanation })`
   - 使用 `debounce`（lodash 或手写，不引入新依赖）实现
   - 保存进行中显示「正在保存...」，保存成功显示「已保存」（2s 后消失），保存失败显示「保存失败」

3. **AC-3**：「给我一个出发点」按钮：
   - 点击后触发（使用已有的 LLM Command 或临时使用 `knowledge_generate_summary` 的结果作为草稿起点——具体实现方式由 Dev 根据已有 Command 能力判断，不强制要求新增 Command）
   - 生成的草稿填入 textarea（用户可直接编辑），不覆盖用户已填写的内容（若 textarea 非空，弹窗确认或追加到末尾）

4. **AC-4**：「和 AI 核对一下」按钮：
   - 仅在 textarea 有内容（非空字符串，trim 后非空）时可点击；无内容时按钮 disabled
   - 点击后：先调用 `invoke('knowledge_save_user_note', ...)` 确保最新内容已保存，再调用 `invoke('knowledge_validate_explanation', { conceptId, userExplanation })`
   - 调用期间按钮显示「核对中...」并 disabled，`mirrorStatus` = 'streaming'

5. **AC-5**：`MirrorFeedbackDisplay.tsx` 存在，当 `mirrorStatus = 'done'` 时渲染镜子反馈：
   - 显示「你的解释捕捉到了 [covered_count] 个核心要点 ✓」
   - 显示「在你的文档里，还有一些关于这个概念的角度你可能感兴趣：」+ `additional_perspectives` 列表（每项带来源标注）
   - 若 `difference_note` 非 null，显示「你的理解和文档的一个细微差异是：[difference_note]」
   - **禁止出现以下词语**：「不完整」「有误」「遗漏了」「错误」「不正确」「你缺少」（若 LLM 返回包含这些词，前端显示通用探索式措辞替代，或在此处加 FIXME 注释标注待 Reviewer 关注）

6. **AC-6**：`MirrorFeedbackDisplay` 流式渲染：监听 `"knowledge:mirror:chunk"` 事件，实时追加 `mirrorStreamBuffer`；收到 `is_final: true` 后解析完整 JSON，渲染结构化结果。

7. **AC-7**：数据隔离验证：`UserNotesSection` 的 textarea 内容不修改 `concepts.definition` 字段（纯前端验证：确认 `invoke` 调用的是 `knowledge_save_user_note` 而非任何 concept 更新 Command）。

8. **AC-8**：`UserNotesSection` 嵌入 `KnowledgeUnderstandingPage` 中，位于 ExplanationSection 下方（按 PRD 线框图顺序）。

---

## 技术约束

- **1s debounce 自动保存**：用 `useRef` + `setTimeout`/`clearTimeout` 手写，或使用 lodash `debounce`（确认项目是否已有 lodash，若无则手写，不新增依赖）
- **「给我一个出发点」实现约束**：若实现需要新 LLM Command，优先考虑复用 `knowledge_generate_explanation` 的 `essence_sentence` 作为草稿起点（已有数据，零 LLM 成本）；若用户 explanation 为空，直接将精华句填入 textarea 作为出发点
- **镜子反馈 JSON 解析**：`mirror_feedback` 字段为 JSON 字符串（从 SQLite 读取）或直接是结构体（从 Command 返回），前端统一按 `MirrorFeedbackResult` 接口处理；若 JSON 解析失败，显示「反馈解析失败，请重试」
- **Zustand 约束**：`UserNotesSection` 使用 `knowledgeUnderstandingStore` 的 `setUserNote`、`setMirrorFeedback`、`appendMirrorChunk`、`setStatus` actions，不直接修改 Store 之外的状态
- **不修改 concepts 表**：此功能的任何 `invoke` 调用中不出现与 concept 定义更新相关的 Command
- **Tauri Event 监听生命周期**：`"knowledge:mirror:chunk"` 的监听在组件 unmount 或核对完成后取消
- **TypeScript 严格类型**：无 `any`

**关键 UI 文字（不可修改）**：
- 区域标题：`用你自己的话解释这个概念`
- 草稿按钮：`给我一个出发点`
- 核对按钮：`和 AI 核对一下`
- 核对中状态：`核对中...`
- 反馈开头：`你的解释捕捉到了 [N] 个核心要点 ✓`
- 附加视角：`在你的文档里，还有一些关于这个概念的角度你可能感兴趣：`
- 差异说明前缀：`你的理解和文档的一个细微差异是：`

---

## 参考文件

- **PRD 功能 4（用我的话说 + 镜子反馈）**：`sessions/知识概念迭代为知识/prd/knowledge_evolution_prd_v1.md` — 「功能 4」章节（自动保存规范、镜子反馈格式、禁止措辞列表、探索式措辞要求）
- **PRD Prompt 3（validate_user_explanation）**：同文件 — 「五、LLM Prompt 规范 - Prompt 3」（理解 LLM 返回的 JSON 格式）
- **技术方案 Command 3、5、6**：`sessions/知识概念迭代为知识/conductor/tasks/task_001_architect/output.md` — API 设计章节
- **Store 和类型**：`src/stores/knowledgeUnderstandingStore.ts`、`src/types/knowledge-understanding.types.ts`

---

## 预估影响范围

**新建文件**：
- `src/components/KnowledgeUnderstanding/UserNotesSection.tsx`（约 100-150 行）
- `src/components/KnowledgeUnderstanding/MirrorFeedbackDisplay.tsx`（约 60-80 行）

**修改文件**：
- `src/components/KnowledgeUnderstanding/KnowledgeUnderstandingPage.tsx`：在 ExplanationSection 下方嵌入 `<UserNotesSection>`
- `src/components/KnowledgeUnderstanding/index.ts`：添加 `UserNotesSection`、`MirrorFeedbackDisplay` 的 export
