# NoteCapt（NCdesktop）测试用例总表

> 本文档与仓库内自动化测试配套：**Rust**（`cargo test`）+ **前端**（`pnpm test`）。  
> 内置日志约定见下文「测试日志约定」。

**版本**：与当前代码库同步（2026-03-26）  
**应用路径**：`项目启动/NCdesktop/`

---

## 一、自动化测试（已接入）

| 类型 | 命令 | 说明 |
|------|------|------|
| Rust 单元测试 | `cd 项目启动/NCdesktop/src-tauri && cargo test` | 含 `classify_parse`、数据库迁移等 |
| 前端单元测试 | `cd 项目启动/NCdesktop && pnpm test` | Vitest，`src/**/*.test.ts(x)` |
| Rust 日志级别 | `RUST_LOG=info cargo test` | 看 `notecapt_test` / `log` 输出 |

**代码位置**

- Rust 测试初始化：`src-tauri/src/testing/mod.rs`（`init_test_logger`、`test_log!` 宏）
- LLM 分类解析用例：`src-tauri/src/llm/classify_parse.rs` → `#[cfg(test)] mod tests`
- 数据库迁移用例：`src-tauri/src/db/mod.rs` → `#[cfg(test)] mod tests`
- 前端测试日志工具：`src/lib/test-log.ts`；示例用例：`src/lib/__tests__/test-log.test.ts`

---

## 二、测试日志约定（内置）

| 层级 | 用法 | 目的 |
|------|------|------|
| Rust 测试 | 每个测试开头调用 `crate::testing::init_test_logger()`；关键步骤 `crate::test_log!("...", ...)` | `cargo test` 时输出带 `[TEST]` 前缀，便于 CI 日志检索 |
| 前端测试 | `testLog("info", "scope", "message", payload?)` | 统一 `[TEST][scope]` 前缀，与 Vitest 并存 |

**注意**：业务代码路径请继续使用 `log::info!` 等，勿高频调用测试专用宏。

---

## 三、用例列表（完整清单）

说明：**P0** 阻塞发布，**P1** 重要，**P2** 一般；**类型**为 自动 / 手动。

### 3.1 环境与构建

| ID | 优先级 | 类型 | 用例名称 | 前置条件 | 步骤 | 预期 |
|----|--------|------|----------|----------|------|------|
| TC-ENV-01 | P0 | 手动 | 开发启动 | 已安装 Rust、Node、pnpm | `pnpm tauri:dev` | 主窗口与 Vite 正常，无端口死锁 |
| TC-ENV-02 | P0 | 手动 | TypeScript 检查 | 依赖已安装 | `pnpm check` | 无 error |
| TC-ENV-03 | P0 | 手动 | ESLint | 依赖已安装 | `pnpm lint` | 无 error（warning 可记录） |
| TC-ENV-04 | P0 | 手动 | 前端生产构建 | 同上 | `pnpm build` | `dist` 生成成功 |
| TC-ENV-05 | P1 | 手动 | Tauri 生产构建 | 同上 | `pnpm tauri:build` | 产出 `.app`/安装包（依平台） |

### 3.2 Rust 单元测试（已实现）

| ID | 优先级 | 类型 | 用例名称 | 步骤 | 预期 |
|----|--------|------|----------|------|------|
| TC-RUST-01 | P0 | 自动 | 分类 JSON 纯文本解析 | `cargo test parse_plain_json` | pass |
| TC-RUST-02 | P0 | 自动 | 分类 JSON markdown 围栏 | `cargo test parse_markdown_fence` | pass |
| TC-RUST-03 | P0 | 自动 | 前缀废话 + JSON | `cargo test parse_extracts_from_prefix_text` | pass |
| TC-RUST-04 | P1 | 自动 | 缺字段默认补齐 | `cargo test parse_defaults_missing_fields` | pass |
| TC-RUST-05 | P0 | 自动 | DB 打开并迁移 | `cargo test open_runs_migrations` | `user_version >= 1` |

### 3.3 前端单元测试（已实现）

| ID | 优先级 | 类型 | 用例名称 | 步骤 | 预期 |
|----|--------|------|----------|------|------|
| TC-WEB-01 | P2 | 自动 | testLog 可调用 | `pnpm test` | 1 passed |

### 3.4 主窗口 — 知识库 / 项目

| ID | 优先级 | 类型 | 用例名称 | 前置条件 | 步骤 | 预期 |
|----|--------|------|----------|----------|------|------|
| TC-MAIN-01 | P0 | 手动 | 默认知识库 | 空库首次启动 | 打开应用 | 可创建或出现「默认知识库」逻辑与数据一致 |
| TC-MAIN-02 | P0 | 手动 | 新建项目 | 已选知识库 | ⌘N 或工具栏 New | 新项目出现在列表，`ui.active_project_id` 写入（切换项目验证） |
| TC-MAIN-03 | P1 | 手动 | 项目列表加载 | 库内有项目 | 进入项目列表视图 | 与 `get_projects` 一致 |
| TC-MAIN-04 | P1 | 手动 | 删除项目 | 存在测试项目 | 执行删除 | DB 与 UI 同步 |

### 3.5 悬浮窗 — 拖放导入

| ID | 优先级 | 类型 | 用例名称 | 前置条件 | 步骤 | 预期 |
|----|--------|------|----------|----------|------|------|
| TC-DZ-01 | P0 | 手动 | 打开悬浮窗 | 主程序已运行 | 触发显示悬浮窗 | 窗口可见，`/dropzone` 路由 |
| TC-DZ-02 | P0 | 手动 | 单文件拖入 | 已有项目或空库 | 拖入 `.txt` | `processing → complete`；无「没有可用的项目」 |
| TC-DZ-03 | P0 | 手动 | 文件落盘 | 拖入任意支持的文件 | 检查应用数据目录 | `.../assets/<projectId>/<assetId>_<文件名>` 存在 |
| TC-DZ-04 | P1 | 手动 | DB 素材记录 | 拖入后 | 主窗口打开对应项目 | 素材列表出现，`file_path` 指向落盘路径 |
| TC-DZ-05 | P1 | 手动 | `onDragDropEvent` | macOS | 拖入文件 | 使用 `event.payload`，阶段 attract/processing 正确 |
| TC-DZ-06 | P2 | 手动 | 文件夹拖入 | - | 拖目录 | 提示不支持或 failures 可读 |
| TC-DZ-07 | P1 | 手动 | 最近导入列表 | 展开悬浮窗 | 查看列表 | `done` 显示「已入库」，`error` 显示「失败」 |

### 3.6 LLM — 配置与分类

| ID | 优先级 | 类型 | 用例名称 | 前置条件 | 步骤 | 预期 |
|----|--------|------|----------|----------|------|------|
| TC-LLM-01 | P1 | 手动 | 未配置 Key | 不设置 `ARK_API_KEY` | 设置页 / 分类 | `get_llm_config` 提示未配置；拖入仍可入库 |
| TC-LLM-02 | P0 | 手动 | 方舟 Coding 配置 | 设置 `ARK_API_KEY`、`ARK_BASE_URL`、`ARK_MODEL` | 重启 `tauri:dev`，拖 txt | `ai_analyses` 有记录，`tags` 关联 |
| TC-LLM-03 | P1 | 手动 | Base URL | Coding / 在线推理两套 Base | 错误时切换 `ARK_BASE_URL` | HTTP 非 4xx 且可解析分类 JSON |
| TC-LLM-04 | P1 | 手动 | 摘要 | 已配置 LLM | 调用润色/摘要入口 | 返回非空文本 |

### 3.7 时间轴 / 素材 / 标签 / 笔记

| ID | 优先级 | 类型 | 用例名称 | 步骤 | 预期 |
|----|--------|------|----------|------|------|
| TC-ASSET-01 | P1 | 手动 | 素材 CRUD | 在项目内增删改素材 | DB 与 UI 一致 |
| TC-TAG-01 | P1 | 手动 | 标签关联 | AI 或手动打标签 | `asset_tags` 有记录 |
| TC-NOTE-01 | P2 | 手动 | 笔记增删改 | 编辑器操作 | `get_notes` 一致 |
| TC-TL-01 | P1 | 手动 | 时间轴加载 | 选中有 Timeline 的项目 | 轨道/关键帧接口不报 IPC 错 |

### 3.8 TF 卡同步（如已接硬件）

| ID | 优先级 | 类型 | 用例名称 | 前置条件 | 步骤 | 预期 |
|----|--------|------|----------|----------|------|------|
| TC-SYNC-01 | P1 | 手动 | 检测 TF | 插入合规卡 | `scan_tf_card` 类流程 | 返回设备信息或明确无设备 |
| TC-SYNC-02 | P1 | 手动 | 预览导入 | 有会话数据 | preview | 列表与 manifest 一致 |
| TC-SYNC-03 | P0 | 手动 | 执行导入 | 用户确认 | import | 项目/素材/时间轴写入 |

### 3.9 搜索 / 导出 / 系统集成

| ID | 优先级 | 类型 | 用例名称 | 步骤 | 预期 |
|----|--------|------|----------|------|------|
| TC-SEARCH-01 | P1 | 手动 | 全局搜索 ⌘K | 打开搜索面板 | 输入关键词 | 有结果或空态正确 |
| TC-EXPORT-01 | P1 | 手动 | Markdown 导出 | 选项目导出 | 文件/剪贴板内容含标题与素材 |
| TC-EXPORT-02 | P2 | 手动 | LLM 增强导出 | 已配置 LLM | 增强开关 | 输出与原文语义保留 |
| TC-SYS-01 | P2 | 手动 | 全局快捷键 | - | ⌘K / ⌘N 等 | 与 `useGlobalShortcuts` 一致 |
| TC-SYS-02 | P2 | 手动 | 权限 | - | 悬浮窗 `invoke` | `capabilities` 含 `dropzone` |

### 3.10 非功能（性能 / 安全）

| ID | 优先级 | 类型 | 用例名称 | 预期 |
|----|--------|------|----------|------|
| TC-NFR-01 | P2 | 手动 | 冷启动体量 | 开发模式下可接受；发布需对齐宪章指标 |
| TC-NFR-02 | P0 | 手动 | API Key | Key **仅环境变量**，勿写入仓库 |
| TC-NFR-03 | P1 | 手动 | CSP / IPC | 敏感命令需 capabilities |

---

## 四、建议的回归顺序（冒烟）

1. `pnpm check && pnpm lint && pnpm test`  
2. `cd src-tauri && cargo test`  
3. `pnpm tauri:dev`：主窗口 + 新建项目 + 悬浮窗拖 txt + 看 DB / 资产目录 +（可选）LLM  

---

## 五、后续可补齐的自动化方向（未实现）

- Rust：`import_drop_paths` 在临时目录上的集成测试（需 mock `AppHandle::path()` 较重组件，可抽纯函数测路径解析）。  
- 前端：React 组件 + `@testing-library/react`，需加回 `jsdom` 与 `vitest.setup.ts`。  
- E2E：使用 `tauri-driver` 或 macOS UI 自动化，对悬浮窗拖放做端到端。

---

*文档结束。*
