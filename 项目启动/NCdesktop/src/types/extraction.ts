export interface ExtractedContent {
  id: string;
  assetId: string;
  status: ExtractionStatus;
  errorMessage: string | null;
  retryCount: number;
  rawText: string | null;
  structuredMd: string | null;
  qualityLevel: number;
  extractorType: string;
  segmentsJson: string | null;
  /**
   * task_026 AC-3：KC 增强状态。null = 未走 KC、"true" = enrich 成功、
   * "false" = enrich 失败、"partial" = LLM 不可用规则兜底（task_011
   * PartialLlmUnavailable）。Inspector "重新增强"按钮仅在非 null && 非 "false"
   * 时显示（即 KC 已经介入过该 asset 才允许强制重 enrich）。
   */
  kcEnriched: string | null;
  createdAt: string;
  updatedAt: string;
}

export type ExtractionStatus = 'pending' | 'extracting' | 'extracted' | 'failed' | 'unsupported';

export interface PipelineTask {
  id: string;
  assetId: string;
  taskType: 'extract' | 'enhance' | 'index';
  status: 'queued' | 'running' | 'completed' | 'failed' | 'cancelled';
  retryCount: number;
  maxRetries: number;
  errorMessage: string | null;
  priority: number;
  batchId: string | null;
  createdAt: string;
  startedAt: string | null;
  completedAt: string | null;
}

export interface PipelineProgress {
  queued: number;
  running: number;
  completed: number;
  failed: number;
  cancelled: number;
}

export interface ExtractionProgressEvent {
  assetId: string;
  status: string;
  message: string;
}

export interface ExtractionCompletedEvent {
  assetId: string;
  qualityLevel: number;
  extractorType: string;
}

export interface ExtractionFailedEvent {
  assetId: string;
  errorMessage: string;
  retryCount: number;
}
