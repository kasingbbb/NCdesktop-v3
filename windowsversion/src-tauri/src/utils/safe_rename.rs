//! EXDEV-safe rename：先 `fs::rename`；遇跨卷（EXDEV=18）走 copy-first 两阶段（ADR-002）
//!
//! 顺序：
//! 1. `copy_dir_all(src → dst_tmp)` 递归复制
//! 2. 对所有新文件 `sync_all()` (fsync)
//! 3. `fs::rename(dst_tmp → dst)`（同卷必成）
//! 4. 返回 `CrossDevice { pending_remove_src }` → caller 在 DB COMMIT 成功后调用 `remove_src_after_commit`
//! 5. 任何阶段失败：tmp 已尽力清理；src 绝不删
//!
//! 注：copy 阶段失败应让 caller 抛 `E_CROSS_DEVICE`（本模块只返 `io::Error` / `IpcError`）。

use crate::utils::ipc_error::IpcError;
use std::io;
use std::path::{Path, PathBuf};

/// 模拟跨卷错误（仅 `#[cfg(test)]`）：注入路径后，第一次 fs::rename 强制返回 EXDEV。
#[cfg(test)]
mod test_inject {
    use std::path::PathBuf;
    use std::sync::Mutex;

    pub static SIMULATE_EXDEV_FOR: Mutex<Option<PathBuf>> = Mutex::new(None);

    pub fn set(path: Option<PathBuf>) {
        *SIMULATE_EXDEV_FOR.lock().unwrap() = path;
    }
    pub fn matches(src: &std::path::Path) -> bool {
        let g = SIMULATE_EXDEV_FOR.lock().unwrap();
        match g.as_ref() {
            Some(p) => p == src,
            None => false,
        }
    }
}

/// `safe_rename` 的输出。caller 必须根据 outcome 决定 COMMIT 后是否清理 src。
#[derive(Debug)]
pub enum RenameOutcome {
    /// 同卷 rename 成功；src 已不存在，无需后续清理。
    SameVolume,
    /// 跨卷 copy-first 完成；caller 必须在 DB COMMIT 后调用 `remove_src_after_commit(pending_remove_src)`。
    CrossDevice { pending_remove_src: PathBuf },
}

/// 判定一个 io::Error 是否为跨设备错误（EXDEV）。
/// macOS errno = 18；用 raw_os_error 兜底以避免 unstable 的 `ErrorKind::CrossesDevices` 依赖。
fn is_exdev(e: &io::Error) -> bool {
    if e.raw_os_error() == Some(18) {
        return true;
    }
    // `ErrorKind::CrossesDevices` 在较新 Rust 中存在；用字符串匹配兜底
    format!("{:?}", e.kind()).contains("CrossesDevices")
}

/// EXDEV-safe rename。失败返 `IpcError`（E_CROSS_DEVICE 或 E_INTERNAL）。
pub fn safe_rename(src: &Path, dst: &Path) -> Result<RenameOutcome, IpcError> {
    // 测试注入：模拟跨卷
    #[cfg(test)]
    let force_exdev = test_inject::matches(src);
    #[cfg(not(test))]
    let force_exdev = false;

    // 1) 先试同卷 rename（除非注入模拟跨卷）
    if !force_exdev {
        match std::fs::rename(src, dst) {
            Ok(()) => return Ok(RenameOutcome::SameVolume),
            Err(e) if is_exdev(&e) => {
                // 落入跨卷分支
                log::info!("safe_rename: EXDEV 触发 copy-first {} -> {}", src.display(), dst.display());
            }
            Err(e) => {
                return Err(IpcError::internal(format!(
                    "fs::rename {} -> {} 失败: {}",
                    src.display(),
                    dst.display(),
                    e
                )));
            }
        }
    }

    // 2) copy-first 两阶段
    let dst_tmp: PathBuf = {
        let mut p = dst.as_os_str().to_os_string();
        p.push(".cross_device.tmp");
        PathBuf::from(p)
    };

    // 清理可能残留的同名 tmp
    if dst_tmp.exists() {
        let _ = if dst_tmp.is_dir() {
            std::fs::remove_dir_all(&dst_tmp)
        } else {
            std::fs::remove_file(&dst_tmp)
        };
    }

    if let Err(e) = copy_path_recursive(src, &dst_tmp) {
        // 清理 tmp；src 绝不动
        let _ = remove_path(&dst_tmp);
        return Err(IpcError::cross_device(
            &src.to_string_lossy(),
            &dst.to_string_lossy(),
        ).attach_internal_hint(&format!("copy 阶段失败: {}", e)));
    }
    if let Err(e) = fsync_all(&dst_tmp) {
        // fsync 失败视为 cross_device 失败（数据可能不安全）
        let _ = remove_path(&dst_tmp);
        return Err(IpcError::cross_device(
            &src.to_string_lossy(),
            &dst.to_string_lossy(),
        ).attach_internal_hint(&format!("fsync 失败: {}", e)));
    }

    // 3) tmp -> final（同卷 rename 必成）
    if let Err(e) = std::fs::rename(&dst_tmp, dst) {
        let _ = remove_path(&dst_tmp);
        return Err(IpcError::internal(format!(
            "tmp->final rename 失败: {}",
            e
        )));
    }

    // 4) 不删 src；caller commit 后调用 remove_src_after_commit
    Ok(RenameOutcome::CrossDevice {
        pending_remove_src: src.to_path_buf(),
    })
}

/// caller 在 DB COMMIT 成功后调用；失败仅 log（NEW-R12 残留由启动 cleanup_pending_scan 兜底）。
pub fn remove_src_after_commit(src: &Path) {
    if !src.exists() {
        return;
    }
    let res = if src.is_dir() {
        std::fs::remove_dir_all(src)
    } else {
        std::fs::remove_file(src)
    };
    if let Err(e) = res {
        // 留一个 cleanup_pending 标记；启动期 cleanup_pending_scan 会清
        let marker = {
            let mut p = src.as_os_str().to_os_string();
            p.push(".cleanup_pending");
            PathBuf::from(p)
        };
        let _ = std::fs::write(&marker, b"pending");
        log::warn!(
            "remove_src_after_commit 失败 {}: {} (marker: {})",
            src.display(),
            e,
            marker.display()
        );
    }
}

/// 启动期扫描：清理 `*.cleanup_pending` 标记对应的 src，以及孤立的 `*.cross_device.tmp`。
/// 仅 log，不抛错。
pub fn cleanup_pending_scan(root: &Path) {
    if !root.is_dir() {
        return;
    }
    let walker = match std::fs::read_dir(root) {
        Ok(w) => w,
        Err(e) => {
            log::warn!("cleanup_pending_scan: read_dir {} 失败: {}", root.display(), e);
            return;
        }
    };
    for ent in walker.flatten() {
        let p = ent.path();
        let name = match p.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        if let Some(stem) = name.strip_suffix(".cleanup_pending") {
            // 标记文件 → 删对应 src + 标记本身
            let src = p.with_file_name(stem);
            if src.exists() {
                let _ = remove_path(&src);
            }
            let _ = std::fs::remove_file(&p);
            log::info!("cleanup_pending_scan: cleaned {}", src.display());
        } else if name.ends_with(".cross_device.tmp") {
            // 孤立 tmp（前次中断残留）
            let _ = remove_path(&p);
            log::info!("cleanup_pending_scan: removed orphan tmp {}", p.display());
        } else if p.is_dir() {
            cleanup_pending_scan(&p);
        }
    }
}

// ── 内部 helpers ──────────────────────────────────────────────

fn copy_path_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    let md = std::fs::metadata(src)?;
    if md.is_dir() {
        std::fs::create_dir_all(dst)?;
        for ent in std::fs::read_dir(src)? {
            let ent = ent?;
            let child_src = ent.path();
            let child_dst = dst.join(ent.file_name());
            copy_path_recursive(&child_src, &child_dst)?;
        }
        Ok(())
    } else {
        std::fs::copy(src, dst)?;
        Ok(())
    }
}

fn fsync_all(p: &Path) -> io::Result<()> {
    let md = std::fs::metadata(p)?;
    if md.is_dir() {
        // 目录本身 fsync 在不同平台行为不一致；逐个文件 fsync 即可保证数据持久。
        for ent in std::fs::read_dir(p)? {
            let ent = ent?;
            fsync_all(&ent.path())?;
        }
    } else {
        let f = std::fs::File::open(p)?;
        f.sync_all()?;
    }
    Ok(())
}

fn remove_path(p: &Path) -> io::Result<()> {
    if !p.exists() {
        return Ok(());
    }
    if p.is_dir() {
        std::fs::remove_dir_all(p)
    } else {
        std::fs::remove_file(p)
    }
}

impl IpcError {
    /// 内部 helper：在已有错误上拼接 hint 到 message（保留 code/details）。
    fn attach_internal_hint(mut self, hint: &str) -> Self {
        self.message = format!("{} | {}", self.message, hint);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 同卷 rename：成功后 src 不存在、dst 存在、返回 SameVolume。
    #[test]
    fn same_volume_rename_succeeds() {
        let td = tempfile::tempdir().unwrap();
        let src = td.path().join("src");
        let dst = td.path().join("dst");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(src.join("a.txt"), b"hello").unwrap();

        let out = safe_rename(&src, &dst).expect("same-volume rename ok");
        assert!(matches!(out, RenameOutcome::SameVolume));
        assert!(!src.exists());
        assert!(dst.join("a.txt").exists());
    }

    /// 注入 EXDEV：必须走 copy-first，src 在两阶段后**仍存在**，dst 已完整，返回 CrossDevice。
    #[test]
    fn exdev_triggers_copy_first_and_src_retained() {
        let td = tempfile::tempdir().unwrap();
        let src = td.path().join("src");
        let dst = td.path().join("dst");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(src.join("a.txt"), b"hello").unwrap();
        std::fs::write(src.join("b.txt"), b"world").unwrap();

        test_inject::set(Some(src.clone()));
        let out = safe_rename(&src, &dst).expect("copy-first ok");
        test_inject::set(None);

        match out {
            RenameOutcome::CrossDevice { pending_remove_src } => {
                assert_eq!(pending_remove_src, src);
            }
            _ => panic!("应为 CrossDevice"),
        }
        assert!(src.exists(), "src 必须保留直到 caller COMMIT");
        assert!(dst.join("a.txt").exists());
        assert!(dst.join("b.txt").exists());
        assert_eq!(std::fs::read(dst.join("a.txt")).unwrap(), b"hello");
    }

    /// COMMIT 后清理 src
    #[test]
    fn remove_src_after_commit_clears_src() {
        let td = tempfile::tempdir().unwrap();
        let src = td.path().join("src");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(src.join("a.txt"), b"x").unwrap();

        remove_src_after_commit(&src);
        assert!(!src.exists());
    }

    /// cleanup_pending_scan 能识别并清理 `.cleanup_pending` 标记。
    #[test]
    fn cleanup_pending_scan_removes_marked_src() {
        let td = tempfile::tempdir().unwrap();
        let src = td.path().join("src");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(src.join("a.txt"), b"x").unwrap();
        let marker = td.path().join("src.cleanup_pending");
        std::fs::write(&marker, b"pending").unwrap();

        cleanup_pending_scan(td.path());
        assert!(!src.exists(), "src 应被清理");
        assert!(!marker.exists(), "marker 应被清理");
    }

    /// cleanup_pending_scan 能清理孤立 `.cross_device.tmp`。
    #[test]
    fn cleanup_pending_scan_removes_orphan_tmp() {
        let td = tempfile::tempdir().unwrap();
        let orphan = td.path().join("dst.cross_device.tmp");
        std::fs::create_dir(&orphan).unwrap();
        std::fs::write(orphan.join("x.txt"), b"x").unwrap();

        cleanup_pending_scan(td.path());
        assert!(!orphan.exists());
    }

    #[test]
    fn is_exdev_detects_errno_18() {
        let e = io::Error::from_raw_os_error(18);
        assert!(is_exdev(&e));
        let e2 = io::Error::from_raw_os_error(2); // ENOENT
        assert!(!is_exdev(&e2));
    }
}
