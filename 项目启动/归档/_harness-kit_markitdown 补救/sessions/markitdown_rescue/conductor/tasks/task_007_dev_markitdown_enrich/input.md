# Task 输入 — task_007_dev_markitdown_enrich

## 目标
增强 `extractors/markitdown.rs`：缓存 markitdown 版本字符串、`Err` 转 `error_class`、把嵌入式 venv python 提到候选最高优先级。

## 前置条件
- 依赖 task：task_005（用 `classify_error`）
- 必须先存在：`conversion::classify_error`、`detect_embedded_markitdown_python`

## 验收标准（AC）
1. **AC-1**：`MarkItDownExtractor::extract` 在调用前缓存版本号（`python -m markitdown --version` 的输出），写入返回的 `ExtractionResult.extractor_type` 仍为 `"markitdown"`，但版本被携带（建议扩展 `ExtractionResult` 或通过 task_008 在 scheduler 落库时取适配器实例版本）。
2. **AC-2**：返回的 `ExtractionError::ParseError` 携带的字符串经 `classify_error` 归类，调用方（task_008 scheduler）可不依赖 stderr 文本即可写入 `error_class`。本 task 至少导出 `pub fn last_error_class() -> Option<&'static str>` 或在错误结构里挂载 class。
3. **AC-3**：`python_candidates(options)` 调整后顺序为：① embedded venv（通过新增 `options.markitdown_embedded_python: Option<String>` 注入）② `options.markitdown_python_cmd` ③ `python3` ④ `python`。
4. **AC-4**：单测：
   - 注入伪造 stderr `"ModuleNotFoundError: No module named 'markitdown'"` → `error_class == "markitdown_not_installed"`
   - 注入伪造 stderr `"FileNotFoundError"` → `error_class == "file_not_found"`
   - 注入伪造 stderr 空字符串、退出码非 0 → `error_class == "conversion_error"`
5. **AC-5**：手测（开发机）：在系统已装 markitdown 的情况下，`check_markitdown_status` 返回 `version` 非空；卸载后 `error_class == "markitdown_not_installed"`，前端 `check_markitdown_status` 返回 `available: false`。

## 技术约束
- `Command::args(...)` 数组传参；禁止字符串拼接。
- stderr 仅 `log::warn!`，不外传到 UI（UI 只读 error_class）。
- 不允许 `unwrap()`/`expect()`。
- 不引入新依赖。

## 参考文件
- `src-tauri/src/extraction/extractors/markitdown.rs`
- `src-tauri/src/extraction/models.rs::ExtractOptions`（需要追加 `markitdown_embedded_python: Option<String>`，确保 `Default::default()` 安全）
- `src-tauri/src/extraction/scheduler.rs::detect_embedded_markitdown_python`
- 架构方案 §三 ADR-005、§九 R4

## 预估影响范围
- 新建文件：无
- 修改文件：
  - `src-tauri/src/extraction/extractors/markitdown.rs`
  - `src-tauri/src/extraction/models.rs`（`ExtractOptions` 加字段）
  - 调用方需要同步：`commands/conversion.rs::convert_asset_to_markdown` 构造 options 时填充 embedded 路径（如可拿到 app handle，则注入；否则保留 None 由 task_008 在 scheduler 路径补）
