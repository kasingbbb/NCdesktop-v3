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
