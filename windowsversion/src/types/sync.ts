/** TF 卡清单（manifest.json） */
export interface TFCardManifest {
  deviceId: string;
  deviceName: string;
  firmwareVersion: string;
  sessions: TFCardSessionSummary[];
  lastSyncAt: string | null;
}

/** TF 卡会话摘要 */
export interface TFCardSessionSummary {
  sessionId: string;
  title: string;
  startTime: string;
  endTime: string;
  audioDuration: number;
  photoCount: number;
  scanCount: number;
  isSynced: boolean;
}

/** 完整会话数据（解析后） */
export interface SessionData {
  sessionId: string;
  title: string;
  startTime: string;
  endTime: string;
  audioFilePath: string;
  waveformFilePath: string | null;
  photos: SessionAssetMeta[];
  scans: SessionAssetMeta[];
  liveClips: SessionLiveClip[];
}

/** 会话中的素材元数据 */
export interface SessionAssetMeta {
  fileName: string;
  filePath: string;
  capturedAt: string;
  offsetInAudio: number | null;
  aiAnalysis: {
    summary: string;
    topics: string[];
    ocrText: string | null;
    suggestedTags: string[];
    suggestedName: string;
  } | null;
}

/** 实况知识音频片段 */
export interface SessionLiveClip {
  fileName: string;
  filePath: string;
  linkedAssetFileName: string;
  startOffset: number;
  endOffset: number;
}

/** 同步进度事件 */
export interface SyncProgress {
  sessionId: string;
  phase: "scanning" | "copying" | "parsing" | "building" | "done";
  current: number;
  total: number;
  message: string;
}
