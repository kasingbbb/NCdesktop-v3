# Review Scorecard — task_006_dev_conversion_meta

## 审查前验证（交接契约 8 字段）

- [x] 实现摘要：齐全（含 ConversionAttempt ↔ ConversionMetaRow 字段映射表）
- [x] 修改文件清单：3 个（migration.rs / conversion_meta.rs / mod.rs）
- [x] 架构遵守声明：全部勾选，无偏离
- [x] 测试命令：明确
- [x] 测试结果：4 passed; 0 failed；cargo check finished
- [x] 自测验证矩阵：正常 3 条 + 边界 4 条 + 异常 2 条
- [x] 已知局限：5 条，含 u64→i64 转换说明
- [x] 需 Reviewer 关注：3 处（bool↔INTEGER、FK 不对称、迁移链路）

→ 契约完整，进入实质审查。

## 审查思考过程

### 1. Task 意图
落地 ADR-004 的 `conversion_meta` append-only 日志表（迁移 V6），提供 3 个 CRUD（insert / list_by_source / latest_for_source），为 task_008 scheduler 接线提供基础设施。**不**加唯一约束是核心反宪章原稿的 ADR 决策。

### 2. AC 逐条检查

| AC | 状态 | 验证 |
|----|------|------|
| AC-1 V6 创建表 + 3 索引 | ✅ | migration.rs:37-67；`idx_cm_source` / `idx_cm_derived` / `idx_cm_converted_at` 齐全 |
| AC-2 幂等 IF NOT EXISTS + user_version=6 | ✅ | `CREATE TABLE IF NOT EXISTS` + `CREATE INDEX IF NOT EXISTS` + `PRAGMA user_version = 6` |
| AC-3 三函数签名 + DESC + Ok(None) | ✅ | `insert` / `list_by_source`（ORDER BY converted_at DESC，conversion_meta.rs:72）/ `latest_for_source`（match rows.next() → Ok(None)，:112） |
| AC-4 字段一一对应 + camelCase + bool 转换 | ✅ | `#[serde(rename_all = "camelCase")]`；`fallback_used as i32` 写、`fallback_int != 0` 读；u64→i64 在调用层（task_008）由已知局限#3 兜底 |
| AC-5 4 个测试 | ✅ | list_by_source_returns_rows_desc / latest_for_source_picks_most_recent_and_handles_missing / deleting_source_asset_cascades_conversion_meta / serde_derived_asset_id_none_is_json_null 全 PASS |

### 3. 关键发现

**[关键正向] UNIQUE 复核通过**：`grep -i "UNIQUE" db/migration.rs db/conversion_meta.rs` 命中 6 行，全部位于 V1（tags.name、ai_analyses.asset_id、timelines.project_id、transcriptions.audio_track_id）/ V4（concept_user_notes.concept_id、idx_concept_relations_pair）—— **conversion_meta 表内 0 命中**。ADR-004 反宪章原稿（append-only 日志）严格落实。

**[关键正向] 跨 task 字段一致性**：output.md 给出明确映射表，9 个共用字段 + 3 个本表新增（id/source_asset_id/derived_asset_id）。类型错位（`u64` → `Option<i64>`）已在已知局限#3 明示由调用层 `as i64` 解决；本 task 范围内 schema 合理。

**[关键正向] FK 策略合理**：source CASCADE / derived SET NULL 的不对称配置有清晰理据（日志是"对 source 的事实记录"），且实测 cascade 行为有专测覆盖。

## 评分

| 维度 | 权重 | 分数 | 说明 |
|------|------|------|------|
| 功能正确性 | 30% | 5 | 5 条 AC 全部满足；4 测试全 PASS；ADR-004 严格落实 |
| 架构一致性 | 20% | 5 | 与 architect §五.2 字段定义一致；§三 ADR-004 无唯一约束严格执行；目录/命名/无新依赖 |
| 可维护性 | 15% | 5 | 模块自包含；映射表清晰；u64↔i64 显式说明；M-1 不变量明确声明未触动 scheduler 注释 |
| 安全性 | 10% | 5 | 全部 `params![]` 参数化；无字符串拼接 SQL；无 `unwrap`/`expect` 在生产代码；错误用 `map_err` 包装 |
| 测试覆盖 | 15% | 5 | 顺序/None/CASCADE/serde null 四场景全覆盖；内存库 + 真实迁移链路；显式 PRAGMA foreign_keys=ON |
| 代码质量 | 10% | 5 | row_to_meta 抽出避免 list/latest 重复；注释清晰；类型转换显式（`as i32` 而非依赖 `ToSql for bool`） |

**综合分：5.0/5**（加权 30×5 + 20×5 + 15×5 + 10×5 + 15×5 + 10×5 = 5.00）

## 总体判断

- [x] **PASS**

## 问题列表

### BLOCKER
无。

### MAJOR
无。

### MINOR
1. **测试用 `assert_eq!(back.fallback_used, true)`** clippy 风格上可写 `assert!(back.fallback_used)`；不影响功能，不要求修复。
2. **`fallback_used as i32`** rusqlite 原生支持 `ToSql for bool`，可直接传 `row.fallback_used`；当前显式转换让 schema 意图更清晰，保留亦可接受。Dev 在"需关注处 #1"已主动说明，无需修改。

## 给 Dev 的修复指引

无（PASS）。task_008 接线时按 Dev 已知局限 #3 做 `Some(attempt.conversion_ms as i64)` 即可。
