# Task 输入 — task_005_dev_conversion_abstraction

## 目标
新建 `src-tauri/src/extraction/conversion.rs`，承载 `ConversionAttempt` 结构、`file_sha256` 工具、`classify_error` 错误归类函数。**不**引入新 trait（见 ADR-005）。

## 前置条件
- 依赖 task：无（独立新建文件）
- 必须先存在：`sha2` 已在 `Cargo.toml`（scheduler 已经在用 `compute_sha256`）

## 验收标准（AC）
1. **AC-1**：`ConversionAttempt` 结构存在，字段如下，全部 `Serialize` + camelCase：
   - `converter_name: String`、`converter_version: String`、`source_mime: String`、`source_hash: String`、`quality_level: i32`、`fallback_used: bool`、`error_class: Option<String>`、`conversion_ms: u64`、`converted_at: String`（RFC3339）
2. **AC-2**：`file_sha256(path: &Path) -> std::io::Result<String>` 流式读取 8KB 块，输出 hex 小写。
3. **AC-3**：`classify_error(stderr_or_err: &str) -> &'static str` 至少覆盖 8 种 error_class：`file_not_found / permission_denied / unsupported_format / markitdown_not_installed / python_unavailable / empty_output / timeout / conversion_error`。匹配规则用大小写不敏感包含。
4. **AC-4**：单元测试覆盖 `file_sha256` 与已知字节序列的 hash 对照；`classify_error` 对 8 个典型 stderr 片段全部正确归类。
5. **AC-5**：`extraction/mod.rs` re-export `pub mod conversion;`。

## 技术约束
- **不**新建 `Converter` trait；现有 `Extractor` trait 不动（ADR-005）。
- `ConversionAttempt` 是纯数据 + serde，不持有业务行为。
- 不允许 `unwrap()`/`expect()`。
- 哈希算法固定 SHA-256；不引入其他算法。

## 参考文件
- `src-tauri/src/extraction/scheduler.rs::compute_sha256`（现有实现，本 task 落地后 scheduler 可改用 `conversion::file_sha256`，由 task_008 完成迁移）
- 架构方案 `task_001_architect/output.md` §三 ADR-005、§五.4

## 预估影响范围
- 新建文件：
  - `src-tauri/src/extraction/conversion.rs`
- 修改文件：
  - `src-tauri/src/extraction/mod.rs`（re-export）
