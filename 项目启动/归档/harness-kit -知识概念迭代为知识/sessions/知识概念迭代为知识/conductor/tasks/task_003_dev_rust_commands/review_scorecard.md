# Review Scorecard — task_003_dev_rust_commands

> Reviewer: Code Reviewer（最终评分卡）
> 审查日期: 2026-04-11
> 审查范围: commands/knowledge_understanding.rs（新建）、db/knowledge_understanding.rs（新建）、llm/prompts.rs（追加）、lib.rs（invoke_handler 注册）
> 参照文档: task_003 input.md、code_review.md、session_context.md

---

## 一、审查思考过程（逐条 AC 检查）

### AC-1：6 个 `#[tauri::command]` 函数存在且编译无错误

**检查结果：PASS**

实际代码确认 6 个函数均存在且带 `#[tauri::command]` 标注：
- `knowledge_get_understanding_data`（第 103 行）
- `knowledge_generate_summary`（第 126 行）
- `knowledge_generate_explanation`（第 213 行）
- `knowledge_validate_explanation`（第 354 行）
- `knowledge_save_user_note`（第 423 行）
- `knowledge_get_relations`（第 440 行）

output.md 报告 `cargo check` 0 errors，4 条预存警告均来自已有代码，本次未引入新警告。

---

### AC-2：3 个 Prompt 函数含 CRITICAL RULES（BLOCKER 专项）

**检查结果：PASS**

逐字核查 `llm/prompts.rs` 中追加的三个函数：

`build_summary_prompt`（第 141–159 行）：
- 第 154 行：`"ONLY use information from provided documents."` ✓
- 第 155 行：`"cite source for EVERY point."` ✓

`build_explanation_prompt`（第 166–210 行）：
- 第 193 行：CRITICAL RULES 第 1 条：`"You MUST ONLY use information from the student's documents provided above."` ✓
- 第 199 行：第 6 条：`"ONLY use information from provided documents."` ✓
- 第 200 行：第 7 条：`"cite source for EVERY point."` ✓
- 还包含第 2 条（不引入文档外信息）、第 3 条（每个点必须引用来源）、第 5 条（不捏造），共 7 条约束

`build_mirror_prompt`（第 217–255 行）：
- 第 250 行：第 6 条：`"ONLY use information from provided documents."` ✓
- 第 252 行：第 7 条：`"cite source for EVERY point."` ✓
- 还包含第 1 条（仅对比文档）、第 2 条（鼓励性语言）、第 4 条（探索式表述）、第 5 条（不评判）

**无 BLOCKER。** 三个函数均满足 PRD 第五章的强制约束，且约束内容丰富（不仅仅是最低要求的两条）。

**轻微问题（F-009）**：系统 Prompt 的约束文本（在 Command 函数中硬编码）与 prompts.rs 中的用户 Prompt 约束文本存在重复，且系统 Prompt 未集中到 prompts.rs 管理。违反 session_context.md"Prompt 模板集中管理"规范，但不影响功能正确性。

---

### AC-3：generate_summary 缓存逻辑 + LLM + 事件推送 + DB 写入

**检查结果：PASS**

逐项验证：
- 缓存判断（第 132–138 行）：`!force_regenerate` 且 `get_summary()?.is_some()` → 返回 `"cached"` ✓
- force_regenerate=true 时跳过缓存检查，直接进入 LLM 调用 ✓
- LLM 调用：`chat_completion(&client, messages).await` ✓
- 事件推送：`app.emit("knowledge:summary:chunk", ChunkPayload { is_final: true })` ✓（注：单 chunk 非流式，已注释说明）
- DB 写入：`save_summary(&conn, &summary)` ✓

**已知局限（output.md 主动披露）**：`source_asset_ids: vec![]` 始终为空，来源追溯功能未完整实现（F-005）。此为功能性缺口，但 output.md 明确标注为已知局限。

---

### AC-4：generate_explanation JSON 校验 + mechanism.source 非空校验

**检查结果：PASS（条件性）**

验证：
- JSON 解析（第 283–289 行）：`serde_json::from_str(&result[json_start..json_end])` ✓
- 解析失败返回 `Err`（前缀 `"Invalid LLM response: "`）✓
- `mechanism.source.trim().is_empty()` 非空校验（第 292–294 行）✓
- 校验失败返回 `Err` 拒绝写入 ✓
- 事件名称 `"knowledge:explanation:chunk"` ✓

**问题（F-002，中）**：JSON 边界提取使用 `find('{')` + `rfind('}')`，前者取第一个 `{`，后者取最后一个 `}`，在 LLM 输出包含说明文字时能工作，但若 LLM 用 markdown 代码块包裹（` ```json {...} ``` `），`find('{')` 找到的 `{` 正确，但 `rfind('}')` 之后还有 ` ``` ` 字符，截取范围为 `{...}` + 后面的非 JSON 内容，导致解析失败。

此问题在实际调用中出现概率中等（部分 OpenAI 兼容实现会添加 markdown 包裹），但不影响 AC-4 的核心设计意图满足。

---

### AC-5：validate_explanation 无缓存 + mirror_feedback 写入

**检查结果：PASS（条件性）**

验证：
- 无缓存检查（直接调用 LLM）✓
- 事件名称 `"knowledge:mirror:chunk"` ✓
- `save_mirror_feedback(&conn, &concept_id, &result, &now)` 写入 DB ✓

**问题（F-003，中）**：`mirror_feedback` 存储的是 LLM 原始响应字符串，未做 JSON 提取和校验。AC-5 要求"将 mirror_feedback 序列化为 JSON 存入"，但实际是将原始字符串直接存储。若 LLM 在 JSON 前后输出说明性文字，存储内容将包含非 JSON 文本，前端 `JSON.parse` 会失败。

此问题比 explanation 的处理更宽松（explanation 有 JSON 校验，mirror 无），不一致性需要注意。

---

### AC-6：get_understanding_data 返回三个 Option 字段

**检查结果：PASS**

`UnderstandingData { summary: Option<ConceptSummary>, explanation: Option<ConceptExplanation>, user_note: Option<ConceptUserNote> }`（第 63–69 行）

三个字段均为 `Option`，数据库查询使用 `.optional()` 扩展，无数据时返回 `None` 而非错误（db/knowledge_understanding.rs 第 88、147 行）。

---

### AC-7：save_user_note SQL 只更新 user_explanation 和 updated_at，不含 mirror_feedback

**检查结果：PASS**

`save_user_explanation` 函数（db/knowledge_understanding.rs 第 226–253 行）：

UPDATE 分支（第 235–241 行）：
```sql
UPDATE concept_user_notes
SET user_explanation = ?2, updated_at = ?3
WHERE concept_id = ?1
```
仅更新 `user_explanation` 和 `updated_at`，不含 `mirror_feedback` ✓

INSERT 分支（第 243–252 行）：`mirror_feedback` 显式设为 `NULL`，不使用调用方传入的任何 mirror 数据 ✓

**镜子反馈隔离硬约束完全满足。**

---

### AC-8：get_relations 双向查询 + JOIN concepts + DESC 排序

**检查结果：PASS（条件性）**

SQL 验证（db/knowledge_understanding.rs 第 293–305 行）：
- `WHERE cr.concept_a_id = ?1 OR cr.concept_b_id = ?1` — 双向查询 ✓
- `LEFT JOIN concepts c ON c.id = (CASE WHEN ... THEN ... ELSE ... END)` — JOIN 获取对方概念名 ✓
- `ORDER BY cr.co_occurrence_count DESC` ✓
- `LIMIT 8` ✓

**轻微问题（F-007，低）**：AC-8 要求"JOIN source assets 表获取文档名称（如相关表存在）"。当前未 JOIN source assets 表，`source_asset_ids` 以 JSON 数组形式返回，由前端自行查询文档名。括号中的"如相关表存在"给予了实现自由度，判定为 PASS。

---

### AC-9：6 个 Commands 注册到 invoke_handler

**检查结果：PASS**

`lib.rs` 第 138–145 行（`// v3: Knowledge Understanding` 注释块）：
```rust
commands::knowledge_understanding::knowledge_get_understanding_data,
commands::knowledge_understanding::knowledge_generate_summary,
commands::knowledge_understanding::knowledge_generate_explanation,
commands::knowledge_understanding::knowledge_validate_explanation,
commands::knowledge_understanding::knowledge_save_user_note,
commands::knowledge_understanding::knowledge_get_relations,
```
6 个 Command 全部注册，顺序与 AC-1 声明的函数列表一致 ✓

---

### AC-10：KnowledgeError 实现 serde::Serialize

**检查结果：PASS（技术层），含技术债务说明**

`#[derive(Debug, Serialize)]` + `#[serde(tag = "kind", content = "message")]`（第 21–28 行）

KnowledgeError 实现了 Serialize，可序列化为 `{"kind": "LlmError", "message": "..."}` 格式 ✓

**技术债务（F-001，中）**：所有 6 个 Command 实际返回 `Result<T, String>` 而非 `Result<T, KnowledgeError>`，前端收到的是字符串而非结构化错误对象。AC-10 的"前端可正确接收错误信息"以字符串形式满足，但失去了错误类型区分能力。output.md 主动披露并说明了理由（与已有 `knowledge.rs` 保持一致性）。

---

## 二、6 维评分表

（权重采用 task_003 input.md 指定的领域特定权重：功能 35% / 安全 20% / 代码质量 15% / 测试 10% / 架构 15% / 可维护性 5%）

| 维度 | 权重 | 得分（/10） | 加权得分 | 评分理由 |
|------|------|-------------|----------|----------|
| **功能正确性** | 35% | 7.5 | 2.63 | 6 个 Command 逻辑正确，缓存判断、Event 命名、DB 隔离全部满足核心 AC。扣分点：mirror_feedback 未做 JSON 校验（F-003，中），explanation JSON 边界提取不对称（F-002，中），source_asset_ids 为空（F-005，低，但影响数据完整性），get_relations 未 JOIN source assets（F-007，低）。 |
| **安全性（Prompt+数据隔离）** | 20% | 9.0 | 1.80 | 三个 Prompt 均包含强制安全约束（F-010 无 BLOCKER）。镜子反馈隔离严格（AC-7 完全满足）。mechanism.source 非空校验存在（AC-4）。唯一扣分：mirror_feedback 存储原始字符串，未校验内容是否符合 JSON schema，LLM 幻觉内容可能以非预期格式写入（F-003）。系统 Prompt 约束重复且分散（F-009）。 |
| **代码质量** | 15% | 7.5 | 1.13 | 代码结构清晰，注释完整，分层合理（commands/db/prompts 三层分离）。扣分：JSON 边界提取不对称（F-002），系统 Prompt 硬编码在函数中（F-009），KnowledgeError 定义与实际使用脱节（F-001），other_concept_id/other_id 命名不一致（F-008）。 |
| **测试覆盖** | 10% | 5.0 | 0.50 | output.md 报告 cargo check 0 errors，但无单元测试（Command 层无测试，db 层无测试）。本 task input.md 说明"主要是 cargo check，LLM 无法单元测试"，但 db/knowledge_understanding.rs 中的 SQL 逻辑（get_summary、get_explanation 等）完全可以写单元测试（类似 task_002 的 migration 测试模式）。缺少任何 DB 层测试是明显的覆盖盲区。 |
| **架构一致性** | 15% | 8.5 | 1.28 | 遵循扁平模块结构（commands/db/llm），与已有代码库完全一致。正确使用 `State<'_, Database>`、`Mutex` 锁模式、`tauri::Emitter`。invoke_handler 注册格式与已有条目一致。扣分：系统 Prompt 未集中到 prompts.rs（F-009），违反 session_context.md 规范。Command 错误类型 String 而非 KnowledgeError（F-001），虽然有一致性理由，但结构设计机会损失。 |
| **可维护性** | 5% | 8.0 | 0.40 | 三条已知局限主动披露，透明度高。代码注释说明非流式实现及原因。KnowledgeError 设计说明清晰。扣分：Prompt 文本分散管理导致未来修改需双处同步（F-009）。JSON 边界提取的临时方案（F-002）未添加 TODO 注释。 |

**综合得分：2.63 + 1.80 + 1.13 + 0.50 + 1.28 + 0.40 = 7.74 / 10**

---

## 三、总体判断

**PASS**

理由：
- 所有 10 个 AC 的核心功能均已实现，`cargo check` 0 errors
- 无 BLOCKER（Prompt 安全约束全部存在，镜子反馈隔离满足硬约束）
- 发现的问题中，最高严重性为"中"（F-001/F-002/F-003），均为功能完备性或鲁棒性问题，不影响核心功能的正确运行
- Dev 主动披露 3 条已知局限，信息透明
- 所有中级问题可在后续 task 或专项修复中处理，不阻断进入 task_004

---

## 四、问题列表

### BLOCKER（阻断级）

无

### FIX（必须追踪，应在后续 task 或热修复中处理）

| 编号 | 问题 | 位置 | 建议 |
|------|------|------|------|
| FIX-001（F-003）| `knowledge_validate_explanation` 将 LLM 原始响应直接存入 `mirror_feedback`，未做 JSON 提取和格式校验，前端 `JSON.parse` 可能失败 | commands/knowledge_understanding.rs 第 406–414 行 | 参照 `generate_explanation` 的处理模式，用 `find('{')` + `rfind('}')` 提取 JSON 边界，解析后验证包含 `covered_count`、`additional_perspectives` 等关键字段，再存储；解析失败时返回 Err 不写入 |
| FIX-002（F-002）| explanation 的 JSON 边界提取使用 `find('{')` + `rfind('}')`（前后不对称），LLM 用 markdown 包裹时可能截取到 ` ``` ` 尾部字符导致解析失败 | commands/knowledge_understanding.rs 第 281–289 行 | 改为先尝试 `find('{')..rfind('}')` 截取解析，失败时返回有意义错误；或使用 `serde_json::from_str` 并逐步向后找有效 JSON 起始位置 |

### MINOR（建议修复，可在专项 tech-debt 或下次接触时处理）

| 编号 | 问题 | 位置 | 建议 |
|------|------|------|------|
| M-001（F-001）| `KnowledgeError` 定义了 Serialize 但所有 Command 返回 `String` 错误，前端无法区分错误类型 | commands/knowledge_understanding.rs 全部 Command 签名 | 在确认与前端错误处理协议后，将 Command 签名改为 `Result<T, KnowledgeError>`；短期内可保持现状，但建议添加 TODO 注释 |
| M-002（F-005）| `source_asset_ids` 始终为空数组，来源追溯功能缺失 | commands/knowledge_understanding.rs 第 195、335 行 | 从 `ConceptCase` 提取 `source_asset_id`（若字段存在）构建列表；此功能对"离线查看时追溯来源"有实际意义（session_context.md 硬约束第 4 条） |
| M-003（F-009）| 系统 Prompt 硬编码在 Command 函数中，未集中到 prompts.rs | commands/knowledge_understanding.rs 第 163–172、255–264、382–391 行 | 将三个系统 Prompt 字符串提取为 `prompts.rs` 中的常量或函数，满足 session_context.md"Prompt 模板集中管理"规范 |
| M-004（F-004）| `save_user_explanation` 和 `save_mirror_feedback` 独立 INSERT 分支在理论竞争时可触发 UNIQUE 约束冲突 | db/knowledge_understanding.rs 第 234、262 行 | 改用 `INSERT OR IGNORE` 确保记录存在，再执行各自的 UPDATE，消除 INSERT 竞争 |
| M-005（F-006）| `project_name` 始终为空字符串，Prompt 来源标注格式异常 | commands/knowledge_understanding.rs 第 149–157、244–249 行 | 解析 `ConceptCase.title` 按 `" / "` 分隔提取 project_name 和 asset_name |
| M-006（F-007）| get_relations 未 JOIN source assets 表，前端无法单次获取文档名 | db/knowledge_understanding.rs 第 288–329 行 | 若需要，可在后续迭代中 JOIN source assets 表（assets 或 source_assets）扩展返回字段；当前以 ID 返回可接受 |

### INFO（知识性记录，无需行动）

| 编号 | 问题 | 说明 |
|------|------|------|
| I-001（F-008）| `ConceptRelation.other_concept_id` 与 SQL `AS other_id` 别名不一致，按位置访问 | 功能正确，不影响运行；下次修改此 SQL 时可统一别名为 `other_concept_id` |
| I-002（F-011）| 缓存检查与 DB 写入之间的理论双写窗口 | 单用户桌面 app，Mutex 锁缩短竞争窗口，实际风险极低；记录供未来多窗口场景参考 |
| I-003 | task_002 review 指出 `concept_summaries` 和 `concept_explanations` 的 `concept_id` 缺少 UNIQUE 约束（M-001），task_003 使用 `INSERT OR REPLACE` upsert 语义，在该约束缺失的情况下仍能保证"最多一条"语义 | INSERT OR REPLACE 依赖 PRIMARY KEY 的唯一性（`id` 字段），而非 `concept_id` 字段的唯一性；若两次写入使用不同的 UUID，则会产生两条 concept_id 相同的记录。建议在 task_004 或 V5 migration 中补加 `UNIQUE(concept_id)` 约束（跨任务 M-001 的延续） |

---

## 五、修复指引

本次判断为 **PASS**，可进入 task_004 开发阶段。

**优先修复项（建议在 task_004 之前或期间处理）**：

1. **FIX-001（mirror_feedback JSON 校验）**：`knowledge_validate_explanation` 应与 `knowledge_generate_explanation` 保持一致的 JSON 提取和校验流程。前端期望 `mirror_feedback` 是 `{covered_count, covered_points, additional_perspectives, difference_note}` 格式的 JSON，当前存储裸字符串可能导致前端解析失败。建议在 task_004 开始前修复，避免前后端联调时发现问题。

2. **FIX-002（JSON 边界提取）**：在调试期间若遇到 LLM 返回 markdown 包裹的 JSON，会导致 `generate_explanation` 持续失败。建议简单加固：解析失败时打印原始内容前 300 字，便于调试。

**task_004 输入文档需补充说明（跨任务协调）**：

- `concept_relations` 写入时必须强制 `concept_a_id < concept_b_id` 排序（来自 task_002 M-002 的延续要求）
- `concept_summaries` 和 `concept_explanations` 的 upsert 需保证"相同 concept_id 只有一条"，建议在 task_004 中或 V5 migration 中补加 `UNIQUE(concept_id)` 约束（来自 task_002 M-001 + I-003 的延续）

---

## 六、综合结论

task_003 是一个功能完整性良好、安全约束到位的 Rust 后端实现交付物：

- 6 个 Command 全部实现并注册，`cargo check` 0 errors
- 三个 Prompt 的 CRITICAL RULES 约束完整，无 BLOCKER
- 镜子反馈隔离（AC-7）严格满足，save_user_explanation 和 save_mirror_feedback 职责分离清晰
- Event 命名精确匹配前端依赖，缓存逻辑正确
- Dev 主动披露 3 条已知局限，透明度高

主要问题集中在两个"中级"发现：
1. mirror_feedback 存储未做 JSON 校验（F-003），与 explanation 处理不一致
2. explanation JSON 边界提取不够健壮（F-002），LLM 加 markdown 包裹时可能失败

这两个问题在 LLM 标准输出下不影响功能，但在边缘情况下可能导致数据写入异常，建议在前后端联调前修复。

测试覆盖是本次交付的明显短板（5.0/10）：DB 层的 SQL 逻辑（类似 task_002 的模式）完全可以添加单元测试，但本 task 未包含任何测试。

**综合得分：7.74 / 10**
**判断：PASS — 可进入 task_004 开发阶段，FIX-001 和 FIX-002 建议优先处理**
