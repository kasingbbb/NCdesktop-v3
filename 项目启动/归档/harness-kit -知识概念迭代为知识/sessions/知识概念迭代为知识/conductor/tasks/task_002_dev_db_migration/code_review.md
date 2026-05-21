# Code Review — task_002_dev_db_migration

> 审计者：Code Reviewer
> 审计日期：2026-04-11
> 审计范围：migration.rs（V4 函数追加）+ mod.rs（测试追加 + 断言修改）
> 参照文档：task_001_architect/output.md（数据模型 Schema）、session_context.md

---

## 审计发现列表

---

### F-001：版本门控逻辑存在累积跳过缺陷（严重性：中）

**位置**：`migration.rs` 第 9–23 行，`run_migrations()` 函数

**描述**：
`run_migrations()` 在函数入口处只读取一次 `current_version`，随后用四个独立的 `if current_version < N` 分支做版本判断。

问题：当数据库从 `version=0` 全新初始化时，执行 V1 迁移后 `PRAGMA user_version = 1` 已写入数据库，但 Rust 函数中的 `current_version` 变量仍是原始值 `0`。因此 V1 完成后，V2/V3/V4 的条件（`0 < 2`、`0 < 3`、`0 < 4`）同样为 true，四个 migration 都会依次执行。

这在当前实现中没有实际 bug（因为所有 CREATE TABLE 均使用 `IF NOT EXISTS`，V2 的 `ALTER TABLE` 在全新库上只执行一次），但这个模式是脆弱的：

- V2 函数使用了 `ALTER TABLE assets ADD COLUMN original_name`，该语句**不是幂等的**（SQLite 不支持 `ADD COLUMN IF NOT EXISTS`）。如果 V1 已在之前的 session 执行、`user_version=1` 已保存，则重新打开时 `current_version=1`，V2 会正常执行一次。但如果 `current_version` 读取是 `0`，V1 和 V2 会都执行。这在全新库场景是安全的，但逻辑意图（"每个版本只在 `current_version < N` 时执行一次"）被 `current_version` 不刷新所破坏。
- 从审计角度，这个模式是 V1/V2/V3 遗留的，V4 遵从了同样的模式，因此 V4 本身没有引入新风险。**但此发现值得记录，供后续重构参考。**

**与 Spec 关系**：Architect output.md 未指定 migration 的执行流程控制方式，但幂等性要求明确。

---

### F-002：`concept_summaries` 表缺少"每个 concept_id 最多一条"的 UNIQUE 约束（严重性：中）

**位置**：`migration.rs` 第 153–162 行，`concept_summaries` 表定义

**描述**：
Architect output.md 明确说明：

> 每个 `concept_id` 最多一条记录（若已有则返回缓存，不重复生成）

PRD Schema 中也说明"DELETE + INSERT"的 upsert 语义依赖业务逻辑而非数据库约束，但若不在数据库层加 `UNIQUE(concept_id)` 约束，无法在数据层防止意外写入多条记录（例如并发调用 `knowledge_generate_summary` 时）。

对比参考：`concept_user_notes` 表在 `concept_id` 上正确加了 `UNIQUE` 约束（第 183 行），且 Architect Schema 也明确标注了 `UNIQUE`。

`concept_summaries` 的 Architect Schema 中未写 `UNIQUE(concept_id)`，但业务说明是"最多一条"，与 `concept_explanations` 相同（后者同样未加 UNIQUE 约束）。

**实际情况**：实现与 Architect Schema 字面一致（两者均无 UNIQUE 约束），但业务语义要求与表结构不匹配，需要下游 Command 层严格保证串行，否则存在数据重复风险。

---

### F-003：`concept_explanations` 表同样缺少"每个 concept_id 最多一条"的 UNIQUE 约束（严重性：中，与 F-002 同源）

**位置**：`migration.rs` 第 165–178 行，`concept_explanations` 表定义

**描述**：同 F-002，Architect output.md 说明"每个 `concept_id` 最多一条记录"，但实现和 Spec Schema 均未加 `UNIQUE(concept_id)` 约束。此发现与 F-002 一并记录。

---

### F-004：Architect 方案要求 `concept_relations` 强制 `concept_a_id < concept_b_id`，但实现未在数据库层强制（严重性：低）

**位置**：`migration.rs` 第 193–206 行，`concept_relations` 表定义

**描述**：
Architect output.md 明确说明：

> 共现关系：`concept_a_id < concept_b_id`（字符串排序，保证无重复）

当前实现使用 `CREATE UNIQUE INDEX IF NOT EXISTS idx_concept_relations_pair ON concept_relations(concept_a_id, concept_b_id, relation_type)`。此唯一索引防止了完全相同的 `(a, b, type)` 三元组重复，但无法防止 `(a, b, type)` 和 `(b, a, type)` 同时存在（即方向互换的两条记录均能写入）。

实现层面的方向排序约束依赖 `knowledge_compute_co_occurrence` Command（task_003/task_004）在写入前强制 `a < b`，数据库层无 CHECK 约束。Dev 在 output.md 的"已知局限 2"中已主动提及。

**影响**：若 Command 层疏忽方向排序，关系网络查询结果将出现重复边（两个方向各一条），导致前端展示异常。这是一个跨任务的协调依赖，需要在 task_003/task_004 中明确处理。

---

### F-005：V3 测试断言从 `assert_eq!(v, 3)` 改为 `assert!(v >= 3)`，掩盖了可能的版本回退场景（严重性：低）

**位置**：`mod.rs` 第 81 行、第 122 行

**描述**：
修改的理由是正确的：全新库运行 V4 后 `user_version=4`，V3 测试若继续断言 `v == 3` 会失败。

但 `assert!(v >= 3)` 的宽松条件意味着：如果将来某个版本（如 V5）引入 bug 导致 `user_version` 被重置，或 migration 逻辑出现倒退，这两个测试无法捕捉到问题。

更精确的替代方案为：`assert_eq!(v, 4, "全量迁移后应为最新版本")`（由 `open_runs_migrations` 覆盖语义），或对 V3 测试使用独立的受控环境，仅跑到 V3 然后断言 `v == 3`，但这需要暴露更细粒度的迁移函数，改动较大。

**当前实现的实际风险**：低，因为 `migration_v4_creates_knowledge_tables` 和 `migration_v4_is_idempotent` 已断言 `v == 4`（精确匹配），能够捕捉主要异常。

---

### F-006：`migration_v4_is_idempotent` 测试的注释描述与实际行为不完全一致（严重性：低，文档质量问题）

**位置**：`mod.rs` 第 113 行注释

**描述**：
`migration_v3_is_idempotent` 的注释写："第二次打开（所有 migration 已跑，user_version=3，不应重复执行）"。但当 V4 上线后，第二次打开时 `user_version` 实际上是 4，而不是 3。注释未更新，与实际行为不符。

`migration_v4_is_idempotent` 的对应注释（第 172 行）是正确的："user_version=4"。

---

### F-007：V4 测试未覆盖"从已有 V3 数据库升级到 V4"的路径（严重性：低）

**位置**：`mod.rs`，`migration_v4_creates_knowledge_tables` 和 `migration_v4_is_idempotent` 测试

**描述**：
两个 V4 测试均从空数据库（version=0）直接升级到 V4，等价于"全新安装"路径。

存量 V3 用户（`user_version=3`）在升级后打开 app，实际路径是：进入 `run_migrations`，`current_version=3`，仅执行 `if current_version < 4` 分支。这个路径未被测试覆盖。

该场景在当前实现中应是安全的（V4 函数只做 CREATE TABLE IF NOT EXISTS），但缺少显式测试，属于测试覆盖盲区。

---

### F-008：`concept_user_notes` 的 `idx_concept_user_notes_concept_id` 索引在 UNIQUE 约束已存在的情况下冗余（严重性：极低，优化建议）

**位置**：`migration.rs` 第 190 行

**描述**：
`concept_user_notes.concept_id` 上已有 `UNIQUE` 约束（第 183 行），SQLite 会自动为 UNIQUE 约束创建隐式唯一索引。额外的 `CREATE INDEX IF NOT EXISTS idx_concept_user_notes_concept_id ON concept_user_notes(concept_id)` 会创建一个非唯一普通索引，与隐式唯一索引重叠，形成冗余索引（两个索引都维护，浪费写入开销）。

对于小规模桌面 app 影响可以忽略，但属于 SQLite 使用上的不精确。

---

### F-009：V4 SQL 缩进风格与 V1/V2/V3 不一致（严重性：极低，代码风格）

**位置**：`migration.rs` 第 150–209 行

**描述**：
V1/V2/V3 的 SQL 使用 8 空格缩进（对齐字段名），V4 使用 2 空格缩进。在单文件多版本的 migration 模式下，风格不统一略影响可读性。

---

## Schema 逐字段对照结果

### `concept_summaries`

| 字段 | Spec | 实现 | 一致性 |
|------|------|------|--------|
| id | TEXT PRIMARY KEY | TEXT PRIMARY KEY | 一致 |
| concept_id | TEXT NOT NULL | TEXT NOT NULL | 一致 |
| summary | TEXT NOT NULL | TEXT NOT NULL | 一致 |
| source_asset_ids | TEXT NOT NULL | TEXT NOT NULL | 一致 |
| model | TEXT NOT NULL | TEXT NOT NULL | 一致 |
| generated_at | TEXT NOT NULL | TEXT NOT NULL | 一致 |
| FOREIGN KEY | concepts(id) ON DELETE CASCADE | concepts(id) ON DELETE CASCADE | 一致 |

### `concept_explanations`

| 字段 | Spec | 实现 | 一致性 |
|------|------|------|--------|
| id | TEXT PRIMARY KEY | TEXT PRIMARY KEY | 一致 |
| concept_id | TEXT NOT NULL | TEXT NOT NULL | 一致 |
| mechanism | TEXT NOT NULL | TEXT NOT NULL | 一致 |
| typical_scenarios | TEXT NOT NULL | TEXT NOT NULL | 一致 |
| common_misconceptions | TEXT（可为 NULL） | TEXT（无 NOT NULL）| 一致 |
| essence_sentence | TEXT NOT NULL | TEXT NOT NULL | 一致 |
| source_asset_ids | TEXT NOT NULL | TEXT NOT NULL | 一致 |
| model | TEXT NOT NULL | TEXT NOT NULL | 一致 |
| generated_at | TEXT NOT NULL | TEXT NOT NULL | 一致 |
| FOREIGN KEY | concepts(id) ON DELETE CASCADE | concepts(id) ON DELETE CASCADE | 一致 |

### `concept_user_notes`

| 字段 | Spec | 实现 | 一致性 |
|------|------|------|--------|
| id | TEXT PRIMARY KEY | TEXT PRIMARY KEY | 一致 |
| concept_id | TEXT NOT NULL UNIQUE | TEXT NOT NULL UNIQUE | 一致 |
| user_explanation | TEXT NOT NULL DEFAULT '' | TEXT NOT NULL DEFAULT '' | 一致 |
| mirror_feedback | TEXT（可为 NULL） | TEXT（无 NOT NULL）| 一致 |
| last_validated_at | TEXT（可为 NULL） | TEXT（无 NOT NULL）| 一致 |
| created_at | TEXT NOT NULL | TEXT NOT NULL | 一致 |
| updated_at | TEXT NOT NULL | TEXT NOT NULL | 一致 |
| FOREIGN KEY | concepts(id) ON DELETE CASCADE | concepts(id) ON DELETE CASCADE | 一致 |

### `concept_relations`

| 字段 | Spec | 实现 | 一致性 |
|------|------|------|--------|
| id | TEXT PRIMARY KEY | TEXT PRIMARY KEY | 一致 |
| concept_a_id | TEXT NOT NULL | TEXT NOT NULL | 一致 |
| concept_b_id | TEXT NOT NULL | TEXT NOT NULL | 一致 |
| relation_type | TEXT NOT NULL | TEXT NOT NULL | 一致 |
| source_asset_ids | TEXT NOT NULL | TEXT NOT NULL | 一致 |
| co_occurrence_count | INTEGER DEFAULT 1 | INTEGER DEFAULT 1 | 一致 |
| created_at | TEXT NOT NULL | TEXT NOT NULL | 一致 |
| FOREIGN KEY (a) | concepts(id) ON DELETE CASCADE | concepts(id) ON DELETE CASCADE | 一致 |
| FOREIGN KEY (b) | concepts(id) ON DELETE CASCADE | concepts(id) ON DELETE CASCADE | 一致 |

---

## 架构一致性核查

| 审计项 | 状态 | 备注 |
|--------|------|------|
| 所有写操作在 Rust 侧执行（ADR-001）| 符合 | migration.rs 纯 Rust |
| 增量添加新表，不修改已有表（ADR-002）| 符合 | V3 表未被触碰 |
| `CREATE TABLE IF NOT EXISTS` 幂等性 | 符合 | 4 张表全部使用 |
| `PRAGMA user_version = 4` 在 SQL batch 内 | 符合 | 在 execute_batch 字符串末尾 |
| `if current_version < 4` 版本门控 | 符合 | 位置正确 |
| `foreign_keys=ON` 在 `Database::open()` 启用 | 符合 | mod.rs 第 33 行 |
| 所有 4 张表均设置 `ON DELETE CASCADE` | 符合 | 全部正确 |
| 所有 Spec 要求的索引已创建 | 符合 | 共 6 个索引，含 1 个 UNIQUE INDEX |
| `concept_relations_pair` 复合唯一索引 | 符合 | `(concept_a_id, concept_b_id, relation_type)` |
| 未引入计划外新依赖（Cargo.toml 未修改）| 符合 | output.md 确认 |

---

## 发现汇总

| 编号 | 严重性 | 类别 | 简述 |
|------|--------|------|------|
| F-001 | 中 | 逻辑风险（遗留） | `current_version` 未在版本间刷新，V2 ALTER TABLE 不幂等，全量初始化依赖执行顺序安全性 |
| F-002 | 中 | Schema 约束缺失 | `concept_summaries.concept_id` 未加 UNIQUE，"最多一条"约束依赖业务层 |
| F-003 | 中 | Schema 约束缺失 | `concept_explanations.concept_id` 未加 UNIQUE，同 F-002 |
| F-004 | 低 | 跨任务协调依赖 | `concept_relations` 方向性排序（`a < b`）未在数据库层约束，依赖 Command 层保证 |
| F-005 | 低 | 测试质量 | V3 测试断言宽松化，掩盖潜在版本回退场景 |
| F-006 | 低 | 文档质量 | V3 幂等性测试注释过时，与 V4 上线后实际行为不符 |
| F-007 | 低 | 测试覆盖盲区 | 缺少"从 V3 升级到 V4"的显式测试路径 |
| F-008 | 极低 | 优化建议 | `concept_user_notes.concept_id` 同时有 UNIQUE 约束和普通索引，后者冗余 |
| F-009 | 极低 | 代码风格 | V4 SQL 缩进风格与 V1/V2/V3 不一致 |
