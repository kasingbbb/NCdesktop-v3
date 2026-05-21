//! PDF 单页渲染（→ PNG bytes）+ "看起来像扫描版" 判定
//!
//! 用于让 `ocr_pdf_page` 在 `pdf-extract` 抽不到文本（扫描版课本/参考书）时，
//! 把目标页渲染成图像后走 OpenAI Vision OCR。本 unit 是 NoteCapt Windows 版
//! 三 unit 套件中的渲染层，coordinator 会在三 unit 全部完成后统一接入。
//!
//! # 当前状态：混合实现
//!
//! - `page_likely_scanned`：**真实现**。基于已有依赖 `pdf-extract` 抽取该页文本
//!   后判定（trim 后长度 < 50 视为"看似扫描版"）。可立即使用。
//! - `render_pdf_page_to_png`：**占位实现**，直接返回
//!   `ExtractionError::OcrError`。原因是真正的逐页栅格化在 Rust 生态需要 C
//!   依赖（PDFium / MuPDF），引入新的二进制 / 打包步骤超出单 unit 范围。
//!
//! # 启用真实渲染的步骤（留给 coordinator 统一接入阶段）
//!
//! 1. `windowsversion/src-tauri/Cargo.toml` 的 `[dependencies]` 增加：
//!    ```toml
//!    pdfium-render = "0.8"
//!    ```
//! 2. 在 `BUILD-WINDOWS.md` 增补一节"下载 PDFium 二进制"：
//!    - 下载 `pdfium-windows-x64.zip`（bblanchon/pdfium-binaries 官方 release）
//!    - 解压 `pdfium.dll` 到 `windowsversion/src-tauri/resources/pdfium.dll`
//! 3. `windowsversion/src-tauri/tauri.conf.json` 的 `bundle.resources`
//!    数组追加 `"resources/pdfium.dll"`，让安装包随版本分发。
//! 4. 把本文件 `render_pdf_page_to_png` 的占位实现替换为基于 `pdfium-render`
//!    的真实实现（约 30 行）：
//!    ```ignore
//!    use pdfium_render::prelude::*;
//!    let bindings = Pdfium::bind_to_library(
//!        Pdfium::pdfium_platform_library_name_at_path("./resources/"),
//!    )
//!    .or_else(|_| Pdfium::bind_to_system_library())
//!    .map_err(|e| ExtractionError::OcrError(format!("加载 pdfium 失败: {e}")))?;
//!    let pdfium = Pdfium::new(bindings);
//!    let document = pdfium
//!        .load_pdf_from_path(pdf_path, None)
//!        .map_err(|e| ExtractionError::ParseError(format!("打开 PDF 失败: {e}")))?;
//!    let page = document
//!        .pages()
//!        .get(page_index as u16)
//!        .map_err(|e| ExtractionError::ParseError(format!("获取页失败: {e}")))?;
//!    // dpi → 像素宽度（PDF 点 = 1/72 英寸，纵向取 page.height 同理）
//!    let scale = dpi as f32 / 72.0;
//!    let render_config = PdfRenderConfig::new()
//!        .set_target_width((page.width().value * scale) as i32)
//!        .set_target_height((page.height().value * scale) as i32);
//!    let mut buf: Vec<u8> = Vec::new();
//!    page.render_with_config(&render_config)
//!        .map_err(|e| ExtractionError::OcrError(format!("渲染失败: {e}")))?
//!        .as_image()
//!        .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
//!        .map_err(|e| ExtractionError::OcrError(format!("PNG 编码失败: {e}")))?;
//!    Ok(buf)
//!    ```
//!
//! 接入完成后，`ocr_pdf_page` 在 `page_likely_scanned == true` 时改为：
//! `render_pdf_page_to_png(.., page_index, 180) → base64 → ocr_data_url`。

use std::path::Path;

use crate::extraction::models::ExtractionError;

/// 推荐的渲染分辨率：兼顾清晰度与体积（OpenAI Vision 单图上限 20MB）。
/// 150~200 DPI 对 A4 纸大小约产出 1.5~2.5MB 的 PNG，留足 base64 膨胀空间。
#[allow(dead_code)] // coordinator 接入前暂未被调用；保留为接入阶段参考。
pub const RECOMMENDED_DPI: u32 = 180;

/// 判定"看起来像扫描版"的文本长度阈值（trim 后字符数）。
const SCANNED_TEXT_LEN_THRESHOLD: usize = 50;

/// 把 PDF 指定页渲染为 PNG bytes（可直接 base64 后传给 OpenAI Vision）。
///
/// - `page_index` 0-based
/// - `dpi` 推荐 150~200（见 [`RECOMMENDED_DPI`]）
///
/// **当前为占位实现**：始终返回 `ExtractionError::OcrError`。
/// 启用真实渲染的步骤见模块顶层 doc comment。
pub fn render_pdf_page_to_png(
    pdf_path: &Path,
    page_index: i32,
    dpi: u32,
) -> Result<Vec<u8>, ExtractionError> {
    // 占位实现里也对入参做最小校验，避免 coordinator 真正接入前掉进无意义 panic
    if page_index < 0 {
        return Err(ExtractionError::ParseError(format!(
            "page_index 不能为负: {page_index}"
        )));
    }
    if dpi == 0 {
        return Err(ExtractionError::ParseError("dpi 不能为 0".to_string()));
    }
    Err(ExtractionError::OcrError(format!(
        "PDF 渲染未启用（占位实现）：pdf={}, page_index={}, dpi={}。\
         需 coordinator 在统一接入阶段引入 pdfium-render（或 mupdf-rs）依赖，\
         并按模块 doc comment 步骤把 pdfium.dll 随 bundle 分发后，再把本函数替换为真实实现。",
        pdf_path.display(),
        page_index,
        dpi
    )))
}

/// 判断 PDF 单页是否"看起来像扫描版"（无文本或文本极少）。
///
/// 判定逻辑：
/// 1. 用 `pdf-extract` 抽取 PDF 全文；按 form feed `\x0C` 切页（与
///    `ocr_openai_vision::ocr_pdf_page` 同一约定）；
/// 2. 若能定位到目标页：trim 后字符数 < [`SCANNED_TEXT_LEN_THRESHOLD`] 即视为
///    扫描版；
/// 3. 若 `\x0C` 分页失败（页数对不上、整份只有一段文本）：降级为
///    `全文 trim 后长度 / 总页数 < 阈值` 的平均判定。
///
/// 注意：本函数是同步 CPU 工作（`pdf-extract` 内部就是同步）；caller 若在
/// async 上下文里调用，应自行 `spawn_blocking` 包裹，以免阻塞 runtime。
pub fn page_likely_scanned(
    pdf_path: &Path,
    page_index: i32,
) -> Result<bool, ExtractionError> {
    if page_index < 0 {
        return Err(ExtractionError::ParseError(format!(
            "page_index 不能为负: {page_index}"
        )));
    }

    // 用 lopdf 拿总页数；同时也能在路径无效时提前 ParseError 出去
    let doc = lopdf::Document::load(pdf_path).map_err(|e| {
        ExtractionError::ParseError(format!(
            "lopdf 打开 PDF 失败 ({}): {e}",
            pdf_path.display()
        ))
    })?;
    let total_pages = doc.get_pages().len();
    if total_pages == 0 {
        // 没有任何页：视为"不像扫描版"（也算"没东西可扫"），把决策权交回上层
        return Ok(false);
    }
    if (page_index as usize) >= total_pages {
        return Err(ExtractionError::ParseError(format!(
            "页码越界: page_index={page_index} >= total_pages={total_pages}"
        )));
    }

    let all_text = pdf_extract::extract_text(pdf_path)
        .map_err(|e| ExtractionError::ParseError(format!("pdf-extract 抽取失败: {e}")))?;

    // pdf-extract 按 \x0C 分页；先尝试精确定位目标页
    let pages: Vec<&str> = all_text.split('\u{000C}').collect();
    if pages.len() == total_pages {
        let page_text_len = pages
            .get(page_index as usize)
            .map(|s| s.trim().chars().count())
            .unwrap_or(0);
        return Ok(page_text_len < SCANNED_TEXT_LEN_THRESHOLD);
    }

    // 分页对不上：降级为"平均每页字符数 < 阈值"。
    // 用 chars().count() 而非 len()，避免对 CJK 多字节字符低估文本长度。
    let total_chars = all_text.trim().chars().count();
    let avg_per_page = total_chars / total_pages;
    Ok(avg_per_page < SCANNED_TEXT_LEN_THRESHOLD)
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
        let err = render_pdf_page_to_png(&nonexistent_pdf(), 0, 0).unwrap_err();
        match err {
            ExtractionError::ParseError(msg) => assert!(msg.contains("dpi")),
            other => panic!("expected ParseError, got {other:?}"),
        }
    }

    #[test]
    fn render_pdf_page_returns_ocr_error_for_placeholder() {
        // 入参合法时，占位实现统一返回 OcrError；coordinator 真正接入后这条断言
        // 自然会变红，提醒维护者更新测试预期。
        let err = render_pdf_page_to_png(&nonexistent_pdf(), 0, RECOMMENDED_DPI).unwrap_err();
        match err {
            ExtractionError::OcrError(msg) => {
                assert!(msg.contains("占位"), "msg={msg}");
                assert!(msg.contains("pdfium") || msg.contains("PDF 渲染"), "msg={msg}");
            }
            other => panic!("expected OcrError, got {other:?}"),
        }
    }

    #[test]
    fn page_likely_scanned_rejects_negative_index() {
        let err = page_likely_scanned(&nonexistent_pdf(), -1).unwrap_err();
        match err {
            ExtractionError::ParseError(msg) => assert!(msg.contains("page_index")),
            other => panic!("expected ParseError, got {other:?}"),
        }
    }

    #[test]
    fn page_likely_scanned_returns_parse_error_for_missing_file() {
        // 不存在的 PDF 路径走 lopdf::Document::load 失败分支
        let err = page_likely_scanned(&nonexistent_pdf(), 0).unwrap_err();
        assert!(matches!(err, ExtractionError::ParseError(_)));
    }
}
