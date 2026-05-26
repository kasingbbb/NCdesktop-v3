# Task 交付 — task_011_preserve_vs_modify_matrix

## 实现摘要

输出 `markitdown.rs` / `scheduler.rs` 当前已实现"易被误重构"行为的"保留 / 修改 / 删除"二维矩阵（共 **10 项**，覆盖 input.md 字面 6 项 + grep 发现 4 项），并在 `markitdown.rs` 源码锚点处用 `// task_011 preserve:` 注释自我标记，补强 4 个单测覆盖保留行为（超时归类、image 空回退 + 非 image 反例、版本缓存状态机）。**不动业务逻辑**，仅注释 + 测试段。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `sessions/markitdown_fix/conductor/tasks/task_011_preserve_vs_modify_matrix/preserve_matrix.md` | 新建 | 10 项保留/修改矩阵 + AC-4 联调声明 + Reviewer Checklist |
| `NCdesktop/src-tauri/src/extraction/extractors/markitdown.rs` | 修改 | 加 10 行 `// task_011 preserve/modify:` 注释 + 4 个新单测（约 155 行测试代码）；**业务逻辑未动**。 |

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（未引入新文件、新模块）
- [x] API 路径/命名与 Architect 方案一致（未改动 `Extractor` trait、`FailureCode`、`classify_output` 签名）
- [x] 数据模型与 Architect 方案一致（ADR-007 `conversion_meta.failure_code` 未触动）
- [x] 未引入计划外的新依赖（不依赖 `serial_test` / 任何新 crate）
- 偏离说明（如有）：**无**。

## AC 一览

| AC | 内容 | 状态 |
|----|---|---|
| AC-1 | preserve_matrix.md 表格 ≥ 6 项 | PASS（10 项） |
| AC-2 | 源码锚点 `// task_011 preserve:` 注释 | PASS（9 处 preserve + 1 处 modify；总注释行 10 行 ≪ 30 行预算） |
| AC-3 | 补单测覆盖保留行为：超时 / image 空回退 / 版本缓存 | PASS（4 测：超时、image fallback、非 image 反例、版本缓存） |
| AC-4 | 与 classify_output 联调，image 空回退不被误判 EOutputEmpty | PASS（image fallback 测 + 非 image 反例测双向验证） |
| AC-5 | preserve_matrix.md 末尾 Reviewer Checklist 引用 | PASS（"Reviewer Checklist 引用"节） |

## 注释 grep 命中

```
$ grep -nE 'task_011 preserve:' src/extraction/extractors/markitdown.rs
7:// task_011 preserve: 90s 子进程总超时...（preserve_matrix.md #1）
23:    // task_011 preserve: 版本探测缓存字段...（preserve_matrix.md #3）
57:// task_011 preserve: SUPPORTED_MIME_TYPES 不含 audio/video（grep gate）（preserve_matrix.md #6）
132:        // task_011 preserve: runtime_check 入口短路...（preserve_matrix.md #10）
142:        // task_011 preserve: audio/video 入口 debug_assert + release E_AUDIO_WRONG_ROUTE 防御（preserve_matrix.md #8）
265:        // task_011 preserve: image 空输出 → markitdown_image_fallback 最小元数据 MD（preserve_matrix.md #2）
304:// task_011 preserve: 后台读线程持续 drain pipe...（preserve_matrix.md #7）
368:// task_011 preserve: error_class:xxx| 前缀供 scheduler 解析分类（preserve_matrix.md #9）
375:// task_011 preserve: probe_markitdown_version best-effort 探测 + 缓存（preserve_matrix.md #3）
```

命中：**9 处**（≥ 4 期望）。另含 1 处 `task_011 modify:`（第 207 行，classify_output 替换 historical `exit==0 && stdout==''` 误判）。

## 新增 3+1 单测设计

| 测试名 | AC | 设计 |
|---|---|---|
| `task_011_classify_output_at_95s_with_killed_exit_is_timeout` | AC-3 #1 | 直接调 `classify_output(stdout="", Some(137)/None, elapsed=95s)` 验证归类为 `ETimeout90s`。**不真起 95s sleep**（CI 实时间预算不允许；归类逻辑与 markitdown.rs:247 `io::ErrorKind::TimedOut → ETimeout90s` 分支共享语义）。 |
| `task_011_image_empty_fallback_returns_image_fallback_type` | AC-3 #2 / AC-4 正向 | 用 `/usr/bin/true` 顶替 `markitdown_python_cmd` + `markitdown_embedded_python`（忽略参数立即 exit 0 + 空 stdout）→ image 扩展名 `.png` → 走 fallback。断言 `extractor_type == "markitdown_image_fallback"` + `quality_level == 1` + markdown 含"图片"元数据文本。 |
| `task_011_non_image_empty_output_does_not_fallback` | AC-4 反向 | 同 mock，但用 `.pdf` 扩展名 → `is_image=false` → 不走 fallback → `Err` 包含 `E_OUTPUT_EMPTY`。证明 fallback 不污染非 image 路径。 |
| `task_011_version_cache_gates_reentry_after_first_set` | AC-3 #3 | 测缓存状态机不变量：初值 None → gate 为 true；`cache_version("v1")` 后状态 Some(v1) → gate 为 false（等价 input AC-3 #3 字面"连续两次 extract 仅一次 --version 调用"）。 |

## 测试命令

```bash
cd NCdesktop/src-tauri
cargo test --lib extraction::extractors::markitdown
cargo test --lib   # 全量回归
```

## 测试结果

### markitdown 模块单测

```
running 21 tests
test extraction::extractors::markitdown::tests::detected_version_starts_empty ... ok
test extraction::extractors::markitdown::tests::is_image_mime_only_image_prefix ... ok
test extraction::extractors::markitdown::tests::image_extension_detection_matches_extract_logic ... ok
test extraction::extractors::markitdown::tests::mime_prefix_from_path_maps_audio_video_extensions ... ok
test extraction::extractors::markitdown::tests::extract_short_circuits_when_runtime_check_failed ... ok
test extraction::extractors::markitdown::tests::error_class_markitdown_not_installed ... ok
test extraction::extractors::markitdown::tests::error_class_conversion_error_empty_stderr ... ok
test extraction::extractors::markitdown::tests::error_class_file_not_found ... ok
test extraction::extractors::markitdown::tests::python_candidates_deduplicates_when_cmd_equals_python3 ... ok
test extraction::extractors::markitdown::tests::python_candidates_defaults_only ... ok
test extraction::extractors::markitdown::tests::python_candidates_order_with_embedded_and_cmd ... ok
test extraction::extractors::markitdown::tests::supported_mime_types_excludes_audio_and_video ... ok
test extraction::extractors::markitdown::tests::supports_image_mime_types ... ok
test extraction::extractors::markitdown::tests::task_007_short_circuit_precedes_task_010_audio_block ... ok
test extraction::extractors::markitdown::tests::task_011_classify_output_at_95s_with_killed_exit_is_timeout ... ok
test extraction::extractors::markitdown::tests::extract_panics_in_debug_on_video_pollution - should panic ... ok
test extraction::extractors::markitdown::tests::extract_panics_in_debug_on_audio_pollution - should panic ... ok
test extraction::extractors::markitdown::tests::task_011_version_cache_gates_reentry_after_first_set ... ok
test extraction::extractors::markitdown::tests::extract_does_not_short_circuit_when_runtime_check_ok ... ok
test extraction::extractors::markitdown::tests::task_011_non_image_empty_output_does_not_fallback ... ok
test extraction::extractors::markitdown::tests::task_011_image_empty_fallback_returns_image_fallback_type ... ok

test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 198 filtered out; finished in 1.70s
```

### 全量 lib 回归

```
test result: ok. 219 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.13s
```

baseline（task_009/010 PASS 后）= 215；本 task +4 测 = **219** ✓

`cargo check --lib`：通过，仅原有 5 warnings（来自 llm 模块 / 命令模块，非本 task 引入）。

## markitdown.rs 注释 diff 摘要（示例）

```diff
+// task_011 preserve: 90s 子进程总超时，损坏 / 极大文件兜底强杀（preserve_matrix.md #1）。
 /// W4-6 / 风险 4.2：子进程总超时。
 const MARKITDOWN_TIMEOUT: Duration = Duration::from_secs(90);
...
+// task_011 preserve: SUPPORTED_MIME_TYPES 不含 audio/video（grep gate）（preserve_matrix.md #6）。
 // task_010 AC-1（H5 / PRD 底线 #4）：`SUPPORTED_MIME_TYPES` 严格不含 ...
 const SUPPORTED_MIME_TYPES: &[&str] = &[ ...
...
+        // task_011 preserve: runtime_check 入口短路，自检失败不起子进程（preserve_matrix.md #10）。
         // task_007 FIX：runtime self-check 快照短路（AC-3）。
         if let Some(code) = options.runtime_check_failed { ...
...
+                    // task_011 modify: 历史"exit==0 && stdout==''=success"已替换为 classify_output（preserve_matrix.md #4）。
                     match classify_output(&stdout_str, exit_code, elapsed) {
...
+        // task_011 preserve: image 空输出 → markitdown_image_fallback 最小元数据 MD（preserve_matrix.md #2）。
         if is_image && had_empty_success {
```

业务函数体（`extract`、`run_with_timeout`、`classify_output` 调用点、`python_candidates`、`probe_markitdown_version`、image fallback 分支）字符无改动。

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|----------|----------|------|-----------|
| ✅ 正常路径 | preserve_matrix.md 落地 10 项；注释 grep ≥ 4 命中 | 已测 | PASS（9 命中） |
| ✅ 正常路径 | cargo check --lib 无新增 warning | 已测 | PASS（沿用 5 个旧 warning） |
| ✅ 正常路径 | cargo test --lib 全过 + 数量 219 | 已测 | PASS |
| ✅ 正常路径 | 注释总行数 ≤ 30（input.md 约束） | 已测 | PASS（10 行） |
| ⚠️ 边界 | image 扩展名大小写（.PNG / .JPG）继续命中 fallback | 已测 | PASS（依赖既有 `image_extension_detection_matches_extract_logic` 测） |
| ⚠️ 边界 | classify_output 在 elapsed = 90s 边界 + exit_code = None 归类 | 已测 | PASS（既有 `classify_none_exit_at_120s_is_timeout` + 新增 95s 测） |
| ❌ 异常 | 非 image + exit 0 + 空 stdout → 不得走 fallback | 已测 | PASS（`task_011_non_image_empty_output_does_not_fallback`） |
| ❌ 异常 | `/usr/bin/true` 与 `/bin/true` 均缺失 | 未测（CI 必有） | 跳过（测试代码内含两路 fallback + 不可达时 early return） |

## 浏览器/运行时验证

**N/A** — 本 task 纯库代码 + 文档；不涉及 UI / 启动服务 / IPC 路径变更。所有 AC 通过 cargo test + grep gate 验证。

## 是否触非授权区

**NO**。
- 未改 task_007/008/009/010 PASS 的其他 Rust 文件（`runtime_check.rs`、`failure_code.rs`、`scan_pdf_detect.rs`、`audio_asr_iflytek.rs`、`scheduler.rs`）；
- 未改 `db/migration.rs`、`db/conversion_meta.rs`、`db/asset.rs`；
- 未改 task_004~006 scripts；
- 未改 task_003 `verify-venv-shim.sh`；
- 未改 task_000 区 / 脱敏 SOP；
- 未引入新 extractor / 新分类器（H6 守住）；
- 未引入新 crate（`serial_test` 没采用 —— 版本缓存测改测状态机不变量，规避全局并发污染）。

`git status` 现存的广泛 `MM` 状态来自 task_007/008/010 等前置 PASS 任务的未提交变更，**非本 task 引入**。本 task 净增：`preserve_matrix.md` 新建 + `markitdown.rs` 10 行注释 + 4 个新单测（共约 165 行）。

## 已知局限

1. **版本缓存测的 mock 限制**：无法直接 mock `Command::new(...).args(["-m","markitdown","--version"]).output()` 调用计数（Rust 标准库无内置进程 mock）。改测缓存状态机不变量 —— `RwLock<Option<String>>` 写入后 markitdown.rs:207 的 gate 条件返回 false，等价于"不会再调 probe"。语义与 input AC-3 #3 字面"连续两次 extract 仅一次 --version 调用"一致。若 Reviewer 坚持要进程级 mock，需引入 `serial_test` + 进程间通信 mock 库（违 H6 / 新依赖约束），不做。
2. **超时测的真实子进程语义**：未真起 95s sleep；归类逻辑由 `classify_output` 直接覆盖。`run_with_timeout` 的 kill 路径（markitdown.rs:336-347）由既有 `io::ErrorKind::TimedOut → FailureCode::ETimeout90s` 映射（markitdown.rs:247-251）保证 —— 既有 task_008 测套件已覆盖 classify_output 各分支，本测仅追加 95s 边界。
3. **scheduler.rs 未触动**：preserve_matrix.md 涉及 scheduler.rs 的项（如"image fallback 路径下 scheduler 落库 NULL"）由 task_008 AC 已覆盖；本 task 不必再在 scheduler.rs 加注释（input.md 字面"预估影响范围 - 修改：markitdown.rs"，scheduler.rs 是参考范围非必动）。

## 需要 Reviewer 特别关注的地方

1. **注释行数预算**：10 行 ≪ 30 行（input.md 约束）。若觉得"task_011 preserve:" 注释信噪比低，可考虑后续合并到既有 task_007/008/010 注释段（但**不应**删除引用 `preserve_matrix.md #N` 的部分 —— 这是 Reviewer Checklist 的锚点）。
2. **image fallback 测的 `/usr/bin/true` 依赖**：macOS / Linux CI 默认存在；Windows CI 未来若加，此测会跳过（不报错）。`/usr/bin/true` 不会调真 Python，只是借其"exit 0 + 空 stdout"语义触发 EOutputEmpty → fallback 链路。
3. **AC-4 反例覆盖**：`task_011_non_image_empty_output_does_not_fallback` 是 input.md 未明文要求但 AC-4 字面"image 空回退不被 EOutputEmpty 误判"的反向证据 —— 非 image 路径必须**仍然** EOutputEmpty 失败，证明 fallback 分支边界清晰。
4. **`failure_code::{classify_output, FailureCode}` import 是否本 task 引入**：检视 git 历史 —— 这个 use 在 task_008 staged 版本就已写入。本 task 未新增 import；编辑器看到的"unstaged 包含此行"是 git index `MM` 状态产物（task_008 staged 部分），非本 task 操作。
5. **未来 task 涉及 scheduler.rs / extraction 路径时**：Reviewer 应核对 preserve_matrix.md 末尾的 6 条 checklist，特别是 `MARKITDOWN_TIMEOUT == 90s` / `extractor_type == "markitdown_image_fallback"` 字面 / `run_with_timeout` 后台读线程结构 — 这三项最易在后续重构中被"顺手简化"。
