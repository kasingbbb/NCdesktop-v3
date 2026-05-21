use serde::Serialize;
use std::path::Path;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioMetadata {
    pub duration: f64,
    pub sample_rate: u32,
    pub channels: u32,
    pub format: String,
    pub file_size: u64,
}

/// 提取音频文件元信息（不完整解码）
pub fn extract(file_path: &Path) -> Result<AudioMetadata, String> {
    let file_size = std::fs::metadata(file_path)
        .map(|m| m.len())
        .unwrap_or(0);

    let file = std::fs::File::open(file_path)
        .map_err(|e| format!("打开音频文件失败: {e}"))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = file_path.extension() {
        hint.with_extension(&ext.to_string_lossy());
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| format!("音频格式探测失败: {e}"))?;

    let format = probed.format;
    let track = format
        .default_track()
        .ok_or("未找到音频轨道")?;

    let params = &track.codec_params;
    let sample_rate = params.sample_rate.unwrap_or(44100);
    let channels = params.channels.map(|c| c.count() as u32).unwrap_or(1);

    let duration = params
        .n_frames
        .map(|frames| frames as f64 / sample_rate as f64)
        .unwrap_or(0.0);

    let fmt = file_path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();

    Ok(AudioMetadata {
        duration,
        sample_rate,
        channels,
        format: fmt,
        file_size,
    })
}
