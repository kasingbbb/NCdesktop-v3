use std::path::Path;
use serde::Serialize;

use crate::extraction::video_audio;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractAudioResponse {
    /// 生成的音频文件绝对路径
    pub output_path: String,
    /// true = 流拷贝（零转码），false = AAC 重编码
    pub stream_copy: bool,
    /// 耗时（毫秒）
    pub elapsed_ms: u128,
}

/// 从视频文件提取音频，输出到同目录。
///
/// - `video_path`：视频文件绝对路径（支持 mp4 / mov / mkv / avi 等 ffmpeg 支持的格式）
/// - 返回：`ExtractAudioResponse`，包含输出路径与耗时
#[tauri::command]
pub fn extract_audio_from_video(video_path: String) -> Result<ExtractAudioResponse, String> {
    let path = Path::new(&video_path);
    if !path.exists() {
        return Err(format!("文件不存在: {video_path}"));
    }

    let result = video_audio::extract_audio(path)?;

    Ok(ExtractAudioResponse {
        output_path: result.output_path.to_string_lossy().into_owned(),
        stream_copy: result.stream_copy,
        elapsed_ms: result.elapsed_ms,
    })
}
