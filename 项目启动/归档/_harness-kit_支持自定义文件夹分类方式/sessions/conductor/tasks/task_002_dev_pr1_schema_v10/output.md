# Task 交付 — task_002_dev_pr1_schema_v10

## 实现摘要
落地 V10 migration：新增 `categories` / `category_aliases` / `assets_v9_classification_backup` 表，`assets` 增 `updated_at` + `category_slug` 列与触发器，复合索引 `idx_assets_proj_cat_updated` 就位，PARA 五项内置种子按 library 注入；旧 `ai_analyses.topics` 全表备份 + `assets.category_slug` 智能回填（JSON 数组首项 / 裸字符串首段 / `__uncategorized__` 兜底）。整体 `BEGIN IMMEDIATE; ... COMMIT;` 包裹，失败 `ROLLBACK`。

**实现前计划与 PM 确认的偏离**：
- 备份表名由 `categories_v9_backup` → `assets_v9_classification_backup`（V9 之前不存在 categories 实体，备份对象是 topics 字段，命名更准确，已在计划阶段披露）
- 顺手为 `assets` 加 `updated_at` 列 + 维护触发器（ADR-003 cursor 分页硬前置，PM 已批 A 选项）

## 修改的文件
| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `src-tauri/src/db/migration.rs` | 修改 | 新增 `v10_categories` / `v10_inner` / `add_column_if_missing` / `seed_builtin_categories` / `backfill_category_slug` / `resolve_legacy_slug`；run_migrations 加分支 |
| `src-tauri/src/db/mod.rs` | 修改 | 新增 7 个 V10 测试 |

## 对 Architect 方案的遵守声明
- [x] 目录结构：仅触及 `db/migration.rs` 与 `db/mod.rs`，与 Architect 方案一致
- [x] API 命名：`categories` / `category_aliases` / `category_slug` 全部按 ADR-001 / 数据模型章节落地
- [x] 数据模型：UNIQUE(library_id, slug) + parent_id CHECK NULL + 索引 + 备份表 + 30 天保留全部就绪
- [x] 未引入计划外的新依赖（serde_json 已在）
- 偏离说明：见上"实现摘要"两条，已 PM 确认

## 测试命令
```bash
cd 项目启动/NCdesktop/src-tauri && cargo test --lib db::tests::migration_v10
cd 项目启动/NCdesktop/src-tauri && cargo test --lib                     # 回归
```

## 测试结果
```
running 7 tests
test db::tests::migration_v10_resolve_legacy_slug ... ok
test db::tests::migration_v10_unique_constraint ... ok
test db::tests::migration_v10_creates_tables ... ok
test db::tests::migration_v10_seeds_para_per_library ... ok
test db::tests::migration_v10_is_idempotent ... ok
test db::tests::migration_v10_seeds_existing_library ... ok
test db::tests::migration_v10_updated_at_trigger ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 77 filtered out; finished in 1.24s

# 全量回归
test result: ok. 84 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.85s
```

## 自测验证矩阵
| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|----------|----------|------|-----------|
| ✅ 正常路径 | V9 → V10 升级，三表 + 列 + 索引就位 | 已测 | PASS（migration_v10_creates_tables） |
| ✅ 正常路径 | 已有 library 的库升级时被注入 5 项 PARA 种子（含 `__uncategorized__`） | 已测 | PASS（migration_v10_seeds_existing_library） |
| ✅ 正常路径 | UPDATE assets 后 updated_at 自动刷新（带 WHEN 守卫防递归） | 已测 | PASS（migration_v10_updated_at_trigger） |
| ✅ 正常路径 | `resolve_legacy_slug` 覆盖 JSON 数组 / 裸字符串 / 含分隔符 / 不命中 / 空 | 已测 | PASS（migration_v10_resolve_legacy_slug，6 case） |
| ⚠️ 边界条件 | 重复打开（V10 已跑），run_migrations 幂等 | 已测 | PASS（migration_v10_is_idempotent） |
| ⚠️ 边界条件 | 空库情形（无 library）跑 V10 不报错 | 已测 | PASS（migration_v10_seeds_para_per_library） |
| ❌ 异常路径 | UNIQUE(library_id, slug) 冲突插入被拒 | 已测 | PASS（migration_v10_unique_constraint） |
| ❌ 异常路径 | 事务包裹：v10_inner 中途失败触发 ROLLBACK | 未测 | 缺失：未在测试中模拟"中途 SQL 失败"。靠代码 review 保证：`v10_categories` 用 BEGIN IMMEDIATE + Result 分支 + ROLLBACK，符合 SQLite 事务语义 |

## 已知局限
1. **新 library 的 seed**：本 task 仅对"V10 跑时已存在的 library" seed。新建 library 时的种子注入需在 `db/library.rs` 的创建逻辑里调用 `seed_builtin_categories(conn)`（建议 task_012 内一并处理；本 task 留 hook：函数 `pub(crate)` 已暴露给 crate 内部）。
2. **回填解析的覆盖面**：`resolve_legacy_slug` 仅识别 4 个内置 slug；若用户历史使用 `other` 或自定义字符串均归 `__uncategorized__`。这是 PRD 既定行为。
3. **事务回滚未单测**：见上表最后一行；建议 Reviewer 用 `cargo expand` / 人工 review 关注 `v10_categories` 函数的 Err 分支。

## 需要 Reviewer 特别关注
- `migration.rs::v10_categories` 的事务边界（BEGIN/COMMIT/ROLLBACK）：是否真的能在中途 SQL 失败时回滚 user_version
- `add_column_if_missing` 的幂等保护（多次跑不重复 ALTER）
- `resolve_legacy_slug` 的字符切分 `split(['/', '\\', ',', ' '])` 是否覆盖足够多旧数据形态
- 触发器 `tr_assets_updated_at` 的 WHEN 守卫是否可避免无限递归（已含手动测试通过）
- `seed_builtin_categories` 仅对"已存在 library" seed，新建 library 时的种子注入策略需在后续 task 落实（已在已知局限说明）
