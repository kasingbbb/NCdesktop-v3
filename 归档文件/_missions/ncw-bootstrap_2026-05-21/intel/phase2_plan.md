# Phase 2 执行 Plan

## 当前已知状态

- notecapt-windows main @ `b52be92c`（hotfix PR #4 已 merge）
- 本地 clone：`/tmp/ncw-test/notecapt-windows/` 刚 `git pull --ff-only` 同步到 main
- Phase 1 已验：`cargo check --lib` exit 0 / `cargo test --lib` 406 passed
- 用户 5 项决策已拍板（D1-D5 全采纳默认）

## Phase 2 切片

### 切片 A：rerun cargo check + cargo test（项 1, 2）
- 在新的"刚 pull"环境再跑一次，确认 reproducible
- 时间：~10 min

### 切片 B：JS toolchain（项 3, 4, 5）
- pnpm install + pnpm test + pnpm dev + Chrome MCP 渲染验证
- 时间：~15-30 min

### 切片 C：full stack tauri dev（项 6）
- pnpm tauri dev 起来，computer-use 截图
- 时间：~10-30 min（取决于初次 cargo build dev 时长）

### 切片 D：功能链路（项 7, 8, 9）
- 创建 Library / Dropzone / PDF / EPUB
- 时间：~30-60 min
- 包含 D2/D5 降级验

### 切片 E（并行/独立）：D1 windowsversion 清理
- D1 默认 = "删除 RENCdesktop 主仓里 windowsversion/ 旧副本或留 README 标已迁移"
- 这是主 NCdesktop 仓的操作，**不影响 Phase 2 切片 A-D**
- chris 需要先搞清楚：
  1. 主 NCdesktop 仓 main 分支当前的 windowsversion/ 状态
  2. 是用 git rm 删除整个目录 + commit + 走 PR？
  3. 还是改成只剩一个 README.md 标"已迁移到 https://github.com/kasingbbb/notecapt-windows"？
- 推荐：先在 mission 内写好 PR 的 commit message 草稿和 README 内容，等主要切片跑完再交给主对话执行
- 时间：~10 min

## 建议执行顺序

```
1. 切片 A（10 min）   <- 立即开始
2. 切片 B（30 min）   <- 并行验前端
3. 切片 C（30 min）   <- 顺序，依赖 A
4. 切片 D（60 min）   <- 顺序，依赖 C
5. 切片 E（10 min）   <- 完整切片 D 后，避免分心
```

## 风险与已知不可控因素

| 风险 | 影响 | 缓解 |
|---|---|---|
| macOS dev 模式起 tauri 时找不到资源（pdfium / runtime-manifest） | 项 6 起不来 | 已有 stub；如确实卡，先看 stderr，可能只是 warn |
| Library / Dropzone UI 控件位置变化 | computer-use 找不到按钮 | 截图先看 UI，列举 controls 后再操作 |
| 拖拽到 Tauri 窗口 macOS 上的 quirk | 项 8 失败 | 如果拖不进，转用 IPC 直接调（pnpm dev mode 下可用 Tauri JS API） |
| extraction 模块在 macOS host 上 runtime fail（依赖 Windows API） | 项 9 失败 | 看 audio_asr_iflytek / pdf_text 是否纯 Rust；epub_text 是新的，应该跨平台 |
| SQLite 写入路径在 macOS 上的位置 | 项 7 找不到 db 文件 | 先用 `find ~/Library -name "*.db"` 之类搜索 |

## 第一步行动（chris 现在要主对话做）

**指令给主对话 Claude（我）：**

> 主对话，请在 `/tmp/ncw-test/notecapt-windows/` 上执行切片 A：
>
> 1. 先 `git log --oneline -3` 确认 HEAD = b52be92c
> 2. `cd src-tauri && cargo check --lib`，给我完整 stderr 末尾 + exit code
> 3. `cd src-tauri && cargo test --lib`，给我 test summary（"X passed; Y failed"）+ exit code
>
> 如果两项都 PASS，chris 进入切片 B。如果任何一项 FAIL，chris 决策。
