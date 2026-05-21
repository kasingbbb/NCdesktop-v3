//! 简化的 MarkItDown 转换接口（不接入 scheduler/物化）
//!
//! 提供两个命令：
//! - `check_markitdown_status`：探测 markitdown 是否可用
//! - `convert_asset_to_markdown`：对单个 asset 文件运行 markitdown，返回 markdown 文本

use crate::db::conversion_meta::ConversionMetaRow;
use crate::db::{self, Database};
use crate::extraction::extractors::get_extractor_for;
use crate::extraction::models::ExtractOptions;
use serde::Serialize;
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::State;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MarkitdownStatus {
    pub available: bool,
    pub version: Option<String>,
    pub python_cmd: Option<String>,
    pub reason: Option<String>,
    pub install_hint: Option<String>,
}

/// W4-6：健康检查缓存（5 分钟 TTL）。
/// 避免 Inspector 状态指示器 / 重试按钮等每次操作都触发一次子进程探测。
static HEALTH_CACHE: Mutex<Option<(Instant, MarkitdownStatus)>> = Mutex::new(None);
const HEALTH_TTL: Duration = Duration::from_secs(300);

fn probe(python_cmd: &str) -> Result<String, String> {
    let out = Command::new(python_cmd)
        .args(["-m", "markitdown", "--version"])
        .output()
        .map_err(|e| format!("无法执行 {python_cmd}: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[tauri::command]
pub fn check_markitdown_status() -> MarkitdownStatus {
    // W4-6 缓存命中（5 min TTL）
    if let Ok(guard) = HEALTH_CACHE.lock() {
        if let Some((stamp, cached)) = guard.as_ref() {
            if stamp.elapsed() < HEALTH_TTL {
                return cached.clone();
            }
        }
    }

    let status = probe_status();

    if let Ok(mut guard) = HEALTH_CACHE.lock() {
        *guard = Some((Instant::now(), status.clone()));
    }
    status
}

fn probe_status() -> MarkitdownStatus {
    for cmd in ["python3", "python"].iter() {
        match probe(cmd) {
            Ok(version) => {
                return MarkitdownStatus {
                    available: true,
                    version: Some(version),
                    python_cmd: Some((*cmd).to_string()),
                    reason: None,
                    install_hint: None,
                };
            }
            Err(_) => continue,
        }
    }
    MarkitdownStatus {
        available: false,
        version: None,
        python_cmd: None,
        reason: Some("未在系统 PATH 中找到 markitdown 模块".to_string()),
        install_hint: Some("pip install markitdown[all]".to_string()),
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionResult {
    pub extractor_type: String,
    pub markdown: String,
    pub quality_level: i32,
    pub segment_count: usize,
}

#[tauri::command]
pub fn convert_asset_to_markdown(
    database: State<'_, Database>,
    asset_id: String,
) -> Result<ConversionResult, String> {
    let conn = database
        .conn
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    let asset = db::asset::get_by_id(&conn, &asset_id)?
        .ok_or_else(|| format!("素材不存在: {asset_id}"))?;
    drop(conn);

    let options = ExtractOptions {
        markitdown_enabled: true,
        ..Default::default()
    };
    let extractor = get_extractor_for(&asset.mime_type, &options)
        .ok_or_else(|| format!("无法处理 mime 类型: {}", asset.mime_type))?;

    let file_path = Path::new(&asset.file_path);
    let result = extractor
        .extract(file_path, &options)
        .map_err(|e| format!("提取失败: {e:?}"))?;

    Ok(ConversionResult {
        extractor_type: result.extractor_type,
        markdown: result.structured_md,
        quality_level: result.quality_level,
        segment_count: result.segments.len(),
    })
}

#[tauri::command]
pub fn get_conversion_meta(
    database: State<'_, Database>,
    asset_id: String,
) -> Result<Vec<ConversionMetaRow>, String> {
    let conn = database
        .conn
        .lock()
        .map_err(|e| format!("数据库锁获取失败: {e}"))?;
    db::conversion_meta::list_by_source(&conn, &asset_id)
}
