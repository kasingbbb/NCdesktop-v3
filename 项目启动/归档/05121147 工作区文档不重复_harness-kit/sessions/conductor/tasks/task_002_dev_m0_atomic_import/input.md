# Task 输入 — task_002_dev_m0_atomic_import

## 目标
按 ADR-006 把 `import_drop_paths` 的 `INSERT asset(pending) → enqueue conversion` 两阶段事务边界明确化：enqueue 失败保留 asset 行（不回滚），让后续 M5/M9 兜底。

## 前置条件
- 依赖 task：无（task_001_architect 已完成）
- 必须先存在的文件/接口（均已存在）：
  - `src-tauri/src/commands/dropzone.rs::import_drop_paths`
  - `src-tauri/src/extraction/scheduler.rs::PipelineScheduler::enqueue`
  - `src-tauri/src/db/asset.rs::insert`

## 验收标准（AC）
1. **AC-1**：抽出一个纯函数 `commands::dropzone::import_files_core(conn, scheduler, project_id, paths) -> ImportDropSummary`（不持 AppHandle，便于单测），保留 `import_drop_paths` 命令为薄包装（仅做 AppHandle / State 解构 + emit 事件）。
2. **AC-2**：若 `PipelineScheduler::enqueue` 返回 `Err`，**不**删除 asset 行 / **不**删除已复制的源文件；将该 asset_id 计入 `ImportDropSummary` 的新字段 `failures_to_enqueue: Vec<String>`（warn log + 用户可见提示）。仍 emit `notecapt/import-drop-finished`。
3. **AC-3**：单测 `commands::dropzone::tests::enqueue_failure_keeps_asset` 覆盖：在内存 DB 上 mock 一个永远失败的 enqueue（用 trait 注入或直接构造错误），断言 asset 行依然在 `assets` 表中，且物理文件存在。
4. **AC-4**：单测 `commands::dropzone::tests::happy_path_inserts_root_and_enqueues` 覆盖：导入两个文件 → 两条 root asset（`source_asset_id IS NULL`）+ 两个 `pipeline_tasks(status='queued')`。
5. **AC-5**：`cargo test -p app_lib --lib commands::dropzone` 全部通过；新增测试不依赖网络。

## 技术约束
- 不在 commands/ 内拼 SQL：所有 DB 操作走 `db::asset::insert` 等已有 API。
- 不裸 `tokio::spawn`：保持现有 `spawn_dropzone_ai_job` 与 `PipelineScheduler::start` 调用，但不新增 spawn。
- 失败语义遵守 ADR-006：失败的 enqueue 不可让 asset 行消失。
- 不在 import_files_core 中跨 await 持 MutexGuard。

## 参考文件
- `src-tauri/src/commands/dropzone.rs`（当前 717 行；改造焦点 484–666 的 `import_drop_paths`）
- `src-tauri/src/extraction/scheduler.rs::PipelineScheduler::enqueue`
- `task_001_architect/output.md` §ADR-006、§十一 task_002 估算

## 预估影响范围
- 新建文件：无
- 修改文件：
  - `src-tauri/src/commands/dropzone.rs`（提取 `import_files_core` + 改失败语义 + 新增单测）
- 估算变更：~250 行（其中 ~120 行为新增测试）
