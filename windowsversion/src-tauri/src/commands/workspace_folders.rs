//! 项目工作区（下载目录下 NoteCaptWorkPlace/<projectId>）的文件夹列举与在访达中打开

use serde::Serialize;
use std::fs;
use std::path::Path;
use crate::workspace;

/// 前端展示用：工作区内的一个文件夹（AI 归类子目录或用户自建根目录）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceFolderEntry {
    /// 相对项目工作区根的路径，如 `organized/1-项目` 或 `参考资料`；`__ROOT__` 表示根目录下直接文件（导入副本）
    pub relative_path: String,
    pub display_label: String,
    /// `ai_organized` | `root` | `root_import`
    pub kind: String,
}

/// 项目工作区绝对路径（供前端筛选素材时做路径前缀匹配）
#[tauri::command]
pub fn get_project_workspace_root(project_id: String) -> Result<String, String> {
    let p = workspace::project_workspace_dir(&project_id)?;
    Ok(p.to_string_lossy().to_string())
}

fn push_organized_entries(root: &Path, out: &mut Vec<WorkspaceFolderEntry>) -> Result<(), String> {
    let organized = root.join("organized");
    if !organized.is_dir() {
        return Ok(());
    }
    let mut names: Vec<String> = fs::read_dir(&organized)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| !n.starts_with('.'))
        .collect();
    names.sort();
    for name in names {
        out.push(WorkspaceFolderEntry {
            relative_path: format!("organized/{name}"),
            display_label: name,
            kind: "ai_organized".to_string(),
        });
    }
    Ok(())
}

fn push_root_custom_entries(root: &Path, out: &mut Vec<WorkspaceFolderEntry>) -> Result<(), String> {
    let mut names: Vec<String> = fs::read_dir(root)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| !n.starts_with('.') && n != "organized")
        .collect();
    names.sort();
    for name in names {
        out.push(WorkspaceFolderEntry {
            relative_path: name.clone(),
            display_label: name,
            kind: "root".to_string(),
        });
    }
    Ok(())
}

/// 列出 `organized/<类别>`（AI 归类）以及项目根下其它文件夹（用户新建）
#[tauri::command]
pub fn list_project_workspace_folders(project_id: String) -> Result<Vec<WorkspaceFolderEntry>, String> {
    let root = workspace::project_workspace_dir(&project_id)?;
    if !root.is_dir() {
        return Ok(Vec::new());
    }

    let mut entries: Vec<WorkspaceFolderEntry> = Vec::new();

    entries.push(WorkspaceFolderEntry {
        relative_path: "__ROOT__".to_string(),
        display_label: "项目根目录".to_string(),
        kind: "root_import".to_string(),
    });

    push_organized_entries(&root, &mut entries)?;
    push_root_custom_entries(&root, &mut entries)?;

    entries.sort_by(|a, b| {
        let rank = |k: &str| match k {
            "root_import" => 0,
            "ai_organized" => 1,
            "root" => 2,
            _ => 3,
        };
        rank(&a.kind)
            .cmp(&rank(&b.kind))
            .then(a.relative_path.cmp(&b.relative_path))
    });

    Ok(entries)
}

/// 在访达中打开项目工作区下的子文件夹（或项目根）
#[tauri::command]
pub fn reveal_project_workspace_folder(project_id: String, relative_path: String) -> Result<(), String> {
    let root = workspace::project_workspace_dir(&project_id)?;
    let root_canon = root
        .canonicalize()
        .map_err(|e| format!("无法解析工作区根目录: {e}"))?;

    let target = if relative_path.is_empty() || relative_path == "__ROOT__" {
        root_canon
    } else {
        let p = root_canon.join(&relative_path);
        let p = p
            .canonicalize()
            .map_err(|e| format!("无法解析路径: {e}"))?;
        if !p.starts_with(&root_canon) {
            return Err("路径越界".to_string());
        }
        p
    };
    if !target.exists() {
        return Err(format!("路径不存在: {}", target.display()));
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&target)
            .status()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer.exe")
            .arg(&target)
            .status()
            .map_err(|e| format!("failed to open in Explorer: {}", e))?;
        return Ok(());
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = target;
        return Err("当前平台不支持在文件管理器中打开".to_string());
    }
}
