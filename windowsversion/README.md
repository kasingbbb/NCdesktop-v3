# NoteCapt for Windows

Windows 10/11 版本的 NoteCapt（macOS 版的复刻），基于 Tauri 2 + React + Rust。

## 构建

需要 Windows 10/11 主机（不支持 macOS 交叉编译），并预装：
- Node.js 20+ + pnpm 9+
- Rust stable + `rustup target add x86_64-pc-windows-msvc`
- Visual Studio Build Tools（C++ workload）

```bash
pnpm install --frozen-lockfile
pnpm tauri build  # 产出 .msi + .exe
```

详见 `BUILD-WINDOWS.md`（由 Unit 15 提供）。

## 与 macOS 版的差异

详见 `MIGRATION-NOTES.md`（由 Unit 15 提供）。核心差异：OCR/ASR 走云端 API、文件管理器调 `explorer.exe`、标题栏样式调整。
