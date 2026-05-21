// v2.1 — 知识关联相关类型

/** 概念（基础信息 + 统计，用于左侧列表） */
export interface ConceptWithStats {
  id: string;
  name: string;
  aliases: string[];
  definition: string | null;
  sourceProjectCount: number;
  viewpointCount: number;
  caseCount: number;
  userEdited: boolean;
}

/** 概念完整信息（用于右侧详情面板） */
export interface Concept {
  id: string;
  libraryId: string;
  name: string;
  aliases: string[];
  definition: string | null;
  sourceAssetIds: string[];
  sourceProjectIds: string[];
  userEdited: boolean;
  createdAt: string;
  updatedAt: string;
}

/** 概念观点 */
export interface ConceptViewpoint {
  id: string;
  conceptId: string;
  perspective: string;
  summary: string;
  sourceContext: string | null;
  sourceAssetId: string | null;
  generatedAt: string;
}

/** 概念案例 */
export interface ConceptCase {
  id: string;
  conceptId: string;
  title: string;
  excerpt: string;
  sourceAssetId: string | null;
  sourceLocation: string | null;
  relevanceNote: string | null;
}

/** 知识拓展（上游前置 / 下游应用） */
export interface ConceptExtension {
  id: string;
  conceptId: string;
  direction: "upstream" | "downstream";
  name: string;
  description: string | null;
  relationship: string | null;
}

/** 概念详情（右侧面板完整数据） */
export interface ConceptDetail {
  concept: Concept;
  viewpoints: ConceptViewpoint[];
  cases: ConceptCase[];
  extensions: ConceptExtension[];
}

/** 概念提取进度（后台任务） */
export interface ExtractionProgress {
  totalAssets: number;
  processed: number;
  conceptsFound: number;
  status: "running" | "completed" | "error";
  /**
   * 错误信息（仅 status === "error" 时由后端 emit 或前端兜底填入）。
   * concept_rescan_perf_v1 / task_perf_02：错误态进度条文案展示用。
   */
  error?: string | null;
}
