# Task 交付 — task_007_dev_m7_source_scan

## 实现摘要

完成 task_007 全部 7 个 AC：

1. 新建 `src-tauri/src/source_scan.rs`，导出 `SourceMissingSet { inner: RwLock<HashSet<String>> }`，包含 `new / contains / insert / remove / snapshot` 五个方法；新增 `scan_with_conn`（纯函数）与 `scan_all_projects(&AppHandle)`（生产入口）。
2. 修正 task_003 偏离 (a)：原本临时放在 `commands::asset` 的 `SourceMissingSet` 骨架已迁出，`commands::asset` 改为 `pub use crate::source_scan::SourceMissingSet;` 维持兼容路径，不破坏任何既有 import / `try_state::<SourceMissingSet>` 调用。
3. `lib.rs::setup` 末尾追加 `app.manage(SourceMissingSet::new())` 与 `tauri::async_runtime::spawn(async move { scan_all_projects(&app_handle) })`，**不阻塞** setup hook；扫描完成后 emit `notecapt/source-scan-finished { scanned, missing }`。
4. `#[cfg(debug_assertions)]` 注册 debug 命令 `source_scan_get_missing()`，便于 task_009 集成测试断言。
5. 失败仅 `log::warn!`，绝不让应用启动崩溃。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `NCdesktop/src-tauri/src/source_scan.rs` | 新建 | SourceMissingSet 定义 + scan_with_conn 纯函数 + scan_all_projects + debug 命令 + 3 个单测 |
| `NCdesktop/src-tauri/src/commands/asset.rs` | 修改 | 删除原 SourceMissingSet 定义，改 `pub use crate::source_scan::SourceMissingSet`；清理无用 imports（HashSet / RwLock） |
| `NCdesktop/src-tauri/src/lib.rs` | 修改 | `pub mod source_scan;`；setup 注册 SourceMissingSet + async_runtime::spawn 扫描；invoke_handler 注册 `source_scan_get_missing`（debug 限定） |

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（`source_scan.rs` 在 `src-tauri/src/` 一级，与 ADR-004 §六指定位置一致）
- [x] API 路径/命名与 Architect 方案一致（事件名 `notecapt/source-scan-finished`，payload `{ scanned, missing }`，与 task_008 前端约定一致）
- [x] 数据模型与 Architect 方案一致（`SourceMissingSet` 内存态，不持久化；不改变四态本身）
- [x] 未引入计划外的新依赖（仅使用已存在的 tempfile / rusqlite / tauri / serde / log）
- 偏离说明：input.md AC-7 写的 `cargo test -p app_lib --lib source_scan`，但实际 Cargo.toml 的 `[package].name = "notecapt"`（`app_lib` 是 `[lib].name`）；按 user prompt 用 `cargo test -p notecapt` 跑通。

## 测试命令

```bash
cd NCdesktop/src-tauri && cargo test -p notecapt --lib source_scan
cd NCdesktop/src-tauri && cargo build -p notecapt
```

## 测试结果

```
   Compiling notecapt v0.1.0 (/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 4.38s
     Running unittests src/lib.rs (target/debug/deps/app_lib-8c2b6ae4be6c948e)

running 3 tests
test source_scan::tests::set_basic_ops ... ok
test source_scan::tests::empty_project_is_noop ... ok
test source_scan::tests::detects_missing_file ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 125 filtered out; finished in 0.01s
```

`cargo build -p notecapt`：

```
warning: `notecapt` (lib) generated 5 warnings (run `cargo fix --lib -p notecapt` to apply 4 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.18s
```

（5 个 warnings 均来自其他模块的既有遗留，与 task_007 无关。）

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|----------|----------|------|-----------|
| ✅ 正常路径 | 一个 root 文件真实存在 + 一个被 unlink（未创建），断言 missing_set 行为正确 | 已测 | PASS — `detects_missing_file` |
| ✅ 正常路径 | 空 project 调 `scan_with_conn` 返回 (0, 0) 且 set 为空 | 已测 | PASS — `empty_project_is_noop` |
| ✅ 正常路径 | SourceMissingSet 基本 CRUD（contains/insert/remove） | 已测 | PASS — `set_basic_ops` |
| ⚠️ 边界条件 | 启动期 DB 锁失败 → 仅 warn 不 panic | 未测 | scan_all_projects 中已用 `map_err` 转 String 上抛，调用方在 setup 中已 `if let Err` 兜底 warn；无法在单测中构造真实 lock poison |
| ⚠️ 边界条件 | emit 失败 | 未测 | 代码路径有 warn 兜底（`if let Err(e) = app.emit(...)`）；需真实 Tauri runtime，留给 task_009 集成测试 |
| ❌ 异常路径 | scan_all_projects 在 setup 之外被并发调用 | 未测 | 设计上仅 setup 一次性触发；RwLock 即便并发也安全（`unwrap_or(false/default)` 兜底） |

## 已知局限

1. **task_009 待补集成测试**：`notecapt/source-scan-finished` 事件 emit、debug 命令 `source_scan_get_missing` 的端到端验证须在 task_009 的集成测试覆盖（本 task 只跑 `--lib` 单测）。
2. **AC-5 纯函数版**：`scan_with_conn(&Connection, &SourceMissingSet, project_id)` 签名与 input.md 描述一致但顺序为 (conn, missing_set, project_id)；与 input.md 文字描述顺序相同。
3. **package 名差异**：input.md AC-7 写的 `app_lib`（lib name），实际 cargo package 为 `notecapt`；测试命令以 user prompt 与 Cargo.toml 真实值为准。

## 需要 Reviewer 特别关注的地方

- **`commands/asset.rs` 第 9–11 行 `pub use`**：这是 task_003 偏离 (a) 的修复关键，保证旧 import 路径 `commands::asset::SourceMissingSet` 仍可用，所有 `try_state::<SourceMissingSet>` 不破。
- **`lib.rs::setup` 末尾的 spawn 块**：放在 `log::info!` 之前、`Ok(())` 之前；不动既有 `PipelineScheduler::manage` / `needs_wake` 顺序，避免与 task_004/005 冲突。
- **`scan_all_projects` 中 `drop(conn)` 后再 emit**：故意先释放 DB 锁再 emit，避免事件分发时间内阻塞其他命令的 DB 访问。
