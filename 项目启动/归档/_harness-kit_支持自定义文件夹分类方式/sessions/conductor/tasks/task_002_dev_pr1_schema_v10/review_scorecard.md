# Review Scorecard — task_002_dev_pr1_schema_v10

## 审查前验证
- [x] 测试结果存在且非空（7/7 V10 + 84/84 全量）
- [x] 自测验证矩阵正常路径全 PASS
- [x] 架构遵守声明完整

## 审查思考过程
1. **Task 意图**：落地 V10 schema（categories 表族 + assets 列扩展 + 索引 + 备份 + 种子 + 回填），事务包裹失败回滚。
2. **AC 检查**：
   - AC-1 5 行 PARA 种子 builtin=1 ✅（migration_v10_seeds_existing_library）
   - AC-2 备份表 + retention_until ✅（schema 含 `DEFAULT (datetime('now', '+30 days'))`）
   - AC-3 category_slug 回填 ✅（resolve_legacy_slug + backfill_category_slug）
   - AC-4 idx_assets_proj_cat_updated ✅（migration_v10_creates_tables）
   - AC-5 parent_id IS NULL CHECK ✅（schema 内）
   - AC-6 单测覆盖幂等 / 备份 / 种子 / UNIQUE ✅（7 个测试）
   - AC-7 transaction 包裹失败回滚 ⚠️（代码符合，但无单测模拟"中途失败"，Dev 已诚实披露）
3. **关键发现**：
   - 实施过程发现 ADR-003 需要的 `assets.updated_at` 列在原 schema 不存在，Dev 计划阶段已披露并经 PM 确认（A 选项），合理处理；
   - 备份表命名调整 `categories_v9_backup` → `assets_v9_classification_backup`，更准确反映备份对象。

## 评分

| 维度 | 权重 | 分数 | 说明 |
|------|------|------|------|
| 功能正确性 | 30% | 5 | 7 个单测全过，覆盖创建/种子/触发器/幂等/唯一/回填解析/空库 |
| 用户体验 | 25% | 5 | task 为底层 schema，UX 中性；不阻塞用户 |
| 安全性 | 15% | 4 | 事务回滚 + ROLLBACK 兜底；无注入面（纯 DDL + 受控 INSERT）；扣 1：事务回滚未单测 |
| 架构一致性 | 10% | 5 | 严格遵守 ADR-001 / 数据模型；偏离均经 PM 批准 |
| 测试覆盖 | 10% | 4 | 关键路径全覆盖；扣 1：缺中途失败回滚单测 |
| 可维护性 | 10% | 5 | `add_column_if_missing` / `seed_builtin_categories` / `backfill_category_slug` / `resolve_legacy_slug` 单一职责清晰；中文 log 错误友好 |

**综合分（按 session_context §4 权重加权）**：
- 30% × 5 + 25% × 5 + 15% × 4 + 10% × 5 + 10% × 4 + 10% × 5 = 1.5 + 1.25 + 0.6 + 0.5 + 0.4 + 0.5 = **4.75 / 5**

## 总体判断
- [x] **PASS**

## 问题列表

### BLOCKER
无

### MAJOR
无

### MINOR
1. 事务中途失败回滚未单测：可考虑在 v10_inner 注入一个故意失败的 SQL 做集成测试。**不阻塞合并**，记入 task_003 自测前置环境的"已知信任假设"即可。
2. `seed_builtin_categories` 仅对迁移时已存在的 library 注入；新建 library 时需在 `db/library.rs` 调用此函数。已在 output.md "已知局限" 披露，task_012 落实。

## 通行下游
- task_003 / task_004 可基于 V10 schema 启动。
- `pub(crate) fn resolve_legacy_slug` 已暴露给 crate 内部，task_003 self-healing 可直接复用解析逻辑（避免重复实现）。
