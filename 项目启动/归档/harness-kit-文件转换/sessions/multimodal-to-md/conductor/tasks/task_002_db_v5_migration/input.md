# Task 输入 — task_002_db_v5_migration

## 目标
V5 数据库迁移：新增 `extracted_content` 表、`pipeline_tasks` 表、`v_asset_content` VIEW、`fts_content` FTS5 虚拟表及相关触发器。

## 前置条件
- 依赖 task：无
- 必须先存在的文件/接口：`src-tauri/src/db/migration.rs`（V1-V4 已实现）

## 验收标准（Acceptance Criteria）
1. AC-1：`v5_extraction_pipeline` 函数已添加到 `migration.rs`，`run_migrations` 中 `current_version < 5` 分支正确调用
2. AC-2：`extracted_content` 表包含 PRD 定义的所有列，约束与默认值正确
3. AC-3：`pipeline_tasks` 表包含所有列，`UNIQUE(asset_id, task_type)` 约束生效
4. AC-4：`v_asset_content` VIEW 正确联结 `assets` + `extracted_content` + `ai_analyses`，COALESCE 逻辑与 PRD 一致
5. AC-5：`fts_content` FTS5 虚拟表创建成功，INSERT/DELETE/UPDATE 触发器正确维护索引
6. AC-6：新增 `db/extraction.rs` 模块，包含 `extracted_content` 和 `pipeline_tasks` 的基础 CRUD 函数
7. AC-7：`cargo build` 编译通过，无警告；现有测试全部通过

## 技术约束
- 使用 `rusqlite`（bundled 模式），与现有迁移风格一致
- 迁移必须幂等（使用 `CREATE TABLE IF NOT EXISTS`）
- `PRAGMA user_version = 5` 必须在迁移末尾设置
- `extracted_content.status` 的 CHECK 约束值：`pending`, `extracting`, `extracted`, `failed`, `unsupported`
- `pipeline_tasks.status` 的 CHECK 约束值：`queued`, `running`, `completed`, `failed`, `cancelled`
- `fts_content` 触发器须处理 `raw_text IS NULL` 的情况（不写入空索引）

## 参考文件
- `src-tauri/src/db/migration.rs`（V1-V4 迁移格式）
- `src-tauri/src/db/asset.rs`（CRUD 格式参考）
- `src-tauri/src/db/mod.rs`（Database 结构与 Mutex 模式）
- Architect output.md §数据模型 — 完整 SQL Schema

## 预估影响范围
- 新建文件：`src-tauri/src/db/extraction.rs`
- 修改文件：`src-tauri/src/db/migration.rs`（新增 V5）、`src-tauri/src/db/mod.rs`（pub mod extraction）
