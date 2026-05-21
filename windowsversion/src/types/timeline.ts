/** 时间轴 — 有录音时自动生成 */
export interface Timeline {
  id: string;
  projectId: string;
  startTime: string;
  endTime: string;
  duration: number;
  audioTracks: AudioTrack[];
  keyframes: Keyframe[];
  markers: Marker[];
}

/** 音频轨道 */
export interface AudioTrack {
  id: string;
  timelineId: string;
  filePath: string;
  fileName: string;
  format: AudioFormat;
  duration: number;
  sampleRate: number;
  channels: number;
  waveformData: string;
  transcription: Transcription | null;
  offsetInTimeline: number;
}

export type AudioFormat = "wav" | "m4a" | "mp3" | "aac";

/** AI 转录结果 */
export interface Transcription {
  id: string;
  audioTrackId: string;
  language: string;
  segments: TranscriptionSegment[];
  status: TranscriptionStatus;
}

export type TranscriptionStatus =
  | "pending"
  | "processing"
  | "completed"
  | "failed";

/** 转录片段 */
export interface TranscriptionSegment {
  startTime: number;
  endTime: number;
  text: string;
  confidence: number;
  speaker: string | null;
}

/** 关键帧 — 素材在时间轴上的锚定 */
export interface Keyframe {
  id: string;
  timelineId: string;
  assetId: string;
  anchorTime: number;
  liveAudioClipId: string | null;
  source: "auto" | "manual";
}

/** 时间标记 */
export interface Marker {
  id: string;
  timelineId: string;
  time: number;
  label: string;
  color: string;
  type: MarkerType;
}

export type MarkerType = "bookmark" | "chapter" | "important" | "question";
