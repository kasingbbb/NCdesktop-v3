/** OpenAI 对话消息 */
export interface ChatMessage {
  role: "system" | "user" | "assistant";
  content: string;
}

/** LLM 运行时配置（不持久化 API Key） */
export interface LLMConfig {
  isConfigured: boolean;
  model: string;
  maxTokens: number;
  temperature: number;
  baseUrl: string;
}

/** LLM 请求记录（仅本地存储） */
export interface LLMRequestLog {
  id: string;
  timestamp: string;
  type: LLMRequestType;
  model: string;
  inputTokens: number;
  outputTokens: number;
  latencyMs: number;
  success: boolean;
  errorMessage?: string;
}

export type LLMRequestType =
  | "summarize"
  | "chat"
  | "classify"
  | "transcribe"
  | "embed";
