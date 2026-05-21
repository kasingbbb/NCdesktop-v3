# Task 输入 — task_008_pipeline_scheduler

## 目标
实现管道调度器 `PipelineScheduler`：管理提取任务队列，顺序执行，支持启动恢复、失败重试、Tauri Event 进度推送。

## 前置条件
- 依赖 task：task_002（pipeline_tasks 表）、task_003（Extractor trait）
- 必须先存在的文件/接口：`db/extraction.rs` CRUD、`extraction/mod.rs` trait

## 验收标准（Acceptance Criteria）
1. AC-1：`extraction/scheduler.rs` 实现 `PipelineScheduler` 结构体
2. AC-2：`enqueue(asset_id, task_type) -> Result<task_id>` — 将任务写入 `pipeline_tasks` 表
3. AC-3：`enqueue_batch(asset_ids, task_type) -> Result<batch_id>` — 批量入队
4. AC-4：`start()` 方法启动后台 tokio task，循环从 DB 取 `queued` 任务执行
5. AC-5：执行时根据 `asset.mime_type` 路由到正确的提取器
6. AC-6：提取成功 → 写入 `extracted_content` + 更新 `pipeline_tasks.status = 'completed'`
7. AC-7：提取失败 → 若 `retry_count < max_retries` 则重新入队；否则标记 `failed`
8. AC-8：启动恢复：应用启动时将 `status = 'running'` 的任务重置为 `queued`
9. AC-9：Tauri Event 推送：`extraction:progress`, `extraction:completed`, `extraction:failed`, `extraction:batch_progress`
10. AC-10：IPC 命令 `extract_asset`, `extract_project_assets`, `retry_extraction`, `get_pipeline_progress` 正确注册

## 技术约束
- 顺序执行（MVP），不做并发提取
- 使用 `AppHandle.emit()` 推送全局事件
- Scheduler 实例通过 `app.manage()` 托管到 Tauri State
- 使用 `Mutex` 保护内部队列状态
- DB 操作须通过现有 `Database` 的 `Mutex<Connection>`

## 参考文件
- `src-tauri/src/lib.rs` — app.manage 模式
- `db/extraction.rs` — 任务 CRUD
- `extraction/mod.rs` — Extractor trait
- Architect output.md §API 设计 — IPC 命令表和事件表
- PRD §3.2 F04 — 管道调度器需求

## 预估影响范围
- 新建文件：`src-tauri/src/extraction/scheduler.rs`, `src-tauri/src/commands/extraction.rs`
- 修改文件：`src-tauri/src/lib.rs`（manage Scheduler + 注册 IPC 命令）、`src-tauri/src/commands/mod.rs`（pub mod extraction）
- 修改文件：`src-tauri/capabilities/default.json`（事件权限）
