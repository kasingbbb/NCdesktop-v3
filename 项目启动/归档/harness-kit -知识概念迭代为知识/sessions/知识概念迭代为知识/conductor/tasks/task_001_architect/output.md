# 技术方案 — NCdesktop 知识功能迭代 v1.0

> 产出者：Architect
> 日期：2026-04-11
> 基于：PRD v1.0（knowledge_evolution_prd_v1.md）+ Session Context

---

## 项目概述

NCdesktop v2.1 已实现「概念词汇表」级别的知识关联功能（提取概念名称+一句话定义），但仅停留在 Bloom 分类法第一层（Remember）。本次迭代目标是在不破坏任何已有数据的前提下，通过增量添加 4 张新 SQLite 表和对应的 Tauri Commands，在概念详情页新增「深入理解」模式。该模式包含：基于用户文档的整合摘要、AI 结构化理解框架（核心机制/典型场景/常见误区/一句话精华）、用户自述与 AI 镜子反馈、以及基于共现关系的概念关联网络。所有 LLM 内容严格锚定用户文档，每条必须携带来源链接；所有写操作在 Rust 侧执行；用户手动编辑的内容不可被 AI 覆盖。

---

## 技术选型

| 选型维度 | 决策 | ADR |
|---------|------|-----|
| 后端写操作位置 | Rust（Tauri Commands） | ADR-001 |
| 数据迁移策略 | 增量添加新表，不修改已有表 | ADR-002 |
| LLM 调用架构 | 复用已有 llmProbe/llmPreview 调用架构，流式输出 | ADR-003 |
| Prompt 管理 | 集中在 Rust 侧 `prompts` 模块，不散落在前端组件 | ADR-004 |
| 前端状态管理 | Zustand，新增 KnowledgeUnderstandingStore，不跨 Store import | ADR-005 |
| 概念关系计算 | 纯 SQLite 查询（共现），不调用 LLM | ADR-006 |
| 前端组件目录 | `KnowledgeUnderstanding/`（与 v2.1 知识关联组件隔离） | ADR-007 |

---

## Architecture Decision Records (ADR)

### ADR-001: 所有 SQLite 写操作在 Rust 侧执行

- **状态**：已接受
- **上下文**：session_context.md 明确规定"SQLite：所有写操作在 Rust 侧执行，前端只读取"；Tauri 2.x 的安全模型也推荐这一分层。
- **决策**：新增的 4 张表的所有 INSERT/UPDATE/DELETE 操作通过新增 Tauri Commands 在 Rust 侧执行。前端通过 `invoke()` 调用 Command，接收 Rust 序列化的数据结构。
- **被排除项**：前端直接通过 Tauri SQL plugin 执行写操作——排除，违反项目约束，且绕过 Rust 侧的数据校验逻辑。
- **后果**：前端所有写入都需要对应的 Tauri Command；Rust 侧需要定义完整的入参/出参结构体；增加一层序列化/反序列化开销（在桌面端可接受）。

### ADR-002: 增量添加新表，不修改 v2.1 已有表

- **状态**：已接受
- **上下文**：v2.1 已有 `concepts`、`concept_viewpoints`、`concept_cases`、`concept_extensions` 四张表，用户在这些表中存有真实学习数据，且 `concepts.definition` 中存在用户手动编辑内容（`user_edited = true`）。PRD 明确要求不破坏已有数据。
- **决策**：新功能的所有数据存储在 4 张全新的表中（`concept_summaries`、`concept_explanations`、`concept_user_notes`、`concept_relations`），通过 `concept_id` 外键与已有表关联。migration 脚本只执行 `CREATE TABLE IF NOT EXISTS`，不执行任何 `ALTER TABLE`。
- **被排除项**：在 `concepts` 表新增字段——排除，风险高，影响所有使用 concepts 的已有代码路径；在 concept_cases 表新增摘要字段——排除，混淆表职责。
- **后果**：新旧表通过 `concept_id` 关联，读取时需 JOIN 操作；数据模型更清晰，每张表单一职责；未来可独立演化新表结构。

### ADR-003: LLM 调用复用 llmProbe/llmPreview 架构，流式输出优先

- **状态**：已接受
- **上下文**：session_context.md 说明已有 `llmProbe` 和 `llmPreview` 两个 Tauri Commands；用户自配 LLM endpoint（OpenAI 兼容接口）；性能要求理解框架首字 ≤ 3s。
- **决策**：新增的 3 个 LLM 相关 Commands（`knowledge_generate_summary`、`knowledge_generate_explanation`、`knowledge_validate_explanation`）复用已有的 HTTP client 和 streaming 架构。流式输出通过 Tauri Event 系统传递到前端（emit chunks）。生成完成后同步写入对应的 SQLite 缓存表。
- **被排除项**：完整等待 LLM 响应再返回——排除，无法满足首字 ≤ 3s 要求；使用新的 HTTP 客户端库——排除，增加维护负担。
- **后果**：前端需要监听 Tauri Events 实现流式渲染；Rust 侧需要将流式输出 buffer 完成后写入 SQLite（不能在流式过程中写入半成品）。

### ADR-004: Prompt 模板集中在 Rust 侧 `prompts` 模块管理

- **状态**：已接受
- **上下文**：PRD 提供了 3 个完整的 Prompt 模板（generate_summary、generate_explanation、validate_explanation）；session_context.md 规定"Prompt 模板集中管理，不散落在组件中"。
- **决策**：在 Rust 侧新建 `src-tauri/src/knowledge/prompts.rs` 模块，统一存放和构建所有 Prompt 字符串。每个 Command 调用 `prompts::build_xxx_prompt(...)` 获取最终 Prompt。前端不持有任何 Prompt 文本。
- **被排除项**：Prompt 存储在前端 TypeScript 文件——排除，违反代码规范，且 Prompt 包含敏感的系统约束，不应暴露在前端。
- **后果**：修改 Prompt 时只需改 Rust 侧一处；Prompt 与 Command 逻辑同文件（prompts.rs）便于维护。

### ADR-005: 新增 KnowledgeUnderstandingStore，不跨 Store import

- **状态**：已接受
- **上下文**：Zustand 约束"不跨 Store import，跨 Store 数据在组件层组合"；需要管理深入理解页面的状态（加载状态、流式内容、用户笔记、缓存数据）。
- **决策**：新增 `src/stores/knowledgeUnderstandingStore.ts`，只管理深入理解页面所需状态。与已有的 `conceptStore`（v2.1）数据交汇点（如 `conceptId`）在组件层从两个 Store 分别读取后组合使用。
- **被排除项**：在已有 conceptStore 中新增字段——排除，混淆职责，且已有 conceptStore 的 API 设计可能不适合流式状态管理。
- **后果**：深入理解页面的 React 组件需要同时订阅 conceptStore（读取概念基础信息）和 knowledgeUnderstandingStore（读取理解框架数据）。

### ADR-006: 概念关系（共现）通过纯 SQLite 查询计算，不调用 LLM

- **状态**：已接受
- **上下文**：PRD 明确"不需要 LLM 调用"；共现关系定义为"两个概念在同一文档中同时出现"；PRD 估算约 47 个概念 1081 对，纯数据库查询可行。
- **决策**：在概念提取完成后的异步步骤中，遍历所有概念对，查询其 `source_asset_ids` 的交集，若有交集则记录一条 `concept_relations` 记录（relation_type = "co_occurrence"）。同时将 v2.1 的 `concept_extensions`（upstream/downstream）迁移到 `concept_relations` 表（relation_type = "upstream"/"downstream"）以统一管理。
- **被排除项**：使用 LLM 判断语义关联——降级为 P1（语义关联 Lazy LLM）；使用图数据库——过度工程化，SQLite 足够。
- **后果**：共现计算需要在概念提取 Tauri Command 完成后触发（新增 `knowledge_compute_co_occurrence` Command）；关系数据完全离线可用。

### ADR-007: 前端新组件放在 `KnowledgeUnderstanding/` 目录

- **状态**：已接受
- **上下文**：PRD 要求"前端新组件放在 `KnowledgeUnderstanding/` 目录下"；需要与 v2.1 的知识关联组件（假设在 `KnowledgeAssociation/` 或类似目录）保持隔离。
- **决策**：`src/components/KnowledgeUnderstanding/` 下存放所有新增 UI 组件。入口组件 `DeepUnderstandButton` 嵌入已有的概念详情页，其余组件在 `KnowledgeUnderstanding/` 内部组合。
- **被排除项**：将新组件混入已有的知识关联目录——排除，增加耦合，不利于独立迭代。
- **后果**：已有概念详情页组件需要小幅修改（添加「深入理解」按钮和 Tooltip）；其余改动完全隔离在新目录内。

---

## 系统架构

### 模块划分

```
┌─────────────────────────────────────────────────────────────────────┐
│                         前端（React + TypeScript）                    │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  KnowledgeUnderstanding/（新增目录）                          │   │
│  │  ┌─────────────────┐  ┌──────────────────────────────────┐  │   │
│  │  │ DeepUnderstandBtn│  │  KnowledgeUnderstandingPage      │  │   │
│  │  │ (Feature Disc.)  │  │  ├── SummarySection              │  │   │
│  │  └─────────────────┘  │  ├── ExplanationSection          │  │   │
│  │                        │  ├── UserNotesSection            │  │   │
│  │                        │  └── RelationNetworkSection      │  │   │
│  │                        └──────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │  Stores（新增）                                                │  │
│  │  knowledgeUnderstandingStore.ts                              │  │
│  │  （streaming state / cache / user notes local state）         │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │  Types（新增）                                                 │  │
│  │  knowledge-understanding.types.ts                            │  │
│  └──────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
                              │ invoke() / listen()
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     Tauri Commands（Rust）                            │
│                                                                     │
│  src-tauri/src/knowledge/                                           │
│  ├── mod.rs           （模块入口，commands 注册）                     │
│  ├── commands.rs      （所有 knowledge_ 前缀的 Command handlers）     │
│  ├── prompts.rs       （所有 LLM Prompt 模板，build_xxx_prompt()）    │
│  ├── migration.rs     （4张新表的 CREATE TABLE SQL）                  │
│  └── co_occurrence.rs （共现关系计算逻辑）                            │
│                                                                     │
│  新增 Commands：                                                     │
│  • knowledge_generate_summary        (streaming, LLM)              │
│  • knowledge_generate_explanation    (streaming, LLM)              │
│  • knowledge_validate_explanation    (streaming, LLM)              │
│  • knowledge_compute_co_occurrence   (sync, SQLite only)           │
│  • knowledge_get_understanding_data  (sync, SQLite read)           │
│  • knowledge_save_user_note          (sync, SQLite write)          │
│  • knowledge_get_relations           (sync, SQLite read)           │
└─────────────────────────────────────────────────────────────────────┘
                              │ rusqlite
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         SQLite（本地）                               │
│                                                                     │
│  v2.1 已有表（只读，不修改）：                                         │
│  concepts / concept_viewpoints / concept_cases / concept_extensions │
│                                                                     │
│  新增表（本次迭代）：                                                  │
│  concept_summaries / concept_explanations                          │
│  concept_user_notes / concept_relations                            │
└─────────────────────────────────────────────────────────────────────┘
```

### 模块间依赖关系

```
ConceptDetail（已有）
  └── DeepUnderstandButton（新）
        └── KnowledgeUnderstandingPage（新）
              ├── SummarySection
              │     └── invoke(knowledge_generate_summary)
              ├── ExplanationSection
              │     └── invoke(knowledge_generate_explanation)
              ├── UserNotesSection
              │     ├── invoke(knowledge_save_user_note)
              │     └── invoke(knowledge_validate_explanation)
              └── RelationNetworkSection
                    └── invoke(knowledge_get_relations)

knowledgeUnderstandingStore（新）
  └── 组件层订阅，不 import 其他 Store

概念提取流程（已有，触发时机扩展）
  └── 提取完成后 → invoke(knowledge_compute_co_occurrence)
```

---

## 数据模型

### 新增表 1：concept_summaries（文档整合摘要）

```sql
CREATE TABLE IF NOT EXISTS concept_summaries (
  id           TEXT PRIMARY KEY,              -- UUID v4
  concept_id   TEXT NOT NULL,                -- 关联 concepts.id
  summary      TEXT NOT NULL,               -- AI 整合的摘要文本
  source_asset_ids TEXT NOT NULL,           -- JSON 数组，来源素材 ID
  model        TEXT NOT NULL,              -- 使用的 LLM 模型名
  generated_at TEXT NOT NULL,              -- ISO 8601 时间戳
  FOREIGN KEY (concept_id) REFERENCES concepts(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_concept_summaries_concept_id
  ON concept_summaries(concept_id);
```

**说明**：
- 每个 `concept_id` 最多一条记录（若已有则返回缓存，不重复生成）
- `source_asset_ids`：`["asset_id_1", "asset_id_2"]` 格式的 JSON 字符串
- 用户点击「重新生成」时：DELETE + INSERT（不 UPDATE，保持历史可追溯）

### 新增表 2：concept_explanations（理解框架）

```sql
CREATE TABLE IF NOT EXISTS concept_explanations (
  id                    TEXT PRIMARY KEY,    -- UUID v4
  concept_id            TEXT NOT NULL,       -- 关联 concepts.id
  mechanism             TEXT NOT NULL,       -- 核心机制（JSON: {text, source}）
  typical_scenarios     TEXT NOT NULL,       -- 典型场景（JSON 数组: [{text, source}]）
  common_misconceptions TEXT,               -- 常见误区（JSON 数组，可为 NULL）
  essence_sentence      TEXT NOT NULL,       -- 一句话精华
  source_asset_ids      TEXT NOT NULL,       -- JSON 数组，来源素材 ID
  model                 TEXT NOT NULL,       -- 使用的 LLM 模型名
  generated_at          TEXT NOT NULL,       -- ISO 8601 时间戳
  FOREIGN KEY (concept_id) REFERENCES concepts(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_concept_explanations_concept_id
  ON concept_explanations(concept_id);
```

**说明**：
- `mechanism`、`typical_scenarios`、`common_misconceptions` 存储为 JSON 字符串，包含 `text` 和 `source` 字段
- Rust 侧定义对应的 serde 结构体，前端通过 TypeScript 接口反序列化
- 每个 `concept_id` 最多一条记录

### 新增表 3：concept_user_notes（用户个人理解笔记）

```sql
CREATE TABLE IF NOT EXISTS concept_user_notes (
  id                  TEXT PRIMARY KEY,      -- UUID v4
  concept_id          TEXT NOT NULL UNIQUE,  -- 关联 concepts.id（一个概念一条）
  user_explanation    TEXT NOT NULL DEFAULT '', -- 用户自己的解释（空字符串表示未填写）
  mirror_feedback     TEXT,                  -- 最近一次 AI 镜子反馈（JSON，可为 NULL）
  last_validated_at   TEXT,                  -- 最近一次核对时间（ISO 8601，可为 NULL）
  created_at          TEXT NOT NULL,         -- ISO 8601 时间戳
  updated_at          TEXT NOT NULL,         -- ISO 8601 时间戳
  FOREIGN KEY (concept_id) REFERENCES concepts(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_concept_user_notes_concept_id
  ON concept_user_notes(concept_id);
```

**说明**：
- `UNIQUE` 约束在 `concept_id` 上，用 `INSERT OR REPLACE` 实现 upsert
- `mirror_feedback` 格式：
  ```json
  {
    "covered_count": 2,
    "covered_points": ["..."],
    "additional_perspectives": [{"text": "...", "source": "..."}],
    "difference_note": "..." | null
  }
  ```
- 此表与 `concepts.definition` 完全独立——`user_explanation` 是用户的个人学习笔记，`definition` 是概念定义，语义不同，不可混用

### 新增表 4：concept_relations（概念关系网络）

```sql
CREATE TABLE IF NOT EXISTS concept_relations (
  id                  TEXT PRIMARY KEY,      -- UUID v4
  concept_a_id        TEXT NOT NULL,         -- 关联 concepts.id
  concept_b_id        TEXT NOT NULL,         -- 关联 concepts.id
  relation_type       TEXT NOT NULL,         -- "co_occurrence" | "upstream" | "downstream"
  source_asset_ids    TEXT NOT NULL,         -- JSON 数组，共现文档 ID
  co_occurrence_count INTEGER DEFAULT 1,     -- 共现次数（upstream/downstream 时为 1）
  created_at          TEXT NOT NULL,         -- ISO 8601 时间戳
  FOREIGN KEY (concept_a_id) REFERENCES concepts(id) ON DELETE CASCADE,
  FOREIGN KEY (concept_b_id) REFERENCES concepts(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_concept_relations_a ON concept_relations(concept_a_id);
CREATE INDEX IF NOT EXISTS idx_concept_relations_b ON concept_relations(concept_b_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_concept_relations_pair
  ON concept_relations(concept_a_id, concept_b_id, relation_type);
```

**说明**：
- 共现关系：`concept_a_id < concept_b_id`（字符串排序，保证无重复）
- 查询时需同时查询 `concept_a_id = X OR concept_b_id = X` 获取所有关联
- v2.1 `concept_extensions` 的 upstream/downstream 数据通过 migration 脚本迁移过来（`relation_type` 设为对应值）

---

## API 设计（Tauri Commands）

所有新增 Command 统一前缀 `knowledge_`，snake_case 命名。

### Command 1：`knowledge_generate_summary`

```rust
// 触发：用户首次打开「深入理解」页面时，若 concept_summaries 无缓存
// 行为：流式 LLM 调用，生成后存入 concept_summaries，通过 Tauri Event 流式推送
#[tauri::command]
pub async fn knowledge_generate_summary(
    app: AppHandle,
    concept_id: String,
    force_regenerate: bool,     // 用户点击「重新生成」时为 true
) -> Result<ConceptSummaryResult, KnowledgeError>

// 返回类型（非流式部分，最终写入结果）
pub struct ConceptSummaryResult {
    pub id: String,
    pub concept_id: String,
    pub summary: String,
    pub source_asset_ids: Vec<String>,
    pub model: String,
    pub generated_at: String,
}

// 流式事件（每个 chunk 通过 app.emit 推送）
// Event name: "knowledge:summary:chunk"
// Event payload: { concept_id: String, chunk: String, is_final: bool }
```

### Command 2：`knowledge_generate_explanation`

```rust
// 触发：用户首次打开「深入理解」页面时，若 concept_explanations 无缓存
// 行为：流式 LLM 调用，生成 JSON 结构化内容，存入 concept_explanations
#[tauri::command]
pub async fn knowledge_generate_explanation(
    app: AppHandle,
    concept_id: String,
    force_regenerate: bool,
) -> Result<ConceptExplanationResult, KnowledgeError>

pub struct ConceptExplanationResult {
    pub id: String,
    pub concept_id: String,
    pub mechanism: ExplanationItem,           // { text: String, source: String }
    pub typical_scenarios: Vec<ExplanationItem>,
    pub common_misconceptions: Vec<ExplanationItem>,
    pub essence_sentence: String,
    pub source_asset_ids: Vec<String>,
    pub model: String,
    pub generated_at: String,
}

pub struct ExplanationItem {
    pub text: String,
    pub source: String,  // 文档名称
}

// 流式事件：
// Event name: "knowledge:explanation:chunk"
// Event payload: { concept_id: String, chunk: String, is_final: bool }
```

### Command 3：`knowledge_validate_explanation`

```rust
// 触发：用户点击「和 AI 核对一下」
// 行为：流式 LLM 调用，生成镜子反馈，存入 concept_user_notes.mirror_feedback
#[tauri::command]
pub async fn knowledge_validate_explanation(
    app: AppHandle,
    concept_id: String,
    user_explanation: String,
) -> Result<MirrorFeedbackResult, KnowledgeError>

pub struct MirrorFeedbackResult {
    pub concept_id: String,
    pub covered_count: u32,
    pub covered_points: Vec<String>,
    pub additional_perspectives: Vec<FeedbackPerspective>,
    pub difference_note: Option<String>,
}

pub struct FeedbackPerspective {
    pub text: String,
    pub source: String,
}

// 流式事件：
// Event name: "knowledge:mirror:chunk"
// Event payload: { concept_id: String, chunk: String, is_final: bool }
```

### Command 4：`knowledge_compute_co_occurrence`

```rust
// 触发：概念提取完成后的异步步骤（非用户直接触发）
// 行为：纯 SQLite 计算，无 LLM 调用
#[tauri::command]
pub async fn knowledge_compute_co_occurrence(
    concept_ids: Vec<String>,   // 需要计算的概念 ID 列表
) -> Result<usize, KnowledgeError>  // 返回新增关系数量
```

### Command 5：`knowledge_get_understanding_data`

```rust
// 触发：深入理解页面挂载时（首屏加载）
// 行为：从 SQLite 读取所有已缓存数据（summary + explanation + user_note），无 LLM 调用
#[tauri::command]
pub async fn knowledge_get_understanding_data(
    concept_id: String,
) -> Result<UnderstandingData, KnowledgeError>

pub struct UnderstandingData {
    pub summary: Option<ConceptSummaryResult>,
    pub explanation: Option<ConceptExplanationResult>,
    pub user_note: Option<UserNoteResult>,
}

pub struct UserNoteResult {
    pub id: String,
    pub concept_id: String,
    pub user_explanation: String,
    pub mirror_feedback: Option<MirrorFeedbackResult>,
    pub last_validated_at: Option<String>,
    pub updated_at: String,
}
```

### Command 6：`knowledge_save_user_note`

```rust
// 触发：用户停止输入 1s 后（自动保存 debounce）
// 行为：upsert concept_user_notes 表
#[tauri::command]
pub async fn knowledge_save_user_note(
    concept_id: String,
    user_explanation: String,
) -> Result<UserNoteResult, KnowledgeError>
```

### Command 7：`knowledge_get_relations`

```rust
// 触发：深入理解页面底部关系网络区域加载时
// 行为：查询 concept_relations 表，返回排序后的关联概念
#[tauri::command]
pub async fn knowledge_get_relations(
    concept_id: String,
    limit: Option<u32>,  // 默认 8，UI 显示前 5-8 个
) -> Result<Vec<ConceptRelationResult>, KnowledgeError>

pub struct ConceptRelationResult {
    pub related_concept_id: String,
    pub related_concept_name: String,
    pub relation_type: String,         // "co_occurrence" | "upstream" | "downstream"
    pub source_asset_names: Vec<String>, // 共现文档名称列表（前端显示用）
    pub co_occurrence_count: u32,
}
```

### 错误类型

```rust
#[derive(Debug, thiserror::Error, serde::Serialize)]
pub enum KnowledgeError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("LLM API error: {0}")]
    LlmApi(String),
    #[error("Concept not found: {0}")]
    ConceptNotFound(String),
    #[error("No source documents found for concept: {0}")]
    NoSourceDocuments(String),
    #[error("Invalid JSON in LLM response: {0}")]
    InvalidLlmResponse(String),
}
```

---

## 目录结构

### Rust 侧新增文件（`src-tauri/src/`）

```
src-tauri/src/
├── knowledge/                    （新增目录）
│   ├── mod.rs                   （模块声明，pub use commands::*）
│   ├── commands.rs              （7个 knowledge_ Commands 的实现）
│   ├── prompts.rs               （3个 LLM Prompt 的 build_xxx_prompt() 函数）
│   ├── migration.rs             （4张新表的 CREATE TABLE IF NOT EXISTS SQL）
│   └── co_occurrence.rs         （共现关系计算逻辑，独立于 commands.rs）
└── main.rs                      （修改：注册新 Commands）
```

**修改已有文件**：
- `src-tauri/src/main.rs`：在 `tauri::Builder` 的 `.invoke_handler()` 中添加 7 个新 Command
- `src-tauri/src/database.rs`（假设存在）：在数据库初始化时调用 `knowledge::migration::run_migrations()`

### 前端新增文件（`src/`）

```
src/
├── components/
│   └── KnowledgeUnderstanding/           （新增目录）
│       ├── index.ts                      （barrel export）
│       ├── KnowledgeUnderstandingPage.tsx （深入理解页面主容器）
│       ├── DeepUnderstandButton.tsx      （「深入理解」入口按钮 + Tooltip）
│       ├── FirstVisitTooltip.tsx         （一次性引导 Tooltip 组件）
│       ├── SummarySection.tsx            （「你的文档怎么说」区域）
│       ├── ExplanationSection.tsx        （理解框架区域：机制/场景/误区/精华）
│       ├── ExplanationItem.tsx           （单条解释条目，含来源链接和「查看依据」）
│       ├── SourceEvidence.tsx            （「查看依据」展开后的原文段落展示）
│       ├── TransparencyBanner.tsx        （顶部透明度声明横幅）
│       ├── UserNotesSection.tsx          （「用你自己的话」区域 + 自动保存）
│       ├── MirrorFeedbackDisplay.tsx     （AI 镜子反馈结果展示）
│       └── RelationNetworkSection.tsx    （概念关系网络卡片列表）
├── stores/
│   └── knowledgeUnderstandingStore.ts   （新增 Zustand Store）
└── types/
    └── knowledge-understanding.types.ts  （新增 TypeScript 接口定义）
```

**修改已有文件**：
- `src/components/ConceptDetail/index.tsx`（或对应的概念详情页组件）：添加 `<DeepUnderstandButton>` 和 FirstVisitTooltip 逻辑
- `src/App.tsx` 或路由配置：如使用页面路由，添加深入理解页面路由（若为面板式切换，则无需修改路由）

### 新增 types 定义（`src/types/knowledge-understanding.types.ts`）

```typescript
// 对应 Rust 侧的所有 Result 类型
export interface ConceptSummaryResult { ... }
export interface ConceptExplanationResult { ... }
export interface ExplanationItem { text: string; source: string }
export interface UserNoteResult { ... }
export interface MirrorFeedbackResult { ... }
export interface FeedbackPerspective { text: string; source: string }
export interface ConceptRelationResult { ... }
export interface UnderstandingData { ... }

// UI 状态类型
export type StreamingStatus = 'idle' | 'streaming' | 'done' | 'error'
export interface KnowledgeUnderstandingState { ... }
```

---

## 安全考量

| 安全项 | 设计决策 | 对应约束 |
|--------|---------|---------|
| 用户文档内容不上传到第三方 | LLM 调用走用户自配 endpoint，Prompt 中只包含用户自己的文档摘录 | session_context 安全底线 #1 |
| 防止 AI 覆盖用户手动编辑 | `knowledge_save_user_note` 只写 `concept_user_notes` 表，与 `concepts.definition` 的 `user_edited` 字段完全隔离，新 Commands 不读取 `user_edited` 字段 | session_context 安全底线 #3 |
| LLM 内容存入新表，不写已有表 | 所有 LLM 生成结果存入 `concept_summaries`/`concept_explanations`/`concept_user_notes`，`knowledge_generate_*` Commands 不执行任何 UPDATE 到已有表 | PRD 技术底线 #3 |
| 来源链接强制要求 | `commands.rs` 在写入 SQLite 前校验 `mechanism.source` 非空，若 LLM 返回的 JSON 缺少 source 字段则拒绝写入并返回 `KnowledgeError::InvalidLlmResponse` | PRD 技术底线 #4 |
| Prompt 注入防御 | Prompt 构建在 `prompts.rs` 中通过 Rust 字符串格式化（非 eval/动态拼接），用户文档内容通过 `{placeholder}` 替换插入，不直接拼接到 system prompt | 通用安全实践 |
| 离线数据安全 | 所有数据存储在本地 SQLite，与已有 v2.1 数据同一数据库文件，无额外网络暴露面 | session_context 安全要求 |

---

## 风险登记表

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|---------|
| LLM 理解框架包含幻觉内容（超出用户文档） | 中 | 高 | Prompt 硬约束（ONLY use provided documents）+ 每条来源链接 + 透明度声明横幅 + 「查看依据」功能；来源字段为空时 Rust 侧拒绝写入 |
| 镜子反馈措辞不符合探索式原则 | 低 | 高 | Prompt 3 包含明确的禁止词列表；可在未来在 Rust 侧对反馈文本做关键词扫描（MINOR 功能，P1） |
| v2.1 已有数据在 migration 时损坏 | 低 | 极高 | 只执行 `CREATE TABLE IF NOT EXISTS`，零 ALTER TABLE；migration 前在 task_002 中明确要求有备份验证步骤 |
| 共现关系大量假阳性影响用户信任 | 中 | 中 | UI 措辞「一起出现在你的文档中」（不说「紧密相关」）；用户自己判断关联意义 |
| 首屏超过 500ms（summary 未缓存时需 LLM 调用） | 中 | 中 | 快速路径：若已有 `concept_summaries` 缓存，直接读取（≤100ms）；无缓存时先渲染骨架屏，再触发流式生成；页面不阻塞等待 LLM |
| 流式内容写入 SQLite 时部分失败（网络中断） | 低 | 中 | 流式过程中不写入 SQLite，仅在流结束（`is_final: true`）后执行一次完整写入；中断则不写入，下次仍可重新生成 |
| 概念详情页已有代码路径不清晰，修改点不明 | 中 | 中 | task_006（前端入口）的 input.md 中标注需要 Dev 先阅读已有 ConceptDetail 代码再修改 |
| LLM 返回非法 JSON（explanation 需要 JSON 格式） | 中 | 低 | Rust 侧 `serde_json::from_str` 失败时返回 `KnowledgeError::InvalidLlmResponse`，前端显示友好错误提示（「生成失败，请重试」） |

---

## Task 清单

| Task ID | 名称 | 描述 | 前置 |
|---------|------|------|------|
| task_002_dev_db_migration | 数据库 Migration | 创建 4 张新 SQLite 表 + 初始化集成 | 无 |
| task_003_dev_rust_commands | Rust Commands + Prompts | 实现 7 个 knowledge_ Commands + Prompt 模块（LLM 调用部分） | task_002 |
| task_004_dev_rust_co_occurrence | 共现关系计算 | 实现 knowledge_compute_co_occurrence + co_occurrence.rs 逻辑 | task_002 |
| task_005_dev_frontend_types_store | 前端 Types + Store | TypeScript 类型定义 + knowledgeUnderstandingStore | task_003 |
| task_006_dev_ui_entry_discovery | 前端：入口 + Feature Discovery | DeepUnderstandButton + FirstVisitTooltip + 已有 ConceptDetail 页改造 | task_005 |
| task_007_dev_ui_understanding_page | 前端：深入理解主页面 | KnowledgeUnderstandingPage + SummarySection + ExplanationSection + 流式渲染 | task_005, task_006 |
| task_008_dev_ui_user_notes_mirror | 前端：用户笔记 + 镜子反馈 | UserNotesSection + 自动保存 + MirrorFeedbackDisplay | task_007 |
| task_009_dev_ui_relation_network | 前端：概念关系网络 | RelationNetworkSection + 关联概念卡片列表 | task_004, task_007 |
| task_010_ux_review | UX 体验审查 | 端到端流程验证 + 措辞审查 + 性能验证 | task_008, task_009 |

---

## Task 依赖拓扑

```
task_002_dev_db_migration
  ├── task_003_dev_rust_commands
  │     └── task_005_dev_frontend_types_store
  │           ├── task_006_dev_ui_entry_discovery
  │           │     └── task_007_dev_ui_understanding_page
  │           │           ├── task_008_dev_ui_user_notes_mirror
  │           │           │     └── task_010_ux_review ←──────────┐
  │           │           └──────────────────────────────────────── │
  │           └── （task_007 也依赖 task_006，见上）                  │
  └── task_004_dev_rust_co_occurrence                               │
        └── task_009_dev_ui_relation_network ──────────────────────┘
              └── task_010_ux_review

可并行执行：
  • task_003 和 task_004（都依赖 task_002，互不依赖）
  • task_008 和 task_009（都依赖 task_007/task_004，互不依赖）

关键路径：
  task_002 → task_003 → task_005 → task_006 → task_007 → task_008 → task_010
```
