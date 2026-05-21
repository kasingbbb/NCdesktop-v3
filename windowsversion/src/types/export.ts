/** 导出配置 */
export interface ExportConfig {
  format: ExportFormat;
  includeAudioTranscription: boolean;
  includeOCRText: boolean;
  includePhotos: boolean;
  includeNotes: boolean;
  includeTimestamps: boolean;
  timeRange: { start: number; end: number } | null;
}

export type ExportFormat = "markdown" | "pdf" | "html" | "json";
