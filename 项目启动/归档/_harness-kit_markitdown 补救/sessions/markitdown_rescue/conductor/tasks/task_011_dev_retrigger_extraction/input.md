# Task 输入 — task_011_dev_retrigger_extraction

## 目标
新增 Tauri 命令 `retrigger_extraction(asset_id)`，把现有 Inspector "重试"按钮统一到此命令，保证从 failed 或 extracted 任一态都能干净重跑。

## 前置条件
- 依赖 task：task_008（fallback 链路已就位，重跑路径走完整新逻辑）

## 验收标准（AC）
1. **AC-1**：`commands/extraction.rs::retrigger_extraction(database, app, asset_id) -> Result<(), String>`：
   - 校验 asset 存在（不存在返回错误字符串）
   - `extracted_content.status` ← `queued`，清空 `error_message`
   - `pipeline_tasks` 中该 asset 的最近一条记录 `status` ← `queued`，`retry_count` ← 0
   - emit 一个 scheduler 唤醒事件（或直接调用 `PipelineScheduler::enqueue`）
2. **AC-2**：`src/lib/tauri-commands.ts::retriggerExtraction(assetId)` 暴露 + 类型。
3. **AC-3**：`stores/extractionStore.ts::retryExtraction` 改为调用 `retriggerExtraction`，移除任何旧的"前端层模拟重试"逻辑（如果存在）。
4. **AC-4**：手测三场景全部 PASS：
   - failed → 点重试 → 状态变 extracting → 最终 extracted
   - extracted → 点重试 → 状态变 extracting → 重新跑一次（验证 `conversion_meta` 新增一行）
   - extraction 进行中点重试 → 命令安全返回 Err 或 noop（不能造成 pipeline_tasks 重复入队）
5. **AC-5**：在 lib.rs 注册命令。

## 技术约束
- 不允许把 status 直接改为 extracted 跳过 pipeline。
- 入队前检查"是否已 queued/extracting"，避免重复入队。
- 失败仅返回字符串错误；不 panic。

## 参考文件
- `src-tauri/src/commands/extraction.rs`
- `src-tauri/src/extraction/scheduler.rs::PipelineScheduler::enqueue`
- `src/components/layout/InspectorExtraction.tsx:61-63`（既有 `handleRetry`）
- `src/stores/extractionStore.ts::retryExtraction`
- 架构方案 §六 task_011

## 预估影响范围
- 新建文件：无
- 修改文件：
  - `src-tauri/src/commands/extraction.rs`
  - `src-tauri/src/lib.rs`
  - `src/lib/tauri-commands.ts`
  - `src/stores/extractionStore.ts`
