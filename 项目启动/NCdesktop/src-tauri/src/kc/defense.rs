//! task_014 实装：OutputStage 三层防御（cwd 隔离临时目录 + 兜底扫描清理 + 配置开关）。
//!
//! ## 设计依据
//!
//! - **ADR-006**（Architect output.md）：三层防御 = KC-MOD-2 `persist=false`（层 1）
//!   + cwd 隔离临时目录（层 2）+ ingest 后扫描清理（层 3）。
//! - **AC-1..7**（input.md）：本模块提供 `get_kc_runtime_dir` / `cleanup_kc_runtime_dir` /
//!   `scan_and_remove_ghost_outputs` 三个公开 API + 模式分层 helper。
//! - **强约束**：`fs::remove_*` 之前**必须**用 prefix 校验确保路径在 `kc_runtime/` 内
//!   （防止误删 NC 工作区，input.md "技术约束 #2" 与 "Reviewer 重点关注项 #1"）。
//! - **配合 task_008 `process.rs`**：`ensure_kc_runtime_dir` 已私有于 process.rs；本模块
//!   暴露公开版本 `get_kc_runtime_dir`，二者**实现等价**（同 `app_local_data_dir/kc_runtime`），
//!   process.rs 的私有版保留兼容（避免改 process.rs），enrichment / commands 走本模块的 pub 版。
//!
//! ## 层级 vs 模式映射
//!
//! | `KcOutputStageDefenseMode`       | 层 1 (`persist=false`) | 层 2 (cwd 隔离) | 层 3 (扫描清理) |
//! |----------------------------------|:----------------------:|:---------------:|:----------------:|
//! | `TrustPersistFalse`              | ✓ 仅信任 KC 一侧       | ✗               | ✗                |
//! | `TempDirIsolation`               | ✓                      | ✓               | ✗                |
//! | `FullDefense`（默认）            | ✓                      | ✓               | ✓                |
//!
//! 层 1 是 KC 一侧的契约（NC 调用 `ingest` 时传 `persist: false`，由 `KcClient` 实现，
//! 与本模块无关）；本模块负责层 2 的目录基础（让 process.rs spawn child 时 cwd 指向此处）
//! 与层 3 的运行时审计扫描。
//!
//! ## 不变量
//!
//! 1. **删除前路径校验**：任何 `fs::remove_file` / `fs::remove_dir_all` 前先调
//!    `is_path_within(target, base)`，确保 `target` 是 `base` 的后代。校验失败 →
//!    `log::warn!` + 跳过（不 panic、不返回 Err，主链路绝不阻塞）。
//! 2. **mkdir 容错**：`get_kc_runtime_dir` 内部 `create_dir_all` 失败仅 warn，
//!    返回 path（让上层用 spawn 时再失败一次，错误更显眼）。
//! 3. **扫描幂等**：`scan_and_remove_ghost_outputs` 在目录不存在时返回空 Vec，
//!    不视为错误（首次启动 / 已清理过的场景常见）。
//! 4. **测试隔离**：所有内部 helper 都同时暴露"AppHandle 版本"和"裸 base path 版本"，
//!    后者供单元测试用 `tempfile::tempdir()` 直接调，避免依赖 Tauri runtime。

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use crate::kc::settings::KcOutputStageDefenseMode;

// =====================================================================
// 1. 路径常量（与 process.rs::ensure_kc_runtime_dir 对齐 + KC 默认 wiki 子目录）
// =====================================================================

/// `app_local_data_dir` 下 KC 运行时目录名（与 process.rs 内部 `ensure_kc_runtime_dir` 同名，
/// 保证两者解析到同一物理目录）。
pub const KC_RUNTIME_SUBDIR: &str = "kc_runtime";

/// KC 内部 OutputStage 默认写入的子目录（KC v6 约定：`<wiki_dir>/pages/enhanced/doc-*.md`
/// 与 `<wiki_dir>/pages/indexes/doc-*.md`）。层 3 扫描的对象。
pub const WIKI_SUBDIR: &str = "wiki";

/// KC v6 OutputStage 写入的两个具体子目录（input.md AC-3）。
pub const SCAN_SUBPATHS: &[&str] = &["pages/enhanced", "pages/indexes"];

// =====================================================================
// 2. AppHandle 版本公共 API（AC-1 / AC-2 / AC-3）
// =====================================================================

/// **AC-1**：返回 `<app_local_data_dir>/kc_runtime/`，首次访问自动 mkdir。
///
/// 用途：
/// - 层 2 (`TempDirIsolation` / `FullDefense`)：作为 KC 子进程的 cwd 与 `WIKI_DIR` env
///   注入值（已由 `process.rs::ensure_kc_runtime_dir` 实装；本函数提供同语义的公开入口
///   供 enrichment / commands / 测试调用）。
/// - 层 3 (`FullDefense`)：作为 `scan_and_remove_ghost_outputs` 的 base 路径计算锚点。
///
/// **失败兜底**：`app_local_data_dir()` 异常时 fallback 到系统 temp 下 `notecapt_kc_runtime`
/// （与 process.rs 同款兜底，保证 dev / CI 环境也可用）；`create_dir_all` 失败仅 `log::warn!`，
/// 仍返回 path（让上层 spawn 时再失败一次，错误位置更明显）。
pub fn get_kc_runtime_dir(app: &AppHandle) -> PathBuf {
    let base = app
        .path()
        .app_local_data_dir()
        .ok()
        .map(|p| p.join(KC_RUNTIME_SUBDIR))
        .unwrap_or_else(|| std::env::temp_dir().join("notecapt_kc_runtime"));
    ensure_runtime_dir_at(&base);
    base
}

/// **AC-2**：清理 `kc_runtime/wiki/` 下所有内容（NC 退出时由 `KcProcessManager::stop` 调）。
///
/// **设计原因**：每次 NC 启动都从干净 wiki/ 开始；上次启动期间 KC 产生的兜底文件
/// （即便层 3 在线扫描已删，崩溃场景下可能漏）在此处兜底清理一次。
///
/// **安全**：
/// - 仅清理 `<kc_runtime>/wiki/` 子目录，**绝不**清理 `kc_runtime/` 本身（避免误删 KC venv
///   等同级潜在物，虽然现在没有，但未来可能）；
/// - 校验目标路径在 `kc_runtime/` 内（不变量 #1）；
/// - 失败仅 `log::warn!`，不抛错（退出路径不能阻塞）。
pub fn cleanup_kc_runtime_dir(app: &AppHandle) {
    let runtime = get_kc_runtime_dir(app);
    cleanup_wiki_at(&runtime);
}

/// **AC-3**：扫描 `kc_runtime/wiki/pages/{enhanced,indexes}/`，删除幽灵文件。
///
/// 参数：
/// - `app`：Tauri AppHandle（用于解析 runtime dir）；
/// - `doc_id`：
///   - `Some("abc")` → 仅删 `doc-abc.md`（按 KC v6 命名约定）；
///   - `None`        → 全删 `enhanced/` 与 `indexes/` 下所有 `.md` 文件。
///
/// 返回：实际被删除的文件绝对路径列表（用于 log + 测试断言）。
///
/// **典型调用点**（task_011 enrichment 完成后）：
/// ```ignore
/// let removed = kc::defense::scan_and_remove_ghost_outputs(&app, Some(&meta.doc_id));
/// if !removed.is_empty() {
///     log::warn!("[kc] OutputStage 层 3 检测到 {} 个幽灵文件并清理: {:?}",
///                removed.len(), removed);
/// }
/// ```
pub fn scan_and_remove_ghost_outputs(app: &AppHandle, doc_id: Option<&str>) -> Vec<PathBuf> {
    let runtime = get_kc_runtime_dir(app);
    scan_and_remove_at(&runtime, doc_id)
}

// =====================================================================
// 3. 模式 → 层级开关（AC-6）
// =====================================================================

/// 层 2 (cwd 隔离) 是否启用：`TempDirIsolation` 与 `FullDefense` 启用。
///
/// **使用约定**：process.rs 在 spawn child 时，若 `layer_2_enabled(mode) == false`
/// 则可省略 `current_dir` / `WIKI_DIR` env 注入（当前 process.rs 是无条件启用，
/// 未来根据 mode 优化时可调用此函数）。
pub fn layer_2_enabled(mode: KcOutputStageDefenseMode) -> bool {
    matches!(
        mode,
        KcOutputStageDefenseMode::TempDirIsolation | KcOutputStageDefenseMode::FullDefense
    )
}

/// 层 3 (扫描清理) 是否启用：仅 `FullDefense` 启用。
///
/// **使用约定**：task_011 enrichment 在 ingest 成功后，仅当 `layer_3_enabled(mode)` 为真
/// 才调 `scan_and_remove_ghost_outputs`；否则跳过扫描以节省 IO。
pub fn layer_3_enabled(mode: KcOutputStageDefenseMode) -> bool {
    matches!(mode, KcOutputStageDefenseMode::FullDefense)
}

// =====================================================================
// 4. 纯函数版（供测试 + 跨调用方复用）
// =====================================================================

/// 确保 runtime 目录存在（创建失败仅 warn，返回 path 让上层再失败一次）。
///
/// 抽出纯函数：测试用 `tempfile::tempdir().path().join("kc_runtime")` 直接调，
/// 不依赖 Tauri AppHandle。
fn ensure_runtime_dir_at(base: &Path) {
    if let Err(e) = std::fs::create_dir_all(base) {
        log::warn!(
            "[kc::defense] 创建 KC runtime 目录失败: path={} err={}",
            base.display(),
            e
        );
    }
}

/// 清理指定 runtime 下 `wiki/` 子树（删除整个 wiki/ 后重建空目录）。
///
/// **安全**：
/// 1. 目标路径校验：必须是 `base/wiki`（绝不允许 base 自身或越界路径）；
/// 2. 删除策略：`remove_dir_all` 整树；再 `create_dir_all` 重建空目录（让 KC 仍可写入）；
/// 3. 失败仅 `log::warn!`，不抛错。
fn cleanup_wiki_at(base: &Path) {
    let wiki = base.join(WIKI_SUBDIR);

    // 不变量 #1：路径校验（虽然这里 wiki = base.join，理论上必在 base 内，
    // 但若 base 含 `..` 或符号链接攻击，仍可能越界；此处仍走 is_path_within 兜底）。
    if !is_path_within(&wiki, base) {
        log::warn!(
            "[kc::defense] cleanup_wiki_at 路径校验失败（拒绝清理）: wiki={} base={}",
            wiki.display(),
            base.display()
        );
        return;
    }

    // 目录不存在 → 无需清理（幂等）。
    if !wiki.exists() {
        return;
    }

    if let Err(e) = std::fs::remove_dir_all(&wiki) {
        log::warn!(
            "[kc::defense] remove_dir_all(wiki) 失败: path={} err={}",
            wiki.display(),
            e
        );
        return;
    }

    // 重建空目录，让 KC 仍可写入（避免它写文件时因父目录不存在而崩）。
    if let Err(e) = std::fs::create_dir_all(&wiki) {
        log::warn!(
            "[kc::defense] 重建 wiki/ 目录失败: path={} err={}",
            wiki.display(),
            e
        );
    }
}

/// 扫描 `base/wiki/pages/{enhanced,indexes}/` 下 `.md` 文件，按 `doc_id` 过滤删除。
///
/// **设计要点**：
/// - 不递归（KC v6 OutputStage 只在 `enhanced/` 与 `indexes/` 一级目录写文件，无嵌套；
///   `WalkDir::max_depth(1)` 既快又防止误扫到未来潜在子目录）；
/// - 文件名匹配：`doc_id = Some("abc")` 时仅匹配 `doc-abc.md`（KC v6 命名约定）；
///   `None` 时匹配所有 `.md`；
/// - 路径越界校验：每个待删文件都校验是否真在 `base/wiki/` 内（防符号链接攻击）；
/// - 单文件删除失败仅 warn，不中断扫描其他文件。
///
/// 返回：实际被删除的文件绝对路径列表。
fn scan_and_remove_at(base: &Path, doc_id: Option<&str>) -> Vec<PathBuf> {
    use walkdir::WalkDir;

    let wiki = base.join(WIKI_SUBDIR);
    let mut removed = Vec::new();

    // 期望的精确文件名（doc_id 有值时），避免每次循环都重复 format。
    let expected_name: Option<String> = doc_id.map(|id| format!("doc-{id}.md"));

    for sub in SCAN_SUBPATHS {
        let dir = wiki.join(sub);
        if !dir.exists() {
            // 目录不存在 → 跳过（首次启动 / 已清理场景常见，input.md "性能"提示）。
            continue;
        }

        // 不变量 #1：扫描根本身先校验（防止 base 含 .. 越界）。
        if !is_path_within(&dir, base) {
            log::warn!(
                "[kc::defense] scan dir 路径校验失败（拒绝扫描）: dir={} base={}",
                dir.display(),
                base.display()
            );
            continue;
        }

        for entry in WalkDir::new(&dir).max_depth(1).into_iter().flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            // 仅匹配 `.md` 文件（KC v6 OutputStage 只产 .md；
            // 其他文件保守不删，避免误删用户在 wiki/ 下手动放的东西）。
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }

            // doc_id 过滤
            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => continue,
            };
            if let Some(ref expected) = expected_name {
                if file_name != expected {
                    continue;
                }
            }

            // 不变量 #1：路径越界校验（每个待删文件单独再校验一次，
            // 即便 WalkDir 跟符号链接遁出 base 范围也能拦下）。
            if !is_path_within(path, base) {
                log::warn!(
                    "[kc::defense] 拒绝删除越界文件: path={} base={}",
                    path.display(),
                    base.display()
                );
                continue;
            }

            match std::fs::remove_file(path) {
                Ok(()) => removed.push(path.to_path_buf()),
                Err(e) => log::warn!(
                    "[kc::defense] remove_file 失败: path={} err={}",
                    path.display(),
                    e
                ),
            }
        }
    }

    removed
}

/// 路径越界守卫：判断 `target` 是否在 `base` 内部（含 base 自身）。
///
/// **实现**：先尝试 `canonicalize`（解析符号链接 + 规范化 `..`），失败兜底用 lexical 比较。
/// `canonicalize` 要求路径必须存在；不存在时（如即将删除前的 race）走 lexical fallback。
///
/// **限制**：lexical fallback 无法拦截"符号链接遁出"攻击；但 NC 控制 base 路径
/// （`app_local_data_dir`，由 OS 沙箱保护），符号链接攻击实际不可能。
fn is_path_within(target: &Path, base: &Path) -> bool {
    let canon_base = std::fs::canonicalize(base).unwrap_or_else(|_| base.to_path_buf());
    let canon_target = std::fs::canonicalize(target).unwrap_or_else(|_| target.to_path_buf());
    canon_target.starts_with(&canon_base)
}

// =====================================================================
// 5. 单元测试（AC-7）
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// 构造一个临时 runtime 根目录（模拟 `app_local_data_dir/kc_runtime`）。
    /// 注意：直接用 `tmp.path()` 作为 runtime 根，不再嵌套 `kc_runtime/`，
    /// 简化测试断言；语义等同（base/wiki/pages/... 结构不变）。
    fn make_runtime() -> TempDir {
        let tmp = tempfile::tempdir().expect("create tempdir");
        ensure_runtime_dir_at(tmp.path());
        tmp
    }

    /// 在 base 下创建 `wiki/pages/<sub>/doc-<id>.md` 文件，内容是 id（用于校验）。
    fn touch_ghost_file(base: &Path, sub: &str, doc_id: &str) -> PathBuf {
        let dir = base.join(WIKI_SUBDIR).join("pages").join(sub);
        fs::create_dir_all(&dir).expect("mkdir pages dir");
        let path = dir.join(format!("doc-{doc_id}.md"));
        fs::write(&path, doc_id).expect("write ghost file");
        path
    }

    // ---------- AC-7.1: get_kc_runtime_dir_creates_dir ----------

    /// `ensure_runtime_dir_at` 创建出不存在的目录。
    /// （AppHandle 版无法在单测调；纯函数版等价覆盖 AC-1。）
    #[test]
    fn get_kc_runtime_dir_creates_dir() {
        let parent = tempfile::tempdir().expect("tempdir");
        let target = parent.path().join("kc_runtime_nested/deep");

        assert!(!target.exists(), "前置：target 应不存在");

        ensure_runtime_dir_at(&target);

        assert!(target.exists(), "ensure_runtime_dir_at 后应存在");
        assert!(target.is_dir(), "应是目录");
    }

    // ---------- AC-7.2: scan_and_remove_ghost_outputs_removes_known_paths ----------

    /// `doc_id == None` 时删除两个 SCAN_SUBPATHS 下所有 .md 文件。
    #[test]
    fn scan_and_remove_ghost_outputs_removes_known_paths() {
        let tmp = make_runtime();
        let base = tmp.path();

        // 在 enhanced/ 与 indexes/ 各写一个 doc-A.md
        let f_enhanced = touch_ghost_file(base, "enhanced", "A");
        let f_indexes = touch_ghost_file(base, "indexes", "A");

        let removed = scan_and_remove_at(base, None);

        assert_eq!(removed.len(), 2, "应删除 2 个文件，实际: {removed:?}");
        assert!(!f_enhanced.exists(), "enhanced/ 下文件应被删除");
        assert!(!f_indexes.exists(), "indexes/ 下文件应被删除");
        // 返回值含两个绝对路径
        assert!(removed.iter().any(|p| p == &f_enhanced));
        assert!(removed.iter().any(|p| p == &f_indexes));
    }

    // ---------- AC-7.3: scan_and_remove_ghost_outputs_with_doc_id_only_removes_matching ----------

    /// `doc_id = Some("A")` 时只删 doc-A.md，doc-B.md 保留。
    #[test]
    fn scan_and_remove_ghost_outputs_with_doc_id_only_removes_matching() {
        let tmp = make_runtime();
        let base = tmp.path();

        // enhanced/ 下放 doc-A.md + doc-B.md
        let f_a_enh = touch_ghost_file(base, "enhanced", "A");
        let f_b_enh = touch_ghost_file(base, "enhanced", "B");
        // indexes/ 下也放 doc-A.md + doc-B.md
        let f_a_idx = touch_ghost_file(base, "indexes", "A");
        let f_b_idx = touch_ghost_file(base, "indexes", "B");

        let removed = scan_and_remove_at(base, Some("A"));

        assert_eq!(removed.len(), 2, "应只删 doc-A.md × 2，实际: {removed:?}");
        assert!(!f_a_enh.exists(), "doc-A.md (enhanced) 应被删");
        assert!(!f_a_idx.exists(), "doc-A.md (indexes) 应被删");
        assert!(f_b_enh.exists(), "doc-B.md (enhanced) 应保留");
        assert!(f_b_idx.exists(), "doc-B.md (indexes) 应保留");
    }

    // ---------- AC-7.4: cleanup_kc_runtime_dir_clears_all ----------

    /// `cleanup_wiki_at` 清空 wiki/ 子树，包括嵌套子目录。
    #[test]
    fn cleanup_kc_runtime_dir_clears_all() {
        let tmp = make_runtime();
        let base = tmp.path();

        // 在 wiki/ 下放各种文件（含嵌套）
        let f1 = touch_ghost_file(base, "enhanced", "X");
        let f2 = touch_ghost_file(base, "indexes", "Y");
        let nested = base.join(WIKI_SUBDIR).join("subdir/deep");
        fs::create_dir_all(&nested).expect("nested mkdir");
        let f3 = nested.join("orphan.md");
        fs::write(&f3, "deep").expect("write nested file");

        cleanup_wiki_at(base);

        let wiki_dir = base.join(WIKI_SUBDIR);
        assert!(wiki_dir.exists(), "wiki/ 应被重建为空目录（让 KC 仍可写）");
        assert!(wiki_dir.read_dir().unwrap().next().is_none(), "wiki/ 应为空");
        assert!(!f1.exists());
        assert!(!f2.exists());
        assert!(!f3.exists());
    }

    // ---------- AC-7.5: 集成模拟（KC 偷偷写 wiki/） + 层 3 清理 ----------

    /// 集成场景：模拟 KC 在 cwd 偷偷写 `wiki/pages/enhanced/doc-X.md`，
    /// 层 3 `scan_and_remove_at` 能检测并清理（input.md AC-7 第 5 条）。
    #[test]
    fn integration_ghost_file_detected_and_cleaned() {
        let tmp = make_runtime();
        let base = tmp.path();

        // 模拟 KC 在 cwd（= base）下写出 wiki/pages/enhanced/doc-INT.md
        let ghost = touch_ghost_file(base, "enhanced", "INT");
        assert!(ghost.exists(), "前置：幽灵文件已写入");

        // 层 3 扫描清理（指定 doc_id）
        let removed = scan_and_remove_at(base, Some("INT"));

        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0], ghost);
        assert!(!ghost.exists(), "幽灵文件应被删除");
    }

    // ---------- AC-6: 三档模式 → 层级开关 ----------

    /// `TrustPersistFalse`：层 2 / 层 3 都不启（仅信 KC `persist=false`）。
    /// `TempDirIsolation`：层 2 启、层 3 不启。
    /// `FullDefense`：层 2 + 层 3 全启（默认）。
    #[test]
    fn mode_layers_enable_correctly() {
        // TrustPersistFalse
        assert!(!layer_2_enabled(KcOutputStageDefenseMode::TrustPersistFalse));
        assert!(!layer_3_enabled(KcOutputStageDefenseMode::TrustPersistFalse));

        // TempDirIsolation
        assert!(layer_2_enabled(KcOutputStageDefenseMode::TempDirIsolation));
        assert!(!layer_3_enabled(KcOutputStageDefenseMode::TempDirIsolation));

        // FullDefense
        assert!(layer_2_enabled(KcOutputStageDefenseMode::FullDefense));
        assert!(layer_3_enabled(KcOutputStageDefenseMode::FullDefense));
    }

    // ---------- 边界：缺失目录 / 空 base / 路径越界 ----------

    /// 目录不存在时 scan 返回空 Vec，不报错（首次启动场景）。
    #[test]
    fn scan_returns_empty_when_no_wiki_dir() {
        let tmp = tempfile::tempdir().expect("tempdir");
        // 故意不创建 wiki/ 目录
        let removed = scan_and_remove_at(tmp.path(), None);
        assert!(removed.is_empty(), "无 wiki/ 目录应返回空 Vec");
    }

    /// cleanup 在 wiki/ 不存在时幂等（不 panic，不报错）。
    #[test]
    fn cleanup_is_idempotent_on_missing_wiki() {
        let tmp = tempfile::tempdir().expect("tempdir");
        cleanup_wiki_at(tmp.path()); // 不应 panic
        cleanup_wiki_at(tmp.path()); // 第二次仍不应 panic
    }

    /// 非 .md 文件保守保留（不在删除清单内）。
    #[test]
    fn scan_skips_non_md_files() {
        let tmp = make_runtime();
        let base = tmp.path();

        // 放一个 .txt 与一个 .md，scan(None) 后 .md 删、.txt 保留
        let dir = base.join(WIKI_SUBDIR).join("pages/enhanced");
        fs::create_dir_all(&dir).expect("mkdir");
        let md = dir.join("doc-Z.md");
        let txt = dir.join("doc-Z.txt");
        fs::write(&md, "md").unwrap();
        fs::write(&txt, "txt").unwrap();

        let removed = scan_and_remove_at(base, None);

        assert_eq!(removed.len(), 1, "只删 .md，实际: {removed:?}");
        assert!(!md.exists());
        assert!(txt.exists(), "非 .md 文件应保留");
    }

    /// `is_path_within` 拒绝目标在 base 之外的路径（防误删 NC 工作区）。
    #[test]
    fn is_path_within_rejects_outside_paths() {
        let base = tempfile::tempdir().expect("base tempdir");
        let outside = tempfile::tempdir().expect("outside tempdir");

        // outside 与 base 是两个独立 tempdir，必不互含
        assert!(
            !is_path_within(outside.path(), base.path()),
            "outside 不应被判定在 base 内"
        );

        // base 自身应判定为 within（边界）
        assert!(
            is_path_within(base.path(), base.path()),
            "base 自身应被判定在 base 内"
        );

        // base 子目录应判定为 within
        let inner = base.path().join("inner");
        std::fs::create_dir_all(&inner).unwrap();
        assert!(
            is_path_within(&inner, base.path()),
            "base 子目录应被判定在 base 内"
        );
    }
}
