use crate::db::{self, Database};
use crate::models;
use tauri::State;

#[tauri::command]
pub fn get_timeline(
    database: State<'_, Database>,
    project_id: String,
) -> Result<Option<models::Timeline>, String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    db::timeline::get_timeline_by_project(&conn, &project_id)
}

#[tauri::command]
pub fn create_timeline(
    database: State<'_, Database>,
    project_id: String,
    start_time: String,
    end_time: String,
    duration: f64,
) -> Result<models::Timeline, String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    let timeline = models::Timeline {
        id: uuid::Uuid::new_v4().to_string(),
        project_id,
        start_time,
        end_time,
        duration,
    };
    db::timeline::insert_timeline(&conn, &timeline)?;
    Ok(timeline)
}

// ── AudioTrack ─────────────────────────────────────

#[tauri::command]
pub fn get_audio_tracks(
    database: State<'_, Database>,
    timeline_id: String,
) -> Result<Vec<models::AudioTrack>, String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    db::timeline::get_audio_tracks_by_timeline(&conn, &timeline_id)
}

#[tauri::command]
pub fn create_audio_track(
    database: State<'_, Database>,
    timeline_id: String,
    file_path: String,
    file_name: String,
    format: String,
    duration: f64,
    sample_rate: i64,
    channels: i64,
) -> Result<models::AudioTrack, String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    let track = models::AudioTrack {
        id: uuid::Uuid::new_v4().to_string(),
        timeline_id,
        file_path,
        file_name,
        format,
        duration,
        sample_rate,
        channels,
        waveform_data: String::new(),
        offset_in_timeline: 0.0,
    };
    db::timeline::insert_audio_track(&conn, &track)?;
    Ok(track)
}

// ── Keyframe ───────────────────────────────────────

#[tauri::command]
pub fn get_keyframes(
    database: State<'_, Database>,
    timeline_id: String,
) -> Result<Vec<models::Keyframe>, String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    db::timeline::get_keyframes_by_timeline(&conn, &timeline_id)
}

#[tauri::command]
pub fn create_keyframe(
    database: State<'_, Database>,
    timeline_id: String,
    asset_id: String,
    anchor_time: f64,
    source: String,
) -> Result<models::Keyframe, String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    let keyframe = models::Keyframe {
        id: uuid::Uuid::new_v4().to_string(),
        timeline_id,
        asset_id,
        anchor_time,
        live_audio_clip_id: None,
        source,
    };
    db::timeline::insert_keyframe(&conn, &keyframe)?;
    Ok(keyframe)
}

#[tauri::command]
pub fn delete_keyframe(database: State<'_, Database>, id: String) -> Result<(), String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    db::timeline::delete_keyframe(&conn, &id)
}

// ── Marker ─────────────────────────────────────────

#[tauri::command]
pub fn get_markers(
    database: State<'_, Database>,
    timeline_id: String,
) -> Result<Vec<models::Marker>, String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    db::timeline::get_markers_by_timeline(&conn, &timeline_id)
}

#[tauri::command]
pub fn create_marker(
    database: State<'_, Database>,
    timeline_id: String,
    time: f64,
    label: String,
    color: String,
    marker_type: String,
) -> Result<models::Marker, String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    let marker = models::Marker {
        id: uuid::Uuid::new_v4().to_string(),
        timeline_id,
        time,
        label,
        color,
        marker_type,
    };
    db::timeline::insert_marker(&conn, &marker)?;
    Ok(marker)
}

#[tauri::command]
pub fn delete_marker(database: State<'_, Database>, id: String) -> Result<(), String> {
    let conn = database.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    db::timeline::delete_marker(&conn, &id)
}
