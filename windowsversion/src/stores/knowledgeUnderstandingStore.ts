/**
 * knowledgeUnderstandingStore — 知识理解功能前端状态管理
 *
 * 职责：
 *   - 持有当前展示概念的理解辅助数据（摘要、理解框架、用户笔记、镜子反馈、关系）
 *   - 管理三路流式请求状态及中间 buffer（summary / explanation / mirror）
 *   - 提供概念切换时的状态重置
 *
 * 约束（宪章 A4）：不 import 其他 Store，跨 Store 数据在组件层组合
 */

import { create } from "zustand";
import type {
  ConceptSummaryResult,
  ConceptExplanationResult,
  UserNoteResult,
  MirrorFeedbackResult,
  ConceptRelationResult,
  StreamingStatus,
  KnowledgeUnderstandingState,
} from "../types/knowledge-understanding.types";

// ─── Store 接口 ───────────────────────────────────────────────────────────────

interface KnowledgeUnderstandingStore extends KnowledgeUnderstandingState {
  // ── 基础数据 Actions ─────────────────────────────────────────────────────

  /** 设置当前展示的概念 ID */
  setConceptId: (conceptId: string | null) => void;

  /** 更新 AI 摘要数据（加载完成后写入） */
  setSummary: (summary: ConceptSummaryResult | null) => void;

  /** 更新 AI 理解框架数据 */
  setExplanation: (explanation: ConceptExplanationResult | null) => void;

  /** 更新用户笔记数据 */
  setUserNote: (userNote: UserNoteResult | null) => void;

  /** 更新镜子反馈数据 */
  setMirrorFeedback: (feedback: MirrorFeedbackResult | null) => void;

  /** 更新概念关系列表 */
  setRelations: (relations: ConceptRelationResult[]) => void;

  // ── 流式 chunk Actions ───────────────────────────────────────────────────

  /**
   * 追加摘要流式 chunk，同时将 summaryStatus 置为 'streaming'
   * 由 Tauri 'knowledge:summary:chunk' 事件监听器调用
   */
  appendSummaryChunk: (chunk: string) => void;

  /**
   * 追加理解框架流式 chunk，同时将 explanationStatus 置为 'streaming'
   * 由 Tauri 'knowledge:explanation:chunk' 事件监听器调用
   */
  appendExplanationChunk: (chunk: string) => void;

  /**
   * 追加镜子反馈流式 chunk，同时将 mirrorStatus 置为 'streaming'
   * 由 Tauri 'knowledge:mirror:chunk' 事件监听器调用
   */
  appendMirrorChunk: (chunk: string) => void;

  // ── 状态 Actions ─────────────────────────────────────────────────────────

  /** 设置摘要流式状态 */
  setSummaryStatus: (status: StreamingStatus) => void;

  /** 设置理解框架流式状态 */
  setExplanationStatus: (status: StreamingStatus) => void;

  /** 设置镜子反馈流式状态 */
  setMirrorStatus: (status: StreamingStatus) => void;

  /**
   * 切换概念时调用：清空所有缓存数据、buffer，设置新 conceptId，所有 status → 'idle'
   */
  resetForConcept: (conceptId: string) => void;
}

// ─── 初始状态 ─────────────────────────────────────────────────────────────────

const initialState: KnowledgeUnderstandingState = {
  conceptId: null,
  summary: null,
  explanation: null,
  userNote: null,
  mirrorFeedback: null,
  relations: [],
  summaryStatus: "idle",
  explanationStatus: "idle",
  mirrorStatus: "idle",
  summaryStreamBuffer: "",
  explanationStreamBuffer: "",
  mirrorStreamBuffer: "",
};

// ─── Store 实现 ───────────────────────────────────────────────────────────────

export const useKnowledgeUnderstandingStore =
  create<KnowledgeUnderstandingStore>((set) => ({
    ...initialState,

    // ── 基础数据 Actions ─────────────────────────────────────────────────────

    setConceptId: (conceptId) => set({ conceptId }),

    setSummary: (summary) => set({ summary }),

    setExplanation: (explanation) => set({ explanation }),

    setUserNote: (userNote) => set({ userNote }),

    setMirrorFeedback: (feedback) => set({ mirrorFeedback: feedback }),

    setRelations: (relations) => set({ relations }),

    // ── 流式 chunk Actions ───────────────────────────────────────────────────

    appendSummaryChunk: (chunk) =>
      set((s) => ({
        summaryStreamBuffer: s.summaryStreamBuffer + chunk,
        summaryStatus: "streaming",
      })),

    appendExplanationChunk: (chunk) =>
      set((s) => ({
        explanationStreamBuffer: s.explanationStreamBuffer + chunk,
        explanationStatus: "streaming",
      })),

    appendMirrorChunk: (chunk) =>
      set((s) => ({
        mirrorStreamBuffer: s.mirrorStreamBuffer + chunk,
        mirrorStatus: "streaming",
      })),

    // ── 状态 Actions ─────────────────────────────────────────────────────────

    setSummaryStatus: (status) => set({ summaryStatus: status }),

    setExplanationStatus: (status) => set({ explanationStatus: status }),

    setMirrorStatus: (status) => set({ mirrorStatus: status }),

    // ── resetForConcept ──────────────────────────────────────────────────────

    resetForConcept: (conceptId) =>
      set({
        conceptId,
        summary: null,
        explanation: null,
        userNote: null,
        mirrorFeedback: null,
        relations: [],
        summaryStatus: "idle",
        explanationStatus: "idle",
        mirrorStatus: "idle",
        summaryStreamBuffer: "",
        explanationStreamBuffer: "",
        mirrorStreamBuffer: "",
      }),
  }));
