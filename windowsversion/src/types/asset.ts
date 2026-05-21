import type { Tag } from "./common";
import type { AssetState } from "./workspaceAsset";

/** 素材 — 所有类型的知识碎片
 *
 * task_008 过渡期备注（AC-1 "并存字段"路径）：
 * - 工作区列表后端命令 `get_assets` 已切到 `WorkspaceAssetView`（task_003），
 *   但其它视图（PhotoViewer / KnowledgeHubView / Timeline / Inspector）仍消费
 *   `Asset` 形状（tags / aiAnalysis / source）。
 * - 为避免大面积破坏，本 task 不更换 `assets` 数组的元素类型，而是
 *   在 `Asset` 上挂可选的 WorkspaceAssetView 派生字段（state / renditionPath 等）。
 * - 工作区视图（AssetListView）读取这些可选字段；非工作区视图照常忽略。
 */
export interface Asset {
  id: string;
  projectId: string;
  type: AssetType;
  /** 工作区内展示名（可被 AI 重命名） */
  name: string;
  /** 拖入时的原始文件名；副本在应用目录内整理，原件路径不会被改写 */
  originalName?: string;
  filePath: string;
  fileSize: number;
  mimeType: string;
  tags: Tag[];
  capturedAt: string;
  importedAt: string;
  /** 后端 `sourceData`：如悬浮窗拖入时为原件绝对路径 */
  sourceData?: string | null;
  source: AssetSource;
  aiAnalysis: AIAnalysis | null;
  isStarred: boolean;
  /** 若本资产由其它资产自动转换而来（如 PDF→.md），指向原件 id；否则为空 */
  sourceAssetId?: string | null;

  // ── WorkspaceAssetView 并存字段（task_008 AC-1）────────────────────
  // 这些字段仅在「工作区列表」走 `get_assets`（命令路径 task_003）时填充；
  // 旧路径（搜索/时间轴等）保持 undefined。
  /** 后端 camelCase 原始字段；前端读 `type` 优先（normalizeAsset 已映射） */
  assetType?: string;
  /** 实时四态徽章（done / converting / failed / offline） */
  state?: AssetState;
  /** 失败原因（仅 state==="failed" 时携带） */
  stateReason?: string | null;
  /** source 文件已缺失（不影响 state，仅 UI 标记） */
  sourceMissing?: boolean;
  /** canonical markdown 衍生件 id */
  renditionId?: string | null;
  /** canonical markdown 绝对路径 */
  renditionPath?: string | null;
  renditionSize?: number | null;
  /** 衍生件版本号（多次重试递增） */
  derivativeVersion?: number;
}

export type AssetType =
  | "photo"
  | "scan_text"
  | "audio_clip"
  | "pdf"
  | "webpage"
  | "markdown"
  | "image"
  | "docx"
  | "pptx"
  | "other";

export type AssetSource =
  | { type: "tf_card_camera" }
  | { type: "tf_card_scanner" }
  | { type: "tf_card_mic" }
  | { type: "dropzone_drag" }
  | { type: "dropzone_paste" }
  | { type: "manual_import" };

/** AI 分析结果 */
export interface AIAnalysis {
  summary: string;
  topics: string[];
  ocrText: string | null;
  language: string;
  suggestedTags: string[];
  suggestedName: string;
}
