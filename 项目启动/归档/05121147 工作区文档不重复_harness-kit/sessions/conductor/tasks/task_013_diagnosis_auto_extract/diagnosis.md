# 诊断报告 — 自动提取/转化未触发

## 1. 用户 DB 实测快照

- `PRAGMA user_version = 11`（V11 conversion_meta hotfix 已落库，时间戳 2026-05-13 08:59:50）

### 1.1 根资产 mime 分布（去除 derivative）

| mime_type | roots | has_pipeline_task | has_md_derivative |
|---|---|---|---|
| application/epub+zip | 11 | 11 | 11 |
| application/pdf | 6 | 6 | **4** |
| application/octet-stream | 3 | 3 | 3 |
| audio/mp4 | 3 | 3 | 3 |
| audio/mpeg | 6 | **5** | **4** |
| image/png | 4 | 4 | 4（**全部 placeholder**）|
| text/html | 1 | 1 | 1 |
| text/markdown | 2 | 2 | 2 |
| text/plain | 1 | 1 | 1 |
| **总计** | **37** | **36** | **33** |

### 1.2 pipeline_tasks 状态

- completed=52，failed=4，queued=0，running=0。
- 无遗留 queued 任务，scheduler 正在工作。

### 1.3 失败 4 条（**全部是单条业务错误，不是链路死掉**）

| asset | task | error_message（截断） |
|---|---|---|
| a919bff0…（一个 epub）| extract | `解析错误: MarkItDown 调用失败：python3: 输出为空 \| python: 输出为空` |
| e85a6974…（mp3） | extract | `OCR 错误: 讯飞上传错误 code=100009: signature is error` |
| 2fe5019b…（mp3） | extract | `code=100020: language[autodialect] does not support` |
| a32e778e…（m4a） | extract | `code=100020: language[autodialect] does not support` |

### 1.4 extracted_content 状态

- extracted=48，extracting=1（卡住一条，asset 不明，可能正在跑），failed=4，unsupported=7。
- 7 条 unsupported 是 **image/png + text/plain + text/markdown + text/html** → scheduler 写 placeholder.md 而非真转换。
- image/png 4 个 derivative 全部 `extractor_type=""`、`raw_text=0 长度`、`derivative_version=0`，证实是 placeholder（`code=unsupported`），不是真 OCR/真转换。

### 1.5 conversion_meta 表

- 行数 = 3（全是同一条音频 a32e778e 的多次重试，converter=audio_asr_iflytek，error_class=conversion_error）。
- V11 修复前的所有成功转换都没写入此表（日志里 `WARN 写 conversion_meta 失败: no such table` 反复出现 20+ 次直到 V11 跑完）。

### 1.6 关键日志摘录（~/Library/Logs/com.notecapt.desktop/NoteCapt.log）

```
2026-05-13 03:01:12 物化 MD v1 完成: c3a67dbc... -> de5bd069... .md
2026-05-13 03:01:12 WARN 写 conversion_meta 失败: no such table: conversion_meta
…（V11 migration 直到 08:59:50 才补上 conversion_meta）
2026-05-13 09:06:13 写 placeholder 完成（不推进版本号）... code=conversion_error  ← 讯飞 ASR 失败兜底
2026-05-13 多次：source_scan 跳过项目（list_root_assets prepare 失败: no such table: conversion_meta）
```

## 2. 链路逐段验证

| 段 | 实测状态 | 证据 |
|---|---|---|
| import 命令 → import_files_core | OK | `dropzone.rs:712` 直接调用 |
| import_files_core → scheduler.enqueue | OK | `dropzone.rs:628` 每条 asset 调 `scheduler.enqueue(&asset.id)`，写 pipeline_tasks 行 |
| scheduler.enqueue → pipeline_tasks row | OK | DB 中 36/37 root 都有 pipeline_tasks（仅 1 个 audio/mpeg 异常缺失，可能 enqueue 失败列表里）|
| **scheduler.start() 唤醒** | **OK** | `dropzone.rs:724-727` `if any_enqueued { scheduler.start(app.clone()); }`；lib.rs:78-88 启动期也会唤醒残留 queued |
| get_extractor_for(mime) | **部分缺口** | `extractors/mod.rs:17-23`：markitdown.SUPPORTED 只覆盖 pdf/docx/pptx/xlsx/html/epub。**不含** image/*、audio/*、text/markdown、text/plain。这些走 fallback → 没有 fallback → `unsupported` placeholder |
| markitdown extractor 真跑 | **大部分 OK** | 11 个 epub 中 10 个真转换成 ~70k-200k 字 MD，1 个失败（输出为空）。markitdown 0.1.5 已在系统 `python3` 中可用；用户**没有**配置 `markitdownPythonCmd`、`markitdownEnabled` 默认 true |
| extraction success → materialize_md → derivative row | OK | 33/37 root 有 MD derivative，48 行 extracted_content status=extracted |

## 3. 根因

**不存在"导入不自动提取"的链路 bug**。链路完整：import → enqueue → start → process → materialize_md。DB 实测 37 个 root 中 36 个进入 pipeline、33 个有真 MD derivative。

用户报告的"没真转换成 MD"实际是**两类已知业务结果被前端误读为"未提取"**：

1. **image/png（4）+ text/markdown/plain/html（4）= 8 个 "unsupported" placeholder**：scheduler 因 `get_extractor_for` 返回 None 直接写 placeholder MD（`derivative_version=0`、`raw_text=空`、`extractor_type=""`），前端看到 .md 文件以为转换好了，打开却是占位文本 → **看起来是"原文件 + 没转换"**。
2. **3 条音频 ASR 失败 + 1 条 epub markitdown stdout 为空**：placeholder 兜底，code=conversion_error。讯飞错误是 **API 密钥/语言参数错误**（100009 签名错 / 100020 autodialect 不支持），不是链路问题。

V11 migration 前 conversion_meta 表缺失（V9/V10 hotfix 残留）已在 2026-05-13 08:59:50 修复，**不再是当前问题**。

## 4. 修复建议（派给 Dev 的最小修复范围）

### Fix-A（**用户最痛点，必修**）：扩展 markitdown.SUPPORTED_MIME_TYPES

- 文件：`src-tauri/src/extraction/extractors/markitdown.rs:54-61`
- 加：`text/plain`, `text/markdown`, `image/png`, `image/jpeg`, `image/heic`, `application/zip`（markitdown 0.1.5 都支持，已本地实测 `python3 -m markitdown --version` = 0.1.5）
- 注意：image/* markitdown 走 OCR（pytesseract），用户若无 tesseract 仍会失败 → 失败时让现有 fallback→placeholder 机制兜底
- 或者为 text/plain、text/markdown 注册一个 trivial 内置 extractor（直接读文件作 raw_text），不要 placeholder

### Fix-B（次要）：讯飞 ASR `language=autodialect` 兼容

- 文件：`src-tauri/src/extraction/extractors/audio_asr_iflytek.rs`
- 当前默认 `language=autodialect`，但用户的应用密钥不支持该值（code 100020）→ 默认改成 `cn` 或在 settings 里暴露给用户。这是 mp3 转写失败的根因，不是链路 bug。

### Fix-C（前端可观察性，可选）：UI 区分 placeholder vs 真 MD

- 检查 `extracted_content.extractor_type` 是否以 `placeholder_` 开头 / `derivative_version=0` → 在工作区列表 badge 出"未支持/转换失败"，避免用户以为系统坏了。

## 5. 用户验证步骤（修复后 1 分钟）

1. 用户拖入一个 `.txt` 或 `.png` 到 dropzone。
2. 等待 5-10 秒，刷新工作区。
3. 打开生成的 .md：
   - txt → 应包含原文（≥1 字）
   - png → 应包含 markitdown OCR 文本（若装了 tesseract）或显式 "提取失败：未检测到 tesseract" 提示
4. 校验 SQL：
   ```sql
   SELECT ec.status, ec.extractor_type, length(ec.raw_text)
   FROM extracted_content ec WHERE asset_id IN (SELECT id FROM assets ORDER BY imported_at DESC LIMIT 5);
   ```
   应看到 `status=extracted, extractor_type=markitdown/text, text_len > 0`。
