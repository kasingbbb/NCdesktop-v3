# 在 Windows 10/11 上构建 NoteCapt

本项目是 NoteCapt 的 Windows 版（macOS 原版的复刻），基于 Tauri 2 + React + Rust。

## 一次性环境准备

1. **Node.js 20 LTS+** 与 **pnpm 9+**
   - 从 https://nodejs.org 安装 Node
   - `npm i -g pnpm`

2. **Rust stable toolchain**
   - 从 https://rustup.rs 安装 rustup
   - `rustup default stable`
   - `rustup target add x86_64-pc-windows-msvc`

3. **Visual Studio 2022 Build Tools**
   - 下载 Visual Studio Installer（社区版亦可）
   - 安装 workload：**"使用 C++ 的桌面开发"**（含 MSVC v143、Windows 11 SDK）

4. **WebView2 Runtime**
   - Windows 11 自带；Windows 10 需手动从 https://developer.microsoft.com/microsoft-edge/webview2/ 安装 "Evergreen Bootstrapper"

5. **OpenAI API Key**（OCR/ASR 用）
   - 在 https://platform.openai.com 获取 key
   - 设置环境变量：`setx OPENAI_API_KEY "sk-..."` 或在 app 内 Settings 配置

6. **PDFium 动态库**（扫描版 PDF OCR 必需）
   - 从 https://github.com/bblanchon/pdfium-binaries/releases 下载最新 `pdfium-windows-x64.zip`
   - 解压后把 `bin/pdfium.dll` 复制到：
     - 开发：`src-tauri/pdfium.dll`（与 Cargo.toml 同级目录）
     - 打包：放到 `src-tauri/resources/pdfium.dll`，并在 `tauri.conf.json` 的 `bundle.resources` 加入 `"resources/pdfium.dll"`
   - 没装时 PDF 扫描页 OCR 会返回错误 "PDFium 加载失败"；文本版 PDF 不受影响（走 `pdf-extract` 文本抽取路径）

7. **HEIC 图片 OCR**（可选）

   当前 Windows 版**不支持 HEIC 直接 OCR**（iPhone 默认格式）。如需使用：
   - Windows 10/11 装 "HEVC 视频扩展"（Microsoft Store 免费）
   - 用 Photos 打开 .heic → 另存为 .jpg，再拖入 NoteCapt

   未来版本将通过 Windows.Graphics.Imaging WinRT API 自动转换（需添加 `windows` crate 依赖）。

## 开发

```powershell
pnpm install --frozen-lockfile
pnpm tauri dev
```

## 打包

```powershell
pnpm tauri build
```

产物在 `src-tauri/target/release/bundle/`：
- `nsis/*.exe`（NSIS 安装包）
- `msi/*.msi`（Wix MSI 包）

## 故障排查

- **`cargo check` 报错找不到 link.exe**：未装 MSVC Build Tools，回到第 3 步
- **WebView2 报错**：Windows 10 上需手动装 Evergreen Runtime
- **OCR/ASR 调用失败**：检查 `OPENAI_API_KEY` 是否生效（`echo %OPENAI_API_KEY%`）
- **应用启动后白屏**：DevTools 看 Console；常见原因是 `tauri.conf.json` 的 `devUrl` 与 vite 端口不一致
