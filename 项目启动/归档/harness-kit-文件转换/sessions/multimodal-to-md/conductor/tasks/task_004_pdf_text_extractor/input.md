# Task 输入 — task_004_pdf_text_extractor

## 目标
实现 PDF 文字型提取器，使用 `pdf-extract` crate 从文字型 PDF 中提取文本内容，输出结构化 Markdown。

## 前置条件
- 依赖 task：task_003（Extractor trait 已定义）
- 必须先存在的文件/接口：`extraction/mod.rs` 中的 `Extractor` trait

## 验收标准（Acceptance Criteria）
1. AC-1：`pdf-extract` 依赖已添加到 `Cargo.toml`
2. AC-2：`extraction/extractors/pdf_text.rs` 实现了 `Extractor` trait
3. AC-3：`can_handle` 对 `application/pdf` 返回 `true`
4. AC-4：对文字型 PDF，正确提取全文，保留段落分隔
5. AC-5：提取结果的 `quality_level` 为 1（段落分割）或 2（有标题结构）
6. AC-6：提取结果的 `segments` 包含按页分割的文本段
7. AC-7：对空 PDF 或无文字 PDF，返回空 raw_text 而非错误（后续由调度器决定是否切换扫描型）
8. AC-8：添加混合型 PDF 检测逻辑：每页文字量 < 50 字符时标记 `needs_ocr_fallback`

## 技术约束
- 仅使用 `pdf-extract` crate，纯 Rust，不引入系统依赖
- 提取须异步兼容（可在 `tokio::task::spawn_blocking` 中执行）
- `segments_json` 格式：`[{"type": "text", "content": "...", "page": 1}]`

## 参考文件
- `extraction/mod.rs` — Extractor trait 定义
- Architect output.md §ADR-002
- PRD §3.3 输入格式支持 — PDF 格式细分
- PRD §3.4 质量分级定义

## 预估影响范围
- 新建文件：`src-tauri/src/extraction/extractors/pdf_text.rs`
- 修改文件：`Cargo.toml`（添加 pdf-extract）、`extraction/extractors/mod.rs`（注册提取器）
