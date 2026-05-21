use crate::db;
use crate::models;
use crate::sync::session_parser::SessionData;
use crate::sync::meta_parser;
use rusqlite::Connection;

/// 从会话数据构建时间轴：创建 Timeline + AudioTrack + Keyframe
pub fn build_from_session(
    conn: &Connection,
    project_id: &str,
    session: &SessionData,
    local_asset_ids: &[(String, String)],
) -> Result<String, String> {
    let timeline_id = uuid::Uuid::new_v4().to_string();
    let audio_duration = estimate_duration(session);

    let timeline = models::Timeline {
        id: timeline_id.clone(),
        project_id: project_id.to_string(),
        start_time: session.start_time.clone(),
        end_time: session.end_time.clone(),
        duration: audio_duration,
    };
    db::timeline::insert_timeline(conn, &timeline)?;

    if let Some(audio_path) = &session.audio_file_path {
        let track = models::AudioTrack {
            id: uuid::Uuid::new_v4().to_string(),
            timeline_id: timeline_id.clone(),
            file_path: audio_path.clone(),
            file_name: std::path::Path::new(audio_path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            format: detect_audio_format(audio_path),
            duration: audio_duration,
            sample_rate: 44100,
            channels: 1,
            waveform_data: session
                .waveform_file_path
                .as_ref()
                .and_then(|p| std::fs::read_to_string(p).ok())
                .unwrap_or_default(),
            offset_in_timeline: 0.0,
        };
        db::timeline::insert_audio_track(conn, &track)?;
    }

    for (asset_file_name, asset_id) in local_asset_ids {
        let offset = find_offset_for_asset(session, asset_file_name);
        if let Some(anchor_time) = offset {
            let kf = models::Keyframe {
                id: uuid::Uuid::new_v4().to_string(),
                timeline_id: timeline_id.clone(),
                asset_id: asset_id.clone(),
                anchor_time,
                live_audio_clip_id: find_live_clip_for_asset(session, asset_file_name),
                source: "auto".to_string(),
            };
            db::timeline::insert_keyframe(conn, &kf)?;
        }
    }

    Ok(timeline_id)
}

fn estimate_duration(session: &SessionData) -> f64 {
    if !session.end_time.is_empty() && !session.start_time.is_empty() {
        if let (Ok(start), Ok(end)) = (
            chrono::DateTime::parse_from_rfc3339(&session.start_time),
            chrono::DateTime::parse_from_rfc3339(&session.end_time),
        ) {
            return (end - start).num_milliseconds() as f64 / 1000.0;
        }
    }
    0.0
}

fn detect_audio_format(path: &str) -> String {
    let ext = std::path::Path::new(path)
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    match ext.as_str() {
        "m4a" | "mp4" => "m4a".to_string(),
        "mp3" => "mp3".to_string(),
        "aac" => "aac".to_string(),
        _ => "wav".to_string(),
    }
}

fn find_offset_for_asset(session: &SessionData, file_name: &str) -> Option<f64> {
    for photo in &session.photos {
        if photo.file_name == file_name {
            if let Some(offset) = photo.offset_in_audio {
                return Some(offset);
            }
            if let Some(ref meta_path) = photo.meta_path {
                if let Some(meta) = meta_parser::try_parse_meta(&Some(meta_path.clone())) {
                    return meta.offset_in_audio;
                }
            }
        }
    }
    for scan in &session.scans {
        if scan.file_name == file_name {
            if let Some(offset) = scan.offset_in_audio {
                return Some(offset);
            }
            if let Some(ref meta_path) = scan.meta_path {
                if let Some(meta) = meta_parser::try_parse_meta(&Some(meta_path.clone())) {
                    return meta.offset_in_audio;
                }
            }
        }
    }
    None
}

fn find_live_clip_for_asset(session: &SessionData, file_name: &str) -> Option<String> {
    session
        .live_clips
        .iter()
        .find(|c| c.linked_asset_file_name == file_name)
        .map(|c| c.file_name.clone())
}
