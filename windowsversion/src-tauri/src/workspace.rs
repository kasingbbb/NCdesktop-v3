//! 用户可见的工作区根目录：`~/Downloads/NoteCaptWorkPlace/`
//! 拖入导入与 AI 整理均在此路径下操作；数据库 `file_path` 存绝对路径。
//! 权限：Rust 后端直接使用 `std::fs`，与访达拖入同源；若未来启用沙盒需配置 Downloads 读写 entitlement。

use std::path::{Path, PathBuf};

/// 下载目录下文件夹名（与用户约定一致）
pub const WORKSPACE_DIR_NAME: &str = "NoteCaptWorkPlace";

/// `~/Downloads/NoteCaptWorkPlace`
pub fn workspace_root() -> Result<PathBuf, String> {
    let downloads = dirs_next::download_dir()
        .ok_or_else(|| "无法解析系统「下载」目录（dirs::download_dir）".to_string())?;
    Ok(downloads.join(WORKSPACE_DIR_NAME))
}

/// `~/Downloads/NoteCaptWorkPlace/<project_id>/`
pub fn project_workspace_dir(project_id: &str) -> Result<PathBuf, String> {
    Ok(workspace_root()?.join(project_id))
}

/// 确保项目工作区目录存在
pub fn ensure_project_workspace(project_id: &str) -> Result<PathBuf, String> {
    let dir = project_workspace_dir(project_id)?;
    std::fs::create_dir_all(&dir).map_err(|e| {
        format!(
            "创建工作区目录失败: {} — {}",
            dir.display(),
            e
        )
    })?;
    Ok(dir)
}

/// 判断路径是否位于当前项目工作区内（用于 AI 整理 / 原地重命名）
pub fn is_under_project_workspace(project_id: &str, path: &Path) -> bool {
    match project_workspace_dir(project_id) {
        Ok(root) => path.starts_with(&root),
        Err(_) => false,
    }
}
