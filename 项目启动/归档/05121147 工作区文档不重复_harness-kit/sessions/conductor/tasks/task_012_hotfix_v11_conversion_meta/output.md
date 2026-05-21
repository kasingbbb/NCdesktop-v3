# task_012 HOTFIX V11 conversion_meta repair — output

## 修复说明

### 根因
- 用户 DB（`~/Library/Application Support/com.notecapt.desktop/notecapt.db`）`PRAGMA user_version = 10`，但缺 `conversion_meta` 表。
- 历史代码遗迹：`startup.rs:39` 注释"含 V10"、`db/repair.rs:141` "V1..V10" 引用确认曾存在 V9/V10 migration，把 user_version 推到 10 但未携带 conversion_meta；之后 V9/V10 实现被删除，当前 `migration.rs` 只到 V8。
- 旧逻辑 `if current_version < 6 { v6_conversion_meta(conn)?; }` 对 user_version=10 用户跳过 V6 → 表永远不会被补建 → `list_root_assets` 报 `no such table: conversion_meta`。

### 修复点
- `src-tauri/src/db/migration.rs`：
  - `run_migrations` 末尾追加 `if current_version < 11 { v11_conversion_meta_repair(conn)?; }`。
  - 新增 `v11_conversion_meta_repair`：复用 V6 的 schema 与三个 `idx_cm_*` 索引，全部 `CREATE TABLE IF NOT EXISTS` / `CREATE INDEX IF NOT EXISTS`；末尾 `PRAGMA user_version = 11;`。
  - 中文错误文案 + `log::info!` 完成日志。
- 未修改 `startup.rs` / `db/repair.rs` 注释（非阻塞，按约束保留）。

### 影响范围
- 仅新增一段幂等 migration 与单测；不动 V1..V8 既有迁移。
- 对所有用户路径安全：
  - user_version ≤ 8 正常用户：V6 已建表 → V11 no-op 仅推版本号至 11。
  - user_version = 9 / 10 残留用户：V6 跳过过 → V11 在此真正补建 conversion_meta + 索引 → 推到 11。
  - 全新用户：V1..V8 跑完 → V11 no-op 推到 11。

### 回归验证
- `cargo test -p notecapt --lib db::migration` 3 个测试全 PASS：
  - `v11_repairs_user_version_10_missing_conversion_meta`（生产残留场景）
  - `fresh_db_runs_all_migrations_to_v11`（全新库）
  - `run_migrations_is_idempotent`（二次调用幂等）
- `cargo build -p notecapt` 通过（仅 pre-existing warning）。
- `cargo test -p notecapt --lib db` 中 `db::knowledge` / `db::co_occurrence` 12 个失败均为 **pre-existing**：V4 注释明确说"concepts 基表不在当前 migration 中"，与本 hotfix 无关。

## §3 Handoff

### 改动文件
- `NCdesktop/src-tauri/src/db/migration.rs`（+约 80 行：V11 函数 + 调用分支 + tests 模块）

### 测试命令与输出

```
$ cargo test -p notecapt --lib db::migration
running 3 tests
test db::migration::tests::v11_repairs_user_version_10_missing_conversion_meta ... ok
test db::migration::tests::run_migrations_is_idempotent ... ok
test db::migration::tests::fresh_db_runs_all_migrations_to_v11 ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 137 filtered out; finished in 0.02s
```

```
$ cargo build -p notecapt
warning: `notecapt` (lib) generated 5 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.21s
```

```
$ cargo test -p notecapt --lib db
test result: FAILED. 37 passed; 12 failed
# 12 failed 全部为 pre-existing（concepts 表不存在），与 hotfix 无关
```

### 风险与残留
- `startup.rs` / `db/repair.rs` 中"V1..V10"残留注释未触动 —— 按硬约束保留，可在后续清理 task 统一处理。
- 已废弃的 V9/V10 migration 在源码中**已不存在**，无残留代码可清除；user_version=10 用户经本次 V11 后将永久跨越该缝隙。
- 用户实际启动后由 `run_migrations` 自动幂等执行，无需人工干预。
