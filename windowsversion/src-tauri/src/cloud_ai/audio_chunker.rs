//! 音频文件分块工具：把超过 OpenAI Whisper 25 MB 上限的音频，按时间切成多个
//! WAV 块，每块附带其在原音频中的 `start_offset_seconds`，供上层合并 segments
//! 时间戳使用。
//!
//! ## 策略
//!
//! 1. 用 `symphonia` 解码整个音频（沿用 `audio::decoder` / `audio::metadata`
//!    已验证可用的特性集：`mp3 / aac / isomp4 / wav / pcm`）。
//! 2. 按 `max_size_bytes` 与 PCM 体积比例换算出每块能容纳的秒数，再对全量
//!    样本按帧切片。
//! 3. 每个时间段重新编码为 WAV (PCM 16-bit) 直接写入 `Vec<u8>`，可直接交给
//!    Whisper multipart upload。
//! 4. 若原文件已 ≤ `max_size_bytes`，**不**重新编码，直接读原文件返回单
//!    个 chunk —— 省时省质量、避免无谓的格式转换。
//!
//! ## 未引入新依赖
//!
//! 项目当前 `Cargo.toml` 没有 `hound`，本模块**手写** 44 字节 RIFF/WAVE WAV
//! header，以避免改公共依赖文件（与同期 unit 互不冲突）。

use std::path::Path;

use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::extraction::models::ExtractionError;

/// 标准 RIFF/WAVE PCM header 长度（固定 44 字节）。
const WAV_HEADER_BYTES: usize = 44;

/// 一段分块结果。
///
/// - `data`：完整的 WAV bytes（含 44 字节 header），可直接作为 multipart
///   upload 的 file body 交给 Whisper。
/// - `start_seconds`：此块在原音频时间线上的起点，用于把 Whisper 返回的
///   segment 时间戳「+= start_seconds」回填到原音频坐标。
/// - `duration_seconds`：此块的实际时长（与 `data` 中 PCM 帧数一致）。
#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub data: Vec<u8>,
    pub start_seconds: f64,
    pub duration_seconds: f64,
}

/// 把音频文件切成多个不超过 `max_size_bytes` 的块。
///
/// 调用方推荐传 `24 * 1024 * 1024`（24 MB，给 multipart overhead / 抖动留余量；
/// Whisper 硬限是 25 MB）。
///
/// ## 行为
///
/// - 若原文件 ≤ `max_size_bytes`：直接读原文件返回单个 chunk（`start = 0`，
///   `duration` 用 symphonia 探测的总时长；若探测不到则填 0.0）。**不**重新
///   编码 —— 保留原格式 / 比特率，省去一次有损往返。
/// - 若 > `max_size_bytes`：用 symphonia 解码为 PCM，按
///   `(max_size_bytes - WAV_HEADER) / bytes_per_second` 算出每块秒数，按
///   帧切片并各自封一份 WAV header 返回。
/// - symphonia 解码失败（不支持的容器 / 编码）：返回
///   [`ExtractionError::UnsupportedFormat`]。
/// - 文件 IO 失败：返回 [`ExtractionError::IoError`]。
///
/// ## 输出保证
///
/// - 返回 `Vec` 非空（至少一个 chunk）。
/// - 每个 chunk 的 `data.len() <= max_size_bytes`（小文件直传分支例外：
///   若原文件本身 > `max_size_bytes` 不会走该分支；走该分支时 size 必然
///   ≤ `max_size_bytes`）。
/// - chunks 按时间顺序排列；前一块的 `start + duration ≈ 下一块的 start`。
pub async fn split_audio_into_chunks(
    file_path: &Path,
    max_size_bytes: usize,
) -> Result<Vec<AudioChunk>, ExtractionError> {
    // ── 1) 小文件直传：跳过解码 ──────────────────────────────────────────────
    let meta = tokio::fs::metadata(file_path)
        .await
        .map_err(ExtractionError::IoError)?;
    if (meta.len() as usize) <= max_size_bytes {
        let bytes = tokio::fs::read(file_path)
            .await
            .map_err(ExtractionError::IoError)?;
        // 尽力探测一下时长（仅作为日志/对账用，失败也不致命）。
        let duration = probe_duration_seconds(file_path).unwrap_or(0.0);
        return Ok(vec![AudioChunk {
            data: bytes,
            start_seconds: 0.0,
            duration_seconds: duration,
        }]);
    }

    // ── 2) 大文件：复用 audio::decoder::decode_to_pcm 拿到 f32 单声道样本 ──
    // symphonia 是同步 / CPU bound 的，挪到 blocking 线程避免阻塞 tokio runtime。
    let path_owned = file_path.to_path_buf();
    let decoded = tokio::task::spawn_blocking(move || {
        crate::audio::decoder::decode_to_pcm(&path_owned)
    })
    .await
    .map_err(|e| ExtractionError::ParseError(format!("音频解码任务 join 失败: {e}")))?
    .map_err(ExtractionError::UnsupportedFormat)?;
    let (f32_samples, sample_rate) = decoded;

    if f32_samples.is_empty() {
        return Err(ExtractionError::UnsupportedFormat(format!(
            "音频解码后无样本: {}",
            file_path.display()
        )));
    }

    // 每块可装 PCM 字节（扣掉 WAV header 余量），换算成 i16 帧数。
    let chunk_pcm_capacity = max_size_bytes.saturating_sub(WAV_HEADER_BYTES);
    if chunk_pcm_capacity == 0 {
        return Err(ExtractionError::ParseError(format!(
            "max_size_bytes={max_size_bytes} 太小（至少需容纳 WAV header 44 字节）"
        )));
    }
    let frames_per_chunk: usize = (chunk_pcm_capacity / 2).max(1);

    let total_frames = f32_samples.len();
    let mut chunks: Vec<AudioChunk> = Vec::new();
    let mut frame_cursor = 0usize;
    while frame_cursor < total_frames {
        let end = (frame_cursor + frames_per_chunk).min(total_frames);
        let i16_slice: Vec<i16> = f32_samples[frame_cursor..end]
            .iter()
            .map(|s| f32_to_i16(*s))
            .collect();
        let start_seconds = frame_cursor as f64 / sample_rate as f64;
        let duration_seconds = i16_slice.len() as f64 / sample_rate as f64;
        let data = encode_wav_mono_i16(&i16_slice, sample_rate);
        debug_assert!(
            data.len() <= max_size_bytes,
            "chunk size {} 超过上限 {}",
            data.len(),
            max_size_bytes
        );
        chunks.push(AudioChunk {
            data,
            start_seconds,
            duration_seconds,
        });
        frame_cursor = end;
    }

    Ok(chunks)
}

// ── 内部：symphonia 探测 ─────────────────────────────────────────────────────

/// 仅用 symphonia 探测时长（不解码所有数据），用于小文件直传分支的日志填充。
/// 任何失败都返回 None，由调用方兜底为 0.0。
fn probe_duration_seconds(file_path: &Path) -> Option<f64> {
    let file = std::fs::File::open(file_path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = file_path.extension().and_then(|s| s.to_str()) {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .ok()?;
    let track = probed.format.default_track()?;
    let sr = track.codec_params.sample_rate.unwrap_or(44_100);
    let n = track.codec_params.n_frames?;
    Some(n as f64 / sr as f64)
}

// ── 内部：WAV 编码（手写 header）──────────────────────────────────────────────

/// 把 f32（symphonia normalized [-1.0, 1.0]）转 i16，并做 clamp。
#[inline]
fn f32_to_i16(s: f32) -> i16 {
    let scaled = (s.clamp(-1.0, 1.0) * i16::MAX as f32).round();
    scaled as i16
}

/// 把单声道 i16 PCM 编码为完整 WAV（RIFF/WAVE/fmt/data，PCM 16-bit, mono）。
///
/// 标准 44 字节 header 布局：
///
/// | offset | size | value                                       |
/// |-------:|-----:|---------------------------------------------|
/// |   0    |   4  | "RIFF"                                      |
/// |   4    |   4  | file size - 8 (le u32)                      |
/// |   8    |   4  | "WAVE"                                      |
/// |  12    |   4  | "fmt "                                      |
/// |  16    |   4  | 16 (PCM fmt chunk size, le u32)             |
/// |  20    |   2  | 1  (PCM format tag, le u16)                 |
/// |  22    |   2  | channels (le u16)                           |
/// |  24    |   4  | sample_rate (le u32)                        |
/// |  28    |   4  | byte_rate = sr * ch * bits/8 (le u32)       |
/// |  32    |   2  | block_align = ch * bits/8 (le u16)          |
/// |  34    |   2  | bits_per_sample (le u16)                    |
/// |  36    |   4  | "data"                                      |
/// |  40    |   4  | data size = samples * ch * bits/8 (le u32)  |
fn encode_wav_mono_i16(samples: &[i16], sample_rate: u32) -> Vec<u8> {
    const CHANNELS: u16 = 1;
    const BITS_PER_SAMPLE: u16 = 16;
    let data_size: u32 = (samples.len() * 2) as u32;
    let file_size_minus_8: u32 = 36u32.saturating_add(data_size);
    let byte_rate: u32 = sample_rate * CHANNELS as u32 * (BITS_PER_SAMPLE as u32 / 8);
    let block_align: u16 = CHANNELS * (BITS_PER_SAMPLE / 8);

    let mut buf = Vec::with_capacity(WAV_HEADER_BYTES + samples.len() * 2);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size_minus_8.to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // PCM fmt chunk size
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM tag
    buf.extend_from_slice(&CHANNELS.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&BITS_PER_SAMPLE.to_le_bytes());
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());

    debug_assert_eq!(buf.len(), WAV_HEADER_BYTES);

    // i16 PCM little-endian 写入 data 段（容量已在 with_capacity 中预留）
    for s in samples {
        buf.extend_from_slice(&s.to_le_bytes());
    }
    buf
}

// ── 单元测试 ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_layout_is_well_formed() {
        let samples: Vec<i16> = vec![0, 1, -1, i16::MAX, i16::MIN];
        let wav = encode_wav_mono_i16(&samples, 16_000);

        // 长度 = header + samples * 2
        assert_eq!(wav.len(), WAV_HEADER_BYTES + samples.len() * 2);

        // 关键 ASCII 标记
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(&wav[36..40], b"data");

        // PCM format tag (1) + mono (1) + 16-bit
        assert_eq!(u16::from_le_bytes([wav[20], wav[21]]), 1, "PCM tag");
        assert_eq!(u16::from_le_bytes([wav[22], wav[23]]), 1, "channels");
        assert_eq!(
            u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]),
            16_000,
            "sample rate"
        );
        assert_eq!(
            u32::from_le_bytes([wav[28], wav[29], wav[30], wav[31]]),
            16_000 * 1 * 2,
            "byte rate"
        );
        assert_eq!(u16::from_le_bytes([wav[32], wav[33]]), 2, "block align");
        assert_eq!(u16::from_le_bytes([wav[34], wav[35]]), 16, "bits/sample");

        // data 长度声明 = samples * 2
        assert_eq!(
            u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]),
            (samples.len() * 2) as u32,
            "data size"
        );

        // file size declaration = 36 + data
        assert_eq!(
            u32::from_le_bytes([wav[4], wav[5], wav[6], wav[7]]),
            36 + (samples.len() * 2) as u32,
            "RIFF chunk size"
        );

        // 首个样本 0x0000 little-endian
        assert_eq!(&wav[44..46], &[0u8, 0u8]);
    }

    #[test]
    fn f32_to_i16_clamps_extremes() {
        assert_eq!(f32_to_i16(0.0), 0);
        assert_eq!(f32_to_i16(1.0), i16::MAX);
        assert_eq!(f32_to_i16(-1.0), -i16::MAX); // 注意 round(-32767) = -32767
        // 超界 clamp
        assert_eq!(f32_to_i16(2.0), i16::MAX);
        assert_eq!(f32_to_i16(-2.0), -i16::MAX);
        // NaN clamp 到 0 区间（clamp(NaN, -1, 1) 在 Rust 中是 NaN，再 *MAX 是 NaN，as i16 是 0）
        // 这里不强制断言 NaN 行为，仅确保不 panic。
        let _ = f32_to_i16(f32::NAN);
    }

    #[test]
    fn empty_samples_produce_just_header() {
        let wav = encode_wav_mono_i16(&[], 48_000);
        assert_eq!(wav.len(), WAV_HEADER_BYTES);
        assert_eq!(
            u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]),
            0,
            "data size 应为 0"
        );
    }

    // 注：`split_audio_into_chunks` 是 async + 依赖真实音频解码，端到端覆盖
    // 留给 coordinator 的集成测试（项目 `tokio` 仅启用 `time` feature，没有
    // `macros / rt`，本文件无法直接写 `#[tokio::test]`）。
}
