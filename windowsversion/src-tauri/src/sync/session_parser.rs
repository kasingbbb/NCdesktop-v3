use serde::{Deserialize, Serialize};
use std::path::Path;
use walkdir::WalkDir;

/// 完整会话数据
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionData {
    pub session_id: String,
    pub title: String,
    pub start_time: String,
    pub end_time: String,
    pub audio_file_path: Option<String>,
    pub waveform_file_path: Option<String>,
    pub photos: Vec<SessionAssetMeta>,
    pub scans: Vec<SessionAssetMeta>,
    pub live_clips: Vec<SessionLiveClip>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionAssetMeta {
    pub file_name: String,
    pub file_path: String,
    pub captured_at: String,
    pub offset_in_audio: Option<f64>,
    pub meta_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionLiveClip {
    pub file_name: String,
    pub file_path: String,
    pub linked_asset_file_name: String,
    pub start_offset: f64,
    pub end_offset: f64,
}

/// 解析 .arca/sessions/<session_id>/ 目录
pub fn parse_session(session_dir: &Path, session_id: &str) -> Result<SessionData, String> {
    let meta_path = session_dir.join("session.json");
    let mut session = if meta_path.exists() {
        let content = std::fs::read_to_string(&meta_path)
            .map_err(|e| format!("读取 session.json 失败: {e}"))?;
        serde_json::from_str::<SessionData>(&content)
            .map_err(|e| format!("解析 session.json 失败: {e}"))?
    } else {
        SessionData {
            session_id: session_id.to_string(),
            title: session_id.to_string(),
            start_time: String::new(),
            end_time: String::new(),
            audio_file_path: None,
            waveform_file_path: None,
            photos: Vec::new(),
            scans: Vec::new(),
            live_clips: Vec::new(),
        }
    };

    let audio_dir = session_dir.join("audio");
    if audio_dir.is_dir() {
        if let Some(audio_file) = find_first_audio(&audio_dir) {
            session.audio_file_path = Some(audio_file.to_string_lossy().to_string());
        }
        let waveform_file = audio_dir.join("waveform.json");
        if waveform_file.exists() {
            session.waveform_file_path = Some(waveform_file.to_string_lossy().to_string());
        }
    }

    let photos_dir = session_dir.join("photos");
    if photos_dir.is_dir() {
        session.photos = scan_assets(&photos_dir);
    }

    let scans_dir = session_dir.join("scans");
    if scans_dir.is_dir() {
        session.scans = scan_assets(&scans_dir);
    }

    let clips_dir = session_dir.join("live_clips");
    if clips_dir.is_dir() {
        session.live_clips = scan_live_clips(&clips_dir);
    }

    Ok(session)
}

fn find_first_audio(dir: &Path) -> Option<std::path::PathBuf> {
    let audio_exts = ["m4a", "wav", "mp3", "aac"];
    for entry in WalkDir::new(dir).max_depth(1).into_iter().flatten() {
        if let Some(ext) = entry.path().extension() {
            if audio_exts.contains(&ext.to_string_lossy().to_lowercase().as_str()) {
                return Some(entry.path().to_path_buf());
            }
        }
    }
    None
}

fn scan_assets(dir: &Path) -> Vec<SessionAssetMeta> {
    let image_exts = ["jpg", "jpeg", "png", "heic", "pdf", "txt", "md"];
    let mut assets = Vec::new();

    for entry in WalkDir::new(dir).max_depth(1).into_iter().flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if let Some(ext) = path.extension() {
            let ext_lower = ext.to_string_lossy().to_lowercase();
            if ext_lower == "json" {
                continue;
            }
            if !image_exts.contains(&ext_lower.as_str()) {
                continue;
            }
        }

        let meta_path = path.with_extension(format!(
            "{}.meta.json",
            path.extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default()
        ));

        assets.push(SessionAssetMeta {
            file_name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            file_path: path.to_string_lossy().to_string(),
            captured_at: file_modified_time(path),
            offset_in_audio: None,
            meta_path: if meta_path.exists() {
                Some(meta_path.to_string_lossy().to_string())
            } else {
                None
            },
        });
    }

    assets.sort_by(|a, b| a.captured_at.cmp(&b.captured_at));
    assets
}

fn scan_live_clips(dir: &Path) -> Vec<SessionLiveClip> {
    let mut clips = Vec::new();
    let clips_meta = dir.join("clips.json");
    if clips_meta.exists() {
        if let Ok(content) = std::fs::read_to_string(&clips_meta) {
            if let Ok(parsed) = serde_json::from_str::<Vec<SessionLiveClip>>(&content) {
                clips = parsed;
            }
        }
    }
    clips
}

fn file_modified_time(path: &Path) -> String {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|t| {
            chrono::DateTime::<chrono::Utc>::from(t)
                .to_rfc3339()
        })
        .unwrap_or_default()
}
