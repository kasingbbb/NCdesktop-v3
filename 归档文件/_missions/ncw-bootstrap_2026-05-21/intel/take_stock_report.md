# Take Stock Report — notecapt-windows @ 40926a7e

时间：2026-05-21
工作目录：`/tmp/ncw-test/notecapt-windows/`（独立 clone，main 分支，HEAD = remote）
对照源（只读）：`/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/`

## TL;DR

1. **macOS host 上 `cargo check` 是失败的**，但**根因不在源码层，而在 build.rs**。次发还有一处源码层缺失。
2. 真正的阻塞 = 2 项。其余都是**预期内的设计差异**（fallback 已做，Windows 实机能跑）。
3. 用户报告 macOS host build 失败是**真的**；Phase 4 worker 自报"cargo check --lib 干净通过"**是错误的**——我无法在自己的环境复现"通过"的结果，且 build.rs 第 6 行 `#[cfg(target_os = "macos")] macos_bridges::build();` 在 macOS 上必然 panic。

## 真实状态

### 仓库 vs macOS 源对照

| 项 | macOS 源 | windows 仓 | 差异性质 |
|---|---|---|---|
| `.rs` 文件数 | 122 | 125 | Windows 多了 cloud_ai/audio_chunker/heic_convert/pdf_render + extraction/extractors/epub_text；少了 macos/*.rs（3 个） |
| `.ts/.tsx` 文件数 | 223 | 222 | 仅少 App.test.tsx；assets/ 目录少；其余 12 处只是文案改了"Finder → 文件资源管理器" |
| `Cargo.lock` | 存在 | 存在（7031 行，已 cargo fetch 验证可下载） | 一致 |
| `package.json` deps | — | 与 macOS 完全等价（react 19/tauri 2/vite 8/vitest 4 等） | 无差异 |
| `tauri.conf.json` | macOS bundle | windows bundle（identifier `com.notecapt.desktop.windows`、`bundle.targets:"all"`，含 nsis/wix 配置） | 已正确 fork |
| `src-tauri/macos/*.swift` + `Info.plist` | 存在 | **不存在**（subtree split 正确剥离） | 预期 |
| `src/macos/*.rs` | 存在 | **不存在**（subtree split 正确剥离） | 预期 |

### 关键模块清点（windows 仓）

**已具备**：
- `src-tauri/src/utils/{ipc_error,nfc,safe_name,safe_rename,write_guard}.rs`（hotfix #2 补齐）
- `src-tauri/src/testing/mod.rs`（hotfix #2 补齐）
- `src-tauri/src/cloud_ai/{audio_chunker,heic_convert,pdf_render,asr_whisper_api,ocr_openai_vision,mod}.rs`（Phase 4 补齐）
- `src-tauri/src/extraction/extractors/{epub_text,pdf_text,docx,pptx,markitdown,audio_asr_iflytek,text,text_passthrough}.rs`
- `src-tauri/src/extraction/{runtime_check,scheduler,scan_pdf_detect,failure_code,conversion,models,mod}.rs`（含 hotfix #3 的 markitdown 短路 → fallback 链路）
- `src-tauri/src/{startup,source_scan,workspace,windows_native,heuristic,ics_parser,llm/*,db/*,commands/*,models/*,sync/*,audio/*}.rs`
- `src-tauri/resources/runtime-manifest.json`（hotfix #1 补齐，stub）
- `tsconfig.app.json`（hotfix #1 补齐）

**故意缺席**（macOS-only，预期）：
- `src-tauri/macos/` 整棵（Swift bridges）
- `src-tauri/src/macos/` 整棵（FFI bindings）

**孤儿源（与 macOS 源完全一致的"未挂载"状态）**：
- `commands/skill_mcp.rs`（引用不存在的 `crate::mcp`，但 `commands/mod.rs` 没声明 `pub mod skill_mcp` → 不编入 → 不影响）
- `commands/{prompts,course_preview,asset_inference,knowledge_unit_learning,workspace_assets,calendar,app_mode}.rs` 同上
- `extraction/extractors/{image_ocr,pdf_scan}.rs`（`extractors/mod.rs` 注释掉了 → 不编入 → 不影响）

这些孤儿是项目历史遗留，**与本 mission 无关**，可忽略。

## 阻塞点（必须修才能让 macOS host buildable）

### B1：build.rs 在 macOS host 上 panic（首发阻塞）

`/tmp/ncw-test/notecapt-windows/src-tauri/build.rs`：

```rust
#[cfg(target_os = "macos")]
macos_bridges::build();   // 行 6，必然进入
```

`macos_bridges::build()` 在行 60-85 试图 `swiftc` 编译 `macos/asr_bridge.swift`、`macos/ocr_bridge.swift`，但 windows 仓没有这两个文件 → 立刻 panic。

实测 `cargo check --lib --offline` 输出（重点）：
```
<unknown>:0: error: error opening input file '/private/tmp/ncw-test/notecapt-windows/src-tauri/macos/asr_bridge.swift'
<unknown>:0: error: error opening input file '/private/tmp/ncw-test/notecapt-windows/src-tauri/macos/ocr_bridge.swift'
thread 'main' panicked at build.rs:85:9:
swiftc 编译 Swift bridge 失败
```

**根因**：subtree split 剥离了 `macos/` 目录和 `src/macos/` 模块，但 `build.rs` 这段 macOS-only 编译逻辑被完整保留下来。

**修复**：把 `build.rs` 改成 Windows 版本 — 删 `macos_bridges` 模块以及 `cfg(target_os = "macos")` 调用，仅保留 `tauri_build::build()` + `inject_bundled_creds()`。

### B2：`crate::macos::*` 残留引用（次发阻塞，被 B1 提前挡）

`extractors/mod.rs:4-5`：
```rust
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub mod audio_asr;
```

→ `extractors/audio_asr.rs` 在 macOS build 时会被编入；`audio_asr.rs:37-71` 在 `cfg(target_os = "macos")` 分支调用 `crate::macos::asr_ffi::transcribe_audio(...)`，但 windows 仓的 `lib.rs` 没声明 `pub mod macos`，编译必然 E0433 / E0432。

`image_ocr.rs` / `pdf_scan.rs` 也有同类引用，但 `extractors/mod.rs:6-8` 把它们注释掉了（不会编入），所以**不影响**。只有 `audio_asr.rs` 受影响。

**修复选项**：
- 选项 A（最小侵入）：把 `extractors/mod.rs:4` 改成 `#[cfg(target_os = "windows")]`，让 audio_asr 只在 Windows 编入。代价：macOS host 上不再能跑 audio_asr 自测，但本 mission 是 Windows 版，无意义；scheduler 也不引用它（已改走 `audio_asr_iflytek`）。
- 选项 B：把 `extractors/audio_asr.rs:37-71` macOS 分支删掉/替换为 `unreachable!()`，保留 mod 声明。代价更小，但和"audio_asr 是 macOS+Windows 共用"的设计语义违和。

推荐 **选项 A**：明确切断 macOS host 链路。

### 综合阻塞优先级

| 阻塞 | 类型 | 修复成本 | 影响范围 |
|---|---|---|---|
| B1 build.rs swift 段 | Build script | ~10 行删改 | 阻塞所有 macOS host cargo 命令 |
| B2 audio_asr macOS 分支 | 源码 cfg | ~1 行 cfg 改 | 阻塞 cargo check（B1 修后浮现） |

## 非阻塞但需用户决策的事项

| 项 | 说明 | 决策内容 |
|---|---|---|
| D1：本仓 `windowsversion/` 旧副本要不要删 | 当前 worktree `NCdesktop/windowsversion/` 是 Phase 4 状态的旧副本（缺 utils/testing/extraction/epub_text 等），与 remote 独立仓不同步。继续保留会让人误以为它是真相。 | 删除？保留？还是改成 git submodule 指向 notecapt-windows？ |
| D2：API key 来源 | OPENAI_API_KEY（OCR/ASR、LLM 用）、ARK_API_KEY（同 LLM 通道）；iflytek 已硬编码不用配 | 验收阶段是否提供测试用 OPENAI_API_KEY？还是只验"配置流程 + 不传 key 时的优雅降级"？ |
| D3：PDFium 依赖 | macOS host 上没有 `pdfium.dylib`，跑扫描型 PDF OCR 会 runtime 错；本 mission 想验吗？ | 跳过 pdf_scan 验证 / 改用 docker windows 测 / 仅在 Windows 实机让用户验 |
| D4：HEIC 支持 | 当前 Windows 版**不支持 HEIC OCR**（heic_convert 是占位）。BUILD-WINDOWS.md 已说明 | 接受占位状态？还是要 chris 推一份引入 windows crate + WinRT 的修复？ |
| D5：markitdown 运行时 | runtime-manifest 是 stub。Windows 上 markitdown 自检会失败 → 走 fallback chain（pdf_text/epub_text/docx/pptx/iflytek_asr/text）。hotfix #3 已支持。 | 接受 stub？还是要 chris 推完整 python-build-standalone + markitdown-venv 打包？ |
| D6：验收颗粒度 | "核心功能通过"边界在哪 | 看下方"功能链路验收清单"逐项确认 |

## 功能链路验收清单（草案，待用户确认）

### 必验（核心，决定 mission 是否完成）

| # | 链路 | macOS host 可验？ | Windows 实机可验？ | 建议谁来验 |
|---|---|---|---|---|
| 1 | `cargo check --lib` 通过 | 是（B1+B2 修后） | — | chris |
| 2 | `pnpm install` + `pnpm dev` + 主窗口空白页能渲染 | 是 | — | chris |
| 3 | `pnpm test`（vitest）通过 | 是 | — | chris |
| 4 | `pnpm tauri dev` 整个 stack 起来主窗口能渲染 | 是（前提 B1/B2 修） | 是 | chris(mac) + 用户(win) |
| 5 | 创建 Library / Project，写入 SQLite，重启后保留 | 是 | 是 | chris(mac) |
| 6 | 拖文件入 Dropzone → 入 Asset 表 | 是 | 是 | chris(mac) + 用户(win) |
| 7 | PDF 文本提取（`pdf-extract` 纯 Rust） | 是 | 是 | chris(mac) |
| 8 | EPUB 提取（hotfix #3 新 `epub_text`） | 是 | 是 | chris(mac) |
| 9 | LLM 集成（分类、概念提取）——需 API key | 是（如 D2 给 key） | 同 | 看 D2 |

### 可选（看 D2-D5）

| # | 链路 | 备注 |
|---|---|---|
| 10 | OpenAI Vision OCR | 需 D2 |
| 11 | OpenAI Whisper ASR | 需 D2 |
| 12 | iflytek ASR（凭据已硬编码） | 可直接验 |
| 13 | 扫描 PDF OCR（pdfium） | macOS host 不行；需 D3 |
| 14 | HEIC OCR | 不支持（D4） |
| 15 | MCP server | 孤儿模块，不影响 |

## 提议的修复 + 测试 Plan

### Phase 1：通 macOS host buildability（约 1-2 h）

**Step 1.1**：开新 hotfix branch `hotfix/macos-host-buildable`（基于 main 40926a7e）
**Step 1.2**：改 `build.rs`：删 macos_bridges 模块和 `cfg(target_os = "macos")` 调用，仅保留 tauri_build + inject_bundled_creds
**Step 1.3**：改 `extraction/extractors/mod.rs:4`：`cfg(any(macos, windows))` → `cfg(target_os = "windows")`（audio_asr 仅 Windows 编入）
**Step 1.4**：本地跑 `cargo check --lib` 验证通过；跑 `cargo test --lib`（fallback 测试 etc）验证通过
**Step 1.5**：开 PR #4（不 merge，先给用户看）
**决策点 D-1**：用户确认 plan 与 PR diff 后 merge

### Phase 2：开发环境验证（约 1-2 h）

**Step 2.1**：`pnpm install --frozen-lockfile`，验证 lockfile 一致、无版本冲突
**Step 2.2**：`pnpm test` 跑 vitest（hotfix #3 加的 EPUB fallback test 必须 pass）
**Step 2.3**：`pnpm dev`（仅前端），主对话/chris 用 chrome MCP 打开 http://localhost:5173，截图验证 React 应用至少能渲染
**Step 2.4**：`pnpm tauri dev` 起完整 stack，chris 用 computer-use 截图主窗口
**Step 2.5**：基础 IPC 烟测：创建 Library → 创建 Project → 拖一个 .txt → 看 SQLite 是否有记录
**Step 2.6**：选 1-2 个 fallback extractor（pdf_text / epub_text）做最小冒烟（带样本文件）
**决策点 D-2**：用户决定是否要继续做 LLM / OCR / ASR 集成测（参考 D2-D5）

### Phase 3：用户 Windows 实机回报（异步）

**Step 3.1**：chris 总结当前 Windows 版**已知未验证**功能 + **已知问题**清单
**Step 3.2**：用户在 Windows 主机上按 BUILD-WINDOWS.md 跑 `pnpm tauri dev` 和 `pnpm tauri build`
**Step 3.3**：用户回报结果，chris 根据回报决定下一轮 hotfix 范围
**决策点 D-3**：用户验完决定 mission 是否收尾（或开启 round 2 hotfix）

## 接下来 chris 想做的第一件事

得到用户对本 plan 的 **GO 信号**后，chris 进入 Phase 1 Step 1.1。

具体动作：
1. 让主对话在 `/tmp/ncw-test/notecapt-windows/` 开 `hotfix/macos-host-buildable` 分支（chris 不直接改代码）
2. 让主对话改 build.rs 和 extractors/mod.rs（diff 我会给精确补丁）
3. 让主对话跑 `cargo check --lib` 验证
4. 让主对话开 PR #4，把 URL 回给我
5. chris 把 PR 信息回给用户决策 merge

## 需要用户拍板的事

1. **D1 D2 D3 D4 D5 D6**：见上表
2. **整体 plan 是否 GO**：Phase 1 / 2 / 3 划分合理？
3. **是否允许 chris 让主对话开 hotfix PR**（不 merge）？
