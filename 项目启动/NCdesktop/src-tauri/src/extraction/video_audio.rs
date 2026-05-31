//! 视频音频提取 — 用 ffmpeg 以最快速度去除视频轨、保留音频文件。
//!
//! 策略：
//!   1. `-vn -acodec copy` — 零转码流拷贝，速度最快（≈ 磁盘 I/O 限制）。
//!   2. 若流拷贝因容器/编码不兼容失败，自动降级为 `-acodec aac`（re-encode）。
//!
//! 用途：导入 mp4/mov 等视频时，先在此把音频抽成 `.m4a`，工作区只置入音频文件
//! （丢弃视频本体），随后走 `audio_asr_iflytek` 语音转写管线。
//!
//! macOS 注意：GUI app（.app）不继承 shell 的 `PATH`，因此 `locate_ffmpeg` 直接探测
//! Homebrew 绝对路径（`/opt/homebrew/bin`、`/usr/local/bin`），保证已 `brew install ffmpeg`
//! 的机器在打包后仍能命中；二者皆缺时才回退 PATH 的 `ffmpeg`。

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

/// 从视频中提取音频，保存到**与输入同目录**、同名 `.m4a`。
///
/// 返回输出文件路径；若 ffmpeg 不可用或两次尝试均失败，返回 `Err(msg)`。
pub fn extract_audio(video_path: &Path) -> Result<AudioExtractResult, String> {
    let output_path = derive_output_path(video_path)?;
    extract_audio_to(video_path, &output_path)
}

/// 从视频中提取音频，写到**指定输出路径** `output_path`（扩展名应为 `.m4a`）。
///
/// 导入管线用此变体把音频抽到临时目录，避免污染用户源目录（如 U 盘）。
/// 先尝试零转码流拷贝；失败则删残缺产物后降级 AAC 重编码。
pub fn extract_audio_to(
    video_path: &Path,
    output_path: &Path,
) -> Result<AudioExtractResult, String> {
    let ffmpeg = locate_ffmpeg()?;

    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("无法创建音频输出目录 {}: {e}", parent.display()))?;
        }
    }

    let start = std::time::Instant::now();

    // 尝试 1：流拷贝（最快）
    let copy_ok = run_ffmpeg(
        &ffmpeg,
        video_path,
        output_path,
        &["-vn", "-acodec", "copy"],
    );

    if copy_ok.is_ok() {
        return Ok(AudioExtractResult {
            output_path: output_path.to_path_buf(),
            stream_copy: true,
            elapsed_ms: start.elapsed().as_millis(),
        });
    }

    // 删掉可能产生的残缺文件再降级
    let _ = std::fs::remove_file(output_path);

    // 尝试 2：AAC 重编码（兼容所有源格式）
    run_ffmpeg(
        &ffmpeg,
        video_path,
        output_path,
        &["-vn", "-acodec", "aac", "-b:a", "192k"],
    )?;

    Ok(AudioExtractResult {
        output_path: output_path.to_path_buf(),
        stream_copy: false,
        elapsed_ms: start.elapsed().as_millis(),
    })
}

// ── 内部工具 ──────────────────────────────────────────────────────────────

fn locate_ffmpeg() -> Result<PathBuf, String> {
    // 0) bundle 内置 ffmpeg（自包含分发，DMG 打包时由 build-macos-dmg.sh 注入）：
    //    <App>.app/Contents/Resources/ffmpeg。可执行文件在 Contents/MacOS/<bin>，
    //    故 ../Resources/ffmpeg。dev/未打包时此路径不存在 → 落到下面 Homebrew/PATH。
    if let Ok(exe) = std::env::current_exe() {
        if let Some(macos_dir) = exe.parent() {
            let bundled = macos_dir.join("../Resources/ffmpeg");
            if bundled.exists() {
                return Ok(bundled);
            }
        }
    }

    // macOS Homebrew 默认绝对路径优先（.app 不继承 shell PATH，故先探绝对路径）
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
    Err("未找到 ffmpeg，无法从视频提取音频。请先安装：brew install ffmpeg".to_string())
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
        .arg("-y") // 覆盖同名文件
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
        Err(format!("ffmpeg 返回非零退出码: {:?}", status.code()))
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

    /// 真机冒烟：用实际 ffmpeg 从 /tmp/nc_video_test.mp4 抽音频到 .m4a。
    /// 默认 ignore（需要 ffmpeg + 预生成的测试视频）。手动：
    ///   cargo test -p notecapt --lib extraction::video_audio::tests::live_extract_audio_smoke -- --ignored --nocapture
    #[test]
    #[ignore = "needs ffmpeg + /tmp/nc_video_test.mp4; run with --ignored"]
    fn live_extract_audio_smoke() {
        let input = Path::new("/tmp/nc_video_test.mp4");
        if !input.exists() {
            eprintln!("跳过：/tmp/nc_video_test.mp4 不存在");
            return;
        }
        let out = std::env::temp_dir().join("nc_video_test_out.m4a");
        let _ = std::fs::remove_file(&out);
        let r = extract_audio_to(input, &out).expect("应成功提取音频");
        assert!(out.exists(), "应生成 .m4a");
        let sz = std::fs::metadata(&out).unwrap().len();
        eprintln!(
            "提取音频: {} bytes, stream_copy={}, {}ms",
            sz, r.stream_copy, r.elapsed_ms
        );
        assert!(sz > 0, ".m4a 不应为空");
        let _ = std::fs::remove_file(&out);
    }
}
