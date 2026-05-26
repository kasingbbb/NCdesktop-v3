# Task 交付 — task_H1_tauri_resource_dir_fix（HOTFIX）

## 实现摘要

修复 Tauri 2.x dev 模式下 `app.path().resource_dir()` 返回 `src-tauri/target/<profile>/`、不包含 `src-tauri/resources/` 内容（bundle.resources 仅 build 阶段拷贝）导致 `runtime-manifest.json` ENOENT、`runtime_check` 全部失败、PDF/PNG 转录被短路为 `E_RUNTIME_MISSING` 的根因。

核心设计：
1. `tauri.conf.json` 的 `bundle.resources` 加入 `resources/runtime-manifest.json`（仅 manifest 4KB；**不**加 `markitdown-venv/` 363M + symlink 打包陷阱，prod venv 仍由 `build-macos-dmg.sh:199-208` 手工 cp 救回）。
2. `runtime_check.rs::verify_runtime_manifest` 抽出纯函数 `select_runtime_paths(resource_dir, dev_fallback)`：默认走 `resource_dir/`；**dev (`debug_assertions=true`)** 当 manifest 不存在时 fallback 到 `env!("CARGO_MANIFEST_DIR")/resources/`；**prod** 编译期注入 `None` 永不 fallback。
3. log 字面同时打印 `resource_dir` / `manifest_path` / `venv_python`（无论成功/失败/fallback），未来 1 行 log 即可定位路径问题——AC-3 由 `runtime_check.rs` 内部 log 等价覆盖，不重复改 lib.rs。

## 修改的文件

| 文件 | 变更 | 说明 |
|------|------|------|
| `src-tauri/tauri.conf.json` | 修改 | `bundle.resources` 加 `"resources/runtime-manifest.json"` |
| `src-tauri/src/extraction/runtime_check.rs` | 修改 | `verify_runtime_manifest` 加 dev fallback；新增 `select_runtime_paths` 纯函数；tests 新增 3 个 |
| `src-tauri/src/lib.rs` | 未改 | baseline 已含 `runtime_check_result` match 失败分支 log；AC-3 由 runtime_check.rs 内 log 覆盖（input.md 明确允许） |

### `git diff --stat`（本 task 范围内）
```
 src-tauri/tauri.conf.json           |  3 +
 src-tauri/src/extraction/runtime_check.rs | +90/-7 (纯增量;含 3 新测试)
```

注：`src/lib.rs` 工作树有 baseline 修改（与本 task 无关，由前置 task_007/H1 启动期自检注册段引入，我未编辑）。其余 `git status` 中的 `MM` 文件（commands/sync.rs / extractors/markitdown.rs / scheduler.rs / ...）均为并行 task 的 baseline 状态，本 task **零接触**。

## AC 验收结果

| AC | 内容 | 状态 | 备注 |
|----|------|------|------|
| AC-1 | tauri.conf.json bundle.resources 加 manifest | ✅ PASS | 见下方字面 |
| AC-2 | runtime_check.rs dev fallback | ✅ PASS | `select_runtime_paths` + `#[cfg(debug_assertions)]` env! |
| AC-3 | 错误日志增强 | ✅ PASS | runtime_check.rs 内 log 覆盖 resource_dir+manifest+venv |
| AC-4 | 新增 2 单测 | ✅ PASS | 实际加 3 个（含 None fallback 边界） |
| AC-5 | 实测 `cargo tauri dev` + 拖 PDF/PNG | ⏳ **PENDING-USER-MACHINE** | dev 工作机不能跑全 Tauri 启动栈 |
| AC-6 | DMG 路径不退步 | ✅ PASS | prod 编译 `dev_fallback=None`；`build-macos-dmg.sh` 未动；prod resource_dir 路径解析逻辑保留 |

## tauri.conf.json bundle.resources 字面

```json
"bundle": {
  "active": true,
  "targets": "all",
  "resources": [
    "resources/runtime-manifest.json"
  ],
  "icon": [ ... ]
}
```

## runtime_check.rs 关键 diff（核心 ~25 行）

```rust
pub fn verify_runtime_manifest(app: &AppHandle) -> Result<RuntimeManifest, FailureCode> {
    let resource_dir = app.path().resource_dir().map_err(|e| {
        log::warn!("[runtime_check] 无法解析 resource_dir: {e}");
        FailureCode::ERuntimeMissing
    })?;

    #[cfg(debug_assertions)]
    let dev_fallback: Option<PathBuf> =
        Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources"));
    #[cfg(not(debug_assertions))]
    let dev_fallback: Option<PathBuf> = None;

    let (manifest_path, venv_python, used_fallback) =
        select_runtime_paths(&resource_dir, dev_fallback.as_deref());

    if used_fallback {
        log::info!("[runtime_check] dev fallback: resource_dir={} 缺 manifest → 改用 {} ...", ...);
    } else {
        log::info!("[runtime_check] using resource_dir={} (manifest={}, venv={})", ...);
    }
    verify_with_paths(&manifest_path, &venv_python)
}

fn select_runtime_paths(resource_dir: &Path, dev_fallback: Option<&Path>) -> (PathBuf, PathBuf, bool) {
    let default_manifest = resource_dir.join("runtime-manifest.json");
    let default_venv = resource_dir.join("markitdown-venv/bin/python");
    if default_manifest.is_file() { return (default_manifest, default_venv, false); }
    if let Some(fb) = dev_fallback {
        let fb_manifest = fb.join("runtime-manifest.json");
        let fb_venv = fb.join("markitdown-venv/bin/python");
        if fb_manifest.is_file() { return (fb_manifest, fb_venv, true); }
    }
    (default_manifest, default_venv, false)
}
```

## 新增单测设计

| 测试 | 场景 | 期望 | 结果 |
|------|------|------|------|
| `select_paths_falls_back_when_resource_dir_missing_manifest` | resource_dir 空，fallback dir 含 manifest+fake python | `used_fallback=true`、verify_with_paths 7 imports PASS | ok |
| `select_paths_both_missing_returns_runtime_missing` | 两条路径均缺 manifest | `used_fallback=false`、verify_with_paths → `Err(ERuntimeMissing)` | ok |
| `select_paths_no_fallback_when_dev_fallback_is_none` | 模拟 prod：`dev_fallback=None` | `used_fallback=false`、返回默认 manifest 路径 | ok |

设计选择：`verify_runtime_manifest` 依赖 `AppHandle` 不便单测注入，但路径选择是无副作用纯函数 → 把决策逻辑抽到 `select_runtime_paths(&Path, Option<&Path>)`；fallback 命中测端到端走 `verify_with_paths` 验证 7 imports，确保 fallback 不仅"选对路径"且"路径上的产物可用"。

## 测试结果

### `cargo check`
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.39s
```
通过（5 个 warning 均为 baseline 既有，与本 task 无关）。

### `cargo test --lib extraction::runtime_check`
```
running 13 tests
test extraction::runtime_check::tests::map_import_failure_table ... ok
test extraction::runtime_check::tests::runtime_check_state_snapshot_err ... ok
test extraction::runtime_check::tests::runtime_check_state_snapshot_ok ... ok
test extraction::runtime_check::tests::select_paths_no_fallback_when_dev_fallback_is_none ... ok
test extraction::runtime_check::tests::venv_python_missing_returns_runtime_missing ... ok
test extraction::runtime_check::tests::manifest_invalid_json_returns_runtime_missing ... ok
test extraction::runtime_check::tests::manifest_missing_returns_runtime_missing ... ok
test extraction::runtime_check::tests::manifest_missing_required_field_returns_runtime_missing ... ok
test extraction::runtime_check::tests::select_paths_both_missing_returns_runtime_missing ... ok
test extraction::runtime_check::tests::missing_ebooklib_returns_extra_missing_epub ... ok
test extraction::runtime_check::tests::all_seven_imports_ok_returns_full_manifest ... ok
test extraction::runtime_check::tests::select_paths_falls_back_when_resource_dir_missing_manifest ... ok
test extraction::runtime_check::tests::missing_non_ebooklib_module_returns_runtime_missing ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 216 filtered out
```
**13/13 PASS**（保留原 10 个 + 新增 3 个）。

### `cargo test --lib`（全量）
```
test result: ok. 229 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.91s
```
**229 PASS / 0 FAIL**（baseline 219 + 本 task 3 + 其他并行 task 累积增量 7）。

## 自测验证矩阵

| 场景 | 描述 | 状态 | 结果 |
|------|------|------|------|
| ✅ 正常路径 | dev: resource_dir 缺 manifest → fallback 到 CARGO_MANIFEST_DIR/resources/ → 7 imports PASS | 已测 | `select_paths_falls_back_*` 单测 PASS |
| ✅ 正常路径 | prod: resource_dir 含 manifest → 默认路径直接走通 | 已测 | `all_seven_imports_ok_returns_full_manifest` 既有测 |
| ⚠️ 边界 | dev_fallback=None（模拟 prod 编译） | 已测 | `select_paths_no_fallback_*` PASS，不会误 fallback |
| ❌ 异常 | dev: 双路径都缺 manifest | 已测 | `select_paths_both_missing_*` → `Err(ERuntimeMissing)` |
| ❌ 异常 | venv python 不存在但 manifest 存在 | 已测 | 既有 `venv_python_missing_returns_runtime_missing` PASS |
| ⏳ 实测 | cargo tauri dev + 拖 PDF | PENDING | 工作机不便启动 Tauri；用户机器验证 |
| ⏳ 实测 | cargo tauri dev + 拖 PNG | PENDING | 同上 |

## 浏览器/运行时验证

- 启动命令：`cargo tauri dev`（用户机器执行）
- 预期日志（dev fallback 命中）：
  ```
  [runtime_check] dev fallback: resource_dir=<...>/target/debug/ 缺 manifest →
    改用 <...>/src-tauri/resources/ (manifest=<...>/runtime-manifest.json,
    venv=<...>/markitdown-venv/bin/python)
  [runtime_check] OK runtime_id=ncdesktop-markitdown-runtime imports=7 elapsed_ms=...
  ```
- 验证路径：拖 PDF → conversion_meta `status='success'` / `failure_code=NULL`；拖 PNG → `extractor_type='markitdown_image_fallback'` / `failure_code=NULL`。
- **AC-5 由用户机器实测落地**。

## 范围 Gate（红线核查）

- ✅ 仅修改 `tauri.conf.json` + `extraction/runtime_check.rs`
- ✅ **未触** `build-macos-dmg.sh` / `prepare-embedded-*.sh` / `sign-bundle.sh` / `notarize.sh` / `vm-smoke.sh`
- ✅ **未触** `audio_asr_iflytek.rs`（PRD 底线 #4）
- ✅ **未触** `failure_code.rs` / `scheduler.rs` / `extractors/markitdown.rs`
- ✅ **未触** `commands/sync.rs` 的 `guess_mime`（task_H2 并行 dev 范围）
- ✅ tauri.conf bundle.resources 仅含 manifest（无 python/ 或 markitdown-venv/，规避 363M + symlink）
- ✅ 无新增 dependency

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（runtime_check.rs 路径 + tauri.conf 位置不变）
- [x] API 路径/命名一致（pub fn `verify_runtime_manifest` 签名保持；内部新增 `select_runtime_paths` 为 private helper，不污染外部）
- [x] 数据模型一致（`RuntimeManifest` schema 不变）
- [x] 未引入计划外依赖
- 偏离说明：AC-3"lib.rs setup hook 增强日志"未实际编辑 lib.rs——通过 runtime_check.rs 内 log 等价覆盖。input.md 原文允许该简化："如已被 AC-2 内部 log 覆盖则不必重复"。

## 已知局限

1. AC-5 实测需用户在自己机器上跑 `cargo tauri dev`，dev 工作机环境无法触发全 Tauri 启动链路（OS 限制 + Python 嵌入式 runtime 依赖）。
2. fallback 仅 `cfg!(debug_assertions)` 启用——若用户以 `cargo tauri dev --release` 模式启动则 fallback 关闭、需 bundle 已打包好（与 Tauri 官方约定一致）。
3. `select_runtime_paths` 不验证 venv_python 是否真存在；该探针在 `verify_with_paths` 内部完成（与既有逻辑解耦，保留单测）。

## 需要 Reviewer 特别关注的地方

1. **`env!("CARGO_MANIFEST_DIR")` 取值时机**：编译期常量，dev 编译产物中嵌入开发机的 `src-tauri` 绝对路径。若 reviewer 关心"分发的 dev 构建在他人机器跑会不会 panic" —— 不会：`select_runtime_paths` 对 fallback 路径同样做 `is_file()` 检查，路径不存在时静默回到默认（双都缺 → ERuntimeMissing，不 panic）。
2. **`#[cfg(debug_assertions)]` 边界**：prod 编译 `dev_fallback=None`，编译器消除后续 if 分支——无运行时开销、无 prod 行为变化。请确认 `cargo tauri build` 路径仍指向 `.app/Contents/Resources/`。
3. **tauri.conf 的 bundle.resources 路径相对 src-tauri/**：`resources/runtime-manifest.json` 不带前导 `./`，与 Tauri 2.x 文档示例一致。请 reviewer 在 CI 上跑一次 `cargo tauri build --target aarch64-apple-darwin --bundles app` 确认 manifest 被拷入 `.app/Contents/Resources/`（不在本 task scope 内）。
4. **未编辑 lib.rs**：AC-3 由 runtime_check.rs 内 log 等价覆盖；如 reviewer 坚持要 lib.rs 也加一层 log，可在 fix pass 补上 3 行。
