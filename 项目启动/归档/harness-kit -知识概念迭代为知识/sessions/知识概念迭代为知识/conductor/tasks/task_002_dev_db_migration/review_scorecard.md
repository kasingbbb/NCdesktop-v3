# Review Scorecard — task_002_dev_db_migration

> Reviewer: Code Reviewer（最终评分卡）
> 审查日期: 2026-04-11
> 审查范围: migration.rs（V4 函数）+ mod.rs（测试追加及断言修改）
> 参照文档: task_002 input.md、code_review.md、实际 migration.rs 代码

---

## 一、审查思考过程（逐条 AC 检查）

### AC-1：运行后存在且仅新增 4 张表

**检查结果：PASS**

实际代码（migration.rs 第 148–215 行）确认新增 4 张表：
- `concept_summaries`（第 153 行）
- `concept_explanations`（第 165 行）
- `concept_user_notes`（第 180 行）
- `concept_relations`（第 193 行）

测试输出确认：V4 四张表各自出现"✓"日志，`migration_v4_creates_knowledge_tables` PASS。

---

### AC-2：字段类型、NOT NULL 约束、DEFAULT 值、FOREIGN KEY 与 Schema 定义完全一致

**检查结果：PASS（与 Spec 字面一致）**

通过 code_review.md 的逐字段对照表和直接阅读 migration.rs 代码，所有 4 张表的字段定义与 input.md 中的 Schema 定义完全一致：

- `concept_summaries`：6 个字段，全部一致，FOREIGN KEY ON DELETE CASCADE 正确
- `concept_explanations`：9 个字段，`common_misconceptions` 正确为可 NULL，全部一致
- `concept_user_notes`：7 个字段，`concept_id UNIQUE` 正确，`user_explanation DEFAULT ''` 正确，全部一致
- `concept_relations`：7 个字段，`co_occurrence_count INTEGER DEFAULT 1` 正确，双 FOREIGN KEY 正确

**特别说明（F-002/F-003 的裁量）**：`concept_summaries` 和 `concept_explanations` 的 `concept_id` 未加 UNIQUE 约束，这与 input.md 的 Schema Spec 字面完全一致（Spec 也未写 UNIQUE）。业务语义上"最多一条"由 Architect output.md 的文字说明体现，但权威 Spec（input.md 的 Schema 定义）并未将其落实为数据库约束。此处实现与 Spec 一致，因此 AC-2 判定为 PASS，F-002/F-003 降级为 MINOR 记录。

---

### AC-3：正确的索引（含 `concept_relations` 复合唯一索引）

**检查结果：PASS**

代码实际索引：
- `idx_concept_summaries_concept_id`（第 162 行）
- `idx_concept_explanations_concept_id`（第 177 行）
- `idx_concept_user_notes_concept_id`（第 190 行）
- `idx_concept_relations_a`（第 204 行）
- `idx_concept_relations_b`（第 205 行）
- `idx_concept_relations_pair` UNIQUE（第 206 行，`concept_a_id, concept_b_id, relation_type`）

共 6 个索引，含 1 个 UNIQUE INDEX，全部符合 input.md AC-3 要求。

**轻微问题**：`concept_user_notes.concept_id` 已有 `UNIQUE` 约束，SQLite 自动创建隐式唯一索引，额外普通索引冗余（F-008）。对桌面 app 无实质影响。

---

### AC-4：使用 `CREATE TABLE IF NOT EXISTS`（幂等）

**检查结果：PASS**

4 张表全部使用 `CREATE TABLE IF NOT EXISTS`（migration.rs 第 153、165、180、193 行）。
测试 `migration_v4_is_idempotent` PASS 验证了重复打开不报错。

---

### AC-5：v2.1 已有表完整保留，无字段变更

**检查结果：PASS**

V4 函数中仅包含 4 张新表的 CREATE TABLE 语句，无 ALTER TABLE、无 DROP TABLE、无对已有表的任何操作。测试日志确认 V3 表（`concepts`、`concept_viewpoints`、`concept_cases`、`concept_extensions`、`course_events`、`course_previews`）均出现"✓"标记，CRUD 测试（`migration_v3_course_events_crud`）PASS。

---

### AC-6：`v4_knowledge_understanding(conn)` 函数 + `run_migrations()` 追加正确

**检查结果：PASS**

- 函数定义：migration.rs 第 149 行 `fn v4_knowledge_understanding(conn: &Connection) -> Result<(), String>`
- 版本门控：第 21–23 行 `if current_version < 4 { v4_knowledge_understanding(conn)?; }`
- 版本写入：SQL batch 末尾 `PRAGMA user_version = 4;`（第 208 行）
- 错误处理：`.map_err(|e| format!("V4 迁移失败: {e}"))?`（第 211 行）

所有要素完全符合 AC-6 要求。

---

### AC-7：已有测试通过 + 新增 `migration_v4_creates_knowledge_tables` 测试

**检查结果：PASS（含合理改动）**

测试结果显示 6/6 全部通过：
- `migration_v3_creates_all_tables` PASS
- `migration_v3_is_idempotent` PASS
- `migration_v3_course_events_crud` PASS
- `migration_v4_creates_knowledge_tables` PASS
- `migration_v4_is_idempotent` PASS
- `open_runs_migrations` PASS

**关于断言改动（F-005）**：V3 测试从 `assert_eq!(v, 3)` 改为 `assert!(v >= 3)` 是逻辑上必要的适配（V4 上线后全量运行后 version=4）。但这略微降低了 V3 测试的精确性。`migration_v4_creates_knowledge_tables` 和 `migration_v4_is_idempotent` 已用精确匹配 `assert_eq!(v, 4)` 覆盖主要版本验证，整体风险可接受。

**测试覆盖盲区（F-007）**：缺少"从 V3 升级到 V4"的显式测试路径（仅测试了从 0 升级到 V4）。业务逻辑上安全，但属于 MINOR 级别的测试盲区。

---

## 二、6 维评分表

| 维度 | 权重 | 得分（/10） | 加权得分 | 评分理由 |
|------|------|-------------|----------|----------|
| **功能正确性** | 40% | 9.0 | 3.60 | 4 张表全部正确创建，字段/约束/索引与 Spec 一致，版本门控逻辑正确，幂等性验证通过，6/6 测试全过。扣分点：缺少"V3→V4 升级路径"测试（F-007），V2 ALTER TABLE 不幂等的遗留风险（F-001，非本 task 引入）。 |
| **安全性** | 15% | 9.5 | 1.43 | 纯数据库结构层 task，无用户数据操作，无隐私风险。FOREIGN KEY ON DELETE CASCADE 正确配置，防止孤儿记录。外键约束依赖 `PRAGMA foreign_keys=ON` 已在 Database::open() 保证。 |
| **代码质量** | 15% | 7.5 | 1.13 | V4 SQL 缩进风格（2 空格）与 V1/V2/V3（8 空格对齐）不一致（F-009）。V3 测试注释过时未更新（F-006）。冗余索引（F-008）。整体代码结构清晰，错误处理与已有模式一致。 |
| **测试覆盖** | 15% | 8.0 | 1.20 | 新增 2 个 V4 测试（创建验证 + 幂等性）。全量 + 幂等两条路径均覆盖。缺少 V3→V4 升级路径（F-007）。断言宽松化（F-005）有轻微质量损失。测试输出有完整日志，可观测性好。 |
| **架构一致性** | 10% | 9.5 | 0.95 | 严格遵守单文件多函数 migration 模式，与 V1/V2/V3 模式一致。ADR-001（Rust 侧写操作）、ADR-002（增量不破坏已有表）完全符合。未引入新依赖。`run_migrations()` 调用链无需修改符合预期。 |
| **可维护性** | 5% | 8.0 | 0.40 | 代码结构清晰，函数命名语义明确（`v4_knowledge_understanding`）。中文注释说明表用途。已知局限 3 条主动披露，信息透明。缩进风格不一致影响可读性（F-009）。 |

**综合得分：3.60 + 1.43 + 1.13 + 1.20 + 0.95 + 0.40 = 8.71 / 10**

---

## 三、总体判断

**PASS**

---

## 四、问题列表

### BLOCKER（阻断级）
无

### MAJOR（必须追踪，但不阻断本 task 合并）
无

### MINOR（建议修复，可在后续 task 或专项 tech-debt PR 处理）

| 编号 | 问题 | 位置 | 建议 |
|------|------|------|------|
| M-001（F-002/F-003）| `concept_summaries` 和 `concept_explanations` 的 `concept_id` 未加 UNIQUE 约束，"最多一条"的业务语义未在数据库层强制 | migration.rs 第 153–177 行 | 在 task_003 实现 Command 层时，严格保证串行写入并用 DELETE+INSERT 的 upsert 语义；如后续发现并发问题，可在 V5 migration 中补加 UNIQUE 约束 |
| M-002（F-004）| `concept_relations` 的 `(a, b)` 和 `(b, a)` 方向互换两条记录均可写入，数据库层无 `CHECK (concept_a_id < concept_b_id)` 约束 | migration.rs 第 193–206 行 | task_004 实现 `knowledge_compute_co_occurrence` 时必须强制 `concept_a_id < concept_b_id` 排序写入；此要求需在 task_004 input.md 中显式注明 |
| M-003（F-005）| V3 测试断言从精确匹配改为 `v >= 3`，略微降低测试精确性 | mod.rs | 可接受现状；如后续添加 V5，建议重新审视 V4 测试是否需要类似调整 |
| M-004（F-007）| 缺少"从已有 V3 数据库（user_version=3）升级到 V4"的显式测试路径 | mod.rs | 建议在 tech-debt 阶段补充 `migration_v3_to_v4_upgrade` 测试，手动将数据库设置为 user_version=3 后再调用 run_migrations，验证仅 V4 分支被执行 |
| M-005（F-008）| `concept_user_notes.concept_id` 同时有 UNIQUE 约束（隐式唯一索引）和显式普通索引，后者冗余 | migration.rs 第 190 行 | 低优先级；如后续维护期执行 schema 清理，可移除 `idx_concept_user_notes_concept_id` |
| M-006（F-006/F-009）| V3 幂等性测试注释过时；V4 SQL 缩进风格与 V1/V2/V3 不一致 | mod.rs 第 113 行；migration.rs 第 150–209 行 | 建议在下一次触碰 migration.rs 时统一缩进风格并更新注释 |

### INFO（知识性记录，无需行动）

| 编号 | 问题 | 说明 |
|------|------|------|
| I-001（F-001）| `current_version` 在函数入口仅读取一次，版本间不刷新；V2 的 ALTER TABLE 不是幂等语句 | 这是 V1/V2/V3 的遗留模式，V4 未引入新风险；全量初始化时因所有 CREATE TABLE 使用 IF NOT EXISTS 而安全。建议在未来重构 migration 框架时改为链式执行模式（每个版本完成后刷新 current_version） |

---

## 五、修复指引

本次判断为 **PASS**，无需强制修复即可进入下一 task。以下为建议追踪事项：

1. **task_003/task_004 输入文档需补充说明**：
   - task_004 input.md 中必须明确要求 `concept_relations` 写入时强制 `concept_a_id < concept_b_id`（对应 M-002）
   - task_003 input.md 中建议说明 `concept_summaries` 和 `concept_explanations` 的 upsert 语义须在 Command 层保证（对应 M-001）

2. **tech-debt 追踪**（低优先级，不阻断当前迭代）：
   - 补充 V3→V4 升级路径测试（M-004）
   - 清理 `concept_user_notes` 的冗余索引（M-005）
   - 统一 SQL 缩进风格（M-006）

---

## 六、综合结论

task_002 是一个高质量的纯数据库结构 migration 交付物：

- 4 张表完整创建，字段与 Spec 精确一致
- 幂等性严格保证，不破坏任何 v2.1 已有表
- 测试覆盖全量初始化和幂等性两条主路径，6/6 全通过
- 错误处理与架构模式与已有代码库保持一致
- Dev 主动披露 3 条已知局限，信息透明

发现的 9 条问题（F-001 至 F-009）中，无 BLOCKER、无 MAJOR，均为 MINOR 或以下级别，且大部分属于跨任务协调依赖（需在 task_003/task_004 中处理）或代码风格问题。

**综合得分：8.71 / 10**
**判断：PASS — 可进入 task_003 和 task_004 并行开发阶段**
