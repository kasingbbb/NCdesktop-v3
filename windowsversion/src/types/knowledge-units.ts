// ─── 知识进化系统类型定义 v5.0 ────────────────────────────────────────────────

// 知识单元状态机
export type KnowledgeStatus =
  | 'raw'          // 素材已采集，尚未合成
  | 'synthesized'  // 已合成摘要，待用户了解
  | 'understood'   // 用户读了摘要+框架
  | 'articulated'  // 用户写了自己的理解
  | 'validated'    // 完成 AI 核对
  | 'consolidated' // 经过至少 1 次间隔复习
  | 'mastered';    // 技能验证通过

export type AssetInferredType =
  | 'class_content'
  | 'self_study'
  | 'reference'
  | 'review'
  | 'unknown';

export type VoiceMemoType =
  | 'supplementary'
  | 'standalone'
  | 'question'
  | 'connection';

// 理解框架条目（带来源引用）
export interface ExplanationItem {
  text: string;
  source: string;
}

// 理解框架
export interface KnowledgeExplanation {
  mechanism: ExplanationItem;
  typicalScenarios: ExplanationItem[];
  commonMisconceptions: ExplanationItem[] | null;
  essenceSentence: string;
  sourceAssetIds: string[];
  model: string;
  generatedAt: string;
}

// 镜子反馈
export interface MirrorFeedbackResult {
  coveredCount: number;
  coveredPoints: string[];
  additionalPerspectives: { text: string; source: string }[];
  differenceNote: string | null;
}

// ─── 知识单元（完整详情）────────────────────────────────────────────────────

export interface KnowledgeUnit {
  id: string;
  libraryId: string;
  title: string;             // 洞见句（X如何/为什么/是什么Y）
  coreInsight: string;       // 一句话本质
  summary: string | null;
  explanation: KnowledgeExplanation | null;  // 存储为 JSON 字符串，前端解析后的类型
  constituentConceptIds: string[];
  sourceAssetIds: string[];
  status: KnowledgeStatus;
  userNote: string | null;
  lastMirrorFeedback: MirrorFeedbackResult | null;
  depthLevel: 1 | 2 | 3 | 4 | 5;
  legacyConceptIds: string[];
  firstCapturedAt: string;
  lastReviewedAt: string | null;
  nextReviewDue: string | null;
  createdAt: string;
  updatedAt: string;
}

// 知识单元（列表摘要，不含全量内容）
export interface KnowledgeUnitSummary {
  id: string;
  libraryId: string;
  title: string;
  coreInsight: string;
  status: KnowledgeStatus;
  depthLevel: number;
  sourceAssetCount: number;
  snapshotCount: number;
  nextReviewDue: string | null;
  lastReviewedAt: string | null;
  updatedAt: string;
}

export interface CreateKnowledgeUnit {
  id: string;
  libraryId: string;
  title: string;
  coreInsight: string;
  constituentConceptIds: string[];
  sourceAssetIds: string[];
  legacyConceptIds: string[];
  firstCapturedAt: string;
  createdAt: string;
  updatedAt: string;
}

// ─── 理解快照 ────────────────────────────────────────────────────────────────

export interface UnderstandingSnapshot {
  id: string;
  knowledgeUnitId: string;
  userExplanation: string;
  mirrorCoveredCount: number;
  mirrorCoveredPoints: string[];
  mirrorMissedAreas: string[];
  depthLevelAtTime: number;
  sourceAssetCountAtTime: number;
  timestamp: string;
}

export interface CreateSnapshot {
  id: string;
  knowledgeUnitId: string;
  userExplanation: string;
  mirrorCoveredCount: number;
  mirrorCoveredPoints: string[];
  mirrorMissedAreas: string[];
  depthLevelAtTime: number;
  sourceAssetCountAtTime: number;
  timestamp: string;
}

// ─── 素材推断 ────────────────────────────────────────────────────────────────

export interface AssetInference {
  id: string;
  assetId: string;
  sessionId: string | null;
  sessionPeerIds: string[];
  dominantTopics: string[];
  noveltyScore: number;
  closestKnowledgeIds: string[];
  closestScores: number[];
  inferredCourse: string | null;
  inferredType: AssetInferredType;
  isSupplementary: boolean;
  supplementTargetId: string | null;
  confidence: number;
  ambiguityReason: string | null;
  createdAt: string;
}

// ─── 语音备注 ────────────────────────────────────────────────────────────────

export interface VoiceMemo {
  id: string;
  assetId: string | null;
  audioPath: string;
  transcript: string;
  memoType: VoiceMemoType;
  linkTargetId: string | null;
  linkReason: string | null;
  capturedAt: string;
  createdAt: string;
}

// ─── SM-2 间隔复习调度常量 ────────────────────────────────────────────────────

export const REVIEW_INTERVALS: Record<number, number> = {
  1: 1,
  2: 3,
  3: 7,
  4: 14,
  5: 30,
  6: 90,
};

// 根据 depthLevel 和核对质量计算下次复习时间（天数）
export function calcNextReviewDays(depthLevel: number, qualityScore: number): number {
  const base = REVIEW_INTERVALS[Math.min(depthLevel, 6)] ?? 90;
  if (qualityScore >= 0.8) return Math.round(base * 1.5);
  if (qualityScore < 0.4) return REVIEW_INTERVALS[Math.max(depthLevel - 1, 1)] ?? 1;
  return base;
}

// 在 ISO 日期基础上加 N 天
export function addDays(isoDate: string, days: number): string {
  const d = new Date(isoDate);
  d.setDate(d.getDate() + days);
  return d.toISOString().split('T')[0];
}

// 状态标签（中文）
export const STATUS_LABELS: Record<KnowledgeStatus, string> = {
  raw: '刚采集',
  synthesized: '待了解',
  understood: '已读摘要',
  articulated: '有笔记',
  validated: '已核对',
  consolidated: '已巩固',
  mastered: '已掌握',
};

// 状态对应的进度图标（五级，○ ◔ ◑ ◕ ●）
export function statusToIcon(status: KnowledgeStatus): string {
  switch (status) {
    case 'raw':
    case 'synthesized':
      return '○';
    case 'understood':
      return '◔';
    case 'articulated':
      return '◑';
    case 'validated':
    case 'consolidated':
      return '◕';
    case 'mastered':
      return '●';
  }
}
