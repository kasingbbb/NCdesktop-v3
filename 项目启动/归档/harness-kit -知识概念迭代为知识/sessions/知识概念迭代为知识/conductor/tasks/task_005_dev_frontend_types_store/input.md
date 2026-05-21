# Task 输入 — task_005_dev_frontend_types_store

## 目标

创建前端 TypeScript 类型定义文件和 `knowledgeUnderstandingStore`（Zustand），为后续所有前端 UI task 提供类型安全的数据层和状态管理基础。

---

## 前置条件

- 依赖 task：**task_003_dev_rust_commands**（Rust 侧 Command 签名和 Tauri Event payload 格式确定）
- 必须先存在的文件/接口（均已确认存在）：
  - `src-tauri/src/commands/knowledge_understanding.rs`（task_003 产出，读取 Rust 结构体定义对应 TypeScript 接口）
  - `src-tauri/src/db/knowledge_understanding.rs`（task_003 产出，包含 ConceptSummary / ConceptExplanation / ConceptUserNote / ConceptRelation struct）
  - `src/stores/knowledgeStore.ts`（已有 v2.1 知识 Store，参考 Zustand create API 模式）
  - `src/types/knowledge.ts`（已有 v2.1 知识类型，参考接口定义风格）
  - `src/components/` — 已有 `features/`、`common/`、`layout/` 三个子目录（新建 `KnowledgeUnderstanding/` 与之并列）

> **Host 架构校正**：Architect 规划的 `knowledge/commands.rs` 不存在，实际文件是 `commands/knowledge_understanding.rs` 和 `db/knowledge_understanding.rs`。

---

## 验收标准（Acceptance Criteria）

1. **AC-1**：`src/types/knowledge-understanding.types.ts` 存在，包含以下所有 TypeScript 接口（严格类型，无 `any`）：
   - `ConceptSummaryResult`
   - `ExplanationItem`（`{ text: string; source: string }`）
   - `ConceptExplanationResult`
   - `UserNoteResult`
   - `FeedbackPerspective`（`{ text: string; source: string }`）
   - `MirrorFeedbackResult`
   - `ConceptRelationResult`
   - `UnderstandingData`
   - `StreamingStatus`（`'idle' | 'streaming' | 'done' | 'error'`）
   - `KnowledgeUnderstandingState`（Store 状态类型）

2. **AC-2**：所有接口字段与 Rust 侧的 `serde::Serialize` 结构体字段名完全一致（camelCase，因为 serde 默认或通过 `rename_all = "camelCase"` 转换）。如字段名有差异，在此文件中加注释说明对应 Rust 字段名。

3. **AC-3**：`src/stores/knowledgeUnderstandingStore.ts` 存在，包含：
   - 当前展示的 `conceptId`（`string | null`）
   - `summaryStatus`（`StreamingStatus`）
   - `explanationStatus`（`StreamingStatus`）
   - `mirrorStatus`（`StreamingStatus`）
   - `summary`（`ConceptSummaryResult | null`）
   - `explanation`（`ConceptExplanationResult | null`）
   - `userNote`（`UserNoteResult | null`）
   - `mirrorFeedback`（`MirrorFeedbackResult | null`）
   - `relations`（`ConceptRelationResult[]`）
   - `summaryStreamBuffer`（`string`，流式中间内容）
   - `explanationStreamBuffer`（`string`）
   - `mirrorStreamBuffer`（`string`）
   - Actions：`setConceptId`、`setSummary`、`setExplanation`、`setUserNote`、`setMirrorFeedback`、`setRelations`、`appendSummaryChunk`、`appendExplanationChunk`、`appendMirrorChunk`、`setStatus`（或分别的 `setSummaryStatus` 等）、`resetForConcept`（切换概念时重置状态）

4. **AC-4**：`knowledgeUnderstandingStore` 不 import 任何其他 Zustand Store（如 conceptStore、assetStore 等），符合 Zustand 跨 Store 约束。

5. **AC-5**：`src/components/KnowledgeUnderstanding/index.ts`（barrel export 文件）创建，初始为空 export（后续 task 添加组件后补充）。

6. **AC-6**：TypeScript 编译（`tsc --noEmit`）无错误。

---

## 技术约束

- **TypeScript 严格模式**：所有接口字段必须有明确类型，禁止 `any`；可选字段用 `?` 或 `| null`（与 Rust 的 `Option<T>` 对应）
- **Zustand 版本**：使用项目已有的 Zustand 版本和 `create` API（不升级，不切换 API 风格）
- **不跨 Store import**：`knowledgeUnderstandingStore.ts` 文件中不出现其他 Store 的 import 语句
- **命名规范**：
  - 类型文件：`src/types/knowledge-understanding.types.ts`（kebab-case，与已有类型文件命名风格一致）
  - Store 文件：`src/stores/knowledgeUnderstandingStore.ts`（camelCase，与已有 Store 文件命名一致）
- **`StreamingStatus` 的用途**：用于前端显示加载骨架屏（streaming）、内容（done）、错误提示（error）；idle 为初始状态
- **`resetForConcept(conceptId: string)`**：切换到新概念时调用，清空所有缓存数据和 buffer，设置新的 conceptId，所有 status 重置为 'idle'

---

## 参考文件

- **Rust 命令签名**：`sessions/知识概念迭代为知识/conductor/tasks/task_001_architect/output.md` — 「API 设计」章节（所有 Rust 结构体定义）
- **Tauri Event 格式**：同上 — Command 3 `knowledge_validate_explanation` 的流式事件说明（`{ concept_id: String, chunk: String, is_final: bool }`）
- **已有 Store 实现**：阅读 `src/stores/` 目录下已有的 Store 文件，确认使用的 Zustand API 模式（`create` + `set` + `get` 等）
- **已有类型定义**：阅读 `src/types/` 目录下已有文件，确认命名规范和接口定义风格

---

## 预估影响范围

**新建文件**：
- `src/types/knowledge-understanding.types.ts`（约 80-120 行）
- `src/stores/knowledgeUnderstandingStore.ts`（约 80-120 行）
- `src/components/KnowledgeUnderstanding/index.ts`（空 barrel，约 1-5 行）

**修改文件**：
- 无（此 task 不修改任何已有文件）
