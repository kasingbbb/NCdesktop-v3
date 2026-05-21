# Task 输入 — task_006_image_ocr_extractor

## 目标
实现图片 OCR 提取器，封装 Vision FFI 为标准 `Extractor` trait 实现，支持从照片中提取文字内容。

## 前置条件
- 依赖 task：task_003（Extractor trait）、task_005（Vision FFI 桥接）
- 必须先存在的文件/接口：`extraction/mod.rs` 的 Extractor trait、`macos/ocr_ffi.rs` 的 `ocr_image` 函数

## 验收标准（Acceptance Criteria）
1. AC-1：`extraction/extractors/image_ocr.rs` 实现了 `Extractor` trait
2. AC-2：`can_handle` 对 `image/jpeg`, `image/png`, `image/heic`, `image/webp` 返回 `true`
3. AC-3：调用 Vision FFI 完成 OCR，将结果组装为 `ExtractionResult`
4. AC-4：OCR 结果的段落按垂直位置排序，合并相邻文本块
5. AC-5：`quality_level` 设为 1（段落分割）
6. AC-6：`segments_json` 包含 OCR 区域信息：`[{"type": "ocr_region", "content": "...", "confidence": 0.95, "bbox": [...]}]`
7. AC-7：识别结果为空时返回空 raw_text（不报错）
8. AC-8：`extractor_type` 设为 `"vision_ocr"`

## 技术约束
- OCR 调用须在 `tokio::task::spawn_blocking` 中执行（Vision Framework 是同步 API）
- 非 macOS 平台返回 `ExtractionError::UnsupportedPlatform`
- 单张图片处理时间目标 ≤ 2s（1200 万像素，M1 芯片）

## 参考文件
- `extraction/mod.rs` — Extractor trait
- `macos/ocr_ffi.rs` — Vision FFI 接口
- PRD §2.2 P0 场景 #1 — 课堂照片批量 OCR
- PRD §3.4 质量分级 Level 1

## 预估影响范围
- 新建文件：`src-tauri/src/extraction/extractors/image_ocr.rs`
- 修改文件：`extraction/extractors/mod.rs`（注册提取器）
