use crate::audio::{metadata, waveform};
use std::path::Path;
use tauri::AppHandle;

#[tauri::command]
pub fn get_audio_metadata(file_path: String) -> Result<metadata::AudioMetadata, String> {
    metadata::extract(Path::new(&file_path))
}

#[tauri::command]
pub fn get_waveform_data(
    app: AppHandle,
    file_path: String,
) -> Result<waveform::WaveformData, String> {
    let audio_path = Path::new(&file_path);
    let cache_dir = app
        .path()
        .app_cache_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("waveforms");

    waveform::generate_cached(audio_path, &cache_dir)
}

use tauri::Manager;
