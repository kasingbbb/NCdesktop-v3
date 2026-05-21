# task_002_dev_db_migration — Output

交付时间: 2026-04-11
执行角色: Dev（开发工程师）
状态: DONE

---

## 实现摘要

在现有 `db/migration.rs` 中追加了 V4 迁移函数 `v4_knowledge_understanding`，新增 4 张 SQLite 表：
- `concept_summaries`：文档整合摘要
- `concept_explanations`：理解框架
- `concept_user_notes`：用户个人理解笔记
- `concept_relations`：概念关系网络

迁移函数通过 `PRAGMA user_version = 4` 版本管理，与 V1/V2/V3 保持一致的幂等模式。所有表均使用 `CREATE TABLE IF NOT EXISTS`，保证重复迁移安全。

---

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---|---|---|
| `src-tauri/src/db/migration.rs` | 追加 | 新增 `v4_knowledge_understanding()` 函数，在 `run_migrations()` 中追加 `if current_version < 4` 调用块 |
| `src-tauri/src/db/mod.rs` | 追加 + 修改 | 新增 `migration_v4_creates_knowledge_tables` 和 `migration_v4_is_idempotent` 测试；将 `migration_v3_creates_all_tables` 和 `migration_v3_is_idempotent` 中的 `assert_eq!(v, 3, ...)` 改为 `assert!(v >= 3, ...)` |

完整文件路径：
- `/Users/zhongjiacheng/Documents/project/办公桌/NCdesktop/项目启动/NCdesktop/src-tauri/src/db/migration.rs`
- `/Users/zhongjiacheng/Documents/project/办公桌/NCdesktop/项目启动/NCdesktop/src-tauri/src/db/mod.rs`

---

## 对 Architect 方案的遵守声明

**完全遵守以下 ADR：**
- ADR-001：所有 SQLite 写操作在 Rust 侧执行（migration.rs 是 Rust 层）
- ADR-002：增量添加 4 张新表，未修改 v2.1 任何已有表，未使用 ALTER TABLE/DROP TABLE
- 所有 4 张表的字段名与 input.md Schema 完全一致，无任何字段名偏离

**迁移模式调整（合理偏离）：**
Architect input.md 描述的是独立 migration 模块的概念性结构，但现有代码库采用单文件 `migration.rs` + 多函数（v1/v2/v3）模式。V4 遵从现有模式追加函数，而非创建独立文件，保持代码库一致性。这是实现层面的合理适配，不影响功能正确性。

---

## 测试命令（精确）

```bash
cd /Users/zhongjiacheng/Documents/project/办公桌/NCdesktop/项目启动/NCdesktop/src-tauri
cargo test db::tests -- --nocapture 2>&1 | tail -30
```

---

## 测试结果（完整）

```
[2026-04-11T14:30:27Z INFO  app_lib::db::migration] 数据库迁移 V1 完成
[2026-04-11T14:30:27Z INFO  app_lib::db::migration] 数据库迁移 V2 完成：assets.original_name
[2026-04-11T14:30:27Z INFO  app_lib::db::migration] 数据库迁移 V3 完成：课程日历 + 知识关联
[2026-04-11T14:30:27Z INFO  app_lib::db::migration] 数据库迁移 V4 完成：知识理解辅助层
[2026-04-11T14:30:27Z INFO  notecapt_test] [TEST] db user_version = 4
[2026-04-11T14:30:27Z INFO  notecapt_test] [TEST] V3 表 course_events ✓
[2026-04-11T14:30:27Z INFO  notecapt_test] [TEST] V3 表 course_previews ✓
[2026-04-11T14:30:27Z INFO  notecapt_test] [TEST] V3 表 concepts ✓
[2026-04-11T14:30:27Z INFO  notecapt_test] [TEST] V3 表 concept_viewpoints ✓
[2026-04-11T14:30:27Z INFO  notecapt_test] [TEST] V4 user_version = 4
[2026-04-11T14:30:27Z INFO  notecapt_test] [TEST] V3 表 concept_cases ✓
[2026-04-11T14:30:27Z INFO  notecapt_test] [TEST] V4 表 concept_summaries ✓
[2026-04-11T14:30:27Z INFO  notecapt_test] [TEST] V3 表 concept_extensions ✓
[2026-04-11T14:30:27Z INFO  notecapt_test] [TEST] V4 表 concept_explanations ✓
[2026-04-11T14:30:27Z INFO  notecapt_test] [TEST] V4 表 concept_user_notes ✓
[2026-04-11T14:30:27Z INFO  notecapt_test] [TEST] V4 表 concept_relations ✓
[2026-04-11T14:30:27Z INFO  notecapt_test] [TEST] V3 幂等性验证通过，user_version = 4
test db::tests::migration_v3_is_idempotent ... ok
test db::tests::open_runs_migrations ... ok
test db::tests::migration_v3_course_events_crud ... ok
test db::tests::migration_v4_creates_knowledge_tables ... ok
test db::tests::migration_v3_creates_all_tables ... ok
[2026-04-11T14:30:27Z INFO  notecapt_test] [TEST] V4 幂等性验证通过，user_version = 4
test db::tests::migration_v4_is_idempotent ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 28 filtered out; finished in 0.06s
```

**结论：6/6 测试通过，包括：**
- `migration_v4_creates_knowledge_tables` PASS
- `migration_v4_is_idempotent` PASS
- `migration_v3_creates_all_tables` PASS（已适配 `assert!(v >= 3)`）
- `migration_v3_is_idempotent` PASS（已适配 `assert!(v >= 3)`）
- `migration_v3_course_events_crud` PASS
- `open_runs_migrations` PASS

---

## 自测验证矩阵

| 场景 | 验证方式 | 结果 |
|---|---|---|
| **正常路径**：全新数据库从 0 迁移到 V4 | `migration_v4_creates_knowledge_tables` | PASS |
| **正常路径**：V4 4 张表全部创建 | 逐表查询 `sqlite_master` | PASS |
| **正常路径**：`user_version` 最终为 4 | pragma 查询断言 | PASS |
| **边界条件**：已有 V3 数据库重新打开只执行 V4 | `migration_v4_is_idempotent` 第二次打开 | PASS |
| **边界条件**：重复打开不报错，version 不回退 | 两次 open 后断言 version == 4 | PASS |
| **边界条件**：V3 既有表（concepts 等）未被修改 | V3 表仍存在且 CRUD 正常 | PASS |
| **异常路径**：所有 `CREATE TABLE IF NOT EXISTS` 幂等 | 二次打开自动校验 | PASS |
| **异常路径**：未引入新 crate | Cargo.toml 未修改 | PASS |

---

## 已知局限

1. **无数据回滚**：本次 migration 不包含 downgrade 路径。如需回滚到 V3，需手动删除 4 张新表并将 `user_version` 设回 3。这与现有 V1/V2/V3 的处理方式一致，是整个代码库的既有局限，非本次引入。

2. **概念关系方向性约束缺失**：`concept_relations` 表有 UNIQUE INDEX `(concept_a_id, concept_b_id, relation_type)`，但 `(a, b)` 和 `(b, a)` 是两条独立记录。业务层需决定是否需要规范化方向（如强制 `concept_a_id < concept_b_id`）。此决策超出本次 migration 范围，应在 task_004 中处理。

3. **外键约束运行时依赖**：外键约束需要 `PRAGMA foreign_keys=ON`，这已在 `Database::open()` 中设置，但直接使用 raw Connection 的代码需注意此前提。

---

## 需要 Reviewer 特别关注的地方

1. **Schema 字段名逐一核查**：请对照 input.md 中的 SQL Schema，逐字段确认 `concept_summaries`、`concept_explanations`、`concept_user_notes`、`concept_relations` 的字段名和类型无遗漏或拼写错误。这是下游 task_003 Rust Command 层和 task_005 TypeScript 类型层的根基。

2. **`concept_user_notes.concept_id` 的 UNIQUE 约束**：此字段标注了 `UNIQUE`，意味着每个概念只能有一条用户笔记记录（upsert 语义）。请确认此业务规则与 PRD 一致。

3. **`migration_v3_*` 测试的断言变更**：两个 V3 测试从 `assert_eq!(v, 3)` 改为 `assert!(v >= 3)`。逻辑上正确（V4 上线后 version 是 4），但 Reviewer 应确认此改动不会掩盖任何回归问题。

4. **`concept_relations` 的 UNIQUE INDEX**：`idx_concept_relations_pair` 是 `(concept_a_id, concept_b_id, relation_type)` 的复合唯一索引。若同一对概念有多种关系类型，需多条记录（relation_type 不同）。请确认此设计与 task_004 共现计算逻辑兼容。
