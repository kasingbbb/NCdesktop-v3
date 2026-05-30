//! Unicode NFC 归一化 + 启动期 NFC 自愈（ADR-005 / NEW-R14）
//!
//! macOS APFS/HFS+ readdir 倾向返回 NFD 字节；DB 永远存 NFC。
//! 若字节不一致 → list 行存在但 select 不到 asset 的"鬼影"。
//! 策略：启动期串行扫描 `~/Downloads/NoteCaptWorkPlace/<project>/`，
//! 把 NFD 名 rename 为 NFC；若 NFC 目标已存在则仅 log::warn 跳过。

use std::path::Path;
use unicode_normalization::UnicodeNormalization;

/// UAX#15 NFC 归一化
pub fn nfc_normalize(s: &str) -> String {
    s.nfc().collect()
}

/// NFC 等价比较（两侧均归一后字节比较）
pub fn nfc_eq(a: &str, b: &str) -> bool {
    nfc_normalize(a) == nfc_normalize(b)
}

/// 对单个项目工作区目录递归扫描；非 NFC 命名 → rename 至 NFC。
/// 失败仅 log，不抛错（启动期不可阻塞）。
pub fn nfc_self_heal(project_root: &Path) {
    if !project_root.is_dir() {
        return;
    }
    let entries = match std::fs::read_dir(project_root) {
        Ok(e) => e,
        Err(e) => {
            log::warn!("nfc_self_heal: read_dir {} 失败: {}", project_root.display(), e);
            return;
        }
    };
    for ent in entries.flatten() {
        let path = ent.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        let nfc = nfc_normalize(&name);
        if nfc != name {
            let target = project_root.join(&nfc);
            if target.exists() {
                log::warn!(
                    "nfc_self_heal: 目标 NFC 名已存在，跳过 {} -> {}",
                    path.display(),
                    target.display()
                );
            } else if let Err(e) = std::fs::rename(&path, &target) {
                log::warn!(
                    "nfc_self_heal: rename {} -> {} 失败: {}",
                    path.display(),
                    target.display(),
                    e
                );
            } else {
                log::info!("nfc_self_heal: healed {} -> {}", path.display(), target.display());
            }
        }
        // 递归子目录（仅对子目录递归，文件无需深入）
        let final_path = project_root.join(nfc_normalize(&name));
        if final_path.is_dir() {
            nfc_self_heal(&final_path);
        }
    }
}

/// 启动期 hook：扫描整个 `~/Downloads/NoteCaptWorkPlace/` 下所有项目目录。
/// 失败仅 log，绝不 panic / 抛错。
pub fn nfc_heal_workspace() {
    let root = match crate::workspace::workspace_root() {
        Ok(r) => r,
        Err(e) => {
            log::warn!("nfc_heal_workspace: 解析 workspace_root 失败: {e}");
            return;
        }
    };
    if !root.is_dir() {
        return;
    }
    let entries = match std::fs::read_dir(&root) {
        Ok(e) => e,
        Err(e) => {
            log::warn!("nfc_heal_workspace: read_dir {} 失败: {}", root.display(), e);
            return;
        }
    };
    for ent in entries.flatten() {
        let project_dir = ent.path();
        if project_dir.is_dir() {
            nfc_self_heal(&project_dir);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// NFD "参考" (U+53C2 U+8003) 与 NFC 的字节序列实际相同（CJK 单字非组合）。
    /// 用拉丁字母带变音符 'é' (NFD: e + ́) 验证 NFC 归一。
    #[test]
    fn nfc_normalize_combines_decomposed() {
        let nfd = "e\u{0301}"; // e + combining acute
        let nfc = nfc_normalize(nfd);
        assert_eq!(nfc, "\u{00E9}"); // é
        assert!(nfc_eq(nfd, "\u{00E9}"));
    }

    /// 即使 CJK 字符 NFD/NFC 相同，nfc_normalize 也应安全 idempotent。
    #[test]
    fn nfc_idempotent_for_cjk() {
        let s = "参考";
        assert_eq!(nfc_normalize(s), s);
        assert!(nfc_eq(s, s));
    }

    #[test]
    fn nfc_eq_handles_mixed_forms() {
        // "café" NFD vs NFC
        let nfd = "cafe\u{0301}";
        let nfc = "caf\u{00E9}";
        assert!(nfc_eq(nfd, nfc));
        assert_ne!(nfd, nfc, "原始字节不等");
    }

    /// 在 tempdir 中构造一个 NFD 命名目录，自愈后应变为 NFC 命名。
    #[test]
    fn nfc_self_heal_renames_nfd_dir() {
        let td = tempfile::tempdir().unwrap();
        let nfd = "cafe\u{0301}";
        let nfc = "caf\u{00E9}";
        let nfd_dir = td.path().join(nfd);
        std::fs::create_dir(&nfd_dir).unwrap();
        assert!(nfd_dir.exists());

        nfc_self_heal(td.path());

        let nfc_dir = td.path().join(nfc);
        assert!(nfc_dir.exists(), "NFC 命名目录应当存在");
        // NFD 路径在文件系统层可能因 normalize 而与 NFC 路径指向同一个 inode（macOS HFS+ 行为）；
        // 这里只断言 NFC 路径可访问即视为愈合成功。
    }

    /// NFC 目标已存在时，自愈应跳过且不覆写。
    #[test]
    fn nfc_self_heal_skips_when_target_exists() {
        let td = tempfile::tempdir().unwrap();
        let nfd = "cafe\u{0301}";
        let nfc = "caf\u{00E9}";
        std::fs::create_dir(td.path().join(nfd)).ok();
        std::fs::create_dir(td.path().join(nfc)).ok();
        // 两个都存在时不应 panic
        nfc_self_heal(td.path());
        assert!(td.path().join(nfc).exists());
    }
}
