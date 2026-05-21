//! task_011 AC-1：在 Finder（或对应平台文件管理器）中显示「原文件」。
//!
//! 与 `workspace_folders::reveal_project_workspace_folder` 区别：本命令不限
//! 制路径必须在工作区内，因为「原文件」可能位于任意磁盘位置（用户拖入时的
//! 来源）。出于安全，仅在路径存在且不是空字符串时执行；不存在直接返回错误。
//!
//! 行为：
//! - macOS：`open -R <source_path>` 在 Finder 中高亮显示该文件。
//! - 其它平台：当前暂未实现，返回错误（与 reveal_project_workspace_folder 一致）。

use std::path::PathBuf;

/// 在文件管理器中显示给定源文件（高亮选中）。
///
/// 参数 `source_path`：绝对路径字符串（来自前端 `WorkspaceAssetView.filePath`
/// 或 `Asset.sourceData`）。
#[tauri::command]
pub fn reveal_source_file(source_path: String) -> Result<(), String> {
    let trimmed = source_path.trim();
    if trimmed.is_empty() {
        return Err("源文件路径为空".to_string());
    }
    let target = PathBuf::from(trimmed);
    if !target.exists() {
        return Err(format!("源文件不存在：{}", target.display()));
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(&target)
            .status()
            .map_err(|e| format!("无法在 Finder 中显示：{e}"))?;
        return Ok(());
    }
    #[cfg(target_os = "windows")]
    {
        // Windows Explorer：/select, 参数可在 Explorer 中高亮指定文件（注意逗号紧贴 select 无空格）
        let path_str = target.to_string_lossy().to_string();
        std::process::Command::new("explorer.exe")
            .args(["/select,", &path_str])
            .status()
            .map_err(|e| format!("failed to reveal in Explorer: {}", e))?;
        return Ok(());
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = target;
        return Err("当前平台不支持在文件管理器中打开".to_string());
    }
}
