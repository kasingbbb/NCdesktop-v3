//! USB 媒体卡「裸图片」自动导入支持。
//!
//! 与 `detector`/`manifest`（结构化 `.arca` 会话卡）不同：真实 Notecapt 设备
//! 直接把拍照结果以裸 `Picture_*.jpg` 写在卡根目录，**没有** `.arca/manifest.json`。
//! 本模块负责：
//!   1. 扫描目标卷根目录的图片文件；
//!   2. 按**内容 hash** 去重，只挑出「从未导入过」的新图片；
//!   3. 持久化已导入 hash 集合（`usb_import_state.json`），跨重连/重启生效。
//!
//! 实际导入（落库 + 提取 → MD → 工作区）复用现成的 `import_drop_paths` 管线，
//! 本模块不碰 DB，只产出「待导入文件清单」。
//!
//! 设计取舍：
//! - **内容 hash 而非文件名/mtime**：设备时间戳恒为 1970（无 RTC），文件名可能
//!   跨卡重复；内容 hash 对重命名鲁棒，「同一张图」判定最稳。用 std `DefaultHasher`
//!   （SipHash，固定密钥 → 确定性），无需引入额外哈希依赖。
//! - **只图片**：按产品决策，只导入图片扩展名；其余类型忽略。

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::hash::Hasher;
use std::path::{Path, PathBuf};

/// 目标设备卷名（产品决策：只认 Notecapt 设备，避免误扫其它 U 盘/移动硬盘）。
pub const TARGET_VOLUME_NAME: &str = "Notecapt";

/// 支持的图片扩展名（小写比较）。
const IMAGE_EXTS: &[&str] = &[
    "jpg", "jpeg", "png", "heic", "heif", "webp", "gif", "bmp", "tiff", "tif",
];

/// 单个待导入的新图片。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewMediaFile {
    /// 绝对路径（喂给 `import_drop_paths`）。
    pub path: String,
    /// 文件名（UI 展示）。
    pub name: String,
    /// 字节大小。
    pub size: u64,
    /// 内容 hash（导入成功后回传 `mark_card_imported` 落入去重集）。
    pub hash: String,
}

/// 一次扫描结果（emit 给前端 / 返回给手动扫描命令）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardScan {
    pub device_name: String,
    pub mount_path: String,
    pub new_files: Vec<NewMediaFile>,
}

/// 已导入媒体去重状态（持久化为 `usb_import_state.json`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedMediaState {
    /// 已导入文件的内容 hash 集合。
    pub imported_hashes: Vec<String>,
}

/// 判断路径是否为支持的图片（按扩展名，大小写不敏感）。
pub fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .map(|ext| IMAGE_EXTS.contains(&ext.as_str()))
        .unwrap_or(false)
}

/// 计算文件内容 hash（确定性，无外部依赖）。
pub fn hash_file(path: &Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hasher.write(&bytes);
    Ok(format!("{:016x}", hasher.finish()))
}

/// 扫描指定目录的图片，返回 hash 不在 `imported` 集合中的「新图片」。
///
/// - 跳过子目录、隐藏文件（`.` 前缀）与 macOS 资源派生文件（`._` 前缀）；
/// - 单次扫描内对同内容文件去重（两个文件内容一致只取其一）；
/// - 结果按文件名稳定排序（UI 展示一致 + 测试确定性）。
pub fn scan_new_images(dir: &Path, imported: &HashSet<String>) -> Vec<NewMediaFile> {
    let mut out: Vec<NewMediaFile> = Vec::new();
    let mut seen_this_scan: HashSet<String> = HashSet::new();

    // 先收集并按路径排序，使整个扫描确定性：`read_dir` 的返回顺序由文件系统决定、
    // 不可依赖，排序后单次扫描内的「同内容去重」会稳定保留字典序最靠前的文件名。
    let mut paths: Vec<PathBuf> = match std::fs::read_dir(dir) {
        Ok(e) => e.flatten().map(|entry| entry.path()).collect(),
        Err(_) => return out,
    };
    paths.sort();

    for path in paths {
        if !path.is_file() {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        // macOS 资源派生文件 `._xxx` 与隐藏文件一并跳过。
        if name.starts_with('.') {
            continue;
        }
        if !is_image(&path) {
            continue;
        }
        let hash = match hash_file(&path) {
            Ok(h) => h,
            Err(_) => continue,
        };
        if imported.contains(&hash) || seen_this_scan.contains(&hash) {
            continue;
        }
        seen_this_scan.insert(hash.clone());
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        out.push(NewMediaFile {
            path: path.to_string_lossy().to_string(),
            name,
            size,
            hash,
        });
    }

    out
}

/// 去重状态文件路径：`<app_data>/com.notecapt.desktop/usb_import_state.json`，
/// 与 `sync_state.json` 同目录（沿用 `commands/sync.rs` 的 `dirs_next::data_dir()` 范式）。
pub fn state_path() -> PathBuf {
    dirs_next::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.notecapt.desktop")
        .join("usb_import_state.json")
}

/// 读取去重状态（文件缺失/损坏 → 返回默认空集，不报错）。
pub fn load_state(path: &Path) -> ImportedMediaState {
    if path.exists() {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        ImportedMediaState::default()
    }
}

/// 持久化去重状态。
pub fn save_state(path: &Path, state: &ImportedMediaState) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建 USB 导入状态目录失败: {e}"))?;
    }
    let json = serde_json::to_string_pretty(state)
        .map_err(|e| format!("序列化 USB 导入状态失败: {e}"))?;
    std::fs::write(path, json).map_err(|e| format!("写入 USB 导入状态失败: {e}"))?;
    Ok(())
}

/// 把一批 hash 标记为已导入（幂等，去重）。
pub fn mark_imported(state: &mut ImportedMediaState, hashes: &[String]) {
    for h in hashes {
        if !state.imported_hashes.contains(h) {
            state.imported_hashes.push(h.clone());
        }
    }
}

/// 扫描目标卷（`/Volumes/<TARGET_VOLUME_NAME>`）的新图片。
///
/// 返回 `None` 表示目标卷未挂载；`Some(scan)` 时 `new_files` 可能为空
/// （卡在但无新图片）。已自动加载/比对去重状态。
pub fn scan_target_card() -> Option<CardScan> {
    let mount = Path::new("/Volumes").join(TARGET_VOLUME_NAME);
    if !mount.is_dir() {
        return None;
    }
    let state = load_state(&state_path());
    let imported: HashSet<String> = state.imported_hashes.into_iter().collect();
    let new_files = scan_new_images(&mount, &imported);
    Some(CardScan {
        device_name: TARGET_VOLUME_NAME.to_string(),
        mount_path: mount.to_string_lossy().to_string(),
        new_files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
        let p = dir.join(name);
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(content).unwrap();
        f.flush().unwrap();
        p
    }

    #[test]
    fn is_image_covers_common_exts_case_insensitive() {
        for n in ["a.jpg", "a.JPG", "a.jpeg", "a.PNG", "a.heic", "a.webp", "a.tif", "a.tiff", "a.bmp", "a.gif"] {
            assert!(is_image(Path::new(n)), "应识别为图片: {n}");
        }
        for n in ["a.pdf", "a.txt", "a.docx", "a.mp3", "a.mp4", "noext", ""] {
            assert!(!is_image(Path::new(n)), "不应识别为图片: {n}");
        }
    }

    #[test]
    fn hash_is_deterministic_and_content_sensitive() {
        let dir = tempfile::tempdir().unwrap();
        let a = write_file(dir.path(), "a.jpg", b"hello-image-bytes");
        let b = write_file(dir.path(), "b.jpg", b"hello-image-bytes"); // 同内容
        let c = write_file(dir.path(), "c.jpg", b"different-bytes"); // 异内容
        let ha = hash_file(&a).unwrap();
        let hb = hash_file(&b).unwrap();
        let hc = hash_file(&c).unwrap();
        assert_eq!(ha, hb, "同内容应同 hash");
        assert_ne!(ha, hc, "异内容应不同 hash");
        // 重复计算稳定
        assert_eq!(ha, hash_file(&a).unwrap());
    }

    #[test]
    fn scan_filters_non_images_hidden_and_dedups_by_content() {
        let dir = tempfile::tempdir().unwrap();
        write_file(dir.path(), "Picture_1.jpg", b"img-1");
        write_file(dir.path(), "Picture_2.png", b"img-2");
        write_file(dir.path(), "dup.jpg", b"img-1"); // 与 Picture_1 同内容 → 单次扫描内去重
        write_file(dir.path(), "notes.txt", b"not an image"); // 非图片
        write_file(dir.path(), "._Picture_1.jpg", b"resource fork"); // macOS 派生
        write_file(dir.path(), ".hidden.jpg", b"hidden"); // 隐藏
        std::fs::create_dir(dir.path().join("subdir")).unwrap(); // 目录

        let empty = HashSet::new();
        let found = scan_new_images(dir.path(), &empty);
        // 只剩 2 个唯一图片内容（img-1 / img-2）
        assert_eq!(found.len(), 2, "应只返回 2 个唯一图片, got {found:?}");
        let names: Vec<&str> = found.iter().map(|f| f.name.as_str()).collect();
        // 稳定排序后，img-1 取到的是排序靠前的文件名（Picture_1.jpg < dup.jpg? 'P'(0x50) < 'd'(0x64) → Picture_1 在前）
        assert!(names.contains(&"Picture_1.jpg"));
        assert!(names.contains(&"Picture_2.png"));
    }

    #[test]
    fn scan_skips_already_imported_hashes() {
        let dir = tempfile::tempdir().unwrap();
        let p1 = write_file(dir.path(), "Picture_1.jpg", b"img-1");
        write_file(dir.path(), "Picture_2.png", b"img-2");

        let mut imported = HashSet::new();
        imported.insert(hash_file(&p1).unwrap()); // 假装 Picture_1 已导入

        let found = scan_new_images(dir.path(), &imported);
        assert_eq!(found.len(), 1, "已导入的应被过滤");
        assert_eq!(found[0].name, "Picture_2.png");
    }

    #[test]
    fn state_roundtrip_and_mark_imported_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("usb_import_state.json");

        let mut st = load_state(&path); // 文件不存在 → 空
        assert!(st.imported_hashes.is_empty());

        mark_imported(&mut st, &["h1".into(), "h2".into()]);
        mark_imported(&mut st, &["h2".into(), "h3".into()]); // h2 重复，幂等
        assert_eq!(st.imported_hashes.len(), 3);

        save_state(&path, &st).unwrap();
        let reloaded = load_state(&path);
        assert_eq!(reloaded.imported_hashes.len(), 3);
        assert!(reloaded.imported_hashes.contains(&"h1".to_string()));
        assert!(reloaded.imported_hashes.contains(&"h3".to_string()));
    }
}
