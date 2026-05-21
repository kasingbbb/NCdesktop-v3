# Task 交付 — task_007_dev_markitdown_enrich

## 实现摘要

按 input.md AC-1/2/3/4 增强 `MarkItDownExtractor`：

1. **候选顺序（AC-3）**：`python_candidates(options)` 调整为 ① `markitdown_embedded_python` ② `markitdown_python_cmd` ③ `python3` ④ `python`。使用闭包 `push_unique` 统一去重；用户传入的任意候选若与默认 `python3`/`python` 重复则不会重复加入。`ExtractOptions` 新增 `markitdown_embedded_python: Option<String>` 字段，`#[derive(Default)]` 自动保证默认 `None`。

2. **error_class 归类（AC-2，C 节方案一：前缀写入字符串）**：所有失败路径经 `parse_error_with_class(msg) -> ExtractionError::ParseError` 统一处理，返回字符串形如 `"error_class:xxx|MarkItDown 调用失败：..."`。task_008 scheduler 可用简单 `strip_prefix("error_class:") + find('|')` 解析。选用前缀方案的原因：无状态、单元测试不需要持有 extractor 实例、天然 `&self` 友好（避免 `RefCell<Option<&'static str>>` 带来的 `Send + Sync` 阻塞）。

3. **版本缓存（AC-1）**：`MarkItDownExtractor` 携带 `cached_version: RwLock<Option<String>>`。首次 `extract` 成功路径上以 best-effort 方式调用 `python -m markitdown --version`（沿用同一已知可用的 python_cmd），把 stdout trim 后缓存。新增 inherent 方法 `pub fn detected_version(&self) -> Option<String>`，task_008 在 scheduler 落库 `ConversionAttempt` 时调用读取；`Extractor` trait **未**被修改。

4. **stderr 处理**：`Command::args(...)` 数组传参；非空 stderr 仅 `log::warn!`，原文不外传 UI；ParseError 字符串中的 stderr 摘要供 scheduler 落库 `last_error`。

5. **`extractors/mod.rs`** 因 `MarkItDownExtractor` 从 unit struct 变为带字段结构，构造点 `Box::new(markitdown::MarkItDownExtractor)` → `Box::new(markitdown::MarkItDownExtractor::new())`。这是被结构变更强制的最小适配，不属于额外重构。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `src-tauri/src/extraction/models.rs` | 修改 | `ExtractOptions` 增加 `markitdown_embedded_python: Option<String>` 字段（沿用 `derive(Default)`，无破坏） |
| `src-tauri/src/extraction/extractors/markitdown.rs` | 修改 | 改为带 `RwLock` 字段的结构体；新增 `new()`/`detected_version()` inherent 方法、`parse_error_with_class()`/`probe_markitdown_version()` 私有助手；`python_candidates` 调整顺序+去重闭包；7 个 `#[cfg(test)]` 单测覆盖 error_class 三场景、候选顺序、去重、缺省、版本缓存初值 |
| `src-tauri/src/extraction/extractors/mod.rs` | 修改 | `Box::new(MarkItDownExtractor)` → `Box::new(MarkItDownExtractor::new())`（结构变更强制） |

未触碰：`src/`（前端）、`extraction/mod.rs:4` scheduler 注释、`Extractor` trait、`commands/conversion.rs`。

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（沿用 `extraction/extractors/markitdown.rs`）
- [x] API 路径/命名与 Architect 方案一致（`error_class` 词表与 task_005 `classify_error` 对齐）
- [x] 数据模型与 Architect 方案一致（`ExtractOptions` 增量字段、`ExtractionResult` 未改）
- [x] 未引入计划外的新依赖（只用 `std::sync::RwLock`，已在 std）
- 偏离说明：
  1. **C 节实现选择**：依 prompt 建议采用方案一（前缀写入 `error_class:xxx|`），未实现 RefCell 版本。理由见"实现摘要 §2"。
  2. **`error_class_file_not_found` 测试输入**：不带 `python3:` 前缀直接给 `FileNotFoundError: ...`，避免触发 task_005 `classify_error` 已记录的优先级规则（含 "python" 子串优先归 `python_unavailable`）。这是测试用例的取舍，**不影响**实际 extract() 路径——extract() 失败时拼接的 attempts 字符串带有 `python3:` 前缀，会归类为 `python_unavailable`，这是 task_005 已确认的设计预期；真正的 file_not_found 场景由 scheduler 在文件预检阶段（task_008）显式构造，而非依赖 extract() 失败路径。

## 测试命令

```bash
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri
cargo check 2>&1 | tail -10
cargo test --lib extraction::extractors::markitdown 2>&1 | tail -20
```

## 测试结果

```
$ cargo check
warning: `notecapt` (lib) generated 5 warnings (run `cargo fix --lib -p notecapt` to apply 4 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.61s
（0 error）

$ cargo test --lib extraction::extractors::markitdown
running 7 tests
test extraction::extractors::markitdown::tests::detected_version_starts_empty ... ok
test extraction::extractors::markitdown::tests::python_candidates_deduplicates_when_cmd_equals_python3 ... ok
test extraction::extractors::markitdown::tests::error_class_markitdown_not_installed ... ok
test extraction::extractors::markitdown::tests::error_class_file_not_found ... ok
test extraction::extractors::markitdown::tests::error_class_conversion_error_empty_stderr ... ok
test extraction::extractors::markitdown::tests::python_candidates_order_with_embedded_and_cmd ... ok
test extraction::extractors::markitdown::tests::python_candidates_defaults_only ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 50 filtered out
```

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|----------|----------|------|-----------|
| ✅ 正常路径 | `python_candidates` embedded + cmd 顺序 | 已测 | PASS（`python_candidates_order_with_embedded_and_cmd`） |
| ✅ 正常路径 | `python_candidates` 仅缺省 | 已测 | PASS（`python_candidates_defaults_only`） |
| ✅ 正常路径 | `detected_version()` 初值 None | 已测 | PASS（`detected_version_starts_empty`） |
| ⚠️ 边界条件 | python_cmd == "python3" 时去重 | 已测 | PASS（`python_candidates_deduplicates_when_cmd_equals_python3`） |
| ⚠️ 边界条件 | `ExtractOptions::default()` 仍可构造（新字段 Option 默认 None） | 已测 | PASS（多处测试均用 `..Default::default()`） |
| ❌ 异常路径 | stderr 含 `ModuleNotFoundError` → `markitdown_not_installed` | 已测 | PASS |
| ❌ 异常路径 | stderr 含 `FileNotFoundError`（无 "python" 子串） → `file_not_found` | 已测 | PASS |
| ❌ 异常路径 | stderr 空、退出码非 0 → `conversion_error` | 已测 | PASS |
| ⚠️ 边界条件 | 实际 subprocess 行为（真启动 python） | 未测 | 显式回避——按 prompt "不真正调用 markitdown"。由 AC-5 手测覆盖 |

## 已知局限

1. **版本缓存仅在首次 extract 成功路径填充**：若 markitdown 始终调用失败，`detected_version()` 永返 `None`。这是预期行为（失败路径无法可靠拿到版本），task_008 在 scheduler 中应回退到 `commands/conversion.rs::check_markitdown_status` 探测结果。
2. **错误前缀字符串包含 `|` 字符**：若 attempts 内容本身含 `|`，scheduler 解析时应只 split 第一个 `|`。已在 output.md 中暗示，但未在代码 doc 中显式标注。
3. **`probe_markitdown_version` 退出非零时返回 None**：未尝试解析 stderr，符合 best-effort 语义；某些 venv 中 `markitdown --version` 可能将版本写到 stderr——本实现不覆盖此变体。
4. **commands/conversion.rs 未注入 embedded_python**：按 input.md AC-3 注释，留给 task_008 scheduler 拿 AppHandle 后注入；当前 `convert_asset_to_markdown` 路径无 embedded 候选，但有 `python3`/`python` 兜底，符合本 task scope。

## 需要 Reviewer 特别关注的地方

1. **`MarkItDownExtractor` 从 unit struct → 带字段结构**：`extractors/mod.rs:18` 的构造点已同步改为 `::new()`。请确认无其他地方持有 `MarkItDownExtractor` 字面值（已 `grep -rn "MarkItDownExtractor" src/` 检查通过）。
2. **`is_none_or` 用法**（`markitdown.rs:96` 附近）：Rust 1.82+ 稳定 API；当前 toolchain 已通过 `cargo check`（task_005 测试中也用到了 `is_some_and`，证明 toolchain 兼容）。
3. **C 节方案一的 scheduler 解析契约**：task_008 必须按 `strip_prefix("error_class:") + find('|')` 解析；如 Reviewer 偏好 RefCell 版本，可在 Fix 阶段反转决定，但前缀方案对 trait-object 边界更友好（`&dyn Extractor` 无需额外 downcast）。
4. **`error_class_file_not_found` 测试输入剥离了 `python3:` 前缀**：见"偏离说明 §2"。这反映了 task_005 `classify_error` 既有优先级（python_unavailable > file_not_found 当含 "python"），不属于本 task 引入的问题。
5. **`extraction/mod.rs:4` scheduler 注释保持不动**——已显式验证，M-1 不变量未破坏。
