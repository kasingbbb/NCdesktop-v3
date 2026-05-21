use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Timeline {
    pub id: String,
    pub project_id: String,
    pub start_time: String,
    pub end_time: String,
    pub duration: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioTrack {
    pub id: String,
    pub timeline_id: String,
    pub file_path: String,
    pub file_name: String,
    pub format: String,
    pub duration: f64,
    pub sample_rate: i64,
    pub channels: i64,
    pub waveform_data: String,
    pub offset_in_timeline: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transcription {
    pub id: String,
    pub audio_track_id: String,
    pub language: String,
    pub segments_json: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Keyframe {
    pub id: String,
    pub timeline_id: String,
    pub asset_id: String,
    pub anchor_time: f64,
    pub live_audio_clip_id: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Marker {
    pub id: String,
    pub timeline_id: String,
    pub time: f64,
    pub label: String,
    pub color: String,
    pub marker_type: String,
}
