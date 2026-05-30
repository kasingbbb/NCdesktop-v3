//! 视频音频提取 — 用 ffmpeg 以最快速度去除视频轨、保留音频文件。
//!
//! 策略：
//!   1. `-vn -acodec copy` — 零转码流拷贝，速度最快（≈ 磁盘 I/O 限制）。
//!   2. 若流拷贝因容器/编码不兼容失败，自动降级为 `-acodec aac`（re-encode）。
//!
//! 输出文件放在与输入相同的目录内，扩展名 `.m4a`（AAC / 流拷贝均兼容）。

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// 音频提取结果
#[derive(Debug)]
pub struct AudioExtractResult {
    /// 生成的音频文件路径
    pub output_path: PathBuf,
    /// 是否为流拷贝模式（false = 降级为重编码）
    pub stream_copy: bool,
    /// ffmpeg 耗时（毫秒）
    pub elapsed_ms: u128,
}

/// 从 MP4（或任何 ffmpeg 支持的视频）中提取音频，保存到同目录。
///
/// 返回输出文件路径；若 ffmpeg 不可用或两次尝试均失败，返回 Err(msg)。
pub fn extract_audio(video_path: &Path) -> Result<AudioExtractResult, String> {
    let ffmpeg = locate_ffmpeg()?;

    let output_path = derive_output_path(video_path)?;

    let start = std::time::Instant::now();

    // 尝试 1：流拷贝（最快）
    let copy_ok = run_ffmpeg(
        &ffmpeg,
        video_path,
        &output_path,
        &["-vn", "-acodec", "copy"],
    );

    if copy_ok.is_ok() {
        return Ok(AudioExtractResult {
            output_path,
            stream_copy: true,
            elapsed_ms: start.elapsed().as_millis(),
        });
    }

    // 删掉可能产生的残缺文件再降级
    let _ = std::fs::remove_file(&output_path);

    // 尝试 2：AAC 重编码（兼容所有源格式）
    run_ffmpeg(
        &ffmpeg,
        video_path,
        &output_path,
        &["-vn", "-acodec", "aac", "-b:a", "192k"],
    )?;

    Ok(AudioExtractResult {
        output_path,
        stream_copy: false,
        elapsed_ms: start.elapsed().as_millis(),
    })
}

// ── 内部工具 ──────────────────────────────────────────────────────────────

fn locate_ffmpeg() -> Result<PathBuf, String> {
    // macOS Homebrew 默认路径优先（避免 PATH 问题）
    for candidate in &[
        "/opt/homebrew/bin/ffmpeg",
        "/usr/local/bin/ffmpeg",
        "ffmpeg",
    ] {
        let p = PathBuf::from(candidate);
        if p.is_absolute() {
            if p.exists() {
                return Ok(p);
            }
        } else {
            // 通过 PATH 查找
            if Command::new(candidate)
                .arg("-version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok()
            {
                return Ok(p);
            }
        }
    }
    Err("未找到 ffmpeg，请先安装（brew install ffmpeg）".to_string())
}

fn derive_output_path(video_path: &Path) -> Result<PathBuf, String> {
    let dir = video_path.parent().ok_or("无法获取视频文件目录")?;
    let stem = video_path
        .file_stem()
        .ok_or("无法获取文件名")?
        .to_string_lossy();
    Ok(dir.join(format!("{stem}.m4a")))
}

fn run_ffmpeg(
    ffmpeg: &Path,
    input: &Path,
    output: &Path,
    extra_args: &[&str],
) -> Result<(), String> {
    let status = Command::new(ffmpeg)
        .arg("-y")             // 覆盖同名文件
        .arg("-i")
        .arg(input)
        .args(extra_args)
        .arg(output)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .status()
        .map_err(|e| format!("无法启动 ffmpeg: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "ffmpeg 返回非零退出码: {:?}",
            status.code()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_output_path_replaces_extension_with_m4a() {
        let p = PathBuf::from("/tmp/录像/会议回放.mp4");
        let out = derive_output_path(&p).unwrap();
        assert_eq!(out, PathBuf::from("/tmp/录像/会议回放.m4a"));
    }

    #[test]
    fn derive_output_path_works_for_mov() {
        let p = PathBuf::from("/Users/test/screen.mov");
        let out = derive_output_path(&p).unwrap();
        assert_eq!(out, PathBuf::from("/Users/test/screen.m4a"));
    }
}
