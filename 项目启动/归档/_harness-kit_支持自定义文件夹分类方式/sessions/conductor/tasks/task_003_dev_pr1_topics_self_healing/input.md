# Task 输入 — task_003_dev_pr1_topics_self_healing

## 目标
新建 `db/repair.rs::run_post_migration_repair`；修复 `ai_analyses.topics` 字段 schema 失配（旧代码写裸字符串，列契约为 JSON 数组），实现读时自愈 + 异步全表回填；同步修正 `dropzone.rs` 写入端。

## 前置条件
- 依赖 task：task_002（V10 schema 已就位）
- 必须先存在的文件/接口：`db/migration.rs::run_v10`、`commands/dropzone.rs`

## 验收标准（AC）
1. 新建 `src-tauri/src/db/repair.rs`，导出 `run_post_migration_repair(conn, mode: RepairMode) -> RepairReport`，`RepairMode ∈ {Strict, Lenient, ReadOnly}`，`RepairReport { scanned, repaired, failed, dur_ms }`
2. 读时自愈：所有读 `topics` 处包装 `parse_topics_or_empty(s) -> Vec<String>`（解析失败返回 `[]` 并 log warn）
3. 异步回填：启动后 spawn task，分批 500 行扫描 → 把裸字符串重新封成 `["原值"]` JSON，更新；进度可通过 `get_repair_progress` 命令查询
4. 写入修正：`commands/dropzone.rs:347` 处 `topics: r.category.clone()` 改为 `topics: serde_json::to_string(&vec![r.category.clone()])?`
5. `Strict` 模式遇 ≥1 行失败立即报错；`Lenient` 模式失败行归 `__uncategorized__` 并 RepairReport.failed++；`ReadOnly` 模式跳过回填仅做读时自愈
6. 单测：(a) 裸字符串自愈 (b) 已是 JSON 不重复包裹 (c) 损坏值降级 (d) 进度可查
7. 不影响主线程启动延迟（异步 spawn，主线程立返）

## 技术约束
- 使用 `tokio::spawn` 而非阻塞主线；命令返回 `Result<T, String>`
- 中文 log；error 友好
- 不引入新依赖（serde_json 已在）

## 参考文件
- `项目启动/NCdesktop/src-tauri/src/commands/dropzone.rs`（L347 写入点；L115-126 sanitize_path_segment）
- `项目启动/NCdesktop/src-tauri/src/db/migration.rs`（V1 schema topics TEXT default `'[]'`）
- task_001 output.md ADR-002

## 预估影响范围
- 新建：`src-tauri/src/db/repair.rs`（~200 行）
- 修改：`commands/dropzone.rs`（写入修正 ~10 行）、`lib.rs`（注册 `get_repair_progress` 命令）
- 测试：`src-tauri/tests/repair.rs`（新建 ~80 行）

## Reviewer 重点关注
- 异步回填是否会与用户操作产生写写冲突（建议 single-writer + queue）
- `parse_topics_or_empty` 是否覆盖所有现有读处
- Strict/Lenient 模式切换的入口（应由 task_004 bootstrap 决定）
