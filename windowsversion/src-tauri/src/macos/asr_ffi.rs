use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;
use serde::Deserialize;

extern "C" {
    fn transcribe_audio_file(path: *const c_char) -> *mut c_char;
    fn free_asr_string(ptr: *mut c_char);
}

#[derive(Debug, Deserialize)]
struct AsrResponse {
    success: bool,
    text: Option<String>,
    error: Option<String>,
}

/// 调用 macOS SFSpeechRecognizer 对音频文件进行语音转写
/// 返回转写文字；失败时返回 Err(描述)
pub fn transcribe_audio(path: &Path) -> Result<String, String> {
    let path_str = path.to_str().ok_or("路径包含非 UTF-8 字符")?;
    let c_path = CString::new(path_str).map_err(|e| format!("路径转换失败: {e}"))?;

    let result_ptr = unsafe { transcribe_audio_file(c_path.as_ptr()) };
    if result_ptr.is_null() {
        return Err("ASR 返回空指针".to_string());
    }

    let result_str = unsafe { CStr::from_ptr(result_ptr) }
        .to_str()
        .map_err(|e| format!("ASR 结果非 UTF-8: {e}"))?
        .to_string();

    unsafe { free_asr_string(result_ptr) };

    let response: AsrResponse = serde_json::from_str(&result_str)
        .map_err(|e| format!("ASR JSON 解析失败: {e}"))?;

    if !response.success {
        return Err(response.error.unwrap_or_else(|| "未知 ASR 错误".to_string()));
    }

    Ok(response.text.unwrap_or_default())
}
