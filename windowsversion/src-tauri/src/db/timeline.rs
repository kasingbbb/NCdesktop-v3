use crate::models::{AudioTrack, Keyframe, Marker, Timeline, Transcription};
use rusqlite::{params, Connection, OptionalExtension};

// ── Timeline ───────────────────────────────────────

pub fn insert_timeline(conn: &Connection, t: &Timeline) -> Result<(), String> {
    conn.execute(
        "INSERT INTO timelines (id, project_id, start_time, end_time, duration)
         VALUES (?1,?2,?3,?4,?5)",
        params![t.id, t.project_id, t.start_time, t.end_time, t.duration],
    )
    .map_err(|e| format!("插入时间轴失败: {e}"))?;
    Ok(())
}

pub fn get_timeline_by_project(
    conn: &Connection,
    project_id: &str,
) -> Result<Option<Timeline>, String> {
    conn.query_row(
        "SELECT id, project_id, start_time, end_time, duration FROM timelines WHERE project_id = ?1",
        params![project_id],
        |row| {
            Ok(Timeline {
                id: row.get(0)?,
                project_id: row.get(1)?,
                start_time: row.get(2)?,
                end_time: row.get(3)?,
                duration: row.get(4)?,
            })
        },
    )
    .optional()
    .map_err(|e| format!("查询时间轴失败: {e}"))
}

// ── AudioTrack ─────────────────────────────────────

pub fn insert_audio_track(conn: &Connection, t: &AudioTrack) -> Result<(), String> {
    conn.execute(
        "INSERT INTO audio_tracks (id, timeline_id, file_path, file_name, format,
         duration, sample_rate, channels, waveform_data, offset_in_timeline)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            t.id, t.timeline_id, t.file_path, t.file_name, t.format,
            t.duration, t.sample_rate, t.channels, t.waveform_data, t.offset_in_timeline,
        ],
    )
    .map_err(|e| format!("插入音频轨道失败: {e}"))?;
    Ok(())
}

pub fn get_audio_tracks_by_timeline(
    conn: &Connection,
    timeline_id: &str,
) -> Result<Vec<AudioTrack>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, timeline_id, file_path, file_name, format,
             duration, sample_rate, channels, waveform_data, offset_in_timeline
             FROM audio_tracks WHERE timeline_id = ?1 ORDER BY offset_in_timeline",
        )
        .map_err(|e| format!("查询音频轨道失败: {e}"))?;

    let rows = stmt
        .query_map(params![timeline_id], |row| {
            Ok(AudioTrack {
                id: row.get(0)?,
                timeline_id: row.get(1)?,
                file_path: row.get(2)?,
                file_name: row.get(3)?,
                format: row.get(4)?,
                duration: row.get(5)?,
                sample_rate: row.get(6)?,
                channels: row.get(7)?,
                waveform_data: row.get(8)?,
                offset_in_timeline: row.get(9)?,
            })
        })
        .map_err(|e| format!("遍历音频轨道失败: {e}"))?;

    let mut result = Vec::new();
    for r in rows {
        result.push(r.map_err(|e| format!("读取行失败: {e}"))?);
    }
    Ok(result)
}

// ── Transcription ──────────────────────────────────

pub fn upsert_transcription(conn: &Connection, t: &Transcription) -> Result<(), String> {
    conn.execute(
        "INSERT INTO transcriptions (id, audio_track_id, language, segments_json, status)
         VALUES (?1,?2,?3,?4,?5)
         ON CONFLICT(audio_track_id) DO UPDATE SET
           language=excluded.language, segments_json=excluded.segments_json, status=excluded.status",
        params![t.id, t.audio_track_id, t.language, t.segments_json, t.status],
    )
    .map_err(|e| format!("写入转录失败: {e}"))?;
    Ok(())
}

pub fn get_transcription_by_audio(
    conn: &Connection,
    audio_track_id: &str,
) -> Result<Option<Transcription>, String> {
    conn.query_row(
        "SELECT id, audio_track_id, language, segments_json, status
         FROM transcriptions WHERE audio_track_id = ?1",
        params![audio_track_id],
        |row| {
            Ok(Transcription {
                id: row.get(0)?,
                audio_track_id: row.get(1)?,
                language: row.get(2)?,
                segments_json: row.get(3)?,
                status: row.get(4)?,
            })
        },
    )
    .optional()
    .map_err(|e| format!("查询转录失败: {e}"))
}

// ── Keyframe ───────────────────────────────────────

pub fn insert_keyframe(conn: &Connection, k: &Keyframe) -> Result<(), String> {
    conn.execute(
        "INSERT INTO keyframes (id, timeline_id, asset_id, anchor_time, live_audio_clip_id, source)
         VALUES (?1,?2,?3,?4,?5,?6)",
        params![k.id, k.timeline_id, k.asset_id, k.anchor_time, k.live_audio_clip_id, k.source],
    )
    .map_err(|e| format!("插入关键帧失败: {e}"))?;
    Ok(())
}

pub fn get_keyframes_by_timeline(
    conn: &Connection,
    timeline_id: &str,
) -> Result<Vec<Keyframe>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, timeline_id, asset_id, anchor_time, live_audio_clip_id, source
             FROM keyframes WHERE timeline_id = ?1 ORDER BY anchor_time",
        )
        .map_err(|e| format!("查询关键帧失败: {e}"))?;

    let rows = stmt
        .query_map(params![timeline_id], |row| {
            Ok(Keyframe {
                id: row.get(0)?,
                timeline_id: row.get(1)?,
                asset_id: row.get(2)?,
                anchor_time: row.get(3)?,
                live_audio_clip_id: row.get(4)?,
                source: row.get(5)?,
            })
        })
        .map_err(|e| format!("遍历关键帧失败: {e}"))?;

    let mut result = Vec::new();
    for r in rows {
        result.push(r.map_err(|e| format!("读取行失败: {e}"))?);
    }
    Ok(result)
}

pub fn delete_keyframe(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM keyframes WHERE id = ?1", params![id])
        .map_err(|e| format!("删除关键帧失败: {e}"))?;
    Ok(())
}

// ── Marker ─────────────────────────────────────────

pub fn insert_marker(conn: &Connection, m: &Marker) -> Result<(), String> {
    conn.execute(
        "INSERT INTO markers (id, timeline_id, time, label, color, marker_type)
         VALUES (?1,?2,?3,?4,?5,?6)",
        params![m.id, m.timeline_id, m.time, m.label, m.color, m.marker_type],
    )
    .map_err(|e| format!("插入标记失败: {e}"))?;
    Ok(())
}

pub fn get_markers_by_timeline(
    conn: &Connection,
    timeline_id: &str,
) -> Result<Vec<Marker>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, timeline_id, time, label, color, marker_type
             FROM markers WHERE timeline_id = ?1 ORDER BY time",
        )
        .map_err(|e| format!("查询标记失败: {e}"))?;

    let rows = stmt
        .query_map(params![timeline_id], |row| {
            Ok(Marker {
                id: row.get(0)?,
                timeline_id: row.get(1)?,
                time: row.get(2)?,
                label: row.get(3)?,
                color: row.get(4)?,
                marker_type: row.get(5)?,
            })
        })
        .map_err(|e| format!("遍历标记失败: {e}"))?;

    let mut result = Vec::new();
    for r in rows {
        result.push(r.map_err(|e| format!("读取行失败: {e}"))?);
    }
    Ok(result)
}

pub fn delete_marker(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM markers WHERE id = ?1", params![id])
        .map_err(|e| format!("删除标记失败: {e}"))?;
    Ok(())
}
