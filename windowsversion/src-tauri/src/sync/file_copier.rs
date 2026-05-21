use std::path::{Path, PathBuf};

/// 复制文件到本地存储，保持目录结构
pub fn copy_file(
    source: &Path,
    dest_base: &Path,
    session_id: &str,
    sub_dir: &str,
) -> Result<PathBuf, String> {
    let file_name = source
        .file_name()
        .ok_or("无法获取文件名")?
        .to_string_lossy()
        .to_string();

    let dest_dir = dest_base.join(session_id).join(sub_dir);
    std::fs::create_dir_all(&dest_dir)
        .map_err(|e| format!("创建目标目录失败: {e}"))?;

    let dest_path = dest_dir.join(&file_name);

    if dest_path.exists() {
        let source_size = std::fs::metadata(source)
            .map(|m| m.len())
            .unwrap_or(0);
        let dest_size = std::fs::metadata(&dest_path)
            .map(|m| m.len())
            .unwrap_or(0);
        if source_size == dest_size {
            log::info!("文件已存在且大小一致，跳过: {}", file_name);
            return Ok(dest_path);
        }
    }

    std::fs::copy(source, &dest_path)
        .map_err(|e| format!("复制文件失败 {file_name}: {e}"))?;

    log::info!("文件复制完成: {} → {}", source.display(), dest_path.display());
    Ok(dest_path)
}

/// 批量复制文件，返回成功复制的路径列表
pub fn copy_files(
    sources: &[String],
    dest_base: &Path,
    session_id: &str,
    sub_dir: &str,
) -> Result<Vec<PathBuf>, String> {
    let mut results = Vec::new();
    for source_str in sources {
        let source = Path::new(source_str);
        if source.exists() {
            let dest = copy_file(source, dest_base, session_id, sub_dir)?;
            results.push(dest);
        } else {
            log::warn!("源文件不存在，跳过: {}", source_str);
        }
    }
    Ok(results)
}
