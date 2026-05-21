# Windows 版与 macOS 版的差异

本目录是 macOS 版 NoteCapt（路径 `项目启动/NCdesktop/`）的 Windows 复刻，**长期双仓并行维护**。

## 5 大平台差异

### 1. OCR / ASR 不再走原生 FFI

- **macOS**：调用 `src-tauri/src/macos/ocr_ffi.rs`（Vision Framework）和 `asr_ffi.rs`（SFSpeechRecognizer），完全本地
- **Windows**：删除 `macos/` 目录，改用 `src-tauri/src/cloud_ai/` 模块调用 OpenAI Vision (gpt-4o-mini) + Whisper API
- **影响**：需联网、需 API key、有 token 成本；好处是不依赖系统语音模型，准确率稳定

### 2. 文件管理器集成

- **macOS**：`Command::new("open").arg("-R")` 在 Finder 中高亮文件
- **Windows**：`Command::new("explorer.exe").args(["/select,", path])` 在 Explorer 中高亮
- 涉及文件：`src-tauri/src/commands/workspace_folders.rs`、`source_view.rs`
- 用户可见文本："在 Finder 中显示" → "在文件资源管理器中显示"

### 3. 标题栏布局

- **macOS**：左侧 80px 红绿灯留白（由 Tauri Overlay titleBarStyle 渲染系统按钮）
- **Windows**：右侧 120px 留白给系统按钮（最小化/最大化/关闭）
- 涉及文件：`src/components/layout/TitleBar.tsx`

### 4. Mica 风格未实现

- **macOS**：使用 Liquid Glass + backdrop-filter 实现毛玻璃
- **Windows**：保留 `backdrop-filter`（WebView2 支持），但**未对接 Windows 11 Mica/Acrylic 系统效果**。后续可在 `src-tauri/src/windows_native.rs` 中用 WinRT API 启用 Mica
- `globals.css` 在 `backdrop-filter` 前加 `background-color` fallback，防止透明失效

### 5. 数据目录

- **macOS**：`~/Library/Application Support/com.notecapt.desktop/notecapt.db`
- **Windows**：`%APPDATA%\com.notecapt.desktop.windows\notecapt.db`（注意 appId 不同）

## 双仓维护约定

- `项目启动/NCdesktop/`：macOS 版主分支
- `windowsversion/`：Windows 版主分支
- 共享业务逻辑修改（DB schema、Tauri command、React 组件、知识理解面板等）：
  - 默认两边同步修改
  - 如某改动 macOS-only（如 Vision API 调优）或 Windows-only（如 Mica 接入），在 commit message 明确标注
- PR 命名约定：`feat(windows): ...` / `feat(mac): ...` / `feat(both): ...`
