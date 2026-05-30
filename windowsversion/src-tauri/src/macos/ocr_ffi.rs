use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;
use serde::Deserialize;

extern "C" {
    fn recognize_text_in_image(path: *const c_char) -> *mut c_char;
    fn recognize_text_in_pdf_page(path: *const c_char, page_index: i32) -> *mut c_char;
    fn get_pdf_page_count(path: *const c_char) -> i32;
    fn free_rust_string(ptr: *mut c_char);
}

#[derive(Debug, Clone, Deserialize)]
pub struct OcrRegion {
    pub text: String,
    pub confidence: f64,
    pub bbox: [f64; 4],
}

#[derive(Debug, Deserialize)]
struct OcrResponse {
    success: bool,
    results: Option<Vec<OcrRegion>>,
    error: Option<String>,
}

/// 获取 PDF 总页数
pub fn pdf_page_count(path: &Path) -> Result<i32, String> {
    let path_str = path.to_str().ok_or("路径包含非 UTF-8 字符")?;
    let c_path = CString::new(path_str).map_err(|e| format!("路径转换失败: {e}"))?;
    let count = unsafe { get_pdf_page_count(c_path.as_ptr()) };
    if count < 0 {
        return Err("无法打开 PDF".to_string());
    }
    Ok(count)
}

/// OCR 识别 PDF 的指定页
pub fn ocr_pdf_page(path: &Path, page_index: i32) -> Result<Vec<OcrRegion>, String> {
    let path_str = path.to_str().ok_or("路径包含非 UTF-8 字符")?;
    let c_path = CString::new(path_str).map_err(|e| format!("路径转换失败: {e}"))?;

    let result_ptr = unsafe { recognize_text_in_pdf_page(c_path.as_ptr(), page_index) };
    if result_ptr.is_null() {
        return Err("OCR 返回空指针".to_string());
    }

    let result_str = unsafe { CStr::from_ptr(result_ptr) }
        .to_str()
        .map_err(|e| format!("OCR 结果非 UTF-8: {e}"))?
        .to_string();

    unsafe { free_rust_string(result_ptr) };

    let response: OcrResponse = serde_json::from_str(&result_str)
        .map_err(|e| format!("OCR JSON 解析失败: {e}"))?;

    if !response.success {
        return Err(response.error.unwrap_or_else(|| "未知 OCR 错误".to_string()));
    }

    Ok(response.results.unwrap_or_default())
}

/// 调用 macOS Vision Framework OCR 识别图片中的文字
pub fn ocr_image(path: &Path) -> Result<Vec<OcrRegion>, String> {
    let path_str = path.to_str().ok_or("路径包含非 UTF-8 字符")?;
    let c_path = CString::new(path_str).map_err(|e| format!("路径转换失败: {e}"))?;

    let result_ptr = unsafe { recognize_text_in_image(c_path.as_ptr()) };
    if result_ptr.is_null() {
        return Err("OCR 返回空指针".to_string());
    }

    let result_str = unsafe { CStr::from_ptr(result_ptr) }
        .to_str()
        .map_err(|e| format!("OCR 结果非 UTF-8: {e}"))?
        .to_string();

    unsafe { free_rust_string(result_ptr) };

    let response: OcrResponse = serde_json::from_str(&result_str)
        .map_err(|e| format!("OCR JSON 解析失败: {e}"))?;

    if !response.success {
        return Err(response.error.unwrap_or_else(|| "未知 OCR 错误".to_string()));
    }

    Ok(response.results.unwrap_or_default())
}
