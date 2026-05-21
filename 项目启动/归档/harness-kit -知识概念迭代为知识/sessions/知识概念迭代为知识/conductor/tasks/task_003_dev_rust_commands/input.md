# Task 输入 — task_003_dev_rust_commands

## 目标

在 Rust 侧实现 6 个 knowledge_ Tauri Commands（不含共现计算），包含 Prompt 模板模块、LLM 流式调用、SQLite 读写，以及完整的 Rust 类型定义，并注册到 Tauri invoke_handler。

---

## 前置条件

- 依赖 task：**task_002_dev_db_migration**（4张新表已在 `db/migration.rs` V4 中创建完毕）
- 必须先存在的文件/接口：
  - `src-tauri/src/commands/knowledge.rs` — **v2.1 已有**，包含 `get_concepts`、`get_concept_detail` 等命令，**不可修改其中已有代码**，新命令放入独立的 `commands/knowledge_understanding.rs`
  - `src-tauri/src/db/knowledge.rs` — **v2.1 已有**，包含 v2.1 数据结构和 CRUD，新数据结构/操作放入 `db/knowledge_understanding.rs`
  - `src-tauri/src/llm/client.rs` — LLMClient 结构体和配置解析（已确认存在）
  - `src-tauri/src/llm/chat.rs` — 流式 `chat_completion` 实现（已在 `commands/knowledge.rs` 中引用）
  - `src-tauri/src/llm/prompts.rs` — 已有 Prompt 集中管理文件，新 Prompt 追加到此文件

> **Host 架构校正**：Architect 规划的 `src/knowledge/` 模块与实际项目结构不符。实际项目遵循扁平模块结构（`commands/`、`db/`、`llm/`），新代码应在此结构内创建，而非建立独立的 `knowledge/` 模块。

---

## 验收标准（Acceptance Criteria）

1. **AC-1**：`src-tauri/src/commands/knowledge_understanding.rs` 存在，包含以下 6 个 `#[tauri::command]` 函数（pub），Rust 编译无错误：
   - `knowledge_generate_summary`
   - `knowledge_generate_explanation`
   - `knowledge_validate_explanation`
   - `knowledge_get_understanding_data`
   - `knowledge_save_user_note`
   - `knowledge_get_relations`

2. **AC-2**：`src-tauri/src/llm/prompts.rs`（已有文件）中新增 3 个 Prompt 构建函数：
   - `build_summary_prompt(concept_name: &str, excerpts: &[ExcerptItem]) -> String`
   - `build_explanation_prompt(concept_name: &str, definition: &str, sections: &[DocumentSection]) -> String`
   - `build_mirror_prompt(concept_name: &str, user_explanation: &str, key_points: &[KeyPoint]) -> String`
   - 三个 Prompt 文本与 PRD 第五章的规范一致（包含 CRITICAL RULES 约束）

3. **AC-3**：`knowledge_generate_summary` 调用时：
   - 若 `concept_summaries` 已有该 `concept_id` 的记录且 `force_regenerate = false`，直接返回缓存结果，不调用 LLM
   - 若无缓存或 `force_regenerate = true`，触发 LLM 调用，通过 Tauri Event `"knowledge:summary:chunk"` 流式推送 chunks，流结束后写入 `concept_summaries`

4. **AC-4**：`knowledge_generate_explanation` 调用时：
   - 缓存逻辑同 AC-3
   - LLM 返回的内容必须是合法 JSON（与 PRD Prompt 2 的 JSON schema 一致）；解析失败时返回 `KnowledgeError::InvalidLlmResponse`，不写入数据库
   - 写入前校验 `mechanism.source` 字段非空，否则返回 `KnowledgeError::InvalidLlmResponse`
   - 流式事件：`"knowledge:explanation:chunk"`

5. **AC-5**：`knowledge_validate_explanation` 调用时：
   - 触发 LLM 调用（不缓存，每次调用都生成新反馈）
   - 生成完成后将 `mirror_feedback` 序列化为 JSON 存入 `concept_user_notes.mirror_feedback`（upsert）
   - 流式事件：`"knowledge:mirror:chunk"`

6. **AC-6**：`knowledge_get_understanding_data` 返回包含 `summary`、`explanation`、`user_note` 三个 `Option` 字段的结构体；无数据时对应字段为 `None`（不报错）。

7. **AC-7**：`knowledge_save_user_note` 对 `concept_user_notes` 表执行 upsert（`INSERT OR REPLACE` 或 `ON CONFLICT DO UPDATE`），更新 `user_explanation` 和 `updated_at`，不修改 `mirror_feedback` 字段。

8. **AC-8**：`knowledge_get_relations` 按 `co_occurrence_count DESC` 排序，同时查询 `concept_a_id = ? OR concept_b_id = ?`，JOIN `concepts` 表获取对方概念名称，JOIN source assets 表获取文档名称（如相关表存在）。

9. **AC-9**：所有 6 个 Commands 已注册到 `src-tauri/src/main.rs` 的 `.invoke_handler(tauri::generate_handler![...])` 中。

10. **AC-10**：`KnowledgeError` 类型实现 `serde::Serialize`，前端可正确接收错误信息。

---

## 技术约束

- **Rust 版本**：与项目已有代码保持一致（不升级 Rust 工具链）
- **依赖**：不引入新的 Cargo crate；复用已有 HTTP client（reqwest 或等价库）、rusqlite、serde/serde_json、uuid 等
- **LLM 流式调用**：复用已有 `llmPreview` 或 `llmProbe` Command 的 HTTP streaming 实现，不重写；若已有实现可抽取为公共函数则抽取
- **Prompt 硬约束**：`prompts.rs` 中的系统 Prompt 必须包含 PRD 第五章规定的 CRITICAL RULES（特别是"ONLY use information from provided documents"和"cite source for EVERY point"）
- **来源字段校验**：`knowledge_generate_explanation` 在写入前必须校验 `mechanism` 字段的 `source` 非空字符串，否则拒绝写入
- **自动保存不覆盖镜子反馈**：`knowledge_save_user_note` 的 SQL 只更新 `user_explanation`、`updated_at`，不更新 `mirror_feedback`
- **错误处理原则**：只在 LLM API 调用和 SQLite IO 边界处理异常，内部逻辑（如 JSON 解析失败）通过 `?` 传播为 `KnowledgeError`
- **类型定义位置**：所有 `pub struct` 定义在 `commands.rs` 顶部或单独的 `types.rs` 文件（同 `knowledge/` 目录下），必须 `#[derive(Debug, serde::Serialize, serde::Deserialize)]`

**关键 Tauri Event 命名（前端依赖，不可改变）**：
- `"knowledge:summary:chunk"` — payload: `{ concept_id: String, chunk: String, is_final: bool }`
- `"knowledge:explanation:chunk"` — payload: `{ concept_id: String, chunk: String, is_final: bool }`
- `"knowledge:mirror:chunk"` — payload: `{ concept_id: String, chunk: String, is_final: bool }`

---

## 参考文件

- **PRD Prompt 规范（必读）**：`sessions/知识概念迭代为知识/prd/knowledge_evolution_prd_v1.md` — 第五章「LLM Prompt 规范」（3个完整 Prompt 模板）
- **技术方案 API 设计**：`sessions/知识概念迭代为知识/conductor/tasks/task_001_architect/output.md` — 「API 设计」章节（Rust 函数签名参考）
- **技术方案数据模型**：同上 — 「数据模型」章节（SQL Schema，字段约束）
- **已有 LLM Command 实现**：需在 `src-tauri/src/` 中定位 `llmProbe` 或 `llmPreview` 的实现文件（复用流式调用架构）
- **已有 Tauri Event 示例**：在已有代码中搜索 `app.emit` 找到事件推送的使用示例

---

## 预估影响范围

**新建文件**：
- `src-tauri/src/commands/knowledge_understanding.rs`（核心，约 400-600 行）
- `src-tauri/src/db/knowledge_understanding.rs`（新表的 CRUD 函数，约 150-200 行）

**修改文件**：
- `src-tauri/src/commands/mod.rs`：追加 `pub mod knowledge_understanding;`
- `src-tauri/src/db/mod.rs`：追加 `pub mod knowledge_understanding;`
- `src-tauri/src/llm/prompts.rs`：追加 3 个 Prompt 构建函数（不修改已有函数）
- `src-tauri/src/lib.rs`（或 `main.rs`）：在 `invoke_handler` 中注册 6 个新 Commands
