# Phase 2 进度报告（实时）

最后更新：2026-05-23

## 已完成

### 切片 A：rerun cargo（项 1, 2）
- **项 1 `cargo check --lib`**：PASS（3.55s exit 0，4 个无害 warning）
- **项 2 `cargo test --lib`**：PASS（406 passed / 0 failed / 3.49s）

### 切片 B 部分：JS toolchain
- **项 3 `pnpm install --frozen-lockfile`**：PASS（1.3s）
- **项 4 `pnpm test`（vitest）**：
  - **PR #5 已 merged**（vitest.setup.ts 补回；main HEAD = `6879fb8`）
  - 88.8% PASS；剩 9 套件 44 testcases fail = macOS 源预存 bug，**不在本 mission scope**
- **项 5 `pnpm dev`**：PASS（HTTP 层）

## 当前阻塞（2026-05-23）

### 切片 C 项 6 `pnpm tauri dev`：port 5173 冲突

**第一次尝试**：直接 `pnpm tauri dev` → 立即 fail，`Error: Port 5173 is already in use`。

**根因诊断**：
- `lsof -nP -iTCP:5173 -sTCP:LISTEN` → PID 96059 (node vite.js)
- 进一步追溯：`ps aux` 显示 PID 95852 (pnpm tauri dev) 在用户的 macOS 源目录跑着：
  ```
  /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/
  ```
  即**用户自己在 macOS 源那边开着 tauri dev**（PID 95852 → 95871 tauri.js → 96059 vite）。
- `curl localhost:5173/` → HTTP 200，确认是活的 vite server。

**决策点（需要老板拍板）**：

A. **用户停掉 macOS 源的 `pnpm tauri dev`**（PID 95852 链），mission 用 default 5173 跑
   - 优点：无需改任何配置
   - 缺点：打断用户当前工作
   - 估时：< 5 min（用户操作 + 我重启）

B. **mission 临时改 port 到 5174**（改 `/tmp/ncw-test/notecapt-windows/` 的 `vite.config.ts` + `tauri.conf.json`）
   - 优点：不打扰用户 macOS 源
   - 缺点：classifier 拒了我的尝试，理由是"未经用户明示授权改预存文件"
   - 我可以等老板明示允许后再改
   - 估时：< 1 min

C. **跳过项 6**（用 stub 验证：cargo build 不跑 + 报告这个 port 冲突无法继续）
   - 缺点：跳过 native window 验收
   - 不推荐

D. **改成在 `_missions/.../sessions/` 内重新 clone 一份独立的 `notecapt-windows`，自带改 port**
   - 优点：完全隔离
   - 缺点：再 clone + 再 pnpm install + 再 cargo build cache miss = 至少 10-30 min 多
   - 不推荐

**chris 推荐：A 或 B**。倾向 **B**（不打扰用户），但需要老板明示允许我改本地 clone 的两个 config 文件（这是 `/tmp/ncw-test/` 下的工作 clone，不是 macOS 源）。

### 当前本地 clone 状态
- 我刚才试着改了 5174，被 classifier 拒绝
- **已回滚**：`git status` clean，HEAD = `6879fb8`，无 diff
- 等老板决定

## 待做

### 切片 C 项 6（阻塞中）
- 等老板决策 port 冲突解法

### 切片 D 项 7-9（依赖项 6）
- 全部等项 6 解封后再开始

## 新发现

### macOS 源活动并行
- 老板在 macOS 源跑着 tauri dev
- 影响：5173 端口被占；可能后续还有其他资源冲突（如 SQLite 文件路径，bundle identifier `com.notecapt.desktop.windows` 两边一致）

### 关于 SQLite 路径冲突（前瞻）
- macOS 源 + windows fork **共用同一个 bundle identifier** `com.notecapt.desktop.windows`
- 意味着 dev mode 下两个进程会写同一个 SQLite 文件（`~/Library/Application Support/com.notecapt.desktop.windows/`）
- 项 7 验证"创建 Library + 重启后保留"时，需要：
  1. 老板先停 macOS 源的 tauri dev（不然两个进程同时写 db）
  2. 或者先备份 db 文件，验证完恢复
- 这是切片 D 的隐含前置条件

## 已知不修
- macOS 源 vitest 9 套件 44 testcases fail（D-vitest-fails）
- 不连 Chrome MCP（D-chrome；computer-use 截 native window 足够）
