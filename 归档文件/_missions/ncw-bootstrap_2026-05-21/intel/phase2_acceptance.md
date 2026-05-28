# Phase 2 必验清单（精确版）

工作目录：`/tmp/ncw-test/notecapt-windows/`（main @ `b52be92c`）
执行时间：2026-05-22
执行者：主对话 Claude 跑命令，chris 决策。

每项含：**前置条件 / 执行命令 / 观察什么 / PASS 判定 / FAIL 处置**。

---

## 1. `cargo check --lib` 通过

- **前置**：repo HEAD = `b52be92c`，`Cargo.lock` 存在
- **命令**：`cd /tmp/ncw-test/notecapt-windows/src-tauri && cargo check --lib`
- **观察**：stderr 末尾 + exit code
- **PASS**：exit 0，无 `error[` 行；只能有 `warning:`
- **FAIL 处置**：截 stderr 给 chris，定位是哪个 module；可能要再开 hotfix branch
- **状态**：Phase 1 已验过 8.58s exit 0；本 Phase 2 再跑一次确认

---

## 2. `cargo test --lib` 通过

- **前置**：项 1 PASS
- **命令**：`cd /tmp/ncw-test/notecapt-windows/src-tauri && cargo test --lib`
- **观察**：测试 summary 行（"X passed; Y failed"）
- **PASS**：`0 failed`；至少 1 个 test passed（hotfix #3 加的 EPUB fallback test）
- **FAIL 处置**：抓失败 test 名 + 错误片段给 chris
- **状态**：Phase 1 已验过 406 passed / 0 failed；本 Phase 2 再跑一次确认

---

## 3. `pnpm install --frozen-lockfile` 通过

- **前置**：repo top-level（不是 src-tauri）
- **命令**：`cd /tmp/ncw-test/notecapt-windows && pnpm install --frozen-lockfile`
- **观察**：summary 行 + exit code
- **PASS**：exit 0；不出 `ERR_PNPM_OUTDATED_LOCKFILE` / `lockfile does not match`
- **FAIL 处置**：如果是 lockfile drift，让主对话报告差异（不立即修），chris 决策

---

## 4. `pnpm test`（vitest）通过

- **前置**：项 3 PASS
- **命令**：`cd /tmp/ncw-test/notecapt-windows && pnpm test`
- **观察**：vitest summary + exit code
- **PASS**：exit 0；包括 hotfix #3 加的测试（搜索 `dropzone-import-detail.test.ts` 在结果中出现且 pass）
- **FAIL 处置**：抓失败 spec 给 chris

---

## 5. `pnpm dev`（仅前端，无 Tauri）能起来 + 主窗口能渲染

- **前置**：项 3 PASS
- **命令**：`cd /tmp/ncw-test/notecapt-windows && pnpm dev`（后台跑）
- **观察**：
  - stdout 出现 `Local:   http://localhost:5173/` 或类似
  - 用 Chrome MCP 打开 `http://localhost:5173/`
  - 截图 + 读 `document.title` + DOM 顶层结构
- **PASS**：
  - vite 启动无 error
  - 浏览器能加载（不是白屏 "Cannot GET /"）
  - 至少 React root mounted（document.body 有非空内容）
  - 控制台 0 个 "ReferenceError" / "SyntaxError" / "module not found"（runtime tauri API not found 可接受，因为非 tauri 模式）
- **FAIL 处置**：截图 + console logs 给 chris

---

## 6. `pnpm tauri dev` 起完整 stack + 主窗口能渲染

- **前置**：项 1+3 PASS；macOS host 上无 pdfium.dylib（验链路是否绕过 PDF 模块进 main）
- **命令**：`cd /tmp/ncw-test/notecapt-windows && pnpm tauri dev`（后台跑）
- **观察**：
  - Rust 编译 + Tauri 启动
  - 是否弹出 native window（用 computer-use 截图）
  - SQLite 初始化日志（一般打 `初始化默认 Library`）
- **PASS**：
  - cargo build dev mode 成功（exit 不为 1）
  - native window 真的出现
  - 窗口能看见 React UI（不是全白）
- **FAIL 处置**：抓 stderr 前 100 行给 chris；如果是 cargo build fail，就是 Phase 1 验证不充分；如果是 runtime panic 看哪个 IPC

---

## 7. 创建 Library + 重启后保留（SQLite 持久化）

- **前置**：项 6 起来且窗口可见
- **执行方法**（顺序）：
  1. 用 computer-use 在 app 窗口里点 "新建 Library" 按钮（或类似），输入名字 "test-lib-phase2"
  2. 关闭 app（停 `pnpm tauri dev`）
  3. 找到 dev mode 的 SQLite 文件位置（macOS 通常在 `~/Library/Application Support/com.notecapt.desktop.windows/` 或类似），用 sqlite3 查 libraries 表
  4. 重启 `pnpm tauri dev`，验证 UI 列表里有 "test-lib-phase2"
- **PASS**：
  - SQLite 文件存在
  - libraries 表里有 1 条 name="test-lib-phase2" 的记录
  - 重启后 UI 显示该 library
- **FAIL 处置**：抓 SQLite 路径 + UI 截图 + console error 给 chris
- **风险**：app 在 macOS 上的实际 bundle identifier 是 `com.notecapt.desktop.windows`，文件路径可能不在常规位置，需要先做发现

---

## 8. 拖文件入 Dropzone → 入 Asset 表

- **前置**：项 7 PASS
- **执行方法**：
  1. 创建一个临时 txt 文件 `/tmp/phase2-test.txt`，内容 "hello world"
  2. 用 computer-use 把它拖到 app 窗口的 Dropzone
  3. 观察 UI 是否显示该 asset
  4. 重启 app 验证持久化
  5. 查 SQLite assets 表
- **PASS**：
  - assets 表里有 1 条 source_path 包含 "phase2-test.txt" 的记录
  - UI 显示该 asset
- **FAIL 处置**：抓 IPC error / log 给 chris
- **风险**：computer-use 拖拽到 desktop app 可能不稳；可降级为通过命令模拟（如果 dev console 有 IPC eval 接口）

---

## 9. PDF 文本提取（pdf-extract 纯 Rust 路径）+ EPUB 提取（hotfix #3 新 epub_text）

- **前置**：项 8 PASS（基本 IPC 工作）
- **执行方法**：
  1. 准备 sample：`/tmp/phase2-test.pdf`（任意带文字 PDF；如果没有，跳到 EPUB only）和 `/tmp/phase2-test.epub`
  2. 拖入 app
  3. 触发 extraction（可能自动可能要点按钮）
  4. 观察 extraction 结果（文本是否提取出来）
  5. 查 extraction 相关表（assets / extractions / 类似）
- **PASS**：
  - PDF：assets 表对应记录的 extracted_text 字段（或类似）非空，含 PDF 原始文字片段
  - EPUB：同理；走 epub_text extractor，不走 markitdown
- **FAIL 处置**：抓 extraction log 给 chris；可能 fallback chain 有问题
- **风险**：扫描型 PDF（无文本层）会走 pdf_scan_detect 然后失败（macOS 上没 pdfium）；这是预期，必须用文本层 PDF

---

## 不验项（用户已拍板 D2-D5）

- ❌ OpenAI Vision OCR（依赖 OPENAI_API_KEY，D2 只验降级）
- ❌ OpenAI Whisper ASR（同上）
- ❌ 扫描 PDF OCR（D3 跳 macOS host）
- ❌ HEIC OCR（D4 接受占位）
- ❌ markitdown 完整实现（D5 接受 stub）

但要**验降级路径**：

### D2 降级验（合并到项 9 后做）

- 不设 OPENAI_API_KEY 时拖 PDF：必须走 pdf_text，不走 ocr_openai_vision
- 不设 OPENAI_API_KEY 时拖 mp3（如果有 sample）：iflytek 凭据已硬编码，应能走 audio_asr_iflytek

### D5 stub 验（合并到项 9 后做）

- runtime_check 在 markitdown stub 状态应短路返回 `unsupported`，触发 fallback chain
- 这是 hotfix #3 行为，应该 cargo test 已覆盖；项 9 实测一次

---

## 总执行顺序

```
项 1  (cargo check)         <- 已验，再跑一次
项 2  (cargo test)          <- 已验，再跑一次
项 3  (pnpm install)        <- 新
项 4  (pnpm test vitest)    <- 新
项 5  (pnpm dev + Chrome)   <- 新；并行项 3 后
项 6  (pnpm tauri dev)      <- 新；项 1+3 后
项 7  (Library 持久化)      <- 新；项 6 后
项 8  (Dropzone)            <- 新；项 7 后
项 9  (PDF + EPUB 提取)     <- 新；项 8 后
```

每项跑完，**主对话 Claude 把原始结果给 chris**，chris 做 PASS/FAIL 判定 → 决定继续还是修。
