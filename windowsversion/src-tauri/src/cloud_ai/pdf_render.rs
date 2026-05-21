//! PDF 单页渲染（→ PNG bytes）+ "看起来像扫描版" 判定
//!
//! 用于让 `ocr_pdf_page` 在 `pdf-extract` 抽不到文本（扫描版课本/参考书）时，
//! 把目标页渲染成图像后走 OpenAI Vision OCR。
//!
//! # 当前状态
//!
//! - `extract_page_text` + `looks_scanned`：基于 caller 已抽出的全文（避免
//!   重复跑 `pdf_extract::extract_text`），定位目标页文本并按字符数阈值
//!   判定"看似扫描版"。
//! - `render_pdf_page_to_png`：使用 `pdfium-render` 真实现，运行时需要
//!   PDFium 动态库（Windows 下 `pdfium.dll`，macOS 下 `libpdfium.dylib`）。
//!   缺失时会返回明确错误，提示用户参考 BUILD-WINDOWS.md 下载二进制。
//!
//! # Windows 打包注意
//!
//! `tauri.conf.json` 的 `bundle.resources` 需添加 `"resources/pdfium.dll"`；
//! 用户在 BUILD-WINDOWS.md 中按指引下载到 `src-tauri/` 根目录或 PATH 即可
//! 在开发态使用。

use std::path::Path;

use crate::extraction::models::ExtractionError;

/// 推荐的渲染分辨率：兼顾清晰度与体积（OpenAI Vision 单图上限 20MB）。
/// 150~200 DPI 对 A4 纸大小约产出 1.5~2.5MB 的 PNG，留足 base64 膨胀空间。
pub const RECOMMENDED_DPI: u32 = 180;

/// 判定"看起来像扫描版"的文本长度阈值（trim 后字符数）。
const SCANNED_TEXT_LEN_THRESHOLD: usize = 50;

/// 把 PDF 指定页渲染为 PNG bytes（可直接 base64 后传给 OpenAI Vision）。
///
/// - `page_index` 0-based
/// - `dpi` 推荐 150~200（见 [`RECOMMENDED_DPI`]）
///
/// 运行时需 PDFium 动态库（pdfium.dll / libpdfium.dylib）。加载顺序：
/// 1. 优先 `bind_to_system_library`（系统 PATH / `/usr/local/lib` 等）；
/// 2. 退回当前工作目录（开发态：与 Cargo.toml 同级目录的 `pdfium.dll`）。
///
/// 都失败时返回 `ExtractionError::OcrError`，并提示 BUILD-WINDOWS.md 步骤。
pub fn render_pdf_page_to_png(
    pdf_path: &Path,
    page_index: i32,
    dpi: u32,
) -> Result<Vec<u8>, ExtractionError> {
    use pdfium_render::prelude::*;

    if page_index < 0 {
        return Err(ExtractionError::ParseError(format!(
            "page_index 不能为负: {page_index}"
        )));
    }
    if dpi == 0 {
        return Err(ExtractionError::ParseError("dpi 不能为 0".to_string()));
    }

    // pdfium-render 0.8 需运行时 PDFium 动态库（Windows 下 pdfium.dll）。
    // 在 Tauri 打包时通过 tauri.conf.json 的 bundle.resources 分发；本地开发
    // 需自行下载（参考 BUILD-WINDOWS.md "PDFium 动态库" 一节）。
    let bindings = Pdfium::bind_to_system_library()
        .or_else(|_| Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./")))
        .map_err(|e| {
            ExtractionError::OcrError(format!(
                "PDFium 加载失败：{e}（需运行时 pdfium 动态库，参考 BUILD-WINDOWS.md）"
            ))
        })?;
    let pdfium = Pdfium::new(bindings);

    let document = pdfium
        .load_pdf_from_file(pdf_path, None)
        .map_err(|e| ExtractionError::ParseError(format!("PDF 加载失败：{e}")))?;

    // PdfPageIndex = u16；越界由 pdfium-render 自己返回 PageIndexOutOfBounds。
    let page_index_u16: u16 = u16::try_from(page_index).map_err(|_| {
        ExtractionError::ParseError(format!("page_index 超出 u16 范围: {page_index}"))
    })?;
    let pages = document.pages();
    let page = pages.get(page_index_u16).map_err(|e| {
        ExtractionError::ParseError(format!("获取第 {page_index} 页失败：{e}"))
    })?;

    // dpi 推荐 150-200，对应 scale = dpi / 72。
    let scale = dpi as f32 / 72.0;
    let config = PdfRenderConfig::new().scale_page_by_factor(scale);

    let bitmap = page
        .render_with_config(&config)
        .map_err(|e| ExtractionError::OcrError(format!("PDF 页渲染失败：{e}")))?;

    let image = bitmap.as_image();
    let mut png_bytes: Vec<u8> = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut png_bytes),
            image::ImageFormat::Png,
        )
        .map_err(|e| ExtractionError::OcrError(format!("PNG 编码失败：{e}")))?;

    Ok(png_bytes)
}

/// 从 `pdf_extract::extract_text` 输出的全文中定位目标页文本。
///
/// `pdf-extract` 按 form feed `\x0C` 分页：
/// - 分页能对上 `total_pages`：精确返回该页（trim）。
/// - 分页对不上（pdf-extract 在某些 PDF 上输出整段无 `\x0C`）：把全文当成
///   "all pages concatenated"，按 `total_pages` 等分后近似返回目标页。
pub fn extract_page_text(all_text: &str, page_index: i32, total_pages: i32) -> String {
    if page_index < 0 || total_pages <= 0 {
        return String::new();
    }
    let total_pages = total_pages as usize;
    let page_index = page_index as usize;

    let pages: Vec<&str> = all_text.split('\u{000C}').collect();
    if pages.len() == total_pages {
        return pages.get(page_index).map(|s| s.trim()).unwrap_or("").to_string();
    }

    // 分页对不上：按字符数等分回退。用 chars() 而非 len()，避免 CJK 字节估算偏差。
    let trimmed: Vec<char> = all_text.trim().chars().collect();
    let per_page = trimmed.len().div_ceil(total_pages).max(1);
    let start = (page_index * per_page).min(trimmed.len());
    let end = ((page_index + 1) * per_page).min(trimmed.len());
    trimmed[start..end].iter().collect::<String>().trim().to_string()
}

/// 判定一段页文本是否"看似扫描版"（trim 后字符数 < [`SCANNED_TEXT_LEN_THRESHOLD`]）。
/// 用 chars().count() 而非 len()，避免对 CJK 多字节字符低估文本长度。
pub fn looks_scanned(page_text: &str) -> bool {
    page_text.trim().chars().count() < SCANNED_TEXT_LEN_THRESHOLD
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn nonexistent_pdf() -> PathBuf {
        PathBuf::from("/nonexistent/__notecapt_pdf_render_test__.pdf")
    }

    #[test]
    fn render_pdf_page_rejects_negative_index() {
        let err = render_pdf_page_to_png(&nonexistent_pdf(), -1, 180).unwrap_err();
        match err {
            ExtractionError::ParseError(msg) => assert!(msg.contains("page_index")),
            other => panic!("expected ParseError, got {other:?}"),
        }
    }

    #[test]
    fn render_pdf_page_rejects_zero_dpi() {
        // page_index = 0 合法，dpi = 0 触发 ParseError("dpi 不能为 0")。
        let err = render_pdf_page_to_png(&nonexistent_pdf(), 0, 0).unwrap_err();
        match err {
            ExtractionError::ParseError(msg) => assert!(msg.contains("dpi")),
            other => panic!("expected ParseError, got {other:?}"),
        }
    }

    /// 真实现路径：系统无 PDFium 动态库时应当返回 OcrError("PDFium 加载失败…")。
    /// 在 CI 上常态命中（既无系统 libpdfium 也无当前目录的 pdfium.dll）。
    ///
    /// 若机器恰好装了 PDFium，则会改走 ParseError 分支（加载 PDF 失败，因路径
    /// 不存在）；两者都断言为"非占位错误信息"即可。
    #[test]
    fn render_pdf_page_errors_when_pdfium_missing_or_path_invalid() {
        let err = render_pdf_page_to_png(&nonexistent_pdf(), 0, RECOMMENDED_DPI).unwrap_err();
        match err {
            ExtractionError::OcrError(msg) => assert!(
                msg.contains("PDFium 加载失败"),
                "未命中预期的 PDFium 加载失败分支：{msg}"
            ),
            ExtractionError::ParseError(msg) => assert!(
                msg.contains("PDF 加载失败") || msg.contains("page_index"),
                "未命中预期的 PDF 加载失败分支：{msg}"
            ),
            other => panic!("expected OcrError or ParseError, got {other:?}"),
        }
    }

    #[test]
    fn extract_page_text_picks_target_page_when_split_matches() {
        let all = "page-zero\u{000C}page-one\u{000C}page-two";
        assert_eq!(extract_page_text(all, 0, 3), "page-zero");
        assert_eq!(extract_page_text(all, 1, 3), "page-one");
        assert_eq!(extract_page_text(all, 2, 3), "page-two");
    }

    #[test]
    fn extract_page_text_falls_back_when_split_does_not_match() {
        // 全文无 \x0C，按 total_pages 等分回退。
        let all = "AAAAAAAAAABBBBBBBBBBCCCCCCCCCC"; // 30 字符 / 3 页 = 每页 10
        let p0 = extract_page_text(all, 0, 3);
        let p1 = extract_page_text(all, 1, 3);
        let p2 = extract_page_text(all, 2, 3);
        assert_eq!(p0.chars().count(), 10);
        assert_eq!(p1.chars().count(), 10);
        assert_eq!(p2.chars().count(), 10);
        assert_ne!(p0, p1);
        assert_ne!(p1, p2);
    }

    #[test]
    fn extract_page_text_returns_empty_for_invalid_inputs() {
        assert_eq!(extract_page_text("anything", -1, 3), "");
        assert_eq!(extract_page_text("anything", 0, 0), "");
        // 越界页：split 命中时直接拿不到，回退路径夹到末尾返回空（trim 后）。
        assert_eq!(extract_page_text("a\u{000C}b", 5, 2), "");
    }

    #[test]
    fn looks_scanned_uses_char_count() {
        assert!(looks_scanned(""));
        assert!(looks_scanned("   "));
        assert!(looks_scanned("少量文本"));
        // 50+ 字符的 CJK 文本应判为非扫描版（chars().count() 计字符而非字节）
        let long = "中".repeat(SCANNED_TEXT_LEN_THRESHOLD + 1);
        assert!(!looks_scanned(&long));
    }
}
