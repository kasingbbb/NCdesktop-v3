# Task 输入 — task_002_dev_pr1_schema_v10

## 目标
落地 V10 schema 迁移：新增 `categories` / `category_aliases` 表 + `assets.category_slug` 列与索引 + `categories_v9_backup` 备份表 + 内置 PARA 五项种子。

## 前置条件
- 依赖 task：无（PR-1 起点）
- 必须先存在的文件/接口：`src-tauri/src/db/migration.rs`（V9 链）、`assets` 表

## 验收标准（AC）
1. 启动 v10 数据库后，`categories` 含 5 行内置种子（slug：`1-项目/2-领域/3-资源/4-存档/__uncategorized__`），`is_builtin=1`
2. `categories_v9_backup` 在 V10 升级前生成，含 `retention_until` 列（now + 30 天）
3. `assets.category_slug` 列 nullable，回填策略：从既有 `topics`（解析失败回 `[]`）+ AI 分类历史推断；推断不出归 `__uncategorized__`
4. 索引 `idx_assets_proj_cat_updated(project_id, category_slug, updated_at DESC, id DESC)` 存在
5. CHECK 约束：`categories.parent_id IS NULL`（F17 schema 保留 UI 不暴露）
6. 单测覆盖：(a) 升级幂等 (b) 备份表生成 (c) 种子计数 (d) UNIQUE(library_id, slug) 冲突拒绝
7. V10 升级整体 transaction 包裹；失败回滚至 V9

## 技术约束
- migration.rs 沿用现有版本链 + 单向 up（无 down）
- slug 白名单 `[a-z0-9一-龥_-]`（执行端校验，schema 不强制 CHECK 以兼容 CJK 历史值）
- Tauri command 不暴露此 task；仅 schema 与 seeds

## 参考文件
- `项目启动/NCdesktop/src-tauri/src/db/migration.rs`（V1 settings KV @L646；V9 末尾接续 V10）
- task_001 output.md §数据模型（完整 SQL）
- ADR-001（主键策略）

## 预估影响范围
- 新建：无
- 修改：`src-tauri/src/db/migration.rs`（+约 350 行）
- 测试：`src-tauri/tests/migration_v10.rs`（新建）

## Reviewer 重点关注
- 备份表 retention_until 是否正确写入；30 天计算时区
- 回填策略对 `topics` 解析失败的容错
- UNIQUE(library_id, slug) 冲突时的迁移失败回滚路径
