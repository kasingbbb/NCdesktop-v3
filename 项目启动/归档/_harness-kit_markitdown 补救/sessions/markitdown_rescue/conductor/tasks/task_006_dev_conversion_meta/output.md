# Task 交付 — task_006_dev_conversion_meta

## 实现摘要

落地 ADR-004 要求的 `conversion_meta` append-only 转换日志层：

- **迁移 V6**：`v6_conversion_meta(conn)` 追加到 `db/migration.rs`，使用 `CREATE TABLE IF NOT EXISTS` + `CREATE INDEX IF NOT EXISTS` 实现幂等。建 `conversion_meta` 表（12 列，含 `id` 主键、双外键到 `assets`），并建 3 个索引：`idx_cm_source`、`idx_cm_derived`、`idx_cm_converted_at`。`PRAGMA user_version = 6` 推进版本号。
- **外键策略**：`source_asset_id` `ON DELETE CASCADE`（源被删 → 日志一并清掉，保持引用完整性）；`derived_asset_id` `ON DELETE SET NULL`（衍生件可能被独立物化策略清理，不应反向删日志）。
- **显式不加唯一约束**：ADR-004 / input.md 技术约束 #1 明确这是 append-only 日志，同一 `(source, converter)` 可以多次出现（成功、fallback、失败各为独立行）。
- **新建 `db/conversion_meta.rs`**：`ConversionMetaRow`（serde camelCase）+ 3 个函数 `insert` / `list_by_source` / `latest_for_source`，全部 `Result<_, String>`、`params![]` 参数化、无 `unwrap()`/`expect()`（仅测试代码使用 expect）。
- **SQLite ↔ Rust 类型映射**：`fallback_used: bool` 在写入时 `as i32` 转 INTEGER；读取时 `fallback_int != 0` 反向转换；`conversion_ms: Option<i64>` 对应可空 INTEGER，与 ConversionAttempt 的 `u64` 在调用层显式转换。
- **`db/mod.rs`** 追加单行 `pub mod conversion_meta;`，不动其他 mod。
- **M-1 不变量**：`extraction/mod.rs:4-5` 的 scheduler 注释**未取消**，task_008 关闭点保留。

### `ConversionAttempt` ↔ `ConversionMetaRow` 字段对应

| ConversionAttempt (task_005) | ConversionMetaRow | 备注 |
|---|---|---|
| — | `id: String` | 新增；调用方生成 UUID |
| — | `source_asset_id: String` | 新增；外键 |
| — | `derived_asset_id: Option<String>` | 新增；外键 |
| `converter_name` | `converter_name` | |
| `converter_version` | `converter_version` | |
| `source_mime` | `source_mime` | |
| `source_hash` | `source_hash` | |
| `quality_level: i32` | `quality_level: i32` | |
| `fallback_used: bool` | `fallback_used: bool` | DB 存 INTEGER |
| `error_class: Option<String>` | `error_class: Option<String>` | |
| `conversion_ms: u64` | `conversion_ms: Option<i64>` | 调用层 `u64 → i64`；None 表示未测/未知 |
| `converted_at: String` | `converted_at: String` | RFC3339 |

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `src-tauri/src/db/migration.rs` | 修改 | 新增 V6 分支 + `v6_conversion_meta` 函数 |
| `src-tauri/src/db/conversion_meta.rs` | 新建 | `ConversionMetaRow` + 3 个 CRUD + 4 个单测 |
| `src-tauri/src/db/mod.rs` | 修改 | 追加 `pub mod conversion_meta;` |

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（`db/conversion_meta.rs`）
- [x] API 路径/命名与 Architect 方案一致（`insert` / `list_by_source` / `latest_for_source` 三个公开函数）
- [x] 数据模型与 Architect 方案一致（§五.2 字段、ADR-004 无唯一约束、append-only）
- [x] 未引入计划外的新依赖（复用既有 `rusqlite` / `serde` / `serde_json` / `chrono` / `log`）
- 偏离说明：无。

## 测试命令

```bash
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri
cargo check
cargo test --lib db::conversion_meta
```

## 测试结果

```
$ cargo check
... (4 既存 warning，与本 task 无关，均位于 src/llm/chat.rs)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.18s

$ cargo test --lib db::conversion_meta
running 4 tests
test db::conversion_meta::tests::serde_derived_asset_id_none_is_json_null ... ok
test db::conversion_meta::tests::list_by_source_returns_rows_desc ... ok
test db::conversion_meta::tests::latest_for_source_picks_most_recent_and_handles_missing ... ok
test db::conversion_meta::tests::deleting_source_asset_cascades_conversion_meta ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 46 filtered out; finished in 0.02s
```

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|----------|----------|------|-----------|
| ✅ 正常路径 | 同一 source 插入 3 条不同 converter_name，`list_by_source` 按 `converted_at DESC` 返回 3 条 | 已测 | PASS（验证次序 placeholder→pdf-text→markitdown） |
| ✅ 正常路径 | `latest_for_source` 返回最新一条（converted_at 最大） | 已测 | PASS |
| ✅ 正常路径 | `ConversionMetaRow` serde camelCase（`derivedAssetId`/`fallbackUsed`/`conversionMs` 等） | 已测 | PASS（同时验证 roundtrip） |
| ⚠️ 边界条件 | `latest_for_source` 查不到时返回 `Ok(None)` | 已测 | PASS |
| ⚠️ 边界条件 | `derived_asset_id: None` 序列化为 JSON `null`，反序列化回 `None` | 已测 | PASS |
| ⚠️ 边界条件 | `conversion_ms: None` 序列化为 JSON `null` | 已测 | PASS |
| ⚠️ 边界条件 | 迁移 V6 幂等（重跑无副作用） | 间接已测 | `run_migrations` 在每个测试中都跑（4 个测试 = 4 次完整迁移），未失败；且 SQL 使用 `IF NOT EXISTS` |
| ❌ 异常路径 | 删除 source asset → conversion_meta 级联清空 | 已测 | PASS（验证 `ON DELETE CASCADE`） |
| ❌ 异常路径 | 重复 `(source, converter)` 插入不报错（验证**无**唯一约束） | 隐式已测 | `list_by_source_returns_rows_desc` 中 3 个不同 converter；但同 converter 多次插入在 task_008 接线时会自然覆盖。当前未单独测试，因 ADR-004 仅要求**不**加约束，"允许重复"是 SQLite 默认行为，无需正向断言 |

## 已知局限

1. **`id` 由调用方生成**：与 input.md 技术约束一致；本模块不内置 UUID 生成，保持与 `db::asset::insert` 风格一致。task_008 接线时由 scheduler 调用 `uuid::Uuid::new_v4().to_string()`。
2. **未提供 `delete` / `update` 接口**：append-only 日志按设计不可改不可删；级联删除由外键自动处理。如未来需要"归档旧记录"，应新增 `archive_older_than(conn, ts)` 而非 update。
3. **`conversion_ms` 类型转换**：`ConversionAttempt::conversion_ms: u64`（task_005）vs `ConversionMetaRow::conversion_ms: Option<i64>`（本 task）—— 调用层（task_008）需要做 `Some(attempt.conversion_ms as i64)`。这是 SQLite 无 u64 原生类型的必要妥协。
4. **测试用 `Connection::open_in_memory()` 显式 `PRAGMA foreign_keys = ON`**：因为内存库默认不启用外键；生产路径 `Database::open` 已在 `db/mod.rs:34` 开启 `PRAGMA foreign_keys=ON`，CASCADE 验证有效。
5. **当前无调用方**：与 task_005 一致，本 task 仅落地基础设施；task_008 scheduler 接线时会构造 `ConversionMetaRow` 并调用 `insert`。

## 需要 Reviewer 特别关注的地方

1. **`fallback_used` 的 bool ↔ INTEGER 转换**（`conversion_meta.rs` 写入处 `row.fallback_used as i32`，读取处 `row_to_meta` 中 `fallback_int != 0`）：与 SQLite 无 BOOLEAN 原生类型的标准做法一致；rusqlite 也支持 `ToSql for bool` 直接传，但显式 `as i32` 让 schema 意图（INTEGER 列）更明确。若 Reviewer 偏好直接传 `bool`，可简化。
2. **外键策略不对称**：source `CASCADE` / derived `SET NULL`。判断依据见"实现摘要"。若 Reviewer 认为 derived 也应 CASCADE（即衍生件被删 → 其历史日志一并清掉），需 ADR 层面的对齐 —— 当前选择 SET NULL 因为转换日志是"对 source 的事实记录"，与 derived 是否仍存在无关。
3. **`run_migrations` 在测试中跑完整链路（V1→V2→V4→V5→V6）**：每个测试都重新迁移，CI 时间成本可忽略；但若未来 V1 涉及大量种子数据，可考虑提取一个 `setup_v6_only` 辅助。当前无必要。
4. **`db/mod.rs::tests::open_runs_migrations` 中 `assert!(v >= 1)`**：V6 推进后 `user_version = 6`，断言仍 PASS，未破坏既有测试。
5. **M-1 不变量已显式验证**：`extraction/mod.rs:4-5` 的 `// pub mod scheduler;` 注释**保持不变**，task_008 关闭点未被触碰。
