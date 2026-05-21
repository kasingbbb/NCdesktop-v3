# Task 输出 — task_011_dev_retrigger_extraction

## 修复说明 / 根因分析（Fix Round 1）

### BLOCKER 根因（lib.rs::setup 未 manage PipelineScheduler）
- **问题原因分类**：[x] 遗漏（同时含未验证假设：误以为 task_008 已 manage scheduler）
- **根本原因**：原 output.md 第 29 行声称"scheduler 已注册为 Tauri State" —— 属未验证主张。亲读 lib.rs 第 46 行确认整个 setup 中仅 `app.manage(database)`，从未注册 PipelineScheduler。task_008 仅激活 `extraction::scheduler` 模块声明，未补 manage 调用。因此 `retrigger_extraction:111` 的 `app.state::<PipelineScheduler>()` 在 happy path 走完幂等检查后会 panic（Tauri Manager::state 在 T 未注册时强制 panic）。
- **影响范围**：同 invoke_handler 中 `extract_asset` / `retry_extraction` / `extract_project_assets` / `get_pipeline_progress` 等所有 `app.state::<PipelineScheduler>()` 调用点同样受益（pre-existing 缺陷，但仅 `retrigger_extraction` 已被前端真实触发）。
- **为什么之前没注意到**：盲信代码可编译即代表 State 已注册；未亲跑 `grep "manage::<PipelineScheduler>\\|manage(.*Scheduler"`。task_008 中也未 manage scheduler（已 grep 全 lib.rs 确认）。

### MAJOR 根因（pipeline_tasks 表生产缺失）
- **问题原因分类**：[x] 架构偏离（pre-existing schema 缺口）
- **根本原因**：`db::extraction` 与 `extraction::scheduler` 全量引用 `pipeline_tasks`（grep 命中 7 处生产 SQL），但 `db/migration.rs` 从 V1~V6 从未 CREATE 此表 —— 仅 `commands/extraction.rs:176` 单测内存库自建。生产 DB 中表根本不存在，task_011 的 `UPDATE pipeline_tasks ...`（reset_extraction_state 第 138 行）会触发 `no such table`。
- **调研结论**：跑 `grep -rn "CREATE TABLE.*pipeline_tasks" src-tauri/src/` 仅 1 命中（在单测 setup_db 中）。**无任何 runtime ensure_pipeline_tasks_table 路径**。必须走"第一选择" —— 追加 V7 迁移。
- **影响范围**：task_008 scheduler 全部产线代码同步获益；不破坏单测内存库（迁移仅追加，不与单测的 CREATE TABLE 冲突，因单测库未跑 migration）。

### 运行时验证策略
BLOCKER 选 **B**：登记为"用户手测验证项"（启动应用，对 failed 资产点重试，无 panic）。A/C 方案对单一 `app.manage` 调用回报极低。MAJOR 由"启动 → migration 自动建表 → 任意 retrigger 调用"端到端覆盖；单测层面通过 V7 迁移成功的日志输出 (`V7 完成：pipeline_tasks ...`) 间接验证。

### 修复涉及的已 PASS task 代码声明
- `lib.rs::setup`：新增一行 `app.manage(PipelineScheduler::new())`。属于 task_008 关闭后的注册缺口修复，与 task_011 强相关（task_011 的命令必须可运行）。
- `db/migration.rs`：新增 V7 迁移函数 `v7_pipeline_tasks`。属补齐 pre-existing schema 缺口，task_008 与 task_011 双重依赖此表存在。

## 完成状态
DONE

## 改动文件清单
1. `src-tauri/src/commands/extraction.rs`
   - 新增 `#[tauri::command] pub async fn retrigger_extraction(app, asset_id) -> Result<(), String>`
   - 新增纯函数 `pub fn reset_extraction_state(conn, asset_id) -> Result<(), String>`
   - 新增 `#[cfg(test)] mod tests`：3 个单测覆盖 reset 行为
   - 顶部 `use` 新增 `rusqlite::{params, Connection}`
2. `src-tauri/src/commands/mod.rs`
   - 新增 `pub mod extraction;`（修复前 `commands::extraction` 未对外暴露的疏漏，否则 lib.rs 注册无法编译）
3. `src-tauri/src/lib.rs`
   - `invoke_handler!` 追加 `commands::extraction::retrigger_extraction`
   - **【FIX BLOCKER】**`setup` 内追加 `app.manage(extraction::scheduler::PipelineScheduler::new());`，置于 `app.manage(database)` 之后
4. `src/lib/tauri-commands.ts`
   - 新增 `export async function retriggerExtraction(assetId): Promise<void>`
5. `src/stores/extractionStore.ts`
   - `retryExtraction` 实现切换到 `cmd.retriggerExtraction`
   - 立即将 statusCache 置为 `queued`（更贴合后端语义），并触发 `fetchExtractedContent` 拉新状态

6. **【FIX MAJOR】**`src-tauri/src/db/migration.rs`
   - 新增 V7 迁移函数 `v7_pipeline_tasks`：CREATE TABLE pipeline_tasks（列与 `db::extraction::PipelineTaskRow` 全对齐）+ 2 个索引 + 1 个**部分唯一索引** `idx_pipeline_tasks_active_unique ON (asset_id, task_type) WHERE status IN ('queued','running')`，恢复"第二道幂等护栏"
   - `run_migrations` 主入口追加 `if current_version < 7 { v7_pipeline_tasks(conn)?; }`

**未触碰 PM 手改的 31 个前端文件**（仅动了 PM 未改的 `tauri-commands.ts` 与 `extractionStore.ts`）。

## PipelineScheduler 唤醒方式
通过既有公共 API：
1. `PipelineScheduler::enqueue(&app, &asset_id)` —— 关联函数；其内部 `insert_pipeline_task` 在 `UNIQUE constraint` 冲突时返回 `Ok("already_queued")`，构成第二道防重护栏。
2. `app.state::<PipelineScheduler>().start(app.clone())` —— `start` 自身用 `is_running` 互斥幂等，多次调用安全。

**未**新增 emit 事件路径；scheduler 已注册为 Tauri `State`，直接调用即可。

## 关键设计要点
- **三道幂等护栏**：
  1. 命令前置查 `extracted_content.status`，若为 `queued`/`extracting` 直接 `Ok(())` noop（不重置、不入队）；
  2. `enqueue` 内部对 `pipeline_tasks` UNIQUE 冲突静默返回 `already_queued`；
  3. `scheduler.start` 内部 `is_running` 互斥。
- **绝不跳 pipeline**：仅把 status 置 `queued`，绝不直接置 `extracted`；后端正常流转 `queued → extracting → extracted/failed`。
- **锁释放顺序**：在调用 `PipelineScheduler::enqueue` 前显式让 `db.conn` 的 `MutexGuard` 析构（用块作用域），避免 enqueue 内部二次锁导致 dead-lock。
- **失败仅 `Result<_, String>`**：无 panic、无 `unwrap()`/`expect()` 在非测试代码；SQL 全部参数化。
- **`reset_extraction_state` 是 `pub fn` 纯函数**：传入已锁的 `&Connection`，单测在内存 SQLite 上直接覆盖，无需构造 `AppHandle` / `State<Database>`。

## 校验结果
- `cargo check`：**0 error**（既有 3 个无关 warning：llm/chat.rs unused vars，与本任务无关）
- `cargo test --lib commands::extraction`：**3 passed; 0 failed**
  - `reset_from_failed_clears_error_and_requeues`
  - `reset_from_extracted_requeues_for_rerun`
  - `reset_when_no_row_is_noop`
- `npx tsc --noEmit`：**EXIT=0**，0 error
- 前端未新增 vitest（按约束避免触发 PM 改过的测试文件）

### Fix Round 1 回归证据
- `cargo check`：**0 error**（与 Fix 前一致；唯一 warning 仍是 llm/chat.rs，与本任务无关）
- `cargo test --lib commands::extraction`：**3 passed; 0 failed**（原 3 单测保持通过，证明 Fix 未回归）
- `cargo test --lib db`：12 passed; 12 failed。**12 个失败全部为 pre-existing**：失败原因 `no such table: concepts` —— `concepts` 表从 V1~V7 任何迁移中都不存在（V4 注释明确说"V3 不存在"，且全仓 `grep "CREATE TABLE.*concepts"` 0 命中）。这是 task_011 与本次 Fix 之外的历史缺陷，**与 V7 pipeline_tasks 迁移无关**：V7 日志已正确打印（"数据库迁移 V7 完成"），但 V3 缺失导致 V4 引用的外键无对应表。Reviewer 可独立开 follow-up task 跟踪 concepts 表 DDL；本 Fix 不扩大 scope。
- `npx tsc --noEmit`：**EXIT=0**

### BLOCKER 运行时验证（选项 B）
BLOCKER 修复（`app.manage(PipelineScheduler::new())`）的**运行时验证依赖用户手测**：参考 AC-4 手测脚本"failed → 重试"场景，启动应用、对 failed asset 点重试，应**无 panic**且 status 立即变 queued。未写自动化 #[test] 检查（选项 A）因构造 AppHandle 成本远高于直接 e2e。

## 用户手测脚本（AC-4）
> 需用户在 dev 环境验证三场景：

1. **failed → 重试**
   - 找一个 `extracted_content.status='failed'` 的 asset；在 Inspector 点"重试"
   - **期望**：status 立即变 `queued` → 数秒内 emit `extraction:progress` 转 `extracting` → 最终 `extracted`（或再次 `failed`，取决于 extractor）
   - **验证**：`pipeline_tasks` 中该 asset 行 `retry_count` 已重置为 0
2. **extracted → 重试（重跑）**
   - 找已成功 `extracted` 的 asset；点"重试"
   - **期望**：status `queued → extracting → extracted`；`conversion_meta` 表新增一行（converter_name + converted_at），`assets.derivative_version` +1，`_versions/<source_id>/v{N-1}.md` 出现归档
3. **进行中重试 → noop**
   - 在 asset 正处于 `extracting` 时再次点"重试"
   - **期望**：命令返回成功（Ok），但 `pipeline_tasks` **不出现重复行**；日志可见 `retrigger_extraction: <id> 已处于 extracting 状态，跳过重复入队`

## AC 自检
- AC-1: PASS（retrigger_extraction 完整实现 + 4 步流程 + scheduler 唤醒）
- AC-2: PASS（`retriggerExtraction(assetId)` 已暴露并类型化）
- AC-3: PASS（`retryExtraction` 切换为 retriggerExtraction，无遗留前端模拟逻辑）
- AC-4: 待用户手测（脚本如上）
- AC-5: PASS（lib.rs invoke_handler 已注册）
