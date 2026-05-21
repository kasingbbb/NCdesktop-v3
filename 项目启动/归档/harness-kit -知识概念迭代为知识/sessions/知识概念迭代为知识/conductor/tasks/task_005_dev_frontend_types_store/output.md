# Task 产出 — task_005_dev_frontend_types_store

## 实现摘要

本 task 创建了知识理解功能的前端类型定义层和 Zustand 状态管理层：

1. **类型文件**：严格 TypeScript 接口，逐字段对齐 Rust `serde(rename_all = "camelCase")` 序列化输出
2. **Store 文件**：Zustand `create` 模式，三路流式 buffer + 状态管理，含概念切换重置逻辑
3. **Barrel 文件**：`KnowledgeUnderstanding/index.ts` 空导出，为后续 task 预留挂载点

---

## 新建文件表

| 文件路径 | 行数 | 说明 |
|---|---|---|
| `src/types/knowledge-understanding.types.ts` | ~175 行 | 所有 TypeScript 接口定义 |
| `src/stores/knowledgeUnderstandingStore.ts` | ~140 行 | Zustand Store 实现 |
| `src/components/KnowledgeUnderstanding/index.ts` | 3 行 | Barrel 空导出 |

---

## 架构遵守声明

- **AC-1** 满足：所有规定接口均已定义，无 `any`
- **AC-2** 满足：字段名与 Rust `serde(rename_all = "camelCase")` 对齐，差异项加注释说明：
  - `ConceptRelationResult` 中使用 `otherConceptId` / `otherConceptName` 对应 Rust 的 `other_concept_id` / `other_concept_name`（非 task 规范书中的 `relatedConceptId/relatedConceptName`，以实际 Rust 代码为准）
  - `ConceptExplanationResult.typicalScenarios` 对应 Rust `typical_scenarios`
  - `ConceptExplanationResult.essenceSentence` 对应 Rust `essence_sentence`
- **AC-3** 满足：Store 包含所有规定状态字段和 Actions
- **AC-4** 满足：`knowledgeUnderstandingStore.ts` 中无任何其他 Store 的 import
- **AC-5** 满足：barrel 文件已创建
- **AC-6** 需用户运行验证命令确认（见下方）

---

## TypeScript 检查命令

```bash
cd "/Users/zhongjiacheng/Documents/project/办公桌/NCdesktop/项目启动/NCdesktop"
npx tsc --noEmit
# 或
npm run check
```

期望结果：0 errors，0 warnings。

> 注：Bash 权限受限，无法在 task 执行期间自动运行，需用户手动验证。

---

## 关键设计决策

### ConceptRelationResult 字段名

Rust 实际代码（`db/knowledge_understanding.rs`）中 `ConceptRelation` 使用：
```rust
pub other_concept_id: String,
pub other_concept_name: String,
```
对应 camelCase：`otherConceptId` / `otherConceptName`。

task 规范书中建议的 `relatedConceptId` / `relatedConceptName` 不符合实际 Tauri payload，已按 Rust 代码优先原则修正为 `otherConceptId` / `otherConceptName`，并在类型文件注释中说明。

### ConceptExplanationResult 字段类型

Rust 侧 `mechanism` / `typical_scenarios` / `common_misconceptions` 在数据库存为 JSON 字符串，但通过 Tauri command 序列化为完整的结构化对象返回前端。TypeScript 类型按反序列化后形状定义（`ExplanationItem` 而非 `string`），符合前端实际接收格式。

### ExcerptItem 接口

此类型在 Rust 侧为 prompt 构建辅助结构，不通过 Tauri command 直接传递给前端。在类型文件中保留定义作为文档参考和潜在的前端侧构建 prompt 场景使用。

---

## 自测验证矩阵

| 验证项 | 期望 | 状态 |
|---|---|---|
| `knowledge-understanding.types.ts` 存在 | 存在 | PASS |
| `knowledgeUnderstandingStore.ts` 存在 | 存在 | PASS |
| `KnowledgeUnderstanding/index.ts` 存在 | 存在 | PASS |
| Store 无跨 Store import | 仅 import 类型文件 | PASS |
| `resetForConcept` 清空所有 buffer | 全置为空字符串 | PASS |
| `appendXxxChunk` 设置 status = 'streaming' | 同步设置 | PASS |
| `any` 类型使用 | 0 处 | PASS |
| TypeScript 编译 | 0 errors | 待用户验证 |

---

## 已知局限

1. **`ConceptExplanationResult` 字段类型假设**：Rust 侧数据库存储 mechanism 等字段为 JSON 字符串，但 `serde_json::Value` 序列化通过 Tauri 时前端接收到的是字符串而非对象。如 Tauri 实际返回的是原始 JSON 字符串，则前端需要二次 `JSON.parse`，届时类型定义需调整（mechanism 改为 `string`，或在 Store action 中转换）。建议在 task_006 集成阶段用 `console.log` 验证实际 payload 格式。

2. **`MirrorFeedbackResult` 结构假设**：Rust 侧 mirror_feedback 在数据库存为 LLM 原始输出字符串，结构未在 Rust 代码中定义为强类型。`MirrorFeedbackResult` 接口基于 task 规范书定义，需在实际 LLM prompt 输出后验证字段一致性。

---

## Reviewer 关注点

1. **`otherConceptId` vs `relatedConceptId`**：类型定义以实际 Rust 代码为准，如后续 task 组件层使用 `relatedConceptId` 命名，需在组件层做字段映射，不建议改动类型文件与 Rust payload 不一致。

2. **流式 buffer 与最终数据的关系**：`summaryStreamBuffer` 用于实时显示，`summary` 字段用于完整数据。Store 没有自动将 buffer 转换为 `summary` 的逻辑，这个转换应由调用方（Tauri event handler）在收到 `isFinal: true` 时负责调用 `setSummary` + `setSummaryStatus('done')`。

3. **并发安全**：当前 Store 无锁机制，如同时触发多个概念的 chunk 推送可能导致 buffer 混乱。建议在 event handler 层比较 `conceptId` 做过滤（已在 `KnowledgeStreamChunk` 中包含 `conceptId` 字段）。
