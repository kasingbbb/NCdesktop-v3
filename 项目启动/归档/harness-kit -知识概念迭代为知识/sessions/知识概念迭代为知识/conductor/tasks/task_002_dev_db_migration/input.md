# Task 输入 — task_002_dev_db_migration

## 目标

在 `src-tauri/src/db/migration.rs` 中新增 V4 migration 函数，添加 4 张新 SQLite 表，集成到现有版本化迁移流程，确保应用启动时自动建表且不破坏任何 v2.1/v3 已有数据。

> **Host 架构校正**：Architect 规划的 `knowledge/migration.rs` 模式与实际代码库不符。实际项目使用 `db/migration.rs` + `user_version` pragma 做版本管理（当前 V3），新 migration 应在此文件追加 V4，而非创建新模块。

---

## 前置条件

- 依赖 task：无（这是所有其他 task 的基础）
- 必须先存在的文件（均已确认存在）：
  - `src-tauri/src/db/migration.rs` — 现有 migration 文件，当前最高版本 V3，需在此追加 V4
  - `src-tauri/src/db/mod.rs` — 声明 `pub mod migration;`，`Database::open()` 调用 `migration::run_migrations(&conn)`
  - `src-tauri/src/db/knowledge.rs` — v2.1 已有，包含 `concepts` 表的 CRUD（`id TEXT PRIMARY KEY` 已确认）

---

## 验收标准（Acceptance Criteria）

1. **AC-1**：运行应用后，SQLite 数据库中存在且仅新增以下 4 张表（通过 `sqlite3` CLI 或测试验证）：
   - `concept_summaries`
   - `concept_explanations`
   - `concept_user_notes`
   - `concept_relations`

2. **AC-2**：每张表的字段类型、NOT NULL 约束、DEFAULT 值、FOREIGN KEY 与技术方案文档中的 Schema 定义完全一致。

3. **AC-3**：所有 4 张表均有正确的索引（`idx_concept_summaries_concept_id` 等）以及 `concept_relations` 表的复合唯一索引。

4. **AC-4**：Migration 脚本使用 `CREATE TABLE IF NOT EXISTS`（幂等），多次运行不报错，不修改已有数据。

5. **AC-5**：v2.1 已有表（`concepts`、`concept_viewpoints`、`concept_cases`、`concept_extensions`）在 migration 后完整保留，无任何字段变更（通过 `.schema concepts` 等命令验证）。

6. **AC-6**：`src-tauri/src/db/migration.rs` 新增 `v4_knowledge_understanding(conn)` 函数，`run_migrations()` 在 V3 检查后追加 `if current_version < 4 { v4_knowledge_understanding(conn)?; }`，并在函数末尾设置 `PRAGMA user_version = 4`。

7. **AC-7**：`db/mod.rs` 的 `migration_v3_creates_all_tables` 相关测试仍能通过（不破坏已有测试），新增 `migration_v4_creates_knowledge_tables` 测试验证 4 张新表存在且 user_version = 4。

---

## 技术约束

- **语言**：Rust；数据库库使用项目已有的 `rusqlite` crate（不引入新依赖）
- **SQL 规范**：全部使用 `CREATE TABLE IF NOT EXISTS`；严禁 `ALTER TABLE`、`DROP TABLE`、`DELETE FROM`（针对已有表）
- **外键约束**：`ON DELETE CASCADE`（当对应 concept 被删除时，关联的新表记录自动清理）
- **migration 调用时机**：在 `db/migration.rs` 的 `run_migrations()` 函数中追加，已有调用链 `Database::open()` → `migration::run_migrations()` 无需修改
- **错误处理**：与已有 V1/V2/V3 保持一致，返回 `Result<(), String>`，`.map_err(|e| format!("V4 迁移失败: {e}"))?`
- **代码位置**：只修改 `src-tauri/src/db/migration.rs`，不新建模块，不修改 `db/mod.rs` 或 `main.rs`/`lib.rs`

**Schema 约束（严格遵守，不可自行调整字段名）**：

```sql
-- 表 1
CREATE TABLE IF NOT EXISTS concept_summaries (
  id TEXT PRIMARY KEY,
  concept_id TEXT NOT NULL,
  summary TEXT NOT NULL,
  source_asset_ids TEXT NOT NULL,
  model TEXT NOT NULL,
  generated_at TEXT NOT NULL,
  FOREIGN KEY (concept_id) REFERENCES concepts(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_concept_summaries_concept_id ON concept_summaries(concept_id);

-- 表 2
CREATE TABLE IF NOT EXISTS concept_explanations (
  id TEXT PRIMARY KEY,
  concept_id TEXT NOT NULL,
  mechanism TEXT NOT NULL,
  typical_scenarios TEXT NOT NULL,
  common_misconceptions TEXT,
  essence_sentence TEXT NOT NULL,
  source_asset_ids TEXT NOT NULL,
  model TEXT NOT NULL,
  generated_at TEXT NOT NULL,
  FOREIGN KEY (concept_id) REFERENCES concepts(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_concept_explanations_concept_id ON concept_explanations(concept_id);

-- 表 3
CREATE TABLE IF NOT EXISTS concept_user_notes (
  id TEXT PRIMARY KEY,
  concept_id TEXT NOT NULL UNIQUE,
  user_explanation TEXT NOT NULL DEFAULT '',
  mirror_feedback TEXT,
  last_validated_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (concept_id) REFERENCES concepts(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_concept_user_notes_concept_id ON concept_user_notes(concept_id);

-- 表 4
CREATE TABLE IF NOT EXISTS concept_relations (
  id TEXT PRIMARY KEY,
  concept_a_id TEXT NOT NULL,
  concept_b_id TEXT NOT NULL,
  relation_type TEXT NOT NULL,
  source_asset_ids TEXT NOT NULL,
  co_occurrence_count INTEGER DEFAULT 1,
  created_at TEXT NOT NULL,
  FOREIGN KEY (concept_a_id) REFERENCES concepts(id) ON DELETE CASCADE,
  FOREIGN KEY (concept_b_id) REFERENCES concepts(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_concept_relations_a ON concept_relations(concept_a_id);
CREATE INDEX IF NOT EXISTS idx_concept_relations_b ON concept_relations(concept_b_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_concept_relations_pair ON concept_relations(concept_a_id, concept_b_id, relation_type);
```

---

## 参考文件

- 已有 migration 文件（**必读，直接修改**）：`src-tauri/src/db/migration.rs` — 阅读 `v3_course_and_knowledge()` 的实现方式，V4 严格照此模式编写
- 已有 db 模块入口：`src-tauri/src/db/mod.rs` — 确认调用链，理解 `Database::open()` 如何触发 migration
- 已有 knowledge DB 层（了解 v2.1 现有表/代码）：`src-tauri/src/db/knowledge.rs`

---

## 预估影响范围

**新建文件**：无

**修改文件**：
- `src-tauri/src/db/migration.rs`：追加 `v4_knowledge_understanding()` 函数 + 在 `run_migrations()` 中添加 `if current_version < 4` 分支
- `src-tauri/src/db/mod.rs`：在 `migration_v3_creates_all_tables` 测试附近新增 `migration_v4_creates_knowledge_tables` 和 `migration_v4_is_idempotent` 测试（可选，但强烈建议）
