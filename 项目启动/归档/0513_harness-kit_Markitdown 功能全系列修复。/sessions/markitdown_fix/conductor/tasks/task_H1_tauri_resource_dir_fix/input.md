# Task 输入 — task_H1_tauri_resource_dir_fix（HOTFIX）

## 背景
用户 2026-05-14 实测：`cargo tauri dev` 启动后所有 PDF/PNG 转录失败为 `E_RUNTIME_MISSING`。

**已确证根因**（不需要再调查）：
- `tauri.conf.json bundle.resources = []` 空数组
- Tauri 2.x dev 模式 `app.path().resource_dir()` 返回 `src-tauri/target/<profile>/` 而非 `src-tauri/resources/`
- `runtime_check.rs:86-93` `resource_dir.join("runtime-manifest.json")` 必然找不到文件
- `load_manifest` 第 156 行 `fs::read` 返回 ENOENT → `Err(FailureCode::ERuntimeMissing)`
- 失败缓存到 `RuntimeCheckState`，markitdown 路由前短路所有 PDF/image/docx/...

**venv 实际正确**：site-packages 完整、7 imports 全 PASS、PBS Python 跑得起。问题纯粹是路径解析。

## 目标
让 `cargo tauri dev` 与 DMG 安装两种模式都能正确解析 `runtime-manifest.json` + `markitdown-venv/bin/python` 路径。

## 验收标准

### AC-1：tauri.conf.json bundle.resources 增加 manifest
```jsonc
"bundle": {
  ...
  "resources": [
    "resources/runtime-manifest.json"
  ]
}
```
**仅添加 manifest（4KB），不加入 python/ 和 markitdown-venv/**（因为 363M 拷贝太重 + Tauri 2.x 对符号链接打包不友好；prod 路径继续由 `build-macos-dmg.sh:199-208` 手工 cp 保留）。

### AC-2：runtime_check.rs 加 dev fallback
- `verify_runtime_manifest()` 在 `app.path().resource_dir()` 返回的 manifest_path 不存在时，fallback 到 `CARGO_MANIFEST_DIR/resources/`（或等效解析为 `src-tauri/resources/`）
- 仅在 `cfg!(debug_assertions)` 启用 fallback（生产模式不 fallback 避免误覆盖 bug）
- venv_python 路径同步 fallback
- 实现可用 `option_env!("CARGO_MANIFEST_DIR")` 或 `std::env::var("CARGO_MANIFEST_DIR")` 推断 dev 源目录

### AC-3：lib.rs setup hook 增强错误日志
- 失败时 log 应明确指出**当前 resource_dir 路径 + manifest_path + 是否走 fallback**
- 便于未来用户报 bug 时 1 行 log 定位

### AC-4：单测
- mock manifest_path 不存在 + fallback path 存在 → 期望 fallback 成功，验证返回 PASS
- mock 双路径都不存在 → 期望 ERuntimeMissing
- 现有 3 测保留不破

### AC-5：实测验证
- 跑 `cargo tauri dev` 启动，日志含 `[runtime_check] OK runtime_id=ncdesktop-markitdown-runtime imports=7 elapsed_ms=...`
- 拖入 PDF 文件，conversion_meta 写入 status='success' + failure_code 为 NULL
- 拖入 PNG 文件，extractor_type='markitdown_image_fallback' + failure_code 为 NULL

### AC-6：保护 DMG 路径不退步
- DMG 模式（prod）仍按 `app.path().resource_dir() = .app/Contents/Resources/` 解析
- 不修改 `build-macos-dmg.sh`（task_006 PASS 边界）
- prod 模式即使 manifest 路径未通过 tauri.conf 被打包，由于 `build-macos-dmg.sh:199-208` 手工 cp 救回，仍可解析

## 严禁（红线）
- 修改 `build-macos-dmg.sh`、`prepare-embedded-*.sh`、`sign-bundle.sh`、`notarize.sh`（task_001~006 PASS 边界）
- 修改 `audio_asr_iflytek.rs`（PRD 底线 #4）
- 修改 `failure_code.rs`、`scheduler.rs`、`extractors/markitdown.rs`（task_007/008/010/011 PASS 边界；只能改 `runtime_check.rs` 路径解析段）
- 修改 `commands/sync.rs guess_mime`（task_H2 并行 dev 范围）
- 在 tauri.conf bundle.resources 加入 python/ 或 markitdown-venv/（363M 体积陷阱）
- 引入新依赖

## 预估影响范围
- 修改：`src-tauri/tauri.conf.json`（bundle.resources 加 1 项）
- 修改：`src-tauri/src/extraction/runtime_check.rs`（verify_runtime_manifest 加 dev fallback 段；约 +20 行）
- 修改：`src-tauri/src/lib.rs`（setup hook 错误日志增强；约 +5 行）

## 参考文件
- `runtime_check.rs:80-95` 当前路径解析
- `lib.rs:55-75` setup hook
- `tauri.conf.json` bundle 字段
- 用户实测 DB 记录：2026-05-14 01:39:28 markitdown failure_code=E_RUNTIME_MISSING
