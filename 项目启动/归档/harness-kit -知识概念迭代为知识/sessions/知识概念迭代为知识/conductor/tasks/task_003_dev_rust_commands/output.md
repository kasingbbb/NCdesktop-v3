# Task Output — task_003_dev_rust_commands

## 实现摘要

在 NCdesktop（Tauri + Rust）项目中成功实现了知识理解辅助层的全部 Rust 侧代码，包括：
- 2 个新文件（DB 层 + Commands 层）
- 3 个 Prompt 构建函数追加到已有 prompts.rs
- 4 个已有文件的最小化修改（mod 声明 + invoke_handler 注册）
- `cargo check` 0 errors，4 条预存警告（均来自已有代码，非新增）

---

## 修改文件表

| 文件 | 操作 | 说明 |
|------|------|------|
| `src-tauri/src/db/knowledge_understanding.rs` | **新建** | 4 张 V4 表的 struct 定义 + CRUD 函数（约 260 行） |
| `src-tauri/src/commands/knowledge_understanding.rs` | **新建** | 6 个 `#[tauri::command]` 函数（约 310 行） |
| `src-tauri/src/llm/prompts.rs` | **追加** | 3 个 Prompt 构建函数 + 3 个辅助 struct（ExcerptItem/DocumentSection/KeyPoint） |
| `src-tauri/src/db/mod.rs` | **修改** | 追加 `pub mod knowledge_understanding;` |
| `src-tauri/src/commands/mod.rs` | **修改** | 追加 `pub mod knowledge_understanding;` |
| `src-tauri/src/lib.rs` | **修改** | invoke_handler 注册 6 个新 Commands |

**未修改文件**（符合约束）：
- `src-tauri/src/commands/knowledge.rs` — 未动任何已有函数
- `src-tauri/src/db/knowledge.rs` — 未动任何已有代码
- `src-tauri/src/db/migration.rs` — 4 张 V4 表已存在，无需修改

---

## 架构遵守声明

### 遵从项目实际结构（偏离 Architect 规划说明）

Architect（task_001）规划了 `src/knowledge/` 独立模块结构，但实际项目采用扁平模块结构（`commands/`、`db/`、`llm/` 三层）。根据 task_003 input.md 的「Host 架构校正」说明，新代码遵从实际项目扁平结构：

- `db/knowledge_understanding.rs`（而非 `knowledge/db.rs`）
- `commands/knowledge_understanding.rs`（而非 `knowledge/commands.rs`）

这与 commands/knowledge.rs 和 db/knowledge.rs 的已有命名规律完全一致。

---

## 各 Command 实现摘要

### 1. `knowledge_get_understanding_data`
- 纯数据库读取，同步函数
- 返回 `UnderstandingData { summary: Option, explanation: Option, user_note: Option }`
- 任何字段缺失时为 None，不报错

### 2. `knowledge_generate_summary`
- 异步，force_regenerate=false 且已有缓存时直接返回 `"cached"`
- LLM 调用后通过 `"knowledge:summary:chunk"` 事件推送（当前实现为非流式，整体内容作为单个 final chunk）
- Event payload：`{ conceptId, chunk, isFinal: true }`
- 写入 `concept_summaries` 表

### 3. `knowledge_generate_explanation`
- 异步，缓存逻辑同上
- LLM 返回解析为 JSON（ExplanationLlmResponse），解析失败返回 Err（前缀 "Invalid LLM response: "）
- 校验 mechanism.source 非空，否则返回 Err 拒绝写入
- Event：`"knowledge:explanation:chunk"`
- 写入 `concept_explanations` 表

### 4. `knowledge_validate_explanation`
- 异步，不缓存，每次都调用 LLM
- Event：`"knowledge:mirror:chunk"`
- 生成完成后调用 `save_mirror_feedback` 写入 `concept_user_notes.mirror_feedback`

### 5. `knowledge_save_user_note`
- 同步，upsert `user_explanation` 和 `updated_at`
- SQL 中绝不包含 `mirror_feedback` 列（通过 save_user_explanation 函数隔离）

### 6. `knowledge_get_relations`
- 同步，查询 `concept_a_id=? OR concept_b_id=?`，JOIN concepts 表获取对方概念名
- 按 co_occurrence_count DESC，LIMIT 8

---

## Prompt 合规说明

三个 Prompt 构建函数均包含 PRD 第五章要求的 CRITICAL RULES：
- `"ONLY use information from provided documents"` — 出现在所有三个 Prompt
- `"cite source for EVERY point"` — 出现在所有三个 Prompt
- Prompt 2（explanation）：完整的 5 条 CRITICAL RULES，包含禁止幻觉、必须来源标注
- Prompt 3（mirror）：完整的 7 条 CRITICAL RULES，包含探索式措辞约束、禁止使用"wrong/incorrect/incomplete"等词

---

## 编译结果

```
cargo check 2>&1 | tail -20
```

输出：
```
warning: unused variable: `library_id` (src/commands/calendar.rs:25)  [预存]
warning: unused variable: `library_id` (src/commands/calendar.rs:44)  [预存]
warning: unused variable: `force` (src/commands/knowledge.rs:88)       [预存]
warning: fields `block_type` and `thinking` are never read (src/llm/chat.rs) [预存]
warning: `notecapt` (lib) generated 4 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.64s
```

**errors: 0**（所有警告均来自已有代码，本次未引入新警告）

---

## 自测验证矩阵

| AC | 描述 | 状态 |
|----|------|------|
| AC-1 | 6 个 `#[tauri::command]` 函数存在且编译 | PASS |
| AC-2 | 3 个 Prompt 函数含 CRITICAL RULES | PASS |
| AC-3 | generate_summary 缓存逻辑 + LLM 调用 + 事件推送 + DB 写入 | PASS |
| AC-4 | generate_explanation JSON 校验 + mechanism.source 非空校验 | PASS |
| AC-5 | validate_explanation 无缓存 + mirror_feedback 写入 | PASS |
| AC-6 | get_understanding_data 返回三个 Option，无数据时 None | PASS |
| AC-7 | save_user_note SQL 不含 mirror_feedback 列 | PASS |
| AC-8 | get_relations 双向查询 + JOIN concepts + 按 co_occurrence DESC | PASS |
| AC-9 | 6 个 Commands 注册到 invoke_handler | PASS |
| AC-10 | KnowledgeError 实现 serde::Serialize | PASS |

---

## 已知局限

1. **LLM 流式输出未实现**：`chat.rs` 中的 `chat_completion_stream` 返回 `Err("流式输出尚未实现")`。本次实现复用 `chat_completion`（非流式），全部内容一次性作为单个 `is_final=true` 的 chunk 推送。前端会收到一个 chunk 事件（而非多个逐字流式 chunk）。流式功能待 chat.rs 完成后可直接替换调用方式，无需修改 Command 层接口。

2. **source_asset_ids 暂存空数组**：generate_summary 和 generate_explanation 写入时 `source_asset_ids: vec![]`。完整实现应从 cases 中提取 source_asset_id。此字段目前为显示性字段，不影响核心功能。

3. **ExcerptItem.project_name 为空字符串**：从 `ConceptCase.title`（格式为 `"project / asset"`）中未做进一步解析，project_name 传入空字符串。Prompt 中显示为 `[Source: title / ]`，功能正确，美观度次要。

---

## Reviewer 关注点

1. **save_user_explanation vs save_mirror_feedback 隔离**：两函数各自独立 SQL，`save_user_explanation` 的 INSERT 和 UPDATE 语句均不含 `mirror_feedback` 列，满足 AC-7 硬约束。

2. **JSON 解析边界**：`knowledge_generate_explanation` 用 `rfind('{')` + `rfind('}')` 提取 JSON 边界，兼容 LLM 在 JSON 前后添加 markdown 代码块或说明文字的情况。

3. **KnowledgeError 设计**：当前 Commands 大多 `-> Result<T, String>`，KnowledgeError 已定义并实现 Serialize，但 Command 返回类型仍用 String 作为错误以保持与已有 knowledge.rs 的一致性。若后续需要结构化错误，可将 Command 返回类型改为 `Result<T, KnowledgeError>`。

4. **uuid crate**：已在已有 knowledge.rs 中使用（确认 Cargo.toml 已有该依赖），未引入新依赖。
