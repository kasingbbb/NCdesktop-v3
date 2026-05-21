# Task 输入 — task_003_extractor_trait

## 目标
定义 `Extractor` trait、提取结果数据模型、以及 `extraction` 模块的骨架结构，为后续提取器实现提供统一接口。

## 前置条件
- 依赖 task：task_002（`extracted_content` 表结构已定义）
- 必须先存在的文件/接口：`db/extraction.rs` 中的 CRUD 函数

## 验收标准（Acceptance Criteria）
1. AC-1：`src-tauri/src/extraction/mod.rs` 定义了 `Extractor` trait，包含 `can_handle(&self, mime_type: &str) -> bool` 和 `async fn extract(&self, file_path: &Path, options: &ExtractOptions) -> Result<ExtractionResult, ExtractionError>`
2. AC-2：`ExtractionResult` 结构体包含 `raw_text`, `structured_md`, `quality_level`, `extractor_type`, `segments` 字段
3. AC-3：`ExtractionError` 枚举覆盖 `UnsupportedFormat`, `IoError`, `ParseError`, `OcrError` 等变体
4. AC-4：`ExtractOptions` 包含 `language_hint: Option<String>`, `max_pages: Option<u32>` 等配置项
5. AC-5：`extraction/extractors/mod.rs` 包含提取器注册函数 `get_extractor_for(mime_type: &str) -> Option<Box<dyn Extractor>>`
6. AC-6：模块在 `lib.rs` 中声明 `pub mod extraction;`
7. AC-7：`cargo build` 编译通过

## 技术约束
- `Extractor` trait 须为 `Send + Sync`（在 tokio::spawn 中使用）
- `ExtractionResult` 须 `#[derive(Serialize, Deserialize)]` 以支持 IPC 传输
- 模块结构须与 Architect output.md 中的目录规划一致
- 不在此 task 实现具体提取器，只定义骨架

## 参考文件
- Architect output.md §系统架构 — 模块图
- Architect output.md §目录结构规划
- PRD §4.3 segments_json Schema
- `src-tauri/src/models/` — 现有模型定义风格参考

## 预估影响范围
- 新建文件：`src-tauri/src/extraction/mod.rs`, `src-tauri/src/extraction/models.rs`, `src-tauri/src/extraction/extractors/mod.rs`
- 修改文件：`src-tauri/src/lib.rs`（添加 `pub mod extraction;`）
