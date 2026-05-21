/**
 * KnowledgeUnderstandingPage — 深入理解页面主容器
 *
 * 挂载时加载缓存数据，无缓存则自动触发摘要生成。
 * 流式 chunk 通过 Tauri Event 监听接收。
 */

import { useEffect, useRef } from "react";
import { ArrowLeft, BookOpen } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useKnowledgeUnderstandingStore } from "../../stores/knowledgeUnderstandingStore";
import { TransparencyBanner } from "./TransparencyBanner";
import { SummarySection } from "./SummarySection";
import { ExplanationSection } from "./ExplanationSection";
import { UserNotesSection } from "./UserNotesSection";
import { RelationNetworkSection } from "./RelationNetworkSection";
import type {
  UnderstandingData,
  KnowledgeStreamChunk,
} from "../../types/knowledge-understanding.types";

interface KnowledgeUnderstandingPageProps {
  conceptId: string;
  conceptName: string;
  onBack: () => void;
}

export function KnowledgeUnderstandingPage({
  conceptId,
  conceptName,
  onBack,
}: KnowledgeUnderstandingPageProps) {
  const store = useKnowledgeUnderstandingStore;
  const unlistenRefs = useRef<UnlistenFn[]>([]);

  // ── 挂载：加载缓存 + 注册事件监听 ──────────────────────────────────────────

  useEffect(() => {
    let cancelled = false;

    const init = async () => {
      // 1. 加载已有缓存数据
      try {
        const data = await invoke<UnderstandingData>(
          "knowledge_get_understanding_data",
          { conceptId }
        );

        if (cancelled) return;

        if (data.summary) {
          store.getState().setSummary(data.summary);
          store.getState().setSummaryStatus("done");
        }
        if (data.explanation) {
          store.getState().setExplanation(data.explanation);
          store.getState().setExplanationStatus("done");
        }
        if (data.userNote) {
          store.getState().setUserNote(data.userNote);
          if (data.userNote.mirrorFeedback) {
            store.getState().setMirrorFeedback(data.userNote.mirrorFeedback);
          }
        }

        // 2. 无 summary 缓存时自动触发生成
        if (!data.summary) {
          store.getState().setSummaryStatus("streaming");
          try {
            await invoke<string>("knowledge_generate_summary", {
              conceptId,
              forceRegenerate: false,
            });
          } catch (e) {
            if (!cancelled) {
              store.getState().setSummaryStatus("error");
            }
          }
        }
      } catch (e) {
        if (!cancelled) {
          store.getState().setSummaryStatus("error");
        }
      }
    };

    // 注册三路流式 chunk 监听
    const setupListeners = async () => {
      const summaryUnlisten = await listen<KnowledgeStreamChunk>(
        "knowledge:summary:chunk",
        (event) => {
          if (event.payload.conceptId !== conceptId) return;
          const s = store.getState();
          if (event.payload.isFinal) {
            s.appendSummaryChunk(event.payload.chunk);
            // 重新加载完整数据
            void reloadSummary(conceptId);
          } else {
            s.appendSummaryChunk(event.payload.chunk);
          }
        }
      );

      const explanationUnlisten = await listen<KnowledgeStreamChunk>(
        "knowledge:explanation:chunk",
        (event) => {
          if (event.payload.conceptId !== conceptId) return;
          const s = store.getState();
          if (event.payload.isFinal) {
            s.appendExplanationChunk(event.payload.chunk);
            void reloadExplanation(conceptId);
          } else {
            s.appendExplanationChunk(event.payload.chunk);
          }
        }
      );

      const mirrorUnlisten = await listen<KnowledgeStreamChunk>(
        "knowledge:mirror:chunk",
        (event) => {
          if (event.payload.conceptId !== conceptId) return;
          const s = store.getState();
          if (event.payload.isFinal) {
            s.appendMirrorChunk(event.payload.chunk);
            s.setMirrorStatus("done");
          } else {
            s.appendMirrorChunk(event.payload.chunk);
          }
        }
      );

      unlistenRefs.current = [summaryUnlisten, explanationUnlisten, mirrorUnlisten];
    };

    void setupListeners();
    void init();

    return () => {
      cancelled = true;
      for (const unlisten of unlistenRefs.current) {
        unlisten();
      }
      unlistenRefs.current = [];
    };
  }, [conceptId]);

  // ── 重新加载完整数据（流结束后） ──────────────────────────────────────────

  const reloadSummary = async (cId: string) => {
    try {
      const data = await invoke<UnderstandingData>(
        "knowledge_get_understanding_data",
        { conceptId: cId }
      );
      const s = store.getState();
      if (data.summary) {
        s.setSummary(data.summary);
      }
      s.setSummaryStatus("done");
    } catch {
      store.getState().setSummaryStatus("error");
    }
  };

  const reloadExplanation = async (cId: string) => {
    try {
      const data = await invoke<UnderstandingData>(
        "knowledge_get_understanding_data",
        { conceptId: cId }
      );
      const s = store.getState();
      if (data.explanation) {
        s.setExplanation(data.explanation);
      }
      s.setExplanationStatus("done");
    } catch {
      store.getState().setExplanationStatus("error");
    }
  };

  // ── 用户操作回调 ──────────────────────────────────────────────────────────

  const handleRegenerateSummary = () => {
    const s = store.getState();
    s.setSummary(null);
    s.setSummaryStatus("streaming");
    s.appendSummaryChunk(""); // 确保 buffer 被标记为 streaming
    // 重置 buffer
    store.setState({ summaryStreamBuffer: "" });

    void invoke<string>("knowledge_generate_summary", {
      conceptId,
      forceRegenerate: true,
    }).catch(() => {
      store.getState().setSummaryStatus("error");
    });
  };

  const handleGenerateExplanation = () => {
    const s = store.getState();
    s.setExplanation(null);
    s.setExplanationStatus("streaming");
    store.setState({ explanationStreamBuffer: "" });

    void invoke<string>("knowledge_generate_explanation", {
      conceptId,
      forceRegenerate: false,
    }).catch(() => {
      store.getState().setExplanationStatus("error");
    });
  };

  const handleNavigateToRelatedConcept = (relatedConceptId: string) => {
    store.getState().resetForConcept(relatedConceptId);
  };

  const handleRegenerateExplanation = () => {
    const s = store.getState();
    s.setExplanation(null);
    s.setExplanationStatus("streaming");
    store.setState({ explanationStreamBuffer: "" });

    void invoke<string>("knowledge_generate_explanation", {
      conceptId,
      forceRegenerate: true,
    }).catch(() => {
      store.getState().setExplanationStatus("error");
    });
  };

  // ── 渲染 ──────────────────────────────────────────────────────────────────

  return (
    <div className="flex flex-col h-full overflow-hidden bg-[var(--surface-primary)]">
      {/* 顶部导航栏 */}
      <div
        className="flex-shrink-0 flex items-center gap-[var(--space-3)] px-[var(--space-4)] py-[var(--space-3)] border-b"
        style={{ borderColor: "var(--border-primary)" }}
      >
        <button
          type="button"
          onClick={onBack}
          className="flex items-center gap-1.5 text-[var(--text-sm)] transition-colors"
          style={{ color: "var(--text-secondary)" }}
        >
          <ArrowLeft size={14} />
          ← 返回概念列表
        </button>
        <div className="flex items-center gap-[var(--space-2)] ml-auto">
          <BookOpen size={14} style={{ color: "var(--brand-navy)" }} />
          <span
            className="text-[var(--text-sm)] font-medium"
            style={{ color: "var(--text-primary)" }}
          >
            {conceptName}
          </span>
        </div>
      </div>

      {/* 透明度声明 */}
      <TransparencyBanner />

      {/* 滚动内容区 */}
      <div className="flex-1 overflow-y-auto">
        <div className="p-[var(--space-5)] space-y-[var(--space-6)] max-w-2xl">
          <SummarySection onRegenerate={handleRegenerateSummary} />

          <div className="h-px" style={{ background: "var(--border-primary)" }} />

          <ExplanationSection
            onGenerate={handleGenerateExplanation}
            onRegenerate={handleRegenerateExplanation}
          />

          <div className="h-px" style={{ background: "var(--border-primary)" }} />

          <UserNotesSection conceptId={conceptId} />

          <div className="h-px" style={{ background: "var(--border-primary)" }} />

          <RelationNetworkSection
            conceptId={conceptId}
            onNavigateToConcept={handleNavigateToRelatedConcept}
          />
        </div>
      </div>
    </div>
  );
}
