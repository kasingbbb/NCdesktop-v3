use crate::audio::decoder;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 波形数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaveformData {
    pub sample_rate: u32,
    pub duration: f64,
    pub peaks_per_second: u32,
    pub peaks: Vec<WaveformPeak>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveformPeak {
    pub min: f32,
    pub max: f32,
}

const PEAKS_PER_SECOND: u32 = 100;

/// 从音频文件生成波形数据
pub fn generate(file_path: &Path) -> Result<WaveformData, String> {
    let (samples, sample_rate) = decoder::decode_to_pcm(file_path)?;
    let total_samples = samples.len();
    let duration = total_samples as f64 / sample_rate as f64;
    let samples_per_peak = (sample_rate / PEAKS_PER_SECOND) as usize;

    let mut peaks = Vec::new();
    let mut i = 0;

    while i < total_samples {
        let end = (i + samples_per_peak).min(total_samples);
        let chunk = &samples[i..end];

        let mut min = f32::MAX;
        let mut max = f32::MIN;
        for &s in chunk {
            if s < min { min = s; }
            if s > max { max = s; }
        }

        peaks.push(WaveformPeak { min, max });
        i = end;
    }

    Ok(WaveformData {
        sample_rate,
        duration,
        peaks_per_second: PEAKS_PER_SECOND,
        peaks,
    })
}

/// 生成并缓存波形数据到文件
pub fn generate_cached(
    audio_path: &Path,
    cache_dir: &Path,
) -> Result<WaveformData, String> {
    let cache_path = cache_path_for(audio_path, cache_dir);

    if cache_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&cache_path) {
            if let Ok(data) = serde_json::from_str::<WaveformData>(&content) {
                log::info!("波形缓存命中: {}", cache_path.display());
                return Ok(data);
            }
        }
    }

    log::info!("生成波形数据: {}", audio_path.display());
    let waveform = generate(audio_path)?;

    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let json = serde_json::to_string(&waveform)
        .map_err(|e| format!("序列化波形数据失败: {e}"))?;
    std::fs::write(&cache_path, json).ok();

    Ok(waveform)
}

fn cache_path_for(audio_path: &Path, cache_dir: &Path) -> PathBuf {
    let hash = simple_hash(&audio_path.to_string_lossy());
    cache_dir.join(format!("waveform_{hash}.json"))
}

fn simple_hash(input: &str) -> String {
    let mut hash: u64 = 5381;
    for byte in input.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    format!("{hash:016x}")
}
