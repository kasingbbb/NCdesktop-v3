# Review Scorecard — task_007_dev_m7_source_scan

## 审查前验证

- [x] 测试结果存在且非空（3 tests passed）
- [x] 自测验证矩阵存在且正常路径全 PASS
- [x] 架构遵守声明已填写
- 进入实质性审查。

## 审查思考过程

1. **Task 意图**：启动期一次性遍历所有 root assets，stat 各自 `file_path`；不存在的写入内存态 `SourceMissingSet`；emit `notecapt/source-scan-finished` 让前端 invalidate list。不引入 fsnotify、不阻塞 setup hook。
2. **AC 检查结果**：
   - AC-1 ✅：`source_scan.rs` 导出 `SourceMissingSet { inner: RwLock<HashSet<String>> }` + new/contains/insert/remove/snapshot + `scan_all_projects(&AppHandle) -> Result<usize, String>`。签名与 input.md 一致。
   - AC-2 ✅：`scan_all_projects` 取 `Database` state → 遍历 `library::get_all` → `project::get_by_library` → 对每个 project 调 `db::asset::list_root_assets`；不存在的 `insert(asset.id)`；emit `notecapt/source-scan-finished` 携带 `{ scanned, missing }`；返回 missing。drop(conn) 后再 emit，避免持锁分发事件。
   - AC-3 ✅：`lib.rs:94` `app.manage(SourceMissingSet::new())`；`lib.rs:97` `tauri::async_runtime::spawn(...)` 包裹 `scan_all_projects`，不阻塞 setup；失败 warn。
   - AC-4 ✅：`commands/asset.rs:35,303` 已注入 `Option<State<SourceMissingSet>>`，派生 `source_missing` 字段（在 task_003 中 wire，本 task 通过 `pub use` 让旧路径继续可用）。
   - AC-5 ✅：`detects_missing_file` 用 `tempfile::tempdir` + 一个真实存在 + 一个不创建，断言 missing_set 行为正确；并附加 `empty_project_is_noop` 与 `set_basic_ops` 两个边界测试。
   - AC-6 ✅：`source_scan_get_missing` 在 `#[cfg(debug_assertions)]` 下定义并注册（lib.rs:192-193 同样 cfg 包裹于 invoke_handler）。
   - AC-7 ✅：`cargo test -p notecapt --lib source_scan` 3 passed。
3. **关键发现**：
   - task_003 偏离 (a) 已正确归位：`commands::asset` 改为 `pub use crate::source_scan::SourceMissingSet`，grep 确认全仓库**仅** `source_scan.rs` 有定义，无残留重复定义。
   - 实现严格遵守 PRD 硬约束（不引入 fsnotify、scan 走 `list_root_assets` 唯一入口、`RwLock<HashSet>`、失败仅 warn）。
   - 与 ADR-004 在"启动期是否同步等待第一遍"上有差异：ADR-004 §"风险登记表"提到"setup hook 中同步等待完成第一遍"，而 input.md AC-3 明确要求 "**不**阻塞 setup（用 async_runtime::spawn）"。本实现按 input.md。这是 input.md 与 ADR 间的一致性偏差，应由 Conductor 知悉，但 Dev 严格遵循了 input.md。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | 7 个 AC 全部满足；scan_with_conn 纯函数化便于单测；drop(conn) 再 emit 避免锁竞争。 |
| 用户体验 | 25% | 4 | emit `source-scan-finished` 让前端 invalidate；唯一遗憾是未同步等待首遍（ADR-004 §风险登记表的本来建议），冷启动短时窗内列表可能未带 `source_missing=true`。但这是 input.md 显式要求的取舍，扣 0.5。 |
| 架构一致性 | 20% | 5 | 文件位置（`src-tauri/src/source_scan.rs`）与 ADR-004 §六指定一致；走 `list_root_assets` 唯一查询入口；不引入新依赖；commands/ 不拼 SQL。task_003 偏离 (a) 已归位。 |
| 代码质量 | 10% | 5 | 模块级文档清晰；纯函数与生产入口分离；锁错误统一 `unwrap_or(_)` 不 panic；命名清晰。 |
| 测试覆盖 | 10% | 4 | detects_missing_file / empty_project_is_noop / set_basic_ops 三个测试覆盖正常路径与基本边界；emit 失败、DB 锁失败、并发未覆盖（合理 deferred 给 task_009 集成测试）。 |
| 可维护性 | 10% | 5 | 错误处理一致 warn；每段都注明 ADR / task 出处；scan_with_conn 抽出避免与 AppHandle 耦合。 |

**综合分：4.75 / 5**（= 5*0.25 + 4*0.25 + 5*0.20 + 5*0.10 + 4*0.10 + 5*0.10）

## 总体判断

- [x] **PASS**
- [ ] FIX
- [ ] BLOCKER

## 问题列表

### BLOCKER
（无）

### MAJOR
（无）

### MINOR
1. **`Cargo.toml` 含 `notify = "8"` 依赖**：PRD 硬约束"不引入 fsnotify"。审查确认本仓库源码 grep 无 `use notify` / `notify::` 实际调用，且该依赖在 task_007 之前的 commit 中已加入（不是本 task 引入）。建议后续 task 单独处理：要么删除未使用依赖，要么在 PRD 中显式豁免（疑似 `tauri-plugin-fs-watch` 间接需求）。**不影响本 task PASS**。
2. **ADR-004 vs input.md 一致性偏差**：ADR-004 §"风险登记表"建议"setup 同步等待完成第一遍扫描"，input.md AC-3 改为"不阻塞 setup"。Dev 跟随 input.md，无过错；建议 Conductor 在 progress.md 中显式记录该 trade-off：冷启动至扫描完成的短时窗内，前端可能尚未拿到 source-missing 标志。
3. **`scan_all_projects` 持 Database 锁跨整个遍历**：当前实现取 `conn = database.conn.lock()` 后跨 N 个 library × M 个 project 的扫描全程持锁。对启动期单次扫描可接受（async_runtime::spawn 内，主线程 setup 已返回），但若数据量很大可能短暂阻塞首批命令。可后续优化为"先快照 (library_id, project_id) 列表，释放锁，再批次重获锁扫描"。
4. **emit 失败仅 warn**：若 Tauri runtime 未就绪致 emit 失败，前端将永远收不到 invalidate。当前 warn 兜底合理；建议在 task_009 集成测试中加端到端断言（output.md 已声明）。

## 给 Dev 的修复指引

无（PASS，无需修复）。
