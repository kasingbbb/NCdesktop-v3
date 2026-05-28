# Task 输入 — task_007_runtime_manifest_self_check

## 目标
在应用启动阶段实现一次性 `runtime-manifest.json` 自检：读 manifest → 逐个 `import` 7 关键模块 → 任一失败抛 `E_RUNTIME_MISSING` 或 `E_EXTRA_MISSING_<X>` 并禁用所有转录入口（UI 显式提示而非静默）。

## 前置条件
- 依赖 task：task_002（manifest 生成）、task_008（错误码枚举必须先落地）
- 必须先存在的文件/接口：`src-tauri/resources/runtime-manifest.json`、`FailureCode` 枚举

## 验收标准（Acceptance Criteria）
1. AC-1：新建 `src-tauri/src/extraction/runtime_check.rs`，提供 `fn verify_runtime_manifest(app: &AppHandle) -> Result<RuntimeManifest, FailureCode>`。
2. AC-2：实现：
   - 读 `Resources/runtime-manifest.json`；解析失败 → `E_RUNTIME_MISSING`；
   - 对 `imports` 数组每项依次 `Resources/markitdown-venv/bin/python -c "import X"` 带 10s 超时；
   - 任一失败 → 返回 `E_EXTRA_MISSING_<UPPER_X>`（如 `E_EXTRA_MISSING_EPUB` 对应 `ebooklib`）；映射表硬编码且单测覆盖。
3. AC-3：`lib.rs` 启动时调用一次，结果缓存到 `AppState`；后续 `markitdown::extract` 与 `scheduler` 路由前读缓存，失败时直接返回错误码不走子进程。
4. AC-4：UI 前端：自检失败时所有"转录/导入"入口 disabled 并显示横幅文案 + 错误码 + "一键复制诊断"按钮（PRD §4.3）。
5. AC-5：单测：
   - mock manifest 缺 `ebooklib` → 期望 `E_EXTRA_MISSING_EPUB`；
   - manifest 文件不存在 → 期望 `E_RUNTIME_MISSING`；
   - 7 项全 OK → 返回完整结构。
6. AC-6：自检结果写入 `log::info!`，包含 `runtime_id` 与耗时；不写敏感路径。

## 技术约束
- 严禁每次 `extract()` 重复探测（性能 + 与 ADR-010 冲突）。
- 严禁降级到系统 `python3`（H1）；自检本身只走 venv-shim 路径，失败即失败。
- 子进程必须超时 + 采集 stderr + `log::warn!`（session_context §5）。

## 参考文件
- ADR-010
- `src-tauri/src/extraction/scheduler.rs:528-540`（现有探测函数）
- PRD §3.1 F2

## 预估影响范围
- 新建文件：`src-tauri/src/extraction/runtime_check.rs`
- 修改文件：`src-tauri/src/lib.rs`、`src-tauri/src/extraction/mod.rs`、`src-tauri/src/extraction/scheduler.rs`、前端 `src/App.tsx` 或全局 banner 组件
