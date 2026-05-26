# Task 输入 — task_perf_01_backend

## 目标

把 NCdesktop 后端 `extract_concepts_for_library`（`src-tauri/src/commands/knowledge.rs:88`）从严格串行改造为：① 4 路并发 LLM 调用；② content 截断到 8 KiB；③ 单文档错误隔离（不终止 batch）；④ P1 增量扫描（`assets.concept_extracted_at` 字段 + skip 已扫描 + force 强制全量 escape hatch）。目标：87 文档全量 84 min → < 10 min；增量秒级。

## 前置条件

- 依赖 task：无（直接基于现有 commit `02fd72a` 的 main 分支）
- 必须先存在的文件/接口：
  - `src-tauri/src/commands/knowledge.rs::extract_concepts_for_library`（line 88，主入口）
  - `src-tauri/src/commands/knowledge.rs::emit_progress`（line 398，事件推送 helper）
  - `src-tauri/src/llm/prompt_runtime.rs::assemble_messages_for_concept`（task_004 改造产物，不动）
  - `src-tauri/src/llm/chat.rs::chat_completion`（task_004 修复的 system 合并）
  - `src-tauri/src/db/migration.rs`（最新 V15，本 task 加 V16）

## 验收标准（Acceptance Criteria）

### AC-1（V16 migration — concept_extracted_at 字段）

- 在 `db/migration.rs` 追加 `fn v16_assets_concept_extracted_at(conn)`：
  - `ALTER TABLE assets ADD COLUMN concept_extracted_at TEXT NULL`（**必须用 ALTER 不能 CREATE，避免破坏既有数据**）
  - SQLite ALTER ADD COLUMN 是幂等的 / 单次执行；用 `PRAGMA table_info(assets)` 前置检查避免重复 ADD（参考 V14 / V13 既有幂等范式）
- `PRAGMA user_version = 16` 推进
- `run_migrations` 增加 v16 dispatcher 入口
- 单测：`fresh_db_runs_all_migrations_to_v16` + `run_migrations_is_idempotent` 覆盖到 v16（既有测试模式参考 task_002 V15 单测）

### AC-2（并发 buffer_unordered(4)）

- 改造 `extract_concepts_for_library` 的主循环（`knowledge.rs:128` 的 `for ... in &assets`）：
  - 使用 `futures::stream::iter(assets).map(|item| async { ... }).buffer_unordered(4)`（或 `tokio_stream::StreamExt`，看仓库已有依赖）
  - **并发数 4 是硬编码**（PRD 决策；不做配置化）；如需调整在常量 `CONCEPT_EXTRACTION_CONCURRENCY: usize = 4` 定义
  - 闭包内：拼 prompt + chat_completion + parse + 写 DB + emit_progress
- 单测：mock chat_completion 返回延迟 Promise，验证 4 个 task 真正并发（time-based assertion 或 spy count）

### AC-3（content 截断到 8 KiB）

- 在拼 prompt 前对 `content_snippet`（line 148 附近）做 byte-safe 截断：
  - 函数：`fn truncate_content_for_concept(content: &str, max_bytes: usize) -> String`
  - byte-safe：不能在多字节 UTF-8 字符中间切断，使用 `content.char_indices().take_while(|(i, _)| *i + ch.len_utf8() <= max_bytes)` 或同等安全切法
  - `max_bytes = 8192`（8 KiB）硬编码常量 `CONCEPT_CONTENT_MAX_BYTES: usize = 8192`
- 截断时若实际发生截断，在 prompt 末尾追加一行提示（在用户自定义 system 注入之外的 user message 末尾）：`"\n\n[Note: content truncated to 8 KiB for performance; first chunk shown above.]"`
- 单测：① 输入 < 8 KiB 不截断 ② 输入 >> 8 KiB 截断到 ≤ 8192 字节 + 含 truncated note ③ 多字节 UTF-8（中文/emoji）边界不切坏

### AC-4（错误隔离）

- 单文档 chat_completion / parse 失败时：
  - **不** `?` 抛出（不终止 batch）
  - 记录 `log::error!("concept extraction failed for asset {asset_id}: {err}")`
  - emit_progress 仍推进 processed（这个文档算"处理过"但 conceptsFound 不增）
  - 失败文档**不**写 `concept_extracted_at`（这样下次增量会重试）
- 单测：mock 第 2 个文档 chat_completion 失败，验证第 1/3/4 个文档仍正常处理；batch 不终止；第 2 个文档的 concept_extracted_at 为 NULL

### AC-5（P1 增量扫描）

- `extract_concepts_for_library` 签名追加 `force_full: bool` 参数（默认调用方传 false = 增量）：
  ```rust
  pub async fn extract_concepts_for_library(
      app: tauri::AppHandle,
      db: State<'_, Database>,
      library_id: String,
      force_full: bool,  // 新增
  ) -> Result<(), String>
  ```
- 查询 assets 列表时：
  - `force_full == false`：`SELECT ... WHERE library_id = ? AND concept_extracted_at IS NULL`
  - `force_full == true`：`SELECT ... WHERE library_id = ?`（全量）+ 在开始 batch 前 `UPDATE assets SET concept_extracted_at = NULL WHERE library_id = ?` 重置标记
- 每个文档处理成功后：`UPDATE assets SET concept_extracted_at = datetime('now') WHERE id = ?`
- emit_progress 的 totalAssets 反映实际待处理数（增量时只数未处理的）
- Tauri command 参数桥接：看 invoke_handler 怎么注册的 extract_concepts_for_library，给前端的 `start_concept_extraction` IPC 入口也加 `force_full: bool` 参数（命名匹配前端期望 — task_perf_02 会跟进对齐）
- 单测：① 增量 mode 跳过已有 concept_extracted_at 的文档 ② force_full=true 清空标记后全量扫描 ③ 失败文档的标记仍为 NULL，下次增量会重试

### AC-6（既有 emit_progress 不破坏）

- 改造后 emit_progress 的事件名仍是 `"notecapt/concept-extraction-progress"`，payload 字段命名与既有一致（`processed`, `totalAssets`, `conceptsFound`, `state`）
- 不修改 emit_progress 函数签名（line 398）
- **并发下 emit_progress 的调用必须线程安全**：内部计数器（processed / conceptsFound）用 `Arc<AtomicUsize>` 或 `Arc<Mutex<u32>>` 包裹
- 单测：并发 4 路全部成功后，processed 计数等于 assets.len()

### AC-7（cargo 全绿 + 不破坏既有调用）

- `cd src-tauri && cargo test --lib` 全表 PASS（task_004 基线 342 + 本期新增）
- `cargo test --test user_prompt_e2e` 仍 20/20（task_008 e2e 不受影响）
- `cargo build` 0 error，不引入新 deprecated warning
- **不改 prompt_runtime.rs / chat.rs / commands/llm.rs / commands/user_prompt.rs**（task_002~004 产物）
- **不改 task_004 的 LLM 调用契约**：concept 模块仍走 `assemble_messages_for_concept`，用户自定义 prompt 注入路径保留

## 技术约束

- **代码规范**：
  - 全部 `Result<T, String>`，错误消息中文
  - 全部 `rusqlite::params!` 参数化
  - tokio idiom：`futures::stream::iter().map().buffer_unordered(N)` 是标准范式
- **并发数硬编码 4**：不做 user config；Architect 决策
- **content 截断阈值 8 KiB**：不做 user config；常量
- **错误处理**：单文档失败仅 log，不 emit error event；UI 端用户看不到失败提示（本期接受 — 未来 P2 加"失败列表"UI）
- **不引入新依赖**：`futures` crate 已在 `Cargo.toml`（通过 tokio 或 reqwest 间接）；如未直接声明则在 Cargo.toml 加 `futures = "0.3"`（**这条允许，因为是 P0 性能修复的必需依赖**；其他新依赖一律禁止）
- **不修改 progress.md**

## 参考文件

**必读**：
- 诊断报告：`/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/docs/diagnose_concept_rescan_perf.md`（性能瓶颈定量分析）
- session_context：`sessions/concept_rescan_perf_v1/session_context.md`
- handoff_contracts § 3：`/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/内置 prompt 改只定义_harness-kit/core/handoff_contracts.md`
- Dev 系统提示词：`/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/内置 prompt 改只定义_harness-kit/roles/conductor/dev/prompt.md`

**代码参考（必读）**：
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/commands/knowledge.rs:88-260`（主战场 extract_concepts_for_library）
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/commands/knowledge.rs:398-415`（emit_progress 实现）
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/db/migration.rs:73-104`（V15 范式：CREATE TABLE IF NOT EXISTS + 单测）
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/db/migration.rs:109-180`（V14 ALTER COLUMN 模板，含 PRAGMA table_info 幂等守卫）— 这是 AC-1 ALTER ADD COLUMN 的最佳模板

## 预估影响范围

- **修改文件**：
  - `src-tauri/src/commands/knowledge.rs`（主战场 ~120 行）
  - `src-tauri/src/db/migration.rs`（v16 ~30 行 + 测试推到 v16）
  - `src-tauri/Cargo.toml`（如需显式加 `futures`，~1 行）
- **预估总变更**：~150-180 行（含测试 ~50 行）

## 并行约束

⚠️ task_perf_02_frontend 正在并行进行（改 `src/components/features/knowledge/KnowledgeAssociationView.tsx`）。
- 你**只改 Rust**，零 TS 改动
- 你的 IPC 入口 `start_concept_extraction(library_id: String, force_full: bool)`（**关键**：前端会按这个签名 invoke；如果你改了参数名，必须在 output.md 显式标注让 task_perf_02 跟进）
- 现有事件 `"notecapt/concept-extraction-progress"` 名称保持不变
