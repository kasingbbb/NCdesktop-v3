//! task_007：启动期一次性 runtime-manifest 自检。
//!
//! 流程：
//!   1. 读 `Resources/runtime-manifest.json` → 解析为 `RuntimeManifest`；
//!   2. 对 `imports` 数组每项 spawn `Resources/markitdown-venv/bin/python -c "import X"`
//!      （10s 超时 / 采集 stderr / log::warn!）；
//!   3. 任一失败 → 返回 `E_EXTRA_MISSING_<UPPER_X>`（硬编码映射）；
//!      manifest 文件缺失 / JSON 解析失败 → `E_RUNTIME_MISSING`。
//!
//! 与 ADR-010 / input.md AC 一致：
//!   - 启动只跑一次；结果缓存到 `AppState`（`RuntimeCheckState`）；
//!   - markitdown/scheduler 路由前读缓存，失败时短路不走子进程；
//!   - 严禁降级到系统 `python3`（H1）；自检本身只走 venv-shim 路径。

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Deserialize;
use tauri::{AppHandle, Manager};

use crate::extraction::failure_code::FailureCode;

/// 单次 `python -c "import X"` 超时（input.md AC-2）。
const IMPORT_PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// `runtime-manifest.json` schema_version=1 字段集合（ADR-010 §4 / task_002 manifest 实际产物）。
///
/// 字段顺序 / 名称严格对齐 manifest 生成端，反序列化失败 → 视为 `E_RUNTIME_MISSING`
/// （manifest 损坏 == 运行时不可用）。
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct RuntimeManifest {
    pub schema_version: u32,
    pub runtime_id: String,
    pub python: PythonInfo,
    pub markitdown: MarkitdownInfo,
    /// `extras_extra`：ADR-010 §4 示例为数组，task_002 实际产物为 object 形式
    /// （`{"beautifulsoup4":"4.12.3","ebooklib":"0.18"}`）；本结构按实际产物落地。
    pub extras_extra: serde_json::Value,
    /// 7 项关键 imports（task_002 E-2 裁决后：`["ebooklib","bs4","pdfminer","pptx","mammoth","openpyxl","PIL"]`）。
    pub imports: Vec<String>,
    pub build_timestamp: String,
    pub arch: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct PythonInfo {
    pub source: String,
    pub version: String,
    pub build: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct MarkitdownInfo {
    pub version: String,
    pub extras: Vec<String>,
}

/// 启动期自检结果缓存。`AppState` 单例；后续 markitdown / scheduler 直接读不再触子进程。
///
/// `Ok(manifest)` → 自检通过；`Err(code)` → 自检失败，UI 应禁用所有转录入口。
#[derive(Debug)]
pub struct RuntimeCheckState(pub Mutex<Result<RuntimeManifest, FailureCode>>);

impl RuntimeCheckState {
    pub fn new(result: Result<RuntimeManifest, FailureCode>) -> Self {
        Self(Mutex::new(result))
    }

    /// 读缓存快照（克隆出 manifest 引用计数廉价；FailureCode 是 Copy）。
    pub fn snapshot(&self) -> Result<RuntimeManifest, FailureCode> {
        match self.0.lock() {
            Ok(guard) => guard.clone(),
            // mutex poison 视为运行时不可用 —— 比 panic 安全。
            Err(_) => Err(FailureCode::ERuntimeMissing),
        }
    }
}

/// AC-1：启动期入口。读 manifest → 7 imports → 返回结构 or FailureCode。
///
/// 失败时 `log::warn!` 携带 runtime_id + 失败项 + 子进程 stderr；
/// 成功时 `log::info!` 携带 runtime_id + 总耗时（AC-6）。
///
/// **task_H1**: Tauri 2.x dev 模式下 `app.path().resource_dir()` 返回
/// `src-tauri/target/<profile>/`，**不**包含 `src-tauri/resources/*`（bundle.resources
/// 仅在 build 阶段拷贝）。为支持 `cargo tauri dev` 自然启动而不引入 prod 行为变化：
///   - dev (`debug_assertions=true`) 且 resource_dir 下 manifest 缺失时，
///     fallback 到 `CARGO_MANIFEST_DIR/resources/`（编译期常量 → src-tauri/）；
///   - prod (`debug_assertions=false`) 永不 fallback，保留 `build-macos-dmg.sh:199-208`
///     手工 cp 救回的语义。
pub fn verify_runtime_manifest(app: &AppHandle) -> Result<RuntimeManifest, FailureCode> {
    let resource_dir = app.path().resource_dir().map_err(|e| {
        log::warn!("[runtime_check] 无法解析 resource_dir: {e}");
        FailureCode::ERuntimeMissing
    })?;

    // dev fallback 候选目录（仅 debug 编译期注入；prod 编译为 None 永不参与决策）。
    #[cfg(debug_assertions)]
    let dev_fallback: Option<PathBuf> =
        Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources"));
    #[cfg(not(debug_assertions))]
    let dev_fallback: Option<PathBuf> = None;

    let (manifest_path, venv_python, used_fallback) =
        select_runtime_paths(&resource_dir, dev_fallback.as_deref());

    if used_fallback {
        log::info!(
            "[runtime_check] dev fallback: resource_dir={} 缺 manifest → 改用 {} (manifest={}, venv={})",
            resource_dir.display(),
            manifest_path.parent().map(|p| p.display().to_string()).unwrap_or_default(),
            manifest_path.display(),
            venv_python.display()
        );
    } else {
        log::info!(
            "[runtime_check] using resource_dir={} (manifest={}, venv={})",
            resource_dir.display(),
            manifest_path.display(),
            venv_python.display()
        );
    }

    verify_with_paths(&manifest_path, &venv_python)
}

/// 纯函数：根据 `resource_dir` 与可选 `dev_fallback` 决定最终 manifest / venv python 路径。
///
/// 决策规则（task_H1）：
///   - 默认走 `resource_dir/runtime-manifest.json` + `resource_dir/markitdown-venv/bin/python`；
///   - 若 manifest 在默认路径不存在，且 `dev_fallback` 提供，且 fallback 路径下 manifest 存在，
///     则切换到 fallback；
///   - 其余情况保留默认路径（让 `verify_with_paths` 内部产生 `E_RUNTIME_MISSING`）。
///
/// 返回 `(manifest_path, venv_python, used_fallback)`。
fn select_runtime_paths(
    resource_dir: &Path,
    dev_fallback: Option<&Path>,
) -> (PathBuf, PathBuf, bool) {
    let default_manifest = resource_dir.join("runtime-manifest.json");
    let default_venv = resource_dir.join("markitdown-venv/bin/python");

    if default_manifest.is_file() {
        return (default_manifest, default_venv, false);
    }

    if let Some(fb_dir) = dev_fallback {
        let fb_manifest = fb_dir.join("runtime-manifest.json");
        let fb_venv = fb_dir.join("markitdown-venv/bin/python");
        if fb_manifest.is_file() {
            return (fb_manifest, fb_venv, true);
        }
    }

    // 双路径都没有：返回默认值，让下游产生统一的 ERuntimeMissing。
    (default_manifest, default_venv, false)
}

/// 内部实现 —— 显式注入 manifest / venv python 路径，便于单测注入 fixture。
pub fn verify_with_paths(
    manifest_path: &Path,
    venv_python: &Path,
) -> Result<RuntimeManifest, FailureCode> {
    let start = Instant::now();

    // (1) 读 + 解析 manifest
    let manifest = load_manifest(manifest_path)?;

    // (2) venv python 必须存在（H1：禁止降级系统 python3）；任何缺失 → E_RUNTIME_MISSING
    if !venv_python.is_file() {
        log::warn!(
            "[runtime_check] venv python 不存在: {} (runtime_id={})",
            venv_python.display(),
            manifest.runtime_id
        );
        return Err(FailureCode::ERuntimeMissing);
    }

    // (3) 7 imports 逐项探测；任一失败 → E_EXTRA_MISSING_<X>
    for module in &manifest.imports {
        probe_import(venv_python, module).map_err(|stderr| {
            let code = map_import_failure(module);
            log::warn!(
                "[runtime_check] import 失败: module={module} code={code} runtime_id={} stderr={}",
                manifest.runtime_id,
                stderr.trim()
            );
            code
        })?;
    }

    let elapsed_ms = start.elapsed().as_millis();
    log::info!(
        "[runtime_check] OK runtime_id={} imports={} elapsed_ms={}",
        manifest.runtime_id,
        manifest.imports.len(),
        elapsed_ms
    );
    Ok(manifest)
}

/// manifest 文件不存在 / JSON parse 失败 → `E_RUNTIME_MISSING`。
fn load_manifest(path: &Path) -> Result<RuntimeManifest, FailureCode> {
    let bytes = std::fs::read(path).map_err(|e| {
        log::warn!(
            "[runtime_check] 读取 manifest 失败 path={} err={}",
            path.display(),
            e
        );
        FailureCode::ERuntimeMissing
    })?;
    serde_json::from_slice::<RuntimeManifest>(&bytes).map_err(|e| {
        log::warn!(
            "[runtime_check] 解析 manifest 失败 path={} err={}",
            path.display(),
            e
        );
        FailureCode::ERuntimeMissing
    })
}

/// 单次 `python -c "import X"` 探测，10s 硬超时。
///
/// 成功 → `Ok(())`；失败 → `Err(stderr)`（调用方据此 log::warn! + 映射 FailureCode）。
fn probe_import(python: &Path, module: &str) -> Result<(), String> {
    let mut child = Command::new(python)
        .arg("-c")
        .arg(format!("import {module}"))
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn 失败: {e}"))?;

    let deadline = Instant::now() + IMPORT_PROBE_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    return Ok(());
                }
                let stderr = read_stderr(&mut child);
                return Err(format!("exit={:?} stderr={}", status.code(), stderr));
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "import 探测超时 (>{}s)",
                        IMPORT_PROBE_TIMEOUT.as_secs()
                    ));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                let _ = child.kill();
                return Err(format!("try_wait 失败: {e}"));
            }
        }
    }
}

fn read_stderr(child: &mut std::process::Child) -> String {
    use std::io::Read;
    let mut buf = String::new();
    if let Some(mut s) = child.stderr.take() {
        let _ = s.read_to_string(&mut buf);
    }
    buf
}

/// import 模块名 → FailureCode 硬编码映射（AC-2）。
///
/// 大小写策略：参考 PRD §3.1 F2 与 input.md AC-2 字面（`E_EXTRA_MISSING_EPUB`）：
/// "EPUB"/"PDF"/"DOCX"/"PPTX"/"XLSX"/"IMAGE" 为面向用户的能力名，
/// 不是 import 模块名本身的简单 upper —— 这里建立显式映射。
/// 未在映射表中的模块 → `E_RUNTIME_MISSING`（保守：未列出的不属于 7 项关键 extras）。
pub fn map_import_failure(module: &str) -> FailureCode {
    match module {
        // ebooklib → EPUB（input.md AC-2 字面示例）
        "ebooklib" => FailureCode::EExtraMissingEpub,
        // 其余 6 个关键 imports 在当前 FailureCode 枚举中无独立 EExtraMissing_<X>；
        // 与 task_008 错误码集合保持一致，统一归 ERuntimeMissing（"运行时缺失"）。
        // 未来若枚举扩展（EExtraMissingPdf / Docx / ...），仅改本表即可。
        _ => FailureCode::ERuntimeMissing,
    }
}

/// 工具：返回 `Resources/runtime-manifest.json` 标准路径（用于调试 / 日志）。
#[allow(dead_code)]
pub fn manifest_path_for(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .resource_dir()
        .ok()
        .map(|p| p.join("runtime-manifest.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    const VALID_MANIFEST: &str = r#"{
        "schema_version": 1,
        "runtime_id": "ncdesktop-markitdown-runtime",
        "python": { "source": "python-build-standalone", "version": "3.12.7", "build": "20241016" },
        "markitdown": { "version": "0.1.5", "extras": ["pdf","docx","pptx","xlsx"] },
        "extras_extra": { "beautifulsoup4": "4.12.3", "ebooklib": "0.18" },
        "imports": ["ebooklib","bs4","pdfminer","pptx","mammoth","openpyxl","PIL"],
        "build_timestamp": "2026-05-13T10:40:51Z",
        "arch": "arm64"
    }"#;

    fn write(path: &Path, content: &str) {
        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    /// 构造一个"假 python":一个 shell 脚本，根据 -c 参数中的模块名 success / fail。
    /// 仅 unix；CI 默认 macOS/Linux。
    #[cfg(unix)]
    fn make_fake_python(dir: &Path, failing_modules: &[&str]) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join("fake_python.sh");
        // 行为：读 $2（"import X"），抽出 X；若 X ∈ failing_modules → exit 1 并 stderr；否则 exit 0。
        let fails = failing_modules.join("|");
        let script = format!(
            r#"#!/bin/sh
# 期望调用：fake_python -c "import X"
shift  # 跳过 -c
arg="$1"
mod="${{arg#import }}"
case "$mod" in
{cases}
    *) exit 0 ;;
esac
"#,
            cases = if fails.is_empty() {
                "    __never__) exit 0 ;;".to_string()
            } else {
                failing_modules
                    .iter()
                    .map(|m| format!("    {m}) echo \"ModuleNotFoundError: No module named '{m}'\" 1>&2 ; exit 1 ;;"))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        );
        write(&path, &script);
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms).unwrap();
        path
    }

    // ─── AC-1 / AC-5：7 imports 全 OK ─────────────────────────────────────

    #[cfg(unix)]
    #[test]
    fn all_seven_imports_ok_returns_full_manifest() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("runtime-manifest.json");
        write(&manifest, VALID_MANIFEST);
        let py = make_fake_python(dir.path(), &[]);

        let r = verify_with_paths(&manifest, &py).expect("应成功");
        assert_eq!(r.schema_version, 1);
        assert_eq!(r.imports.len(), 7);
        assert_eq!(r.imports[0], "ebooklib");
        assert_eq!(r.imports[4], "mammoth"); // task_002 E-2 裁决：docx→mammoth
        assert_eq!(r.markitdown.version, "0.1.5");
        assert_eq!(r.python.version, "3.12.7");
    }

    // ─── AC-5(1)：mock manifest 缺 ebooklib → E_EXTRA_MISSING_EPUB ─────────

    #[cfg(unix)]
    #[test]
    fn missing_ebooklib_returns_extra_missing_epub() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("runtime-manifest.json");
        write(&manifest, VALID_MANIFEST);
        let py = make_fake_python(dir.path(), &["ebooklib"]);

        let r = verify_with_paths(&manifest, &py);
        assert_eq!(r, Err(FailureCode::EExtraMissingEpub));
    }

    // ─── AC-5(2)：manifest 文件不存在 → E_RUNTIME_MISSING ─────────────────

    #[test]
    fn manifest_missing_returns_runtime_missing() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("does-not-exist.json");
        let fake_py = dir.path().join("python");

        let r = verify_with_paths(&manifest, &fake_py);
        assert_eq!(r, Err(FailureCode::ERuntimeMissing));
    }

    #[test]
    fn manifest_invalid_json_returns_runtime_missing() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("runtime-manifest.json");
        write(&manifest, "{ not valid json");
        let fake_py = dir.path().join("python");

        let r = verify_with_paths(&manifest, &fake_py);
        assert_eq!(r, Err(FailureCode::ERuntimeMissing));
    }

    #[test]
    fn manifest_missing_required_field_returns_runtime_missing() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("runtime-manifest.json");
        // 缺 imports 字段
        write(
            &manifest,
            r#"{"schema_version":1,"runtime_id":"x","python":{"source":"a","version":"b","build":"c"},"markitdown":{"version":"v","extras":[]},"extras_extra":{},"build_timestamp":"t","arch":"arm64"}"#,
        );
        let fake_py = dir.path().join("python");

        let r = verify_with_paths(&manifest, &fake_py);
        assert_eq!(r, Err(FailureCode::ERuntimeMissing));
    }

    // ─── venv python 不存在 → E_RUNTIME_MISSING（H1：禁止降级系统 python3） ──

    #[test]
    fn venv_python_missing_returns_runtime_missing() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("runtime-manifest.json");
        write(&manifest, VALID_MANIFEST);
        let nonexistent_py = dir.path().join("no_such_python");

        let r = verify_with_paths(&manifest, &nonexistent_py);
        assert_eq!(r, Err(FailureCode::ERuntimeMissing));
    }

    // ─── AC-2：非 ebooklib 模块缺失 → E_RUNTIME_MISSING（映射表保守归类） ──

    #[cfg(unix)]
    #[test]
    fn missing_non_ebooklib_module_returns_runtime_missing() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("runtime-manifest.json");
        write(&manifest, VALID_MANIFEST);
        let py = make_fake_python(dir.path(), &["mammoth"]);

        let r = verify_with_paths(&manifest, &py);
        assert_eq!(r, Err(FailureCode::ERuntimeMissing));
    }

    // ─── map_import_failure 单测（AC-2 映射表本身） ─────────────────────────

    #[test]
    fn map_import_failure_table() {
        assert_eq!(map_import_failure("ebooklib"), FailureCode::EExtraMissingEpub);
        assert_eq!(map_import_failure("bs4"), FailureCode::ERuntimeMissing);
        assert_eq!(map_import_failure("pdfminer"), FailureCode::ERuntimeMissing);
        assert_eq!(map_import_failure("pptx"), FailureCode::ERuntimeMissing);
        assert_eq!(map_import_failure("mammoth"), FailureCode::ERuntimeMissing);
        assert_eq!(map_import_failure("openpyxl"), FailureCode::ERuntimeMissing);
        assert_eq!(map_import_failure("PIL"), FailureCode::ERuntimeMissing);
        // 表外
        assert_eq!(map_import_failure("unknown_module"), FailureCode::ERuntimeMissing);
    }

    // ─── 缓存状态：snapshot 应能克隆 ───────────────────────────────────────

    #[test]
    fn runtime_check_state_snapshot_ok() {
        let m = serde_json::from_str::<RuntimeManifest>(VALID_MANIFEST).unwrap();
        let state = RuntimeCheckState::new(Ok(m.clone()));
        let s = state.snapshot();
        assert_eq!(s.as_ref().map(|r| r.runtime_id.as_str()).ok(), Some("ncdesktop-markitdown-runtime"));
    }

    #[test]
    fn runtime_check_state_snapshot_err() {
        let state = RuntimeCheckState::new(Err(FailureCode::EExtraMissingEpub));
        assert_eq!(state.snapshot(), Err(FailureCode::EExtraMissingEpub));
    }

    // ─── task_H1 AC-4：dev fallback 路径选择 ───────────────────────────────

    /// resource_dir 下没有 manifest，但 dev_fallback 下有 → 应切换到 fallback 并验证 PASS。
    #[cfg(unix)]
    #[test]
    fn select_paths_falls_back_when_resource_dir_missing_manifest() {
        // resource_dir：空目录（模拟 Tauri dev 模式 target/<profile>/）
        let resource_dir = tempdir().unwrap();
        // fallback dir：模拟 src-tauri/resources/，含 manifest + fake python
        let fb_dir = tempdir().unwrap();
        let fb_manifest = fb_dir.path().join("runtime-manifest.json");
        write(&fb_manifest, VALID_MANIFEST);
        // fake python 必须放到 fb_dir/markitdown-venv/bin/python（与 select_runtime_paths
        // 推断的 venv_python 子路径一致）
        let venv_bin = fb_dir.path().join("markitdown-venv/bin");
        std::fs::create_dir_all(&venv_bin).unwrap();
        let fb_python = make_fake_python(&venv_bin, &[]);
        // 重命名为 "python"，匹配 select_runtime_paths 期望的文件名
        let py_final = venv_bin.join("python");
        std::fs::rename(&fb_python, &py_final).unwrap();

        let (manifest_path, venv_python, used_fallback) =
            select_runtime_paths(resource_dir.path(), Some(fb_dir.path()));
        assert!(used_fallback, "应识别为 fallback 命中");
        assert_eq!(manifest_path, fb_manifest);
        assert_eq!(venv_python, py_final);

        // 端到端：fallback 的路径应能让 verify_with_paths 走通 7 imports
        let r = verify_with_paths(&manifest_path, &venv_python).expect("fallback 后应成功");
        assert_eq!(r.imports.len(), 7);
    }

    /// resource_dir 与 dev_fallback 两条路径下 manifest 都缺失 → 期望 E_RUNTIME_MISSING。
    #[test]
    fn select_paths_both_missing_returns_runtime_missing() {
        let resource_dir = tempdir().unwrap();
        let fb_dir = tempdir().unwrap(); // 空目录，无 manifest

        let (manifest_path, venv_python, used_fallback) =
            select_runtime_paths(resource_dir.path(), Some(fb_dir.path()));
        assert!(!used_fallback, "双方都缺时不应标记 fallback");
        // 默认路径返回，但文件不存在 → verify_with_paths::load_manifest 应失败
        assert!(!manifest_path.is_file());

        let r = verify_with_paths(&manifest_path, &venv_python);
        assert_eq!(r, Err(FailureCode::ERuntimeMissing));
    }

    /// dev_fallback 为 None（模拟 prod 编译期）：resource_dir 缺 manifest 时不会 fallback。
    #[test]
    fn select_paths_no_fallback_when_dev_fallback_is_none() {
        let resource_dir = tempdir().unwrap();
        let (manifest_path, _venv, used_fallback) =
            select_runtime_paths(resource_dir.path(), None);
        assert!(!used_fallback);
        assert_eq!(manifest_path, resource_dir.path().join("runtime-manifest.json"));
    }
}
