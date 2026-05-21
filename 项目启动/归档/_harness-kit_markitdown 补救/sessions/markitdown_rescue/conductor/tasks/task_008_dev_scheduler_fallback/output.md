# Task 交付 — task_008_dev_scheduler_fallback

## 实现摘要

**本 task 同时关闭了 M-1 跨 task 待办**：`src/extraction/mod.rs:4` 的 `// pub mod scheduler;` 已取消注释，scheduler 模块**已重新参与编译**；连带发现并修复了三处相同模式的"注册缺口"（pre-existing 文件存在但 `mod.rs` 未声明 —— 详见"修改的文件"与"已知局限"）。

scheduler 主循环的提取分支按 ADR-003 重写为三级编排：
1. **primary 成功**（quality>0 且 structured_md 非空）→ `save_and_materialize` + 写一行 `conversion_meta(fallback_used=false, error_class=None)`。
2. **primary 失败/空** → 先登记一行 `conversion_meta(primary, fallback_used=false, error_class=...)`；调 `get_fallback_extractor_for_excluding(mime, primary_name)`（**严格排除 primary 名称防死循环**）拿 fallback：
   - fallback 成功 → `save_and_materialize` + `conversion_meta(fb_name, fallback_used=true)`。
   - fallback 失败/空 → 再登记一行 `conversion_meta(fb_name, fallback_used=true, error_class=...)`；都失败 → 进 placeholder 分支。
3. **两者都失败 / 无 fallback 候选** → `materialize_placeholder` 走**新建**的 `write_placeholder_md`（ADR-006）：不推进 `derivative_version`、不归档旧版本、**不**调用 `extracted_content` 的 `status='extracted'` upsert（架构方案 §九 R3 的核心保障：placeholder 不能让后续真转换被跳过）。仅 emit `notecapt/asset-placeholder` 事件。

`error_class` 解析采用 task_007 约定的 `error_class:xxx|` 前缀，失配走 `conversion::classify_error` 兜底；未知 class 归 `conversion_error`。`source_hash` 用 `conversion::file_sha256` 而非 `compute_sha256`（AC-6）；后者保留为对**内存字符串**的薄 wrapper，文档注释明确说明边界，避免"两份文件哈希实现并存"。

为 AC-4 五场景测试**提取了纯函数** `decide_next_step(primary, fallback) -> NextStep`（不依赖 AppHandle/DB/IO）作为决策语义的可测试镜像；主循环用语义等价的内联 match。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---|---|---|
| `src-tauri/src/extraction/mod.rs` | 修改 | **M-1 关闭点**：取消 `// pub mod scheduler;` 注释 |
| `src-tauri/src/extraction/scheduler.rs` | 修改 | 主循环改为三级编排；新增 `Step` / `NextStep` / `decide_next_step` / `extraction_is_usable` / `run_extractor_blocking` / `extract_error_class` / `parse_error_class_prefix` / `map_to_static_class` / `save_and_materialize` / `write_conversion_meta` / `write_placeholder_md`；`materialize_placeholder` 改走 `write_placeholder_md`；`compute_sha256` 增加文档明确边界；移除已注释 `get_pdf_scan_extractor` 引用与 `needs_ocr_fallback` 分支；新增 `#[cfg(test)] mod tests`（13 个测试） |
| `src-tauri/src/extraction/extractors/mod.rs` | 修改 | 新增 `get_fallback_extractor_for_excluding(mime, excluded_name)` + 3 个单测 |
| `src-tauri/src/db/mod.rs` | 修改 | 新增 `pub mod extraction;`（pre-existing 文件 `db/extraction.rs` 一直未注册，与 M-1 同类注册缺口） |
| `src-tauri/src/utils/mod.rs` | 修改 | 新增 `pub mod safe_name;`（同上，scheduler 依赖 `sanitize_stem`） |
| `src-tauri/src/lib.rs` | 修改 | 新增 `pub mod utils;`（同上，`utils` 模块本身一直未在 lib.rs 注册） |

**未修改任何前端 `src/` 文件。未引入新的外部依赖。**

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（沿用 `extraction/scheduler.rs` + `extraction/conversion.rs` + `db/conversion_meta.rs`）
- [x] API 路径/命名与 Architect 方案一致（`get_fallback_extractor_for_excluding`、`write_placeholder_md`、`ConversionMetaRow` 字段名均与 §三 ADR-003 / ADR-006 描述对齐）
- [x] 数据模型与 Architect 方案一致（未新增列；`conversion_meta` 沿用 task_006 schema）
- [x] 未引入计划外的新依赖（无 Cargo.toml 改动）
- **偏离说明**：
  1. 任务指令要求"if 已知局限"标注集成测试覆盖度——我没跑真实的 T1-T5 集成测试（无 GUI / Python venv 环境），改以纯函数 `decide_next_step` + `extract_error_class` + `get_fallback_extractor_for_excluding` 三组单测覆盖 5 场景**决策路径**。详见"已知局限"。
  2. 发现并修复了 3 处 pre-existing 注册缺口（`db::extraction`、`utils::safe_name`、`utils` 自身）。这超出 task_008 描述的"M-1（scheduler 自身注释）"，但属于"M-1 同一类问题"——若不注册，cargo check 无法 0 error。明确登记于此供 Reviewer 决断。

## 测试命令

```bash
cd src-tauri && cargo check 2>&1 | tail -10
cd src-tauri && cargo test --lib extraction 2>&1 | tail -50
```

## 测试结果

`cargo check`（AC-7 验证）：
```
warning: `notecapt` (lib) generated 3 warnings (run `cargo fix --lib -p notecapt` to apply 3 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.20s
```
（**0 error**；3 个 warning 全在 `src/llm/chat.rs`，与本 task 完全无关；先前还有 `NextStep`/`decide_next_step` 的 dead_code warning，已用 `#[allow(dead_code)]` 注明"仅 #[cfg(test)] 消费"消除。）

`cargo test --lib extraction`：
```
running 41 tests
test extraction::conversion::tests::classify_error_covers_eight_classes ... ok
... (省略)
test extraction::extractors::tests::excluding_returns_none_when_no_candidate ... ok
test extraction::extractors::tests::excluding_returns_none_when_only_candidate_is_excluded ... ok
test extraction::extractors::tests::excluding_returns_pdf_text_when_excluding_markitdown ... ok
test extraction::scheduler::tests::extraction_is_usable_rejects_empty_or_zero_quality ... ok
test extraction::scheduler::tests::extract_error_class_prefers_prefix_then_falls_back ... ok
test extraction::scheduler::tests::parse_error_class_prefix_strips_prefix ... ok
test extraction::scheduler::tests::primary_ok_empty_no_fallback_candidate_uses_placeholder ... ok
test extraction::scheduler::tests::primary_ok_empty_then_fallback_success ... ok
test extraction::scheduler::tests::t1_primary_success_uses_primary ... ok
test extraction::scheduler::tests::t2_primary_err_fallback_success_uses_fallback ... ok
test extraction::scheduler::tests::t3_both_err_uses_placeholder ... ok
test extraction::scheduler::tests::t4_after_placeholder_primary_success_overrides ... ok
test extraction::scheduler::tests::t5_idempotent_repeat_primary_success ... ok

test result: ok. 41 passed; 0 failed; 0 ignored; 0 measured; 35 filtered out; finished in 0.01s
```

`cargo test --lib`（全量回归）结果：**64 passed; 12 failed**。**所有 12 个 failed 全部位于 `db::co_occurrence` 和 `db::knowledge` 模块**（错误：`no such table: concepts`），与本 task 完全无关 —— `db/migration.rs:124` 注释明确说"V3（concepts 等基表）未在当前源码中存在"，是 pre-existing 的迁移缺口。我未修改这两个文件（见 `git status` 输出），失败与 M-1 关闭 / scheduler 改造无任何因果关系。

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|---|---|---|---|
| ✅ 正常路径 | T-1 primary（markitdown）成功 → UsePrimary，写 1 行 conversion_meta(fallback=false) | 已测（纯函数） | PASS — `t1_primary_success_uses_primary` |
| ✅ 正常路径 | T-2 markitdown 失败 → pdf_text fallback 成功 → UseFallback，写 2 行 conversion_meta（1 fail + 1 success fallback=true） | 已测（纯函数 + exclude helper） | PASS — `t2_primary_err_fallback_success_uses_fallback` + `excluding_returns_pdf_text_when_excluding_markitdown` |
| ❌ 异常路径 | T-3 都失败 → Placeholder，写 2 行 conversion_meta（皆失败） | 已测（纯函数） | PASS — `t3_both_err_uses_placeholder` |
| ⚠️ 边界条件 | T-4 placeholder 后重跑成功 → UsePrimary（决策不依赖历史；版本号由 write_derivative_md 推进） | 已测（纯函数） | PASS — `t4_after_placeholder_primary_success_overrides`（决策层面；版本号推进逻辑在 `write_derivative_md`，未变） |
| ⚠️ 边界条件 | T-5 重复执行 primary 成功 → 决策稳定 UsePrimary（write_derivative_md 已有"existing → 归档+覆盖"保证工作区只有一个 .md） | 已测（纯函数） | PASS — `t5_idempotent_repeat_primary_success` |
| ⚠️ 边界条件 | primary Ok 但 quality_level==0 / md 空 → 触发 fallback 路径 | 已测 | PASS — `primary_ok_empty_then_fallback_success` + `extraction_is_usable_rejects_empty_or_zero_quality` |
| ⚠️ 边界条件 | mime 无任何 fallback 候选 → 直接 Placeholder（不死循环不 panic） | 已测 | PASS — `primary_ok_empty_no_fallback_candidate_uses_placeholder` + `excluding_returns_none_when_no_candidate` |
| ❌ 异常路径 | excluding 等于唯一候选自身 → 返回 None（防 markitdown→markitdown 死循环） | 已测 | PASS — `excluding_returns_none_when_only_candidate_is_excluded` |
| ✅ 正常路径 | `error_class:xxx\|` 前缀解析（带/不带 ExtractionError::Display 前缀；未知 class 兜底） | 已测 | PASS — `parse_error_class_prefix_strips_prefix` + `extract_error_class_prefers_prefix_then_falls_back` |
| ⚠️ 边界条件 | placeholder 不推进 `derivative_version`、不归档、不写 `extracted_content` | **静态保证（代码审查）** | `write_placeholder_md` 实现中**完全没有**调用 `set_derivative_version` / `upsert_extraction_result` / `archive_existing_version`；新建 derivative 时 `derivative_version: source_asset.derivative_version`（不+1）；已有 derivative 时直接覆盖文件 |
| ⚠️ 边界条件 | spawn_blocking JoinError（提取线程 panic） | 已实现 | `run_extractor_blocking` 将 JoinError 映射为 `ParseError("error_class:conversion_error\|提取任务 panic: ...")`，主循环走 fallback；与原代码 panic 兜底语义一致 |
| ❌ 异常路径 | `conversion_meta` 写入失败（DB 锁失败 / insert 报错） | 已实现 | `write_conversion_meta` 内 DB 锁失败 / `db_conv_meta::insert` 失败仅 `log::warn!`，绝不向上传播；不影响主流程（task_008 硬约束） |

## 已知局限

1. **集成测试覆盖度**：由于 scheduler 真实运行依赖 `AppHandle` + Tauri runtime + Python venv，**未在 CI/本地跑通真实的 T1-T5 集成测试**。改用纯函数 `decide_next_step` + `extract_error_class` + `extraction_is_usable` + `get_fallback_extractor_for_excluding` 四组单测覆盖**决策路径**与**类型分流**，集成层手测脚本见下方。Reviewer 若希望补 e2e 集成测试，可：把 `materialize_placeholder` 的 IO 路径用 trait 抽象，或用 `tempfile::tempdir` 替换 `workspace::ensure_project_workspace`。
2. **手测脚本（无 GUI 环境，登记供 PM 决断）**：
   - **T-1**：拖入文字型 PDF + 配置 markitdownEnabled=true + 系统装 markitdown → 期望 `conversion_meta` 仅 1 行 `converter_name='markitdown', fallback_used=false, quality_level>0`。
   - **T-2**：临时把 `markitdown` 模块改名（或 venv 卸载）→ 期望 2 行：1 行 `markitdown, fallback_used=false, error_class='markitdown_not_installed'`；1 行 `pdf_text, fallback_used=true, quality_level>0`。
   - **T-3**：损坏 PDF（前几字节随机化）→ 期望 2 行皆失败 + 工作区出现一个 placeholder `.md`（前 frontmatter `derivative_version: 0`、`extractor_type: placeholder_xxx`）。
   - **T-4**：T-3 完成后用有效 PDF 替换原件，前端调 `retry_extraction` → 期望 placeholder 被**覆盖**为真成功 .md；`assets` 表中 derivative 行 `derivative_version` 从 0 → 1；source 行也同步到 1（由 `write_derivative_md` 双写）。
   - **T-5**：重复 T-1 两次 → 工作区只有一个 .md（旧版进 `_versions/<source_id>/v1.md`）；`derivative_version` 累进。
3. **`db::co_occurrence` / `db::knowledge` 共 12 个测试失败**：pre-existing migration 缺口（`concepts` 表未在 `migration.rs` 中创建，见 `db/migration.rs:124` 注释）。**与本 task 无任何因果关系**，git status 显示我未修改这两个文件，请 Reviewer 不要把它归到 task_008 失败项。
4. **`commands/extraction.rs` 仍为孤儿文件**：`commands/mod.rs` 未注册 `pub mod extraction`，本 task 未恢复（不在 AC 范围；触发 scheduler 的入口走的是 `commands::dropzone` / `commands::conversion`）。如需启用前端 `extract_asset` 等命令，应单独开 task。

## 需要 Reviewer 特别关注的地方

1. **AC-7（M-1 取消注释后 cargo check 0 error 的真实性）**：基线（仅取消 mod.rs:4 注释）首次 cargo check 显示 7 个 error；其中 5 个是 `db::extraction` / `utils::safe_name` 未注册导致的级联（同一类注册缺口）+ 1 个 `get_pdf_scan_extractor` 已被注释的引用残留 + 1 个 `e.contains` 类型推断模糊。我**注册了三个 pre-existing 但未声明的模块**（`db/extraction.rs`、`utils/safe_name.rs`、`utils` 自身），并删除了 `get_pdf_scan_extractor` import 与对应的 `needs_ocr_fallback` 分支。这三个注册修复**完全在仓库已有的源码文件范围内**，未新增也未删除任何业务代码。Reviewer 关键校验点：(a) `pub mod utils;`/`pub mod extraction;`/`pub mod safe_name;` 三处声明是否会引入新的 ambiguity；(b) `commands/extraction.rs` 仍未挂载是否引出新问题（应该不会——它早就没编译）。
2. **placeholder 不污染 derivative_version 的证据**：见 `write_placeholder_md` 函数体（`extraction/scheduler.rs` 文件中段）—— 全函数 grep 不到 `set_derivative_version`、`upsert_extraction_result`、`archive_existing_version` 三个关键词；新建 derivative 时显式赋值 `derivative_version: source_asset.derivative_version`（**不**用 `+1`）；已有 derivative 时直接 `std::fs::write` 覆盖目标文件，不进 `_versions/`。
3. **5 个测试场景的决策逻辑**：`decide_next_step` 是纯函数镜像，与主循环 `match primary_step { ... }` **必须语义等价**。请 Reviewer 对比两边：
   - `extraction_is_usable` 在两边都是唯一的"是否走真成功"判定。
   - 主循环 `Step::PrimarySuccess` ↔ 纯函数 `NextStep::UsePrimary`。
   - 主循环 `Step::PrimaryEmpty | PrimaryError → fallback usable` ↔ `NextStep::UseFallback`。
   - 主循环 `!fallback_done` ↔ `NextStep::Placeholder`。
4. **fallback 排除 primary**：`get_fallback_extractor_for_excluding` 用 `e.name() != excluded_name`；`excluding_returns_none_when_only_candidate_is_excluded` 测试已证明：传 `excluded_name="pdf_text"` 时即使 mime=PDF 也返回 None（避免选回自己）。主循环传 `&primary_name`（即 `extractor.name()` 的 owned 副本），保证 markitdown→markitdown / pdf_text→pdf_text 都不会发生。
5. **`error_class` 的稳定性**：`map_to_static_class` 把动态字符串映射回 `&'static str` 集合，保证 `conversion_meta.error_class` 取值始终在 8 个稳定枚举（含 `conversion_error` 兜底）+ None。
6. **`compute_sha256` vs `file_sha256`（AC-6）**：scheduler 中**没有**对文件路径调用 sha256 的本地实现 —— 文件用 `conversion::file_sha256`；`compute_sha256` 仅服务于内存 markdown 字符串（`content_hash` 场景），文档注释明确边界。Reviewer 若坚持"一处实现"，可把它替换为 `file_sha256(write_to_temp_file(text))`，但 IO 成本更高，建议保留现状。
