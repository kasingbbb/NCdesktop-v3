use crate::db::{self, Database};
use crate::sync::{detector, file_copier, manifest, meta_parser, session_parser, state, timeline_builder, progress};
use serde::Serialize;
use std::path::Path;
use tauri::{AppHandle, State};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub cards: Vec<detector::DetectedCard>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub device_name: String,
    pub device_id: String,
    pub sessions: Vec<manifest::SessionSummary>,
    pub new_sessions: Vec<String>,
}

#[tauri::command]
pub fn scan_tf_card() -> Result<ScanResult, String> {
    let cards = detector::scan_volumes();
    Ok(ScanResult { cards })
}

#[tauri::command]
pub fn preview_import(arca_path: String) -> Result<ImportPreview, String> {
    let arca = Path::new(&arca_path);
    let manifest = manifest::parse_manifest(arca)?;

    let app_data = dirs_next::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let state_path = app_data.join("com.notecapt.desktop").join("sync_state.json");
    let sync_state = state::load_state(&state_path);

    let new_sessions: Vec<String> = manifest
        .sessions
        .iter()
        .filter(|s| !state::is_session_synced(&sync_state, &s.session_id, &manifest.device_id))
        .map(|s| s.session_id.clone())
        .collect();

    Ok(ImportPreview {
        device_name: manifest.device_name,
        device_id: manifest.device_id,
        sessions: manifest.sessions,
        new_sessions,
    })
}

#[tauri::command]
pub async fn import_sessions(
    app: AppHandle,
    database: State<'_, Database>,
    arca_path: String,
    session_ids: Vec<String>,
    library_id: String,
) -> Result<Vec<String>, String> {
    let arca = Path::new(&arca_path);
    let manifest = manifest::parse_manifest(arca)?;
    let sessions_dir = arca.join("sessions");

    let app_data = dirs_next::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let base_dir = app_data.join("com.notecapt.desktop");
    let storage_dir = base_dir.join("storage");
    let state_path = base_dir.join("sync_state.json");
    let mut sync_state = state::load_state(&state_path);

    let mut project_ids = Vec::new();
    let total = session_ids.len() as u32;

    for (idx, session_id) in session_ids.iter().enumerate() {
        if state::is_session_synced(&sync_state, session_id, &manifest.device_id) {
            log::info!("会话 {} 已同步，跳过", session_id);
            continue;
        }

        progress::emit_progress(
            &app, session_id, "scanning", idx as u32, total,
            &format!("扫描会话 {session_id}..."),
        );

        let session_dir = sessions_dir.join(session_id);
        if !session_dir.is_dir() {
            log::warn!("会话目录不存在: {}", session_dir.display());
            continue;
        }

        let session = session_parser::parse_session(&session_dir, session_id)?;

        let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;

        let now = chrono::Utc::now().to_rfc3339();
        let project = crate::models::Project {
            id: uuid::Uuid::new_v4().to_string(),
            library_id: library_id.clone(),
            name: session.title.clone(),
            description: format!("从 TF 卡 {} 导入", manifest.device_name),
            cover_asset_id: None,
            source_type: "tf_card".to_string(),
            source_data: Some(serde_json::json!({
                "deviceId": manifest.device_id,
                "sessionId": session_id,
            }).to_string()),
            is_pinned: false,
            is_archived: false,
            created_at: now.clone(),
            updated_at: now.clone(),
            total_duration: None,
            asset_count: 0,
            word_count: 0,
            imported_at: Some(now),
        };
        db::project::insert(&conn, &project)?;

        progress::emit_progress(
            &app, session_id, "copying", idx as u32, total,
            &format!("复制文件..."),
        );

        if let Some(ref audio_path) = session.audio_file_path {
            let _ = file_copier::copy_file(
                Path::new(audio_path), &storage_dir, session_id, "audio",
            );
        }

        let mut local_asset_ids: Vec<(String, String)> = Vec::new();

        let all_assets: Vec<&session_parser::SessionAssetMeta> = session.photos.iter()
            .chain(session.scans.iter())
            .collect();

        for (asset_idx, asset_meta) in all_assets.iter().enumerate() {
            let dest = file_copier::copy_file(
                Path::new(&asset_meta.file_path),
                &storage_dir,
                session_id,
                if session.photos.contains(asset_meta) { "photos" } else { "scans" },
            );

            let local_path = dest
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| asset_meta.file_path.clone());

            let asset_type = if session.photos.iter().any(|p| p.file_name == asset_meta.file_name) {
                "photo"
            } else {
                "scan_text"
            };

            let asset = crate::models::Asset {
                id: uuid::Uuid::new_v4().to_string(),
                project_id: project.id.clone(),
                asset_type: asset_type.to_string(),
                name: asset_meta.file_name.clone(),
                original_name: asset_meta.file_name.clone(),
                file_path: local_path,
                file_size: std::fs::metadata(&asset_meta.file_path).map(|m| m.len() as i64).unwrap_or(0),
                mime_type: guess_mime(Path::new(&asset_meta.file_path)),
                captured_at: asset_meta.captured_at.clone(),
                imported_at: chrono::Utc::now().to_rfc3339(),
                source_type: "tf_card_camera".to_string(),
                source_data: None,
                is_starred: false,
                ..Default::default()
            };
            db::asset::insert(&conn, &asset)?;

            if let Some(meta) = meta_parser::try_parse_meta(&asset_meta.meta_path) {
                let analysis = crate::models::AIAnalysisRow {
                    id: uuid::Uuid::new_v4().to_string(),
                    asset_id: asset.id.clone(),
                    summary: meta.summary.unwrap_or_default(),
                    topics: serde_json::to_string(&meta.topics.unwrap_or_default()).unwrap_or_default(),
                    ocr_text: meta.ocr_text,
                    language: meta.language.unwrap_or_default(),
                    suggested_tags: serde_json::to_string(&meta.suggested_tags.unwrap_or_default()).unwrap_or_default(),
                    suggested_name: meta.suggested_name.unwrap_or_default(),
                };
                db::asset::upsert_analysis(&conn, &analysis)?;
            }

            local_asset_ids.push((asset_meta.file_name.clone(), asset.id.clone()));

            if asset_idx % 5 == 0 {
                progress::emit_progress(
                    &app, session_id, "building", asset_idx as u32, all_assets.len() as u32,
                    &format!("处理素材 {}/{}...", asset_idx + 1, all_assets.len()),
                );
            }
        }

        progress::emit_progress(
            &app, session_id, "building", total, total,
            "构建时间轴...",
        );

        let _ = timeline_builder::build_from_session(
            &conn, &project.id, &session, &local_asset_ids,
        );

        let asset_count = local_asset_ids.len() as i64;
        conn.execute(
            "UPDATE projects SET asset_count = ?2, updated_at = ?3 WHERE id = ?1",
            rusqlite::params![project.id, asset_count, chrono::Utc::now().to_rfc3339()],
        ).map_err(|e| format!("更新项目统计失败: {e}"))?;

        state::mark_synced(&mut sync_state, session_id, &manifest.device_id, &project.id);
        project_ids.push(project.id);

        progress::emit_progress(
            &app, session_id, "done", idx as u32 + 1, total,
            &format!("会话 {session_id} 导入完成"),
        );
    }

    state::save_state(&state_path, &sync_state)?;
    Ok(project_ids)
}

#[tauri::command]
pub fn get_sync_status(arca_path: String) -> Result<Vec<state::SyncedSessionRecord>, String> {
    let app_data = dirs_next::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let state_path = app_data.join("com.notecapt.desktop").join("sync_state.json");
    let sync_state = state::load_state(&state_path);

    let manifest = manifest::parse_manifest(Path::new(&arca_path))?;
    Ok(sync_state
        .synced_sessions
        .into_iter()
        .filter(|r| r.device_id == manifest.device_id)
        .collect())
}

/// task_H2_mime_sniff_fix：按扩展名查询 MIME（大小写不敏感）。
/// 返回空字符串表示扩展名未命中，调用方需走 infer 兜底。
///
/// 拆为独立函数的原因：
/// 1. 单测可分别覆盖"扩展名匹配"和"infer 兜底"两条路径；
/// 2. 上游若拿不到完整路径（仅有文件名）时仍可只查扩展名表。
fn guess_mime_by_extension(file_name: &str) -> &'static str {
    let ext = Path::new(file_name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        // 图片
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "heic" => "image/heic",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "tiff" | "tif" => "image/tiff",

        // 文档
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "md" => "text/markdown",
        "rtf" => "application/rtf",
        "html" | "htm" => "text/html",
        "xml" => "application/xml",
        "json" => "application/json",

        // 表格 / 数据
        "csv" => "text/csv",
        "tsv" => "text/tab-separated-values",

        // Office
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "doc" => "application/msword",
        "xls" => "application/vnd.ms-excel",
        "ppt" => "application/vnd.ms-powerpoint",

        // 电子书 / 归档
        "epub" => "application/epub+zip",
        "zip" => "application/zip",

        // 音频（路由到 iflytek，不进 markitdown）
        "m4a" => "audio/mp4",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "aac" => "audio/aac",
        "flac" => "audio/flac",
        "ogg" => "audio/ogg",
        "opus" => "audio/opus",

        // 视频（audio_route_guard 走 video reject 分支）
        "mp4" => "video/mp4",
        "mov" => "video/quicktime",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",

        // 未命中：留给 infer 内容嗅探兜底
        _ => "",
    }
}

/// task_H2_mime_sniff_fix：综合 MIME 推断。
///
/// 顺序：扩展名映射 → infer magic bytes 嗅探 → `application/octet-stream`。
/// 必须传入"完整路径"而非"仅文件名"，否则 infer 无法读 magic bytes 兜底。
fn guess_mime(path: &Path) -> String {
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    let by_ext = guess_mime_by_extension(file_name);
    if !by_ext.is_empty() {
        return by_ext.to_string();
    }
    // infer 仅读前 ~256 字节，无性能负担；忽略 I/O 错误退回 octet-stream。
    if let Ok(Some(kind)) = infer::get_from_path(path) {
        return kind.mime_type().to_string();
    }
    "application/octet-stream".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// AC-1：扩展名 → MIME 映射覆盖测（≥ 15 类）。
    #[test]
    fn ext_map_covers_markitdown_formats() {
        let cases: &[(&str, &str)] = &[
            // 图片
            ("a.jpg", "image/jpeg"),
            ("a.jpeg", "image/jpeg"),
            ("a.png", "image/png"),
            ("a.heic", "image/heic"),
            ("a.webp", "image/webp"),
            ("a.gif", "image/gif"),
            ("a.bmp", "image/bmp"),
            ("a.tiff", "image/tiff"),
            ("a.tif", "image/tiff"),
            // 文档
            ("a.pdf", "application/pdf"),
            ("a.txt", "text/plain"),
            ("a.md", "text/markdown"),
            ("a.rtf", "application/rtf"),
            ("a.html", "text/html"),
            ("a.htm", "text/html"),
            ("a.xml", "application/xml"),
            ("a.json", "application/json"),
            // 表格 / 数据
            ("a.csv", "text/csv"),
            ("a.tsv", "text/tab-separated-values"),
            // Office
            (
                "a.docx",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            ),
            (
                "a.xlsx",
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            ),
            (
                "a.pptx",
                "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            ),
            ("a.doc", "application/msword"),
            ("a.xls", "application/vnd.ms-excel"),
            ("a.ppt", "application/vnd.ms-powerpoint"),
            // 电子书 / 归档
            ("a.epub", "application/epub+zip"),
            ("a.zip", "application/zip"),
            // 音频
            ("a.m4a", "audio/mp4"),
            ("a.mp3", "audio/mpeg"),
            ("a.wav", "audio/wav"),
            ("a.aac", "audio/aac"),
            ("a.flac", "audio/flac"),
            ("a.ogg", "audio/ogg"),
            ("a.opus", "audio/opus"),
            // 视频
            ("a.mp4", "video/mp4"),
            ("a.mov", "video/quicktime"),
            ("a.webm", "video/webm"),
            ("a.mkv", "video/x-matroska"),
        ];
        for (name, expected) in cases {
            assert_eq!(
                guess_mime_by_extension(name),
                *expected,
                "扩展名映射失败：{name}"
            );
        }
    }

    /// AC-3：扩展名大小写不敏感。
    #[test]
    fn ext_map_is_case_insensitive() {
        for name in ["a.PDF", "a.pdf", "a.PdF", "A.Pdf"] {
            assert_eq!(guess_mime_by_extension(name), "application/pdf");
        }
        assert_eq!(guess_mime_by_extension("photo.JPG"), "image/jpeg");
        assert_eq!(guess_mime_by_extension("sheet.XLSX"), "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet");
    }

    /// AC-1 / 边界：无扩展名 / 未知扩展名返回空，调用方走 infer 兜底。
    #[test]
    fn ext_map_returns_empty_for_unknown() {
        assert_eq!(guess_mime_by_extension("noext"), "");
        assert_eq!(guess_mime_by_extension("a.unknownext"), "");
        assert_eq!(guess_mime_by_extension(""), "");
    }

    /// AC-2：扩展名未命中时，infer 按 magic bytes 嗅探。
    /// 用临时文件写入 PDF 头，扩展名故意伪装为 `.bin`。
    #[test]
    fn infer_sniffs_pdf_when_ext_unknown() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("disguised.bin");
        let mut f = std::fs::File::create(&path).unwrap();
        // PDF magic：%PDF-1.4 加最小占位 body
        f.write_all(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n1 0 obj<<>>endobj\ntrailer<<>>\n%%EOF")
            .unwrap();
        f.flush().unwrap();
        assert_eq!(guess_mime(&path), "application/pdf");
    }

    /// AC-2 边界：扩展名已知时优先扩展名，不调用 infer。
    /// 用 `.pdf` 文件名 + 空 body：扩展名映射先命中。
    #[test]
    fn ext_takes_priority_over_infer() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("empty.pdf");
        std::fs::File::create(&path).unwrap();
        assert_eq!(guess_mime(&path), "application/pdf");
    }

    /// AC-2：扩展名未知 + magic bytes 也无法识别 → octet-stream。
    #[test]
    fn unknown_ext_and_unknown_bytes_falls_back_to_octet_stream() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("random.xyz");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(b"this is just random text with no magic bytes").unwrap();
        f.flush().unwrap();
        assert_eq!(guess_mime(&path), "application/octet-stream");
    }

    /// 调用方语义保护：path 不存在时不 panic，回退 octet-stream。
    #[test]
    fn missing_file_falls_back_to_octet_stream() {
        let p = Path::new("/non/existent/file.unknownext");
        assert_eq!(guess_mime(p), "application/octet-stream");
    }
}
