# Task 输入 — task_012_dropzone_auto_extract

## 目标
素材拖入导入后自动触发提取，Toolbar 显示提取进度条和状态摘要。

## 前置条件
- 依赖 task：task_008（PipelineScheduler）、task_010（前端 extractionStore）
- 必须先存在的文件/接口：Scheduler 的 `enqueue` 接口、前端事件监听

## 验收标准（Acceptance Criteria）
1. AC-1：Dropzone 导入完成后（`import_drop_paths` 成功），自动对新导入素材调用 `extract_asset`
2. AC-2：TF 卡同步导入完成后，同样自动入队提取
3. AC-3：Toolbar 显示全局提取进度：`"正在提取 3/15..."` 或 `"提取完成 ✓"`
4. AC-4：进度条使用紧凑的横条样式，不干扰主操作区域
5. AC-5：点击 Toolbar 进度区域可展开简要任务列表（当前任务、失败任务数）
6. AC-6：提取完成时显示 Toast 通知

## 技术约束
- 后端：在 `import_drop_paths` 命令内部，导入成功后直接调用 Scheduler 入队
- 前端：监听 `extraction:batch_progress` 事件更新 Toolbar UI
- 不阻塞导入流程：提取异步执行，导入立即返回
- 遵循现有 Toolbar 布局风格

## 参考文件
- `src-tauri/src/commands/dropzone.rs` — 现有 Dropzone 导入
- `src-tauri/src/commands/sync.rs` — TF 卡同步导入
- `src/components/layout/Toolbar.tsx` — 现有 Toolbar
- `src/stores/extractionStore.ts` — 提取状态
- PRD §3.2 F08 — Dropzone 自动提取

## 预估影响范围
- 修改文件：`src-tauri/src/commands/dropzone.rs`（导入后入队）、`src-tauri/src/commands/sync.rs`（同步后入队）
- 修改文件：`src/components/layout/Toolbar.tsx`（添加进度区域）
- 可能新建：`src/components/features/extraction/ExtractionProgress.tsx`
