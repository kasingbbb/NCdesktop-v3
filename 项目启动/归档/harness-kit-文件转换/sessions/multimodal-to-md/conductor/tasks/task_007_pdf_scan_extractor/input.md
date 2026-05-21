# Task 输入 — task_007_pdf_scan_extractor

## 目标
实现 PDF 扫描型提取器：将 PDF 逐页渲染为图片后调用 Vision OCR，支持混合型 PDF 的自动检测与回退。

## 前置条件
- 依赖 task：task_004（PDF 文字提取器，用于混合型检测）、task_006（图片 OCR 提取器）
- 必须先存在的文件/接口：`extractors/pdf_text.rs`, `extractors/image_ocr.rs`, `macos/ocr_ffi.rs`

## 验收标准（Acceptance Criteria）
1. AC-1：`extraction/extractors/pdf_scan.rs` 实现了 `Extractor` trait
2. AC-2：`can_handle` 对 `application/pdf` 返回 `true`（与 pdf_text 共享 MIME，由调度器/注册表决定优先级）
3. AC-3：逐页将 PDF 渲染为图片（临时文件），调用 Vision OCR 识别
4. AC-4：合并所有页面的 OCR 结果为完整文档
5. AC-5：提供混合型 PDF 检测：`is_scan_pdf(path) -> bool` 辅助函数，检查首 N 页是否为扫描型
6. AC-6：`segments_json` 包含页码和 OCR 区域：`[{"type": "ocr_region", "content": "...", "page": 1, "confidence": 0.9}]`
7. AC-7：临时渲染图片在提取完成后自动清理
8. AC-8：提取器注册表 `get_extractor_for` 更新：对 PDF 先尝试文字提取，若 `needs_ocr_fallback` 则自动切换扫描型

## 技术约束
- PDF 页面渲染方案：使用 macOS `CGPDFDocument` + `CGContext` 渲染（同样通过 Swift 桥接），或使用 `pdf` crate 的 Rust 渲染
- 临时文件存放在 `app_data_dir/tmp/` 下
- 大 PDF 逐页处理，每页处理完即推送进度事件
- 非 macOS 平台返回 `UnsupportedPlatform`

## 参考文件
- `extractors/pdf_text.rs` — PDF 文字提取器
- `extractors/image_ocr.rs` — 图片 OCR
- `macos/ocr_ffi.rs` — Vision FFI
- Architect output.md §ADR-002 — 混合型 PDF 策略
- PRD §3.2 F03 — PDF 扫描型提取

## 预估影响范围
- 新建文件：`src-tauri/src/extraction/extractors/pdf_scan.rs`
- 修改文件：`extraction/extractors/mod.rs`（注册 + 路由逻辑）
- 可能修改：`macos/ocr_bridge.swift`（添加 PDF 渲染函数）
