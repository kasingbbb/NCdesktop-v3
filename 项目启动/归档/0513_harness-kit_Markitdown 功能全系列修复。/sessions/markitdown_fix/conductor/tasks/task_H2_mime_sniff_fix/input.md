# Task 输入 — task_H2_mime_sniff_fix（HOTFIX）

## 背景
用户 2026-05-14 实测：CSV / EPUB / HTML 文件被识别为 `application/octet-stream` → 无 extractor → 标 `unsupported` placeholder。

**已确证根因**（不需要再调查）：
- `commands/sync.rs:240-256` `fn guess_mime(file_name: &str) -> String` 只硬编码 11 类扩展名：
  `jpg/jpeg/png/heic/pdf/txt/md/m4a/mp3/wav/aac`
- CSV/EPUB/HTML/DOCX/XLSX/PPTX/XML/JSON 等**全部 fallback** 到 `application/octet-stream`
- scheduler 查询无 extractor 接受 `application/octet-stream` → `placeholder_unsupported`

## 目标
补全 markitdown 支持的所有格式扩展名到 mime 映射；并加 `infer` crate 内容嗅探作为扩展名缺失/不匹配时的兜底。

## 验收标准

### AC-1：guess_mime 扩展名映射补全（最小集）
新增至少以下扩展名映射（含 markitdown 所有可处理格式）：

| 扩展名 | mime |
|---|---|
| `.csv` | `text/csv` |
| `.epub` | `application/epub+zip` |
| `.html`, `.htm` | `text/html` |
| `.xml` | `application/xml` |
| `.json` | `application/json` |
| `.docx` | `application/vnd.openxmlformats-officedocument.wordprocessingml.document` |
| `.xlsx` | `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet` |
| `.pptx` | `application/vnd.openxmlformats-officedocument.presentationml.presentation` |
| `.xls` | `application/vnd.ms-excel` |
| `.doc` | `application/msword` |
| `.ppt` | `application/vnd.ms-powerpoint` |
| `.zip` | `application/zip` |
| `.rtf` | `application/rtf` |
| `.tsv` | `text/tab-separated-values` |
| `.webp`, `.gif`, `.bmp`, `.tiff`, `.tif` | `image/<ext>` |
| `.mp4`, `.mov`, `.webm`, `.mkv` | `video/<ext>` |
| `.flac`, `.ogg`, `.opus` | `audio/<ext>` |

### AC-2：infer crate 内容嗅探兜底
- 在 `Cargo.toml` 添加 `infer = "0.16"` 或类似稳定版本（纯 Rust，无 C 依赖）
- 修改 `guess_mime` 签名为 `guess_mime(path: &Path) -> String`（或同名重载）：
  1. 先按扩展名映射查
  2. 找不到 → 用 `infer::get_from_path(path)` 读 magic bytes 嗅探
  3. infer 也失败 → 最终 fallback `application/octet-stream`
- **不要打开整个文件**：infer 只读前 ~256 字节，性能 OK

### AC-3：扩展名映射不区分大小写
- `.PDF` 和 `.pdf` 等价；`.JPG` 和 `.jpg` 等价
- 用 `to_ascii_lowercase()` 规范化后再 match

### AC-4：单测
- 覆盖至少 15 种扩展名映射（含 csv/epub/html/docx/xlsx/pptx/xml/json/zip/rtf）
- infer 兜底测：mock 一个 magic bytes 是 PDF 但扩展名是 `.bin` 的文件 → 应识别为 `application/pdf`
- 大小写不敏感测：`.PDF` / `.pdf` / `.PdF` 三种都返回 `application/pdf`

### AC-5：实测验证
- 拖入 `.csv` → conversion_meta.source_mime='text/csv'，能走对应 extractor 路径
- 拖入 `.epub` → conversion_meta.source_mime='application/epub+zip'，markitdown 转录成功
- 拖入 `.html` → conversion_meta.source_mime='text/html'，markitdown 转录成功

### AC-6：调用方兼容
- 找到所有 `guess_mime(...)` 调用点，如果改了签名（&str → &Path）需要同步修改调用方
- 用 `grep -n "guess_mime" src-tauri/src/` 找全调用
- 不破坏现有 `commands/sync.rs:163` 等调用

## 严禁（红线）
- 修改 `extractors/*` 任何 extractor 业务逻辑（task_007/008/009/010/011 PASS 边界）
- 修改 `runtime_check.rs`（task_H1 并行 dev 范围）
- 修改 `audio_asr_iflytek.rs`（PRD 底线 #4）
- 修改 `failure_code.rs`、`scheduler.rs`（task_007/008 PASS 边界，仅可读引用）
- 修改 `db/migration.rs`、`db/conversion_meta.rs`、`db/asset.rs`（task_008/014 PASS 边界）
- 修改 `scripts/*`、`tauri.conf.json`（task_H1 范围）
- 引入除 infer crate 之外的新依赖
- 启发式 / 字数判定（H6 PRD 底线）

## 预估影响范围
- 修改：`src-tauri/src/commands/sync.rs`（`guess_mime` 函数 + 可能的调用方签名同步）
- 修改：`src-tauri/Cargo.toml`（`infer` 依赖加入）
- 修改：`src-tauri/Cargo.lock`（infer 传递依赖）
- 单测：`commands/sync.rs` 末尾 `#[cfg(test)]` 段

## 参考文件
- `commands/sync.rs:240-256` 当前 guess_mime 实现
- `commands/sync.rs:163` 调用点（TF 卡导入）
- markitdown 0.1.x 支持格式清单：pdf/docx/pptx/xlsx/xls/html/epub/csv/json/xml/zip/image/audio
- 用户实测 placeholder：CSV / EPUB / HTML 均 mime=application/octet-stream
