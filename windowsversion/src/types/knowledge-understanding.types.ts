/**
 * knowledge-understanding.types.ts — 知识理解功能类型定义
 *
 * 所有接口与 Rust 侧 serde(rename_all = "camelCase") 对齐。
 * Rust 源文件：
 *   src-tauri/src/db/knowledge_understanding.rs
 *   src-tauri/src/commands/knowledge_understanding.rs
 */

// ─── 辅助类型 ─────────────────────────────────────────────────────────────────

/**
 * 对应 Rust prompts 模块的 ExcerptItem（用于 Prompt 构建参考，前端可按需使用）
 * Rust: ExcerptItem { asset_name, project_name, text }
 */
export interface ExcerptItem {
  assetName: string;
  projectName: string;
  text: string;
}

/**
 * 单条有来源的解释项，对应 Rust 内部的 SourcedText / MechanismField
 * 序列化形状：{ "text": "...", "source": "..." }
 */
export interface ExplanationItem {
  text: string;
  source: string;
}

// ─── 数据库行映射类型 ──────────────────────────────────────────────────────────

/**
 * concept_summaries 表行，对应 Rust: ConceptSummary
 * Rust 字段：id, concept_id, summary, source_asset_ids, model, generated_at
 */
export interface ConceptSummaryResult {
  id: string;
  conceptId: string;
  summary: string;
  sourceAssetIds: string[];
  model: string;
  generatedAt: string;
}

/**
 * concept_explanations 表行，对应 Rust: ConceptExplanation
 *
 * 注意：Rust 侧 mechanism / typical_scenarios / common_misconceptions 在数据库中以
 * JSON 字符串存储，但通过 Tauri command 返回给前端时已序列化为对应结构（serde camelCase）。
 * 前端直接按此结构接收，无需二次 parse。
 *
 * Rust 字段：id, concept_id, mechanism(JSON), typical_scenarios(JSON[]),
 *             common_misconceptions(JSON[]|null), essence_sentence,
 *             source_asset_ids, model, generated_at
 */
export interface ConceptExplanationResult {
  id: string;
  conceptId: string;
  /** 对应 Rust mechanism 字段（JSON 字符串），前端接收时为解析后的对象 */
  mechanism: ExplanationItem;
  /** 对应 Rust typical_scenarios 字段 */
  typicalScenarios: ExplanationItem[];
  /** 对应 Rust common_misconceptions 字段（null 表示 LLM 未返回任何误解条目） */
  commonMisconceptions: ExplanationItem[] | null;
  essenceSentence: string;
  sourceAssetIds: string[];
  model: string;
  generatedAt: string;
}

/**
 * 镜子反馈中"还可以了解的角度"条目
 * 结构与 ExplanationItem 相同，单独命名以区分语义
 */
export interface FeedbackPerspective {
  text: string;
  source: string;
}

/**
 * AI 镜子反馈结构（knowledge_validate_explanation 生成，存入 concept_user_notes.mirror_feedback）
 * mirror_feedback 在数据库中以 JSON 字符串存储，前端接收时已解析
 */
export interface MirrorFeedbackResult {
  coveredCount: number;
  coveredPoints: string[];
  additionalPerspectives: FeedbackPerspective[];
  differenceNote: string | null;
}

/**
 * concept_user_notes 表行，对应 Rust: ConceptUserNote
 *
 * 注意：Rust 侧 mirror_feedback 为 Option<String>（JSON 字符串），
 * 前端接收时应为已解析的 MirrorFeedbackResult 或 null。
 *
 * Rust 字段：id, concept_id, user_explanation, mirror_feedback(JSON|null),
 *             last_validated_at(Option<String>), created_at, updated_at
 */
export interface UserNoteResult {
  id: string;
  conceptId: string;
  userExplanation: string;
  mirrorFeedback: MirrorFeedbackResult | null;
  lastValidatedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

/**
 * concept_relations 表行，对应 Rust: ConceptRelation
 *
 * 注意：Rust 侧使用 other_concept_id / other_concept_name（JOIN 后附加），
 * 对应 camelCase: otherConceptId / otherConceptName。
 * 此处额外提供语义别名 relatedConceptId / relatedConceptName 供组件层使用，
 * 实际 Tauri payload 字段名为 otherConceptId / otherConceptName。
 *
 * Rust 字段：id, concept_a_id, concept_b_id, relation_type, source_asset_ids,
 *             co_occurrence_count, created_at, other_concept_id, other_concept_name
 */
export interface ConceptRelationResult {
  id: string;
  conceptAId: string;
  conceptBId: string;
  /** 关系类型，已知值：'co_occurrence' | 'upstream' | 'downstream' */
  relationType: string;
  sourceAssetIds: string[];
  coOccurrenceCount: number;
  createdAt: string;
  /** Rust: other_concept_id — JOIN 后附加的另一侧概念 ID */
  otherConceptId: string;
  /** Rust: other_concept_name — JOIN 后附加的另一侧概念名称 */
  otherConceptName: string;
}

// ─── 聚合数据类型 ─────────────────────────────────────────────────────────────

/**
 * knowledge_get_understanding_data 命令返回的聚合结构
 * 对应 Rust: UnderstandingData { summary, explanation, user_note }
 */
export interface UnderstandingData {
  summary: ConceptSummaryResult | null;
  explanation: ConceptExplanationResult | null;
  userNote: UserNoteResult | null;
}

// ─── 流式状态类型 ─────────────────────────────────────────────────────────────

/**
 * 流式请求状态
 *   idle      — 初始状态，尚未触发请求
 *   streaming — 正在接收流式 chunk，显示骨架屏 + 已有内容
 *   done      — 完成，显示完整内容
 *   error     — 出错，显示错误提示
 */
export type StreamingStatus = "idle" | "streaming" | "done" | "error";

/**
 * Tauri Event payload（流式 chunk）
 * 对应 Rust ChunkPayload { concept_id, chunk, is_final }（serde camelCase）
 * 事件名：'knowledge:summary:chunk' | 'knowledge:explanation:chunk' | 'knowledge:mirror:chunk'
 */
export interface KnowledgeStreamChunk {
  conceptId: string;
  chunk: string;
  isFinal: boolean;
}

// ─── Store 状态类型 ───────────────────────────────────────────────────────────

/**
 * knowledgeUnderstandingStore 的完整状态类型（供组件 import 使用）
 */
export interface KnowledgeUnderstandingState {
  /** 当前展示的概念 ID，null 表示未选中 */
  conceptId: string | null;
  /** AI 摘要数据 */
  summary: ConceptSummaryResult | null;
  /** AI 理解框架数据 */
  explanation: ConceptExplanationResult | null;
  /** 用户笔记数据 */
  userNote: UserNoteResult | null;
  /** AI 镜子反馈（独立字段，便于单独更新） */
  mirrorFeedback: MirrorFeedbackResult | null;
  /** 概念关系列表 */
  relations: ConceptRelationResult[];
  /** 摘要流式状态 */
  summaryStatus: StreamingStatus;
  /** 理解框架流式状态 */
  explanationStatus: StreamingStatus;
  /** 镜子反馈流式状态 */
  mirrorStatus: StreamingStatus;
  /** 摘要流式中间缓冲（累积 chunk） */
  summaryStreamBuffer: string;
  /** 理解框架流式中间缓冲 */
  explanationStreamBuffer: string;
  /** 镜子反馈流式中间缓冲 */
  mirrorStreamBuffer: string;
}
