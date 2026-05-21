# Task 输入 — task_005_vision_ffi

## 目标
建立 macOS Vision Framework 的 Swift-Rust FFI 桥接，使 Rust 代码能调用 Vision OCR 识别图片中的文字。

## 前置条件
- 依赖 task：无（可与 task_002-004 并行）
- 必须先存在的文件/接口：无

## 验收标准（Acceptance Criteria）
1. AC-1：`src-tauri/macos/ocr_bridge.swift` 文件存在，包含 `extern "C"` 可导出的 OCR 函数
2. AC-2：Swift 函数签名：`recognize_text_in_image(path: *const c_char) -> *mut c_char`，返回 JSON 字符串
3. AC-3：返回的 JSON 结构：`{"success": true, "results": [{"text": "...", "confidence": 0.95, "bbox": [x,y,w,h]}], "error": null}`
4. AC-4：`build.rs` 正确编译 Swift 文件并链接所需 macOS 框架（Vision, Foundation, CoreGraphics）
5. AC-5：Rust 侧 `macos/ocr_ffi.rs` 封装了安全的 Rust API：`pub fn ocr_image(path: &Path) -> Result<Vec<OcrRegion>, OcrError>`
6. AC-6：支持 JPEG, PNG, HEIC 格式图片输入
7. AC-7：在 Apple Silicon 和 Intel macOS 上均可编译（`#[cfg(target_os = "macos")]`）
8. AC-8：非 macOS 平台编译时该模块为空实现（返回 `UnsupportedPlatform` 错误）
9. AC-9：`cargo build` 在 macOS 上编译通过

## 技术约束
- Swift 代码必须使用 `@objc` 或 `extern "C"` 暴露 C ABI
- 使用 `VNRecognizeTextRequest` + `VNImageRequestHandler`
- 识别级别使用 `.accurate`（精确模式）
- 支持中英文：`recognitionLanguages = ["zh-Hans", "zh-Hant", "en"]`
- `build.rs` 使用 `cc::Build` 或直接调用 `swiftc` 编译
- 返回的 JSON 通过 `CString` 传递，调用方负责 `free`
- 必须在 `#[cfg(target_os = "macos")]` 条件编译下
- **Day 3 checkpoint**：如果 Swift 桥接编译困难，准备降级方案

## 参考文件
- Architect output.md §ADR-001 — Vision FFI 方案选择
- PRD §3.1 Extract 层 — macOS Vision Framework
- 现有 `src-tauri/macos/` 目录结构
- Apple Vision Framework 文档

## 预估影响范围
- 新建文件：`src-tauri/macos/ocr_bridge.swift`, `src-tauri/src/macos/ocr_ffi.rs`, `src-tauri/src/macos/mod.rs`
- 修改文件：`src-tauri/build.rs`（Swift 编译步骤）, `src-tauri/src/lib.rs`（条件 pub mod macos）
