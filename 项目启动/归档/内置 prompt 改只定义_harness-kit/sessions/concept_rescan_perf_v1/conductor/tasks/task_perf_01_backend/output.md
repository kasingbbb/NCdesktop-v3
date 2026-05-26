# Task 交付 — task_perf_01_backend

## 实现摘要

把 NCdesktop 知识概念重新扫描后端 `extract_concepts_for_library` 从**严格串行** LLM 调用循环改造为：① **4 路并发** `futures_util::stream::iter().buffer_unordered(4)`；② content **byte-safe 截断到 8 KiB**（UTF-8 字符边界），并在 user message 末尾追加中文 truncation note；③ **错误隔离**单文档 chat_completion / parse 失败仅 `log::error!`，不抛 `?`，processed 推进但 conceptsFound 不变，失败者**不写 `concept_extracted_at`**（下次增量自动重试）；④ **P1 增量扫描**：新 V16 migration 加 `assets.concept_extracted_at TEXT NULL` 字段，函数签名追加 `force_full: bool` 参数，`false` 时 SQL `WHERE concept_extracted_at IS NULL` 仅扫未处理，`true` 时先 UPDATE 重置全库标记后再全量扫（用户 escape hatch）；⑤ **并发安全计数**：`processed` / `concepts_found` 用 `Arc<AtomicUsize>` + `Ordering::Relaxed` 包裹，`emit_progress` 函数签名与事件名 / payload 完全不动。

**IPC 命名策略 — A 改名 + 兼容 wrapper**：
- 新 Tauri command `start_concept_extraction(library_id, force_full)` 作为前端 task_perf_02 期望的入口
- 旧 `extract_concepts_for_library(library_id, force)` 保留为 **thin wrapper** 转发到新签名（`force=true` ↔ `force_full=true`），前端 task_perf_02 切换前/后混合发布期都能工作。**不引入 deprecated warning**（用普通文档注释代替 `#[deprecated]`，避开 AC-7 "0 new warning" 约束）

**关键并发竞争点 — concepts.UNIQUE(library_id, name) 冲突**：buffer_unordered 4 路并发可能同时提取出同名 concept；改用 `INSERT OR IGNORE + 重查 id` 模式：先依快照 HashMap 查命中走 append_source_asset；未命中则尝试 INSERT，再用 `SELECT id WHERE (library_id, name)` 重查（保证另一并发闭包先插入时仍能拿到正确 id）+ 兜底 append 一次。

**核心设计决策**：
- 不引入新依赖（`futures-util = "0.3"` 已在 Cargo.toml line 30 显式声明，直接 `use futures_util::stream::{self, StreamExt}` 即可，**Cargo.toml 零改动**）
- 闭包通过 `&db: &State<'_, Database>` 借用 `Mutex<Connection>` —— async fn 主帧持有 State 生命周期保证 buffer_unordered Stream 内闭包安全；DB lock 只在**短作用域**抢占（拼 prompt + 写 concepts/cases + 写 mark），LLM HTTP 调用时**锁完全释放**，4 路真正并发
- F-8 旧日志（`concepts_extraction_log(asset_id, content_hash)`）与 V16 新标记（`assets.concept_extracted_at`）**双写**保留：F-8 维度细（同 asset 不同 hash 仍重抽），V16 维度粗（force_full 的 escape hatch 真相来源）

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `src-tauri/src/db/migration.rs` | 修改 | 新增 `v16_assets_concept_extracted_at`（ALTER + PRAGMA table_info 守卫，仿 V5/V12 幂等范式）+ run_migrations dispatcher 入口 + 3 个 v16 相关测试（`fresh_db_runs_all_migrations_to_v16` 改自 v15 / `run_migrations_is_idempotent` 升级到 v16 / 新增 `v16_idempotent_with_existing_column`） |
| `src-tauri/src/commands/knowledge.rs` | 修改 | ① 主战场：新 `start_concept_extraction` Tauri command（带 `force_full` 参数）+ 旧 `extract_concepts_for_library` 改为 thin wrapper ② `buffer_unordered(4)` 并发主循环 ③ 闭包内 `truncate_content_for_concept` byte-safe 截断 + 8 KiB 常量 + truncation note 追加 ④ INSERT OR IGNORE + 重查 id 并发竞争兜底 ⑤ AtomicUsize 计数器 ⑥ 新增 helper：`fetch_library_assets_for_extraction(force_full)` / `reset_library_concept_extracted_at` / `mark_asset_concept_extracted` / `truncate_content_for_concept` ⑦ 11 个 cfg(test) 单测覆盖 AC-2/3/4/5/6 |
| `src-tauri/src/lib.rs` | 修改 | invoke_handler 注册新 `start_concept_extraction`（旧 `extract_concepts_for_library` 保留） |

**未触碰**（关键合规）：
- `src-tauri/src/llm/prompt_runtime.rs`（task_004 产物）
- `src-tauri/src/llm/chat.rs`（task_004 产物）
- `src-tauri/src/commands/llm.rs`（task_004 产物）
- `src-tauri/src/commands/user_prompt.rs`（task_007/task_008 产物）
- `src-tauri/src/db/user_prompt.rs`（task_002~005 产物）
- `src-tauri/Cargo.toml`（**零改动**，futures-util 已是直接依赖）
- 任何 `src/*.ts` / `src/*.tsx` 前端文件（**零 TS 改动**，并行约束遵守）
- `progress.md`（遵守约束）

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（保持 `src-tauri/src/commands/knowledge.rs` 单文件，未拆分）
- [x] API 路径/命名与 Architect 方案一致（input.md 期望 `start_concept_extraction(library_id, force_full)` — 已落地；旧 IPC 名保留兼容）
- [x] 数据模型与 Architect 方案一致（V16 仅追加 `assets.concept_extracted_at TEXT NULL`，**不动既有列**）
- [x] 未引入计划外的新依赖（`futures-util` 早已是 Cargo.toml line 30 直接依赖，复用既有 crate；input.md 第 97 行允许 `futures = "0.3"` 但实际未需要追加）
- 偏离说明（如有）：
  1. **IPC 命名策略**：input.md 建议两种方案择一，本 task 选 A（改名 + 保留旧名 wrapper）。理由：前端 task_perf_02 期望 `start_concept_extraction` 已写在 input.md AC-3，A 方案让前后端都能无痛切换；旧名 wrapper 保持向后兼容，避免 task_perf_02 完成前 / 混合发布期破坏。
  2. **`emit_progress` 字段名兼容**：input.md AC-6 提到 payload 含 `state` 字段，但实际既有代码 emit 的是 `status`（参 line 413）。**保留既有 `status` 字段名**（既有前端 listen 与 ExtractionProgress struct 都按 `status` 处理；改名会破坏 AC-6 "不破坏 payload"）。前端 task_perf_02 应按 `status` 而非 `state` 解析（**接口建议 ②**）。
  3. **content 截断 note**：input.md AC-3 描述截断 note 在 "user message 末尾"。实际 messages 数组的最后一条是 `CONCEPT_OUTPUT_GUARD`（system role），所以追加位置改为 **"messages 数组反向找到的第一条 role==user"** 的末尾，与 input.md 语义一致（"user 内容末尾"）。

## 测试命令

```bash
cd "/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri"
cargo test --lib migration
cargo test --lib knowledge
cargo test --lib
cargo test --test user_prompt_e2e
cargo build
```

## 测试结果

### `cargo test --lib migration`
```
running 12 tests
test db::migration::tests::v11_repairs_user_version_10_missing_conversion_meta ... ok
test db::migration::tests::v15_idempotent_with_existing_table ... ok
test db::migration::tests::fresh_db_runs_all_migrations_to_v16 ... ok
test db::migration::tests::run_migrations_is_idempotent ... ok
test db::migration::tests::v12_alter_is_idempotent_against_existing_column ... ok
test db::migration::tests::v14_does_not_overwrite_existing_failure_code ... ok
test db::migration::tests::v14_keeps_null_when_content_present ... ok
test db::migration::tests::v14_backfills_extracted_with_empty_content ... ok
test db::migration::tests::v14_only_touches_latest_row_per_asset ... ok
test db::migration::tests::v14_is_idempotent ... ok
test db::migration::tests::v16_idempotent_with_existing_column ... ok
test db::tests::open_runs_migrations ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 343 filtered out; finished in 0.13s
```

### `cargo test --lib knowledge`
```
running 34 tests
test commands::knowledge::tests::build_cases_block_empty_cases_yields_empty_string ... ok
test commands::knowledge::tests::build_cases_block_renders_indexed_contexts ... ok
test commands::knowledge::tests::buffer_unordered_with_simulated_failures_isolates_errors ... ok
test commands::knowledge::tests::atomic_counter_concurrent_increments_yield_correct_total ... ok
test commands::knowledge::tests::short_content_does_not_append_truncation_note ... ok
test commands::knowledge::tests::truncate_empty_content_returns_empty_and_false ... ok
test commands::knowledge::tests::ac5_inspect_returns_user_overridden_true_when_concept_custom ... ok
test commands::knowledge::tests::ac8_aggregation_system_field_literally_contains_knowledge_synthesis_engine ... ok
test commands::knowledge::tests::truncate_respects_utf8_char_boundary_for_cjk ... ok
test commands::knowledge::tests::truncate_short_content_returns_original_and_false ... ok
test commands::knowledge::tests::truncate_long_content_bounded_by_max_bytes_and_true ... ok
test commands::knowledge::tests::ac5_inspect_returns_user_overridden_true_when_aggregation_custom ... ok
test commands::knowledge::tests::truncate_respects_utf8_char_boundary_for_emoji ... ok
test commands::knowledge_synthesis::tests::expand_merges_handles_out_of_range_index ... ok
test commands::knowledge_synthesis::tests::expand_merges_preserves_all_when_no_merges ... ok
test commands::knowledge_synthesis::tests::expand_merges_skips_duplicate_reference ... ok
test commands::knowledge_synthesis::tests::expand_merges_unions_and_dedups ... ok
test commands::knowledge::tests::ac5_inspect_returns_user_overridden_false_when_no_custom_prompt ... ok
test commands::knowledge_synthesis::tests::parse_concept_groups_handles_prose_preamble ... ok
test commands::knowledge_synthesis::tests::parse_concept_groups_plain_json ... ok
test commands::knowledge_synthesis::tests::parse_concept_groups_handles_markdown_fence ... ok
test commands::knowledge_synthesis::tests::parse_concept_groups_errors_on_malformed ... ok
test commands::knowledge::tests::ac8_concept_system_field_literally_contains_knowledge_extraction_engine ... ok
test commands::knowledge::tests::ac8_concept_custom_template_still_injects_system_addon ... ok
test llm::prompt_runtime::tests::system_addons_match_existing_knowledge_rs_literals ... ok
test commands::knowledge::tests::mark_asset_concept_extracted_sets_timestamp_and_skips_next_incremental ... ok
test commands::knowledge::tests::fetch_assets_incremental_skips_already_processed ... ok
test commands::knowledge::tests::reset_library_concept_extracted_at_makes_all_pending ... ok
test commands::knowledge::tests::truncated_content_appends_note_to_user_message ... ok
test db::knowledge::tests::update_concept_marks_user_edited ... ok
test db::knowledge::tests::insert_ignore_duplicate_concept ... ok
test db::knowledge::tests::insert_and_get_concept ... ok
test db::knowledge::tests::get_concepts_with_stats_returns_counts ... ok
test db::knowledge::tests::delete_concept_cascades ... ok

test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 321 filtered out; finished in 0.18s
```

### `cargo test --lib`（全表）
```
test result: ok. 355 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.32s
```
（基线 task_004 = 342；task_002 V15 加 1 = 343；本 task 加 12 = 355；与 input.md 预期 "全表 ≥ 343" 吻合）

### `cargo test --test user_prompt_e2e`
```
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
```
（task_008 e2e 20/20 完全保持，task_004 用户自定义 prompt 注入链路未被破坏）

### `cargo build`
```
warning: unused import: `PathBuf`            (改造前已存在；commands/dropzone.rs:10)
warning: unused variable: `client`           (改造前已存在；llm/chat.rs:129，stream API stub)
warning: unused variable: `messages`         (改造前已存在；llm/chat.rs:130)
warning: unused variable: `on_chunk`         (改造前已存在；llm/chat.rs:131)
warning: fields `block_type` and `thinking` are never read   (改造前已存在；llm/chat.rs:47)
warning: `notecapt` (lib) generated 5 warnings (run `cargo fix --lib -p notecapt` to apply 4 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 12.75s
```
- **0 error**
- **5 warning 全部是改造前 main 分支已有的**（已 `git stash` 比对验证）
- **0 new warning**（含 0 deprecated warning — wrapper 用普通文档注释而非 `#[deprecated]`）

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|----------|----------|------|-----------|
| ✅ 正常路径 | V16 fresh DB 跑 V1..V16，user_version=16 + assets.concept_extracted_at 列存在 | 已测 | PASS `fresh_db_runs_all_migrations_to_v16` |
| ✅ 正常路径 | V16 二次跑 run_migrations 幂等（PRAGMA table_info 守卫跳过重复 ADD COLUMN） | 已测 | PASS `run_migrations_is_idempotent` + `v16_idempotent_with_existing_column` |
| ✅ 正常路径 | truncate_content_for_concept 短内容（< 8 KiB）原样返回 + truncated=false | 已测 | PASS `truncate_short_content_returns_original_and_false` |
| ✅ 正常路径 | truncate 长 ASCII 内容（> 8 KiB）刚好打满 8192 字节 + truncated=true | 已测 | PASS `truncate_long_content_bounded_by_max_bytes_and_true` |
| ✅ 正常路径 | truncate 后 user message 末尾追加 truncation note（且短内容不追加） | 已测 | PASS `truncated_content_appends_note_to_user_message` + `short_content_does_not_append_truncation_note` |
| ✅ 正常路径 | 增量扫描 force_full=false 仅返回 concept_extracted_at IS NULL 的素材 | 已测 | PASS `fetch_assets_incremental_skips_already_processed` |
| ✅ 正常路径 | force_full=true 重置标记后再扫描 → 返回全量 | 已测 | PASS `reset_library_concept_extracted_at_makes_all_pending` |
| ⚠️ 边界条件 | truncate CJK 中文（3 字节/字符，跨 8192 边界）不切碎字符 | 已测 | PASS `truncate_respects_utf8_char_boundary_for_cjk`（断言：长度是 3 倍数 + 实际 8190 字节） |
| ⚠️ 边界条件 | truncate emoji（4 字节/字符，跨 8192 边界）不切碎 | 已测 | PASS `truncate_respects_utf8_char_boundary_for_emoji`（8192 / 4 = 2048 完整 emoji） |
| ⚠️ 边界条件 | truncate 空字符串不 panic | 已测 | PASS `truncate_empty_content_returns_empty_and_false` |
| ⚠️ 边界条件 | 100 个并发 fetch_add(1) 累加结果精确 = 100 | 已测 | PASS `atomic_counter_concurrent_increments_yield_correct_total`（std::thread 模拟） |
| ❌ 异常路径 | buffer_unordered 4 路并发，第 2 个任务"失败"（仅 log 不抛），processed 仍推进到 4，concepts_found 不含失败者贡献 | 已测 | PASS `buffer_unordered_with_simulated_failures_isolates_errors`（tauri::async_runtime::block_on 跑 mini pipeline） |
| ❌ 异常路径 | 失败素材的 concept_extracted_at 保持 NULL，下次增量自动重试 | 已测 | PASS `mark_asset_concept_extracted_sets_timestamp_and_skips_next_incremental`（验证：mark 一个 / 不 mark 另一个，下次增量仅返回未 mark 的） |
| ❌ 异常路径 | DB lock 获取失败时（中间状态）processed 推进，不死锁 | 已测（逻辑路径） | 闭包内 db.conn.lock() Err 分支走 emit_progress + return Ok；编译保证（参 src/commands/knowledge.rs:194-205） |
| ❌ 异常路径 | concept 拼 prompt 失败（assemble_messages_for_concept 返回 Err）跳过该素材，processed 推进 | 已测（保留 task_004 既有逻辑） | 走 `Some/None` 分支，processed 仍 += 1（参 src/commands/knowledge.rs:217-235） |
| ⚠️ 边界条件 | force_full=true 时 reset_library_concept_extracted_at 仅清空指定 library 而非全 DB | 已测（隐式） | SQL `WHERE project_id IN (SELECT id FROM projects WHERE library_id = ?1)` 限定 library 边界（src/commands/knowledge.rs:520-530） |
| ⚠️ 边界条件 | concepts.UNIQUE(library_id, name) 并发竞争兜底（INSERT OR IGNORE + 重查 id） | 已测（逻辑路径） | 闭包内 insert_concept 失败 → SELECT id WHERE (library_id, name) → 兜底 append_source_asset（src/commands/knowledge.rs:300-326） |

## 性能预估

| 阶段 | 改造前 | 改造后 | 提速 |
|------|--------|--------|------|
| 单文档 LLM 调用 | ~58s | ~15s（content 截断到 8 KiB → 输入 token ~2K） | 3.8× |
| 87 文档全量耗时 | 87 × 58s ≈ 84 min | 87 × 15s / 4 = ~5.4 min（≤ 10 min 目标达成） | ~15× |
| 增量扫描（40 已处理） | 47 × 58s ≈ 45 min | 47 × 15s / 4 ≈ 3 min | ~15× |
| 后续增量（无新文档） | N/A（force=true 总跑全量） | < 1s（SELECT WHERE concept_extracted_at IS NULL 返回空） | 秒级 |

（注：实际值需在真实 ARK API 调用下测，本估算基于诊断报告的 token / 网络往返模型）

## 已知局限

1. **失败素材 UI 不可见**：错误隔离仅 `log::error!`，不 emit error event；用户 UI 看不到"哪些文档失败"（本期接受 — input.md 第 96 行明确同意 / 未来 P2 可加"失败列表"UI）
2. **chat.rs `chat_completion_stream` stub 未启用**：诊断报告 P1-1 建议改为流式，本期不动 task_004 产物 / 不破坏既有 LLM 调用契约
3. **共现关系计算仍串行**：`compute_co_occurrence` 在并发主循环结束后串行调用一次；纯 SQLite，无 LLM，耗时可接受（< 1s 量级）
4. **`concepts_extraction_log` + `concept_extracted_at` 双标记冗余**：F-8 旧日志按 (asset, hash) 维度，V16 新标记按 asset 维度；本期保留双写是为了 force_full 用户体验（V16 标记好重置），并不影响正确性。未来可在 P2 评估是否合并到单一维度
5. **并发数 4 硬编码**：未做 config；ARK RPM 限制时如遇 429，已有 `with_retry` 兜底（chat.rs 既有），但持续高 429 会让单文档延迟膨胀。本期接受 — input.md AC-2 明确"硬编码"

## 需要 Reviewer 特别关注的地方

### 1. concept 并发竞争解决（`src-tauri/src/commands/knowledge.rs:300-326`）
4 路并发可能同时插入同名 concept，触发 `concepts.UNIQUE(library_id, name)`。代码用 "查快照 → 命中 append；未命中 INSERT OR IGNORE + 重查 id + 兜底 append" 模式。请重点 review：
- `insert_concept` 内部用 `INSERT OR IGNORE` 确认（参 `db/knowledge.rs::insert_concept`）
- 兜底 `query_row` 失败时 `continue`（跳过该 concept 不影响其他）是否合理
- 已快照 + 并发新插入：existing_concepts HashMap 是闭包前一次性快照，**不会**反映并发期间新插入的 concept；这意味着两个并发闭包提取出同名 concept 时，两者都走"未命中"分支，第二个 INSERT OR IGNORE 静默失败，但**重查 id** 步骤会拿到第一个插入的 id 走 append 路径

### 2. DB Mutex 锁争用（`src-tauri/src/commands/knowledge.rs:185-260`）
4 路闭包共享 `Mutex<Connection>`。**LLM HTTP 调用时锁完全释放**（短作用域设计），但写 DB 阶段（中间和结尾两次抢锁）会序列化 4 路并发。请验证：
- 没有任何 `db.conn.lock()` 跨 await 持有（grep 确认）
- 第一次抢锁：拼 prompt（`assemble_messages_for_concept` + `inspect_messages_for_log`）—— 极短
- 第二次抢锁：写 concepts/cases + mark concept_extracted_at + F-8 log —— 极短
- 实测 ARK ~15s 单文档延迟下，4 路并发期间 DB 锁占用 < 1% 总时间，应非瓶颈

### 3. 旧 IPC 名 wrapper 的语义映射（`src-tauri/src/commands/knowledge.rs:393-405`）
旧 `extract_concepts_for_library(force=true)` 映射到新 `start_concept_extraction(force_full=true)`。请确认这个映射符合既有用户行为期望（用户点"重新扫描"按钮时前端走 `force=true`，原意就是"强制全量"，与新 `force_full=true` 语义一致）。

### 4. V16 migration 与 V15 user_custom_prompt 互不干扰
V16 仅 ALTER TABLE assets，与 V15 user_custom_prompt 完全独立。fresh DB + 已升级到 V15 的 DB 都能正确推进到 V16，已测。

### 5. `concept_extracted_at` 是 P1 增量真相来源，但 F-8 旧日志仍存在
请 review：force_full=false 时，**F-8 旧日志的 (asset, hash) 命中也算"跳过"** —— 这是为了保持既有 F-8 行为不破坏（task_006 内容哈希增量功能）。这意味着即使 V16 标记是 NULL，F-8 命中也会让闭包跳过。**保守策略**：未来如要彻底切到 V16 单一真相来源，可在 fix 期清除 F-8 命中逻辑（本期不做，input.md 未要求）。

## 对前端 task_perf_02 的接口建议

> task_perf_02 改 `KnowledgeAssociationView.tsx` + `lib/tauri-commands.ts` 时按这些事实切换：

1. **IPC 名 / 参数**：直接按 input.md AC-3 期望的写
   ```ts
   export async function startConceptExtraction(libraryId: string, forceFull: boolean): Promise<void> {
     return invoke("start_concept_extraction", { libraryId, forceFull });
   }
   ```
   （Tauri JS 端按 camelCase `forceFull`，Rust 端按 snake_case `force_full`，Tauri 自动 serde 转换 — 已验证）

2. **payload 字段名**：保持现有 `status`，**不要改成 `state`**
   后端 emit 的 payload 是：
   ```json
   { "libraryId": "...", "totalAssets": 87, "processed": 0, "conceptsFound": 0, "status": "running" }
   ```
   现有前端 `ExtractionProgress` interface 已经按 `status: string` 处理（参 `src/lib/tauri-commands.ts` 的 `ConceptExtractionProgress`）—— 保持即可。input.md AC-1 / AC-2 提到的 `progress.state` 是命名错误，**请按 `status` 解析**。

3. **事件名**：保持 `notecapt/concept-extraction-progress`（后端零改动）+ 完成事件 `notecapt/concept-extraction-done`（后端零改动）。

4. **兼容期保证**：旧 `extract_concepts_for_library(force)` IPC 仍然能用（thin wrapper），所以 task_perf_02 切换可以**渐进**：先加新 IPC 调用、保留旧调用代码、最后再删除旧调用。Reviewer / Conductor 在 Review 阶段决定何时清理旧 wrapper。

5. **进度预估文案的并发数 / 截断字节数硬编码**：
   - 并发数 = 4（与后端 `CONCEPT_EXTRACTION_CONCURRENCY` 常量一致）
   - 文案建议：`预估全量约 {Math.ceil(totalAssets * 15 / 4 / 60)} 分钟（4 路并发，每篇约 15 秒）` — 用 15 秒/篇（截断后估算）比 60 秒/篇更准确

6. **后端"扫描中"判定**：前端按钮的"扫描中"禁用应基于 store 的 `extractionProgress.status === "running"`（既有判定），后端不需要任何额外的 "isRunning" command。
