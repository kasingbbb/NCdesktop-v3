//! HEIC/HEIF → JPEG 转换工具
//!
//! ## 背景
//!
//! iPhone 默认拍照格式是 HEIC/HEIF（HEVC 编码的图像容器），而 OpenAI Vision
//! API **只接受** `image/jpeg`、`image/png`、`image/gif`、`image/webp`。因此
//! 当用户从 iPhone 同步过来的 `.heic` 图片走 Windows 版 OCR 流水线时，必须
//! 先转码为 JPEG 再 base64 编码。
//!
//! ## Rust 生态现状（2025）
//!
//! HEIC/HEIF 解码在 Rust 生态里没有"零依赖一行搞定"的选择：
//!
//! - `image` crate（1.x 系列）**不内置** HEIC 解码（HEVC 专利问题）。
//! - `libheif-rs` 是 `libheif` 的 binding：需要 C 依赖 + 平台二进制，CI/打包复杂。
//! - 调用系统命令（`magick` / `heif-convert`）：Windows 用户基本没装。
//! - **Windows 10/11 系统层** 有 HEIC codec（用户需从 Microsoft Store 安装 "HEVC
//!   视频扩展"，部分 OEM 预装），可通过 `Windows.Graphics.Imaging.BitmapDecoder`
//!   WinRT API 调用——这是最干净的方案，**前提是引入 `windows` crate**。
//!
//! ## 本 Unit 折中实现（占位）
//!
//! 出于 unit 内"只新增一个文件、不动 Cargo.toml / mod.rs"的强约束，本模块当前
//! 实现 **两套** `heic_to_jpeg`：
//!
//! 1. `#[cfg(feature = "heic-winrt")]` 版本：调 WinRT `BitmapDecoder` 真实转换。
//!    **该 feature 当前未在 Cargo.toml 声明**，仅作为后续接入的形状预留。
//! 2. 默认版本（`#[cfg(not(feature = "heic-winrt"))]`）：返回 `ExtractionError::
//!    OcrError`，提示用户先用系统 Photos 或 ImageMagick 手动转 JPEG。
//!
//! ### Coordinator 接入时需要做的事
//!
//! 在 3 个 unit 全部完成、coordinator 统一接入阶段，需要把以下依赖加到
//! `windowsversion/src-tauri/Cargo.toml` 的 `[target.'cfg(windows)'.dependencies]`
//! 段：
//!
//! ```toml
//! windows = { version = "0.58", features = [
//!     "Graphics_Imaging",
//!     "Storage_Streams",
//! ] }
//! ```
//!
//! 同时在 `cloud_ai/mod.rs` 添加 `pub mod heic_convert;` 并在 features 段定义
//! `heic-winrt = []`（或直接条件编译 `#[cfg(windows)]`）。在 `ocr_openai_vision::
//! ocr_image` 入口处先 `is_heic_path` 判断，命中则 `heic_to_jpeg` 得到 JPEG
//! bytes 后再进入原本的 base64 + OpenAI Vision 流程。
//!
//! ## 接口契约
//!
//! ```ignore
//! pub fn heic_to_jpeg(file_path: &Path, quality: u8) -> Result<Vec<u8>, ExtractionError>;
//! pub fn is_heic_path(file_path: &Path) -> bool;
//! ```

use std::path::Path;

use crate::extraction::models::ExtractionError;

/// 推荐的 JPEG 压缩质量（0~100）。85 在文件大小与视觉质量间取得普遍好平衡，
/// 也是 ImageMagick / Photoshop "Save for Web" 默认档位。
pub const DEFAULT_JPEG_QUALITY: u8 = 85;

/// 判断给定路径是否为 HEIC/HEIF 图片（仅按扩展名，大小写不敏感）。
///
/// 注意：这是"轻量启发判断"，并不打开文件读取 magic bytes；OCR 入口处的真正
/// 容错应该由 `heic_to_jpeg` 自身（若实现）和上层 magic-bytes 嗅探（`infer` crate
/// 已在 Cargo.toml）共同负责。
pub fn is_heic_path(file_path: &Path) -> bool {
    file_path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .is_some_and(|ext| ext == "heic" || ext == "heif")
}

/// 把 HEIC/HEIF 文件转换为 JPEG bytes。
///
/// 调用方在 `ocr_image` 入口先用 `is_heic_path` 检测扩展名，命中则调用本函数
/// 得到 JPEG bytes，再用 JPEG bytes 走原本的 base64 + OpenAI Vision 流程。
///
/// `quality` 是 JPEG 压缩质量（0~100），推荐 [`DEFAULT_JPEG_QUALITY`]。
///
/// ## 当前实现
///
/// 见模块顶层 doc：默认返回 `ExtractionError::OcrError`，提示用户手动转换。
/// 后续接入 `windows` crate 后，可通过 `heic-winrt` feature 启用 WinRT
/// `BitmapDecoder` 真实转换。
#[cfg(feature = "heic-winrt")]
pub fn heic_to_jpeg(file_path: &Path, quality: u8) -> Result<Vec<u8>, ExtractionError> {
    // 此分支当前 **未启用**：Cargo.toml 暂未声明 `heic-winrt` feature 也未引入
    // `windows` crate。coordinator 接入阶段把依赖与 feature 加入后，本函数才会
    // 被编译。下方仅为形状预留，真实实现需在那一阶段补全（伪代码思路：
    // `BitmapDecoder::CreateAsync(file_stream).await` → `GetSoftwareBitmapAsync`
    // → `BitmapEncoder::CreateAsync(JpegEncoderId, out_stream).await` →
    // `SetSoftwareBitmap` → `FlushAsync` → 读 out_stream 返回 Vec<u8>）。
    let _ = (file_path, quality);
    Err(ExtractionError::OcrError(
        "HEIC→JPEG WinRT 实现尚未补全（heic-winrt feature 形状预留）".to_string(),
    ))
}

/// 占位实现：返回明确错误提示用户先手动转换。
///
/// 这是默认编译路径（未启用 `heic-winrt` feature）。模块顶层 doc 解释了为什么
/// 暂时无法实现真实转换。
#[cfg(not(feature = "heic-winrt"))]
pub fn heic_to_jpeg(file_path: &Path, quality: u8) -> Result<Vec<u8>, ExtractionError> {
    // `quality` 在占位实现里没有意义，但保留参数让接口契约稳定，coordinator
    // 接入真实实现时无需改 call site。
    let _ = quality;
    Err(ExtractionError::OcrError(format!(
        "HEIC 转换暂未启用：请用 Windows Photos 应用或 ImageMagick \
         手动把 {} 转为 JPEG 后重试（NoteCapt Windows 版后续将通过 WinRT \
         BitmapDecoder 原生支持）",
        file_path.display()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn is_heic_path_recognizes_common_extensions() {
        assert!(is_heic_path(Path::new("photo.heic")));
        assert!(is_heic_path(Path::new("photo.HEIC")));
        assert!(is_heic_path(Path::new("photo.Heic")));
        assert!(is_heic_path(Path::new("photo.heif")));
        assert!(is_heic_path(Path::new("photo.HEIF")));
        assert!(is_heic_path(Path::new("/some/dir/photo.heic")));
        assert!(is_heic_path(&PathBuf::from("photo.heic")));
    }

    #[test]
    fn is_heic_path_rejects_non_heic() {
        assert!(!is_heic_path(Path::new("photo.jpg")));
        assert!(!is_heic_path(Path::new("photo.jpeg")));
        assert!(!is_heic_path(Path::new("photo.png")));
        assert!(!is_heic_path(Path::new("photo.gif")));
        assert!(!is_heic_path(Path::new("photo.webp")));
        assert!(!is_heic_path(Path::new("photo")));
        assert!(!is_heic_path(Path::new("")));
        // 类似但不命中
        assert!(!is_heic_path(Path::new("photo.heicx")));
        assert!(!is_heic_path(Path::new("photo.he")));
    }

    /// 默认（未启用 heic-winrt feature）下应返回 OcrError 占位提示。
    #[cfg(not(feature = "heic-winrt"))]
    #[test]
    fn heic_to_jpeg_returns_placeholder_error_by_default() {
        let err = heic_to_jpeg(Path::new("photo.heic"), DEFAULT_JPEG_QUALITY).unwrap_err();
        match err {
            ExtractionError::OcrError(msg) => {
                assert!(
                    msg.contains("HEIC"),
                    "占位错误消息应当提到 HEIC，但实际为: {msg}"
                );
                assert!(
                    msg.contains("photo.heic"),
                    "占位错误消息应包含输入文件路径，但实际为: {msg}"
                );
            }
            other => panic!("期望 OcrError，实际为: {other:?}"),
        }
    }
}
