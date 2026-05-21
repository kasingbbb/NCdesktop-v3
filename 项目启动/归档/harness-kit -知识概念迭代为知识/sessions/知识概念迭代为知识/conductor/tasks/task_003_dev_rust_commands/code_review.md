# Code Review — task_003_dev_rust_commands

> 审计者：Code Reviewer
> 审计日期：2026-04-11
> 审计范围：commands/knowledge_understanding.rs（新建）、db/knowledge_understanding.rs（新建）、llm/prompts.rs（追加部分）、lib.rs（invoke_handler 追加）
> 参照文档：task_003 input.md（验收标准）、session_context.md（质量偏好）、task_002 code_review.md（跨任务协调）

---

## 审计发现列表

---

### F-001：`KnowledgeError` 已定义但 Command 返回类型全部使用 `String` 作为错误（严重性：中）

**位置**：`commands/knowledge_understanding.rs` 第 22–45 行（KnowledgeError 定义），所有 6 个 Command 函数签名

**描述**：
`KnowledgeError` 枚举已实现 `serde::Serialize` 和 `Display`，满足 AC-10 的编译层要求。但所有 6 个 `#[tauri::command]` 函数的实际返回类型均为 `Result<T, String>` 而非 `Result<T, KnowledgeError>`。

实际效果：
- 前端收到的错误是原始字符串（如 `"LLM 调用失败: ..."` 或 `"Invalid LLM response: ..."`)
- 前端无法通过结构化字段（`kind`、`message`）区分错误类型
- 错误消息是硬编码在函数体中的中文字符串，而非通过 `KnowledgeError` 的变体统一管理

output.md 第 142–143 行已主动披露此设计，将其描述为"与已有 `knowledge.rs` 保持一致性"。确认 `knowledge.rs` 中已有命令同样使用 `Result<T, String>`，因此这是刻意的一致性选择，不是错误。

**影响**：
- AC-10 的文字要求是"KnowledgeError 类型实现 serde::Serialize，前端可正确接收错误信息"。当前实现 KnowledgeError 已实现 Serialize（编译通过），但前端实际收到的是 `String` 而非 `KnowledgeError` 结构体，因此 AC-10 的"前端可正确接收错误信息"被字符串形式满足，而非结构化形式满足。
- 若未来前端需要区分错误类型（例如区分 LLM 调用失败 vs JSON 解析失败），当前设计需要解析字符串前缀，脆弱。

**与 Spec 关系**：AC-10 未明确要求 Command 返回类型必须是 `Result<T, KnowledgeError>`，仅要求"实现 Serialize"。技术上 PASS，但有技术债务。

---

### F-002：`knowledge_generate_explanation` 的 JSON 边界提取使用 `find('{')` 而非 `rfind('{')`（严重性：中）

**位置**：`commands/knowledge_understanding.rs` 第 281–289 行

**描述**：
```rust
let json_start = result.find('{').unwrap_or(0);
let json_end = result.rfind('}').map(|i| i + 1).unwrap_or(result.len());
```

当 LLM 在 JSON 前输出了说明性文字（如 `"Here is the JSON:\n{...}"`），`find('{')` 能正确找到第一个 `{`。

但当 LLM 输出的 JSON 本身包含嵌套对象（`mechanism.text` 内容中可能包含 `{` 字符引用），或 LLM 在 JSON 前输出了包含 `{` 的文字（如 `"Based on the format {mechanism, scenarios...}, here is:"...`），`find('{')` 会找到非 JSON 的 `{`，导致截取的 JSON 起始位置错误。

output.md 第 142 行描述使用的是 `rfind('{')` + `rfind('}')`，但实际代码是 `find('{')` + `rfind('}')`（前者取最后一个，后者取第一个）。output.md 的描述与代码不一致。

**实际风险评估**：
- 对于 OpenAI 兼容接口的标准输出，此实现在大多数情况下可工作（LLM 通常先输出 JSON 再关闭）
- 但组合 `find('{')` 和 `rfind('}')` 是不对称的：如果 LLM 在 JSON 外包了 markdown 代码块 ` ```json {...} ``` `，提取结果是 `{...}` + ` ``` ` 之间的额外内容，导致解析失败

**建议**：改为尝试从 `find('{')` 到 `rfind('}')` 截取后解析，失败时尝试 `rfind('{')` 到 `rfind('}')`，或使用 serde_json 的 `from_str` 自动跳过前缀空白。当前实现功能上可工作但不够健壮。

---

### F-003：`knowledge_validate_explanation` 的 mirror_feedback 存储为原始字符串而非 JSON（严重性：中）

**位置**：`commands/knowledge_understanding.rs` 第 406–414 行

**描述**：
AC-5 要求："生成完成后将 mirror_feedback 序列化为 JSON 存入 `concept_user_notes.mirror_feedback`（upsert）"。

当前实现直接将 LLM 的原始输出字符串（`result`）存入数据库：
```rust
save_mirror_feedback(&conn, &concept_id, &result, &now)?;
```

但 `result` 是 LLM 的原始响应文本，不一定是合法 JSON。与 `knowledge_generate_explanation` 的处理方式（解析 JSON → 校验 → 再序列化存储）不同，`knowledge_validate_explanation` 没有：
1. 对 LLM 输出进行 JSON 解析校验
2. 从原始响应中提取 JSON 边界（与 explanation 的 `find('{')` 逻辑相比，mirror 完全跳过了这步）

**影响**：
- 若 LLM 在 JSON 前输出了说明性文字（常见行为），`mirror_feedback` 列存储的是包含 markdown 和说明文字的混合内容
- 前端读取 `mirror_feedback` 时需要额外处理，或直接 JSON.parse 失败
- PRD Prompt 3 要求 LLM 按特定 JSON schema 返回（`covered_count`、`covered_points`、`additional_perspectives`、`difference_note`），但代码没有校验是否符合此 schema

---

### F-004：`save_user_explanation` 在 INSERT 新记录时 `id` 字段依赖内部生成，但 `save_mirror_feedback` INSERT 分支也独立生成 `id`，两个函数可能产生同一 `concept_id` 的两条记录竞争（严重性：低）

**位置**：`db/knowledge_understanding.rs` 第 226–282 行

**描述**：
`save_user_explanation` 和 `save_mirror_feedback` 都包含"若记录不存在则 INSERT"的逻辑，并且都独立生成新的 `uuid`。

潜在竞争场景：
1. 用户第一次调用 `knowledge_validate_explanation`（触发 `save_mirror_feedback`）
2. 与此同时，用户第一次调用 `knowledge_save_user_note`（触发 `save_user_explanation`）
3. 两个函数都读到"record not exists"，都执行 INSERT，触发 UNIQUE constraint violation（`concept_user_notes.concept_id` 有 UNIQUE 约束）

但 `concept_user_notes.concept_id` 有 UNIQUE 约束，第二个 INSERT 会报错，函数返回 `Err`。

**实际风险**：由于当前实现是单线程（Tauri Command 调用不自动并发，且 Mutex 锁确保每次只有一个操作持有连接），实际竞争窗口极小。但设计模式上使用两个独立的 INSERT 替代单一的 `INSERT OR IGNORE` + 分别 UPDATE 不够健壮。

**建议**：使用 `INSERT OR IGNORE INTO concept_user_notes (...) VALUES (...)` 确保记录存在，然后分别执行 `UPDATE SET user_explanation` 或 `UPDATE SET mirror_feedback`，避免 INSERT 竞争。

---

### F-005：`source_asset_ids` 始终写入空数组 `vec![]`，但 `ConceptSummary` 和 `ConceptExplanation` 中该字段的语义是"来源文档 ID 列表"（严重性：低）

**位置**：`commands/knowledge_understanding.rs` 第 195 行（summary），第 335 行（explanation）

**描述**：
output.md 第 130 行已主动披露此问题（"已知局限 2"），说明是暂时实现。

但此字段是用于"离线查看时追溯来源"的核心数据（session_context.md 第 39 行：功能必须在无网络环境下可访问已生成内容），且在 `get_relations` 的 SQL 中 output.md 提到应"JOIN source assets 表获取文档名称"（AC-8）。

实际影响：已生成的 summary/explanation 无法追溯来源文档，若文档被删除或移动，无法知道哪些内容受影响。对于当前功能的正确性不影响，但是产品的数据完整性缺口。

---

### F-006：`ExcerptItem.project_name` 始终为空字符串，Prompt 中显示为 `[Source: title / ]`（严重性：低）

**位置**：`commands/knowledge_understanding.rs` 第 149–157 行（summary 的 excerpts 构建），第 244–249 行（explanation 的 sections 构建）

**描述**：
output.md 第 132 行已主动披露（"已知局限 3"）。

`ConceptCase.title` 的格式是 `"project / asset"`（按 output.md 描述），但实现中没有解析 `"/"` 分隔符提取 project_name，直接将整个 title 放入 `asset_name`，`project_name` 置空字符串。

实际显示效果：Prompt 中 `[Source: project / asset / ]` → 结尾多余的 ` / ` 影响 LLM 的来源引用格式。

**风险**：LLM 可能在引用来源时包含格式异常的文本（如 `"[Source: project / asset / ]"`），存储到 `mechanism.source` 后前端展示有轻微瑕疵。不影响核心功能。

---

### F-007：`knowledge_get_relations` 未 JOIN source assets 表（AC-8 要求"JOIN source assets 表获取文档名称（如相关表存在）"）（严重性：低）

**位置**：`db/knowledge_understanding.rs` 第 288–329 行，`get_relations` 函数

**描述**：
AC-8 要求：JOIN `concepts` 表获取对方概念名称，JOIN source assets 表获取文档名称（如相关表存在）。

当前实现：
- 已正确 JOIN `concepts` 表获取 `other_concept_name`（满足前半部分）
- 未 JOIN source assets 表获取文档名称

output.md 的 AC-8 描述（第 74 行）同样省略了 source assets JOIN，仅说"JOIN concepts 表获取对方概念名"。

`ConceptRelation` 结构体中包含 `source_asset_ids`（JSON 数组），前端可以基于这些 ID 再调用其他 Command 查询文档名。

**影响**：前端无法在单次 `get_relations` 调用中直接获取文档名称，需要额外请求。但 AC-8 的括号说明"（如相关表存在）"给了实现自由度。

---

### F-008：`ConceptRelation` 结构体包含 `other_concept_id` 字段但 SQL 查询中对应的 AS 别名是 `other_id`（严重性：低，字段语义一致，编译通过）

**位置**：`db/knowledge_understanding.rs` 第 295–302 行（SQL），第 49–62 行（结构体定义）

**描述**：
SQL 中使用：
```sql
CASE WHEN cr.concept_a_id = ?1 THEN cr.concept_b_id ELSE cr.concept_a_id END AS other_id,
```
但结构体字段名为 `other_concept_id`，且通过 `row.get(7)?` 按列位置访问，而非按列名访问。

由于使用位置访问（`row.get(7)`），字段名与 SQL 别名不一致不会导致运行错误，但降低了代码可读性。

---

### F-009：三个系统 Prompt（system message）包含安全约束文本，但与 prompts.rs 中的用户 Prompt 内容重复且不一致（严重性：低，文档质量问题）

**位置**：`commands/knowledge_understanding.rs` 第 163–172 行（summary system message），第 255–264 行（explanation system message），第 382–391 行（mirror system message）

**描述**：
系统 Prompt 直接在 Command 函数中硬编码，而用户 Prompt 通过 `prompts.rs` 中的函数构建。

问题：
1. **重复约束**：系统 Prompt 中的 CRITICAL RULES 与 `prompts.rs` 中用户 Prompt 的 CRITICAL RULES 部分重复（例如"ONLY use information from provided documents"在两处均出现）
2. **分散管理**：系统 Prompt 的约束文本散落在 Command 函数中，而非集中在 `prompts.rs` 统一管理，违背了 session_context.md 第 62 行"Prompt 模板集中管理，不散落在组件中"的规范
3. **不一致**：summary 的系统 Prompt 约束（"Do NOT add any external knowledge"）与 prompts.rs 中 build_summary_prompt 的 CRITICAL RULES 措辞不完全一致，存在细微差异

**影响**：若需修改 Prompt 约束，必须同时修改 `prompts.rs` 和 Command 函数中的字符串，容易漏改。

---

### F-010：Prompt 安全约束核查（BLOCKER 专项检查）

**位置**：`llm/prompts.rs` 第 141–255 行

**检查结果：通过（无 BLOCKER）**

逐一核查三个 Prompt 函数：

| Prompt 函数 | "ONLY use information from provided documents" | "cite source for EVERY point" |
|-------------|------------------------------------------------|-------------------------------|
| `build_summary_prompt` | 存在（第 154 行）| 存在（第 155 行）|
| `build_explanation_prompt` | 存在（第 199 行：`"6. ONLY use information from provided documents."`）| 存在（第 200 行：`"7. cite source for EVERY point."`）|
| `build_mirror_prompt` | 存在（第 250 行：`"6. ONLY use information from provided documents."`）| 存在（第 252 行：`"7. cite source for EVERY point."`）|

三个函数均包含两个强制约束文本，满足 PRD 第五章要求，**无 BLOCKER**。

---

### F-011：`knowledge_generate_summary` 的缓存检查和 LLM 调用之间存在二次锁获取，数据库连接不安全持有（严重性：极低，架构观察）

**位置**：`commands/knowledge_understanding.rs` 第 132–160 行

**描述**：
```rust
// 第一次锁：缓存检查
if !force_regenerate {
    let conn = db.conn.lock()...;
    if get_summary(&conn, &concept_id)?.is_some() {
        return Ok("cached".to_string());
    }
} // conn 锁在此释放

// 第二次锁：读取 LLM 配置和概念信息
let (client, concept_name, excerpts) = {
    let conn = db.conn.lock()...;
    ...
}; // conn 锁在此释放

// LLM 调用（无锁，正确）
let result = chat_completion(&client, messages).await...;

// 第三次锁：写入数据库
{
    let conn = db.conn.lock()...;
    save_summary(&conn, &summary)?;
}
```

三次获取和释放锁，设计上是正确的（async 函数不能跨 await 持有 Mutex）。但在缓存检查通过（无缓存）到最终写入之间，另一个并发调用理论上也能通过缓存检查并调用 LLM，导致两次 LLM 调用和两次写入（第二次 INSERT OR REPLACE 会覆盖第一次，功能不损坏但浪费 LLM 调用）。

对于单用户桌面 app，此场景极不可能发生，但值得记录。

---

## 领域特定审查核查矩阵

### 1. Prompt 安全约束（BLOCKER 级专项）

| 检查项 | 要求 | 实际 | 结论 |
|--------|------|------|------|
| `build_summary_prompt` 包含 "ONLY use information from provided documents" | 必须 | 存在（第 154 行）| PASS |
| `build_summary_prompt` 包含 "cite source for EVERY point" | 必须 | 存在（第 155 行）| PASS |
| `build_explanation_prompt` 包含上述两条约束 | 必须 | 存在（第 199-200 行）| PASS |
| `build_mirror_prompt` 包含上述两条约束 | 必须 | 存在（第 250-252 行）| PASS |

### 2. 来源校验

| 检查项 | 要求 | 实际 | 结论 |
|--------|------|------|------|
| `knowledge_generate_explanation` 写入前校验 `mechanism.source` 非空 | AC-4 必须 | 存在（第 292–294 行）| PASS |

### 3. 镜子反馈隔离

| 检查项 | 要求 | 实际 | 结论 |
|--------|------|------|------|
| `save_user_explanation` 的 UPDATE 不含 `mirror_feedback` 列 | AC-7 必须 | UPDATE 仅含 `user_explanation`、`updated_at`（第 235–241 行）| PASS |
| `save_user_explanation` 的 INSERT 将 `mirror_feedback` 显式设为 NULL | AC-7 必须 | INSERT 包含 `mirror_feedback, NULL`（第 247 行）| PASS |

### 4. 缓存逻辑

| 检查项 | 要求 | 实际 | 结论 |
|--------|------|------|------|
| summary：force_regenerate=false 且缓存存在 → 直接返回 | AC-3 | 正确（第 133–138 行）| PASS |
| summary：force_regenerate=true → 跳过缓存检查直接调用 LLM | AC-3 | 正确（if !force_regenerate 分支）| PASS |
| explanation：缓存逻辑同上 | AC-4 | 正确（第 220–225 行）| PASS |
| mirror：不缓存，每次都调用 LLM | AC-5 | 正确（无缓存检查）| PASS |

### 5. Event 命名

| 检查项 | 要求 | 实际 | 结论 |
|--------|------|------|------|
| summary event 名称 | `"knowledge:summary:chunk"` | `"knowledge:summary:chunk"`（第 181 行）| PASS |
| explanation event 名称 | `"knowledge:explanation:chunk"` | `"knowledge:explanation:chunk"`（第 271 行）| PASS |
| mirror event 名称 | `"knowledge:mirror:chunk"` | `"knowledge:mirror:chunk"`（第 398 行）| PASS |

### 6. LLM 流式说明

| 检查项 | 要求 | 实际 | 结论 |
|--------|------|------|------|
| 非流式实现有清晰注释说明 | output.md 已知局限 | 代码注释存在（第 174 行"当前使用非流式"）| PASS |
| 当前功能正确性不受影响 | - | 单 chunk is_final=true，前端可正常接收 | PASS |

### 7. KnowledgeError Serialize

| 检查项 | 要求 | 实际 | 结论 |
|--------|------|------|------|
| `KnowledgeError` 实现 `serde::Serialize` | AC-10 | `#[derive(Debug, Serialize)]`（第 21 行）| PASS |

---

## AC 完整性逐条核查

| AC | 描述 | 检查结论 | 问题 |
|----|------|----------|------|
| AC-1 | 6 个 `#[tauri::command]` 函数存在，编译无错误 | PASS | 无 |
| AC-2 | 3 个 Prompt 函数含 CRITICAL RULES | PASS | 无 BLOCKER；系统 Prompt 散落在 Command 中（F-009，低）|
| AC-3 | generate_summary 缓存逻辑 + LLM + 事件 + DB 写入 | PASS | source_asset_ids 为空（F-005，低）|
| AC-4 | generate_explanation JSON 校验 + source 非空校验 | PASS（条件）| JSON 边界提取不对称（F-002，中）；未对 mirror 做同等校验（F-003 对比）|
| AC-5 | validate_explanation 无缓存 + mirror_feedback 写入 | PASS（条件）| mirror_feedback 存原始字符串未 JSON 校验（F-003，中）|
| AC-6 | get_understanding_data 三个 Option 字段 | PASS | 无 |
| AC-7 | save_user_note SQL 不含 mirror_feedback | PASS | 无 |
| AC-8 | get_relations 双向查询 + JOIN concepts + DESC 排序 | PASS（条件）| 未 JOIN source assets 表（F-007，低）|
| AC-9 | 6 个 Commands 注册到 invoke_handler | PASS | 无 |
| AC-10 | KnowledgeError 实现 Serialize | PASS（技术层）| Command 返回 String 而非 KnowledgeError（F-001，中）|

---

## 发现汇总

| 编号 | 严重性 | 类别 | 简述 |
|------|--------|------|------|
| F-001 | 中 | 错误类型设计 | KnowledgeError 定义了 Serialize 但 Command 均返回 String，前端无法区分错误类型 |
| F-002 | 中 | 鲁棒性 | explanation JSON 边界提取使用 `find('{')` + `rfind('}')`，两端不对称，LLM markdown 包装时可能解析失败 |
| F-003 | 中 | 数据完整性 | mirror_feedback 存储原始 LLM 字符串未 JSON 校验，与 PRD schema 要求不符 |
| F-004 | 低 | 并发安全 | save_user_explanation 和 save_mirror_feedback 独立 INSERT 分支在理论竞争场景下可触发 UNIQUE 冲突 |
| F-005 | 低 | 数据完整性 | source_asset_ids 始终为空数组，来源追溯功能缺失（已知局限，主动披露）|
| F-006 | 低 | Prompt 质量 | project_name 始终为空字符串，Prompt 来源标注格式异常（已知局限，主动披露）|
| F-007 | 低 | AC 覆盖 | get_relations 未 JOIN source assets 表（AC-8 括号说明有自由度）|
| F-008 | 低 | 代码质量 | other_concept_id 字段与 SQL AS other_id 别名不一致，仅靠位置访问维护 |
| F-009 | 低 | 架构一致性 | 系统 Prompt 硬编码在 Command 函数中，未集中到 prompts.rs 统一管理 |
| F-010 | N/A | BLOCKER 专项 | Prompt 安全约束全部存在，**无 BLOCKER** |
| F-011 | 极低 | 并发观察 | 缓存检查与写入之间存在理论双写窗口（单用户桌面 app 实际风险极低）|
