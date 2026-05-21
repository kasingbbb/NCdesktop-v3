# 架构方案 — 文件转换 v1.1

> 产出时间：2026-04-12  
> 状态：DONE，Conductor 可启动 CODING 阶段

---

## 1. 总体方案

在现有提取管线（`extraction/scheduler.rs` → `Extractor` trait）上做最小化改造：

1. **T01/T02**: DB + 模型层：`assets` 表加 `source_asset_id` 可空字段，`Asset` 结构体同步扩展
2. **T03/T04**: 新增 docx / pptx 提取器，注册到 `get_extractor_for()`
3. **T05**: 录音 ASR：Swift `SFSpeechRecognizer` FFI bridge + Rust 侧 `macos/asr_ffi.rs` + `extractors/audio_asr.rs`
4. **T06**: `dropzone.rs` 的 `path_asset_meta()` 补充 `.docx` / `.pptx` MIME 映射
5. **T07**: `scheduler.rs` 在 extraction 成功后：写出 `.md` 文件 → 插入衍生 Asset → 发送 `notecapt/asset-converted` 事件
6. **T08**: 前端 `App.tsx` 监听事件自动刷新；`AssetListView` 工作区栏对衍生 MD 显示「转换自 [原件名]」标记

---

## 2. 文件变更清单

### 新建文件

| 路径 | 用途 |
|------|------|
| `src-tauri/src/extraction/extractors/docx.rs` | Word 文档提取器 |
| `src-tauri/src/extraction/extractors/pptx.rs` | PPT 演示文稿提取器 |
| `src-tauri/src/extraction/extractors/audio_asr.rs` | 录音 ASR 提取器 |
| `src-tauri/src/macos/asr_ffi.rs` | ASR FFI Rust 侧（macOS only） |
| `src-tauri/macos/asr_bridge.swift` | SFSpeechRecognizer Swift bridge |

### 修改文件

| 路径 | 变更摘要 |
|------|---------|
| `src-tauri/src/db/migration.rs` | 增加 V6 migration：ALTER TABLE assets ADD COLUMN source_asset_id |
| `src-tauri/src/models/asset.rs` | Asset 结构体增加 `source_asset_id: Option<String>` |
| `src-tauri/src/db/asset.rs` | ASSET_SELECT / insert / row_to_asset / get_by_project_and_tag 加入 source_asset_id |
| `src-tauri/src/extraction/extractors/mod.rs` | 注册 DocxExtractor、PptxExtractor、AudioAsrExtractor |
| `src-tauri/src/macos/mod.rs` | 暴露 asr_ffi 模块（#[cfg(target_os = "macos")]） |
| `src-tauri/src/commands/dropzone.rs` | path_asset_meta() 增加 .docx / .pptx 映射 |
| `src-tauri/src/extraction/scheduler.rs` | 成功提取后调用物化 + 衍生 Asset 创建 + 事件发送 |
| `src-tauri/Cargo.toml` | 新增 zip = "2", quick-xml = "0.37" |
| `src-tauri/build.rs` | 新增 asr_bridge.swift 编译（Speech framework） |
| `src/types/asset.ts` | Asset 增加 sourceAssetId? 可选字段；AssetType 增加 "document"/"presentation" |
| `src/App.tsx` | setupListeners 增加 notecapt/asset-converted 监听 |
| `src/components/features/AssetListView.tsx` | 右栏工作区列表对衍生 Asset 显示「转换自」标记 |

---

## 3. 关键设计决策

### 3.1 物化触发条件
```
quality_level >= 1 AND structured_md != ""
```
在 `scheduler.rs` 的 `Ok(Ok(extraction_result))` 分支末尾，同步执行（不另启协程）。

### 3.2 衍生 Asset 路径
```
~/Downloads/NoteCaptWorkPlace/<project_id>/<derived_asset_id>_<stem>.md
```
- `<stem>` 取自原件 `asset.name` 的 file_stem
- 路径与其他工作区文件一致，前端 `WorkspaceFolderStrip` 可正常枚举

### 3.3 衍生 Asset 字段
```rust
Asset {
    asset_type: "markdown",
    source_type: "converted_from",
    source_data: Some(source_asset_id),   // 冗余，便于旧代码读取
    source_asset_id: Some(source_asset_id),
    mime_type: "text/markdown",
    ...
}
```

### 3.4 失败降级策略
写出 .md 或插入衍生 Asset 失败时：
- 仅 `log::warn!`，不阻断主流程
- 若文件写出成功但 DB 写入失败，删除磁盘文件（保持一致性）

### 3.5 ASR FFI 策略
- 编译为独立静态库 `libasr_bridge.a`（与 `libocr_bridge.a` 分离）
- 使用 `DispatchSemaphore` 将 async SFSpeechRecognizer 转为同步 FFI 调用
- 超时保护：600 秒（适合 10 分钟以内录音）
- 非 macOS：`audio_asr.rs` 的 `can_handle` 永返 false，整个 ASR 路径编译时排除

### 3.6 MIME 类型
| 扩展名 | asset_type | mime_type |
|--------|-----------|-----------|
| .docx | "document" | "application/vnd.openxmlformats-officedocument.wordprocessingml.document" |
| .pptx | "presentation" | "application/vnd.openxmlformats-officedocument.presentationml.presentation" |
| .mp3 | "audio_clip" | "audio/mpeg" |（已有）
| .m4a/.aac | "audio_clip" | "audio/mp4" |（已有）
| .wav | "audio_clip" | "audio/wav" |（已有）

---

## 4. 接口规范

### `db::asset::insert`（不变签名，内部增加 source_asset_id 列）
```rust
pub fn insert(conn: &Connection, a: &Asset) -> Result<(), String>
```

### `notecapt/asset-converted` 事件 payload
```json
{
  "sourceAssetId": "uuid",
  "derivedAssetId": "uuid",
  "projectId": "uuid"
}
```

### ASR FFI（Rust 侧）
```rust
// macos/asr_ffi.rs
pub fn transcribe_audio(path: &Path) -> Result<String, String>
```

---

## 5. 风险提示

| 风险 | 缓解 |
|------|------|
| SFSpeechRecognizer 授权弹窗 | 首次调用时系统自动弹出；Info.plist 需加 NSSpeechRecognitionUsageDescription |
| quick-xml crate 编译时间增加 | 可接受，与 zip crate 一起引入 |
| PPTX 幻灯片顺序不确定 | slide 文件名排序后按序解析，如 slide1.xml < slide2.xml < slide10.xml（自然数排序） |
| 衍生 MD 重复创建 | scheduler 每次成功提取都会尝试创建，通过检查同 project_id + source_asset_id 是否已存在来防重（本期简化：直接插入，利用 DB 唯一约束）|

> 注：本期「重复创建」问题通过 DB 层唯一索引解决方案可在 V7 migration 中补充；本期 V6 先不加 UNIQUE 约束，让旧数据无影响。
