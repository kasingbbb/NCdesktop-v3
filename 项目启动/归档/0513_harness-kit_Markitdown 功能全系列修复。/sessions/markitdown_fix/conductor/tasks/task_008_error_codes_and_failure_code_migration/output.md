# Task 交付 — task_008_error_codes_and_failure_code_migration

## 实现摘要

8 类 `FailureCode` 枚举 + `conversion_meta.failure_code` 列（V12 migration，幂等）+
`classify_output` 失效四元判定函数 + `update_failure_code` DAO + `markitdown.rs` 用
`classify_output` 替换历史 "exit==0 && stdout==''=成功" 误判 + image 空回退改为
`extractor_type="markitdown_image_fallback"` + 前端 9 项 i18n 文案表。

核心设计决策：
- 错误码独立成 `extraction/failure_code.rs`（不塞 models.rs），便于隔离 + 单测；
- 不实现 `serde::Serialize`，避免 derived 形态混入；DB/IPC 一律经 `as_str()` 显式落字符串；
- migration 幂等用 `PRAGMA table_info` 守卫（与 V5 同一模式），不依赖 `IF NOT EXISTS`
  对 ALTER TABLE 的支持（SQLite 不支持）；
- `update_failure_code` 按 `source_asset_id` 锚定最近一行 conversion_meta，不新增行
  （append-only 由 scheduler 单独写完整行）；
- `classify_output` 边界：`elapsed >= 90s` 用 `>=`（恰好 90s 已视为超时）；
  可打印占比阈值 `< 0.5` 严格小于（0.5 恰好不判 gibberish）；
  "段落"判定要求至少含一个 `alphanumeric` 字符（CJK 已被 Rust 的 `is_alphanumeric` 覆盖）。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|---------|------|
| `src-tauri/src/extraction/failure_code.rs` | 新建 | 8 错误码枚举 + `classify_output` + 14 单测 |
| `src-tauri/src/extraction/mod.rs` | 修改 | 注册 `pub mod failure_code` |
| `src-tauri/src/db/migration.rs` | 修改 | V12 migration：`failure_code` 列 + 索引 + 幂等守卫 + 3 测 |
| `src-tauri/src/db/conversion_meta.rs` | 修改 | `update_failure_code` 方法 + 3 单测 |
| `src-tauri/src/extraction/extractors/markitdown.rs` | 修改 | 用 `classify_output` 替换误判；image 空回退 `extractor_type="markitdown_image_fallback"` |
| `src/lib/extraction-failure-codes.ts` | 新建 | 前端 9 项 code→中文文案映射（8 + `legacy_unverified`） |

## AC 实测结果

| AC | 状态 | 证据 |
|----|------|------|
| AC-1：FailureCode 枚举 + as_str + Display | **PASS** | `failure_code.rs:32-67`；测试 `as_str_returns_screaming_snake_case` + `display_matches_as_str` 全过 |
| AC-2：V12 migration + 索引 + 幂等 | **PASS** | `migration.rs::v12_conversion_meta_failure_code`（PRAGMA table_info 守卫）；测试 `fresh_db_runs_all_migrations_to_v12` + `run_migrations_is_idempotent` + `v12_alter_is_idempotent_against_existing_column` 全过 |
| AC-3：update_failure_code（None 显式落 NULL） | **PASS** | `conversion_meta.rs::update_failure_code`；3 测全过（含 None→NULL、code→SCREAMING、0 行容忍） |
| AC-4：classify_output 每条分支有单测 | **PASS** | 12 个 classify_* 单测全过，含边界（90s 恰好、50% 占比恰好、None exit_code、纯 control char、U+FFFD 残留） |
| AC-5：markitdown.rs 替换误判 + image 回退新 extractor_type | **PASS** | `markitdown.rs:119-219`；空字符串不再返回 success；image+EOutputEmpty 走 `markitdown_image_fallback`；既有 10 个 markitdown 单测无回归 |
| AC-6：前端 i18n 9 条 key | **PASS** | `src/lib/extraction-failure-codes.ts` 含 8 错误码 + `legacy_unverified` 共 9 条 zh-CN 文案 |

## 测试命令 / 测试结果

```bash
cd NCdesktop/src-tauri && cargo check
# → Finished `dev` profile in 5.41s（仅 5 个 pre-existing 无关 warning）

cargo test --lib extraction::failure_code
# → running 14 tests
# → test result: ok. 14 passed; 0 failed; 0 ignored

cargo test --lib db::migration
# → running 4 tests
# → test result: ok. 4 passed; 0 failed; 0 ignored

cargo test --lib db::conversion_meta
# → running 7 tests
# → test result: ok. 7 passed; 0 failed; 0 ignored

cargo test --lib extraction::extractors::markitdown
# → running 10 tests
# → test result: ok. 10 passed; 0 failed; 0 ignored
```

**全 lib 测试**：`159 passed; 12 failed` —— 12 failed 全在 `db::knowledge::*` / `db::co_occurrence::*`，
panic message `"no such table: concepts"`，是 pre-existing 缺陷（concepts 表从未在任何 migration 中
建表，相关测试 `open_db` 用 `Database::open` 跑全 migration 也建不出 concepts）。task_008 未触动这两个文件。

## 单测覆盖一览（task_008 新增 24 测）

| 模块 | 测试 | 覆盖 AC |
|------|------|---------|
| `failure_code::tests::as_str_returns_screaming_snake_case` | 8 变体全字面校验 | AC-1 |
| `failure_code::tests::display_matches_as_str` | Display 复用 as_str | AC-1 |
| `failure_code::tests::classify_nonzero_exit_under_90s_is_runtime_missing` | 非 0 退出 + <90s | AC-4 (1) |
| `failure_code::tests::classify_nonzero_exit_at_90s_is_timeout` | 边界：恰好 90s | AC-4 (1) |
| `failure_code::tests::classify_none_exit_at_120s_is_timeout` | exit==None + 超阈值 | AC-4 (1) |
| `failure_code::tests::classify_exit0_empty_stdout_is_output_empty` | 空 stdout | AC-4 (2) |
| `failure_code::tests::classify_exit0_whitespace_only_is_output_empty` | 纯空白 trim 后为空 | AC-4 (2) |
| `failure_code::tests::classify_pure_control_chars_is_gibberish` | 全 control 字符 | AC-4 (3) |
| `failure_code::tests::classify_exactly_50_percent_printable_is_not_gibberish` | 边界：恰好 50% 不判 gibberish | AC-4 (3) |
| `failure_code::tests::classify_lossy_replacement_dominant_is_gibberish` | U+FFFD + control 残留 | AC-4 (3) |
| `failure_code::tests::classify_no_heading_no_paragraph_is_no_structure` | 纯标点无可读 | AC-4 (4) |
| `failure_code::tests::classify_heading_only_is_ok` | 仅标题 → Ok | AC-4 (5) |
| `failure_code::tests::classify_paragraph_only_is_ok` | 仅段落 (含中文) → Ok | AC-4 (5) |
| `failure_code::tests::classify_full_markdown_is_ok` | 完整 MD → Ok | AC-4 (5) |
| `migration::tests::fresh_db_runs_all_migrations_to_v12` | 新库跑到 V12 + failure_code 列 + 索引 | AC-2 |
| `migration::tests::run_migrations_is_idempotent` | 两次 run_migrations 幂等 | AC-2 |
| `migration::tests::v12_alter_is_idempotent_against_existing_column` | 直接二次调 V12 不报 duplicate column | AC-2 |
| `migration::tests::v11_repairs_user_version_10_missing_conversion_meta` | 兼容 V12 后版本=12 | AC-2 |
| `conversion_meta::tests::update_failure_code_writes_screaming_snake_and_clears_on_none` | success → NULL；fail → SCREAMING | AC-3 |
| `conversion_meta::tests::update_failure_code_targets_latest_row_only` | 多行只更新最新 | AC-3 |
| `conversion_meta::tests::update_failure_code_no_row_is_not_error` | 无 conversion_meta 行不视为错误 | AC-3 |

## markitdown.rs L120-160 替换关键 diff

**替换前**（误判核心）：
```rust
Ok(output) if output.status.success() => {
    let markdown = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if markdown.is_empty() {
        had_empty_success = true;
        attempts.push(format!("{python_cmd}: 输出为空"));
        continue;  // ← 历史：空字符串 + exit 0 静默放过，最坏情况上层判 success
    }
    // ... 成功路径
}
```

**替换后**（task_008 AC-5）：
```rust
Ok(output) => {
    let elapsed = start.elapsed();
    let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
    let exit_code = output.status.code();
    match classify_output(&stdout_str, exit_code, elapsed) {
        Ok(()) => { /* 真成功 */ }
        Err(code) => {
            last_failure_code = Some(code);
            if code == FailureCode::EOutputEmpty { had_empty_success = true; }
            // ... 拼装 attempts 附带 failure_code
        }
    }
}
```

image 空回退 extractor_type 由 `"markitdown"` 改为 `"markitdown_image_fallback"`（markitdown.rs:208）。

## 前端 i18n 9 条 key + value

| code | zh-CN 文案 |
|------|-----------|
| `E_RUNTIME_MISSING` | 内置转换运行时未就绪，请重启应用 |
| `E_EXTRA_MISSING_EPUB` | EPUB 解析组件缺失，无法读取该电子书 |
| `E_SCAN_PDF_UNSUPPORTED` | 扫描型 PDF 暂不支持，需先用 OCR 转为文本 |
| `E_AUDIO_WRONG_ROUTE` | 音频文件应由录音转写处理，不走文档转换 |
| `E_OUTPUT_EMPTY` | 文档内容为空或无法识别，未生成有效文本 |
| `E_OUTPUT_GIBBERISH` | 文档输出含大量乱码，无法用于知识库 |
| `E_OUTPUT_NO_STRUCTURE` | 文档已读出但无可识别的标题或段落 |
| `E_TIMEOUT_90S` | 文档处理超过 90 秒已自动终止，建议拆分后重试 |
| `legacy_unverified` | 旧版本记录未经新校验，重新转换以确认内容可用 |

## 对 Architect 方案的遵守声明
- [x] 目录结构：错误码独立 `extraction/failure_code.rs`（input.md 允许新建路径）
- [x] DB schema：列名 `failure_code TEXT NULL` + 索引 `idx_conversion_meta_failure_code` 与 ADR-007 字符级一致
- [x] 8 错误码字面与 ADR-007 字符级一致（SCREAMING_SNAKE_CASE）
- [x] 未引入新依赖（rusqlite 已在）
- [x] 未修改 `audio_asr_iflytek.rs`（PRD 底线 #4）
- [x] 未修改 task_004 scripts / task_000 desensitize / task_003 venv-shim 脚本
- 偏离说明：无

## 范围 gate
```
本次 task_008 真实增量文件（仅这 6 个）：
- src-tauri/src/extraction/failure_code.rs                 (新建 268 行)
- src-tauri/src/extraction/mod.rs                          (+1 行)
- src-tauri/src/extraction/extractors/markitdown.rs        (+165 -34 行)
- src-tauri/src/db/conversion_meta.rs                      (+120 行)
- src-tauri/src/db/migration.rs                            (+174 行)
- src/lib/extraction-failure-codes.ts                      (新建 80 行)
```
工作树其他 dirty 文件（audio_asr_iflytek.rs / scripts/* / Inspector.tsx 等）均为 pre-existing。

## 浏览器/运行时验证
N/A —— 本 task 是 Rust 库 + 前端文案常量表，无可启动 UI 路径。i18n 映射表已被前端 lib 风格沿用
（参考 `src/lib/ipc-errors.ts`），调用方需在后续 task 接入。

## 已知局限
1. `markitdown.rs` 中 `extract()` 不持有 conversion_meta.id，无法直接调用 `update_failure_code`；
   按设计该调用应在 `scheduler.rs` 落库 conversion_meta 行之后由 scheduler 触发。本 task 已
   提供 `update_failure_code` API + extract() Err 内嵌 `[E_XXX]` 摘要供 scheduler 解析，
   scheduler 接入留给后续 task（推测 task_007 / 改造 scheduler 时落地）。
2. classify_output 的 "non-UTF-8" 判定：因 stdout 已是 `&str`（调用方 `from_utf8_lossy` 转过），
   无法直接探测原始字节非 UTF-8；通过 U+FFFD 大量替换符 + control 残留间接落入 gibberish 分支
   （`classify_lossy_replacement_dominant_is_gibberish` 已覆盖）。
3. 12 个 pre-existing db::knowledge / db::co_occurrence 测试失败（`no such table: concepts`），
   非 task_008 引入，本 task 不修复。

## 需要 Reviewer 特别关注的地方
1. **classify_output 判定顺序**：是否符合 input.md AC-4 字面要求 —— 先 timeout、再 empty、
   再 gibberish、再 no_structure（`failure_code.rs:75-145`）。特别注意 `elapsed >= 90s` 用 `>=`
   而不是 `>`，使得"刚到 90s"被判 timeout（input.md 文字 "若 elapsed ≥ 90s" 与代码一致）。
2. **image fallback 落点**：`extractor_type="markitdown_image_fallback"` 字段值 + 不写 failure_code
   的语义是否与 task_011 保留矩阵 PRD 期望一致（markitdown.rs:213）。
3. **`update_failure_code` 锚定策略**：按 `source_asset_id` 取最近一行 vs 调用方传 conversion_meta.id。
   选择前者是因为内层 extract 不持有 id，方便上层快速接入；若 Reviewer 认为应传 id 显式约束，
   可在 scheduler 接入时改为带 id 的重载。
4. **V12 migration 幂等**：用 PRAGMA table_info 守卫 ALTER TABLE，没有走 try-catch
   "duplicate column" 异常路径。两次直接调 `v12_conversion_meta_failure_code` 已被单测覆盖。
