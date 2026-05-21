use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedCard {
    pub mount_path: String,
    pub arca_path: String,
    pub device_id: String,
    pub device_name: String,
}

/// 扫描 /Volumes 下所有已挂载卷，查找含 .arca 目录的 TF 卡
pub fn scan_volumes() -> Vec<DetectedCard> {
    let volumes_dir = Path::new("/Volumes");
    let mut cards = Vec::new();

    if let Ok(entries) = std::fs::read_dir(volumes_dir) {
        for entry in entries.flatten() {
            let mount_path = entry.path();
            let arca_path = mount_path.join(".arca");
            if arca_path.is_dir() {
                let manifest_path = arca_path.join("manifest.json");
                if manifest_path.exists() {
                    if let Some(card) = read_card_info(&mount_path, &arca_path) {
                        cards.push(card);
                    }
                }
            }
        }
    }

    cards
}

/// 检查指定路径是否为有效的 Arca TF 卡
pub fn is_valid_card(path: &Path) -> bool {
    let arca_path = path.join(".arca");
    arca_path.is_dir() && arca_path.join("manifest.json").exists()
}

fn read_card_info(mount_path: &Path, arca_path: &PathBuf) -> Option<DetectedCard> {
    let manifest_path = arca_path.join("manifest.json");
    let content = std::fs::read_to_string(&manifest_path).ok()?;
    let manifest: serde_json::Value = serde_json::from_str(&content).ok()?;

    let device_id = manifest
        .get("deviceId")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let device_name = manifest
        .get("deviceName")
        .and_then(|v| v.as_str())
        .unwrap_or("Arca Device")
        .to_string();

    Some(DetectedCard {
        mount_path: mount_path.to_string_lossy().to_string(),
        arca_path: arca_path.to_string_lossy().to_string(),
        device_id,
        device_name,
    })
}
