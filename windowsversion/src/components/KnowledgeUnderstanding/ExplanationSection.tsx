/**
 * ExplanationSection — 「理解框架」区域
 *
 * 渲染规范：
 *   - 不在页面挂载时自动触发，需要用户手动点击「生成理解框架」
 *   - 若已有缓存，直接渲染 4 个模块
 *   - 4 模块：核心机制、典型场景、常见误区、一句话精华
 */

import { RefreshCw, Loader2, Sparkles } from "lucide-react";
import { useKnowledgeUnderstandingStore } from "../../stores/knowledgeUnderstandingStore";
import { ExplanationItemCard } from "./ExplanationItem";
import type { StreamingStatus } from "../../types/knowledge-understanding.types";

interface ExplanationSectionProps {
  onGenerate: () => void;
  onRegenerate: () => void;
}

export function ExplanationSection({
  onGenerate,
  onRegenerate,
}: ExplanationSectionProps) {
  const explanation = useKnowledgeUnderstandingStore((s) => s.explanation);
  const status = useKnowledgeUnderstandingStore((s) => s.explanationStatus);
  const buffer = useKnowledgeUnderstandingStore((s) => s.explanationStreamBuffer);

  // 无缓存 + 未触发：显示触发按钮
  if (!explanation && status === "idle") {
    return (
      <section className="space-y-[var(--space-3)]">
        <h3
          className="text-[var(--text-sm)] font-semibold"
          style={{ color: "var(--text-primary)" }}
        >
          理解框架
        </h3>
        <div className="flex justify-center py-[var(--space-4)]">
          <button
            type="button"
            onClick={onGenerate}
            className="flex items-center gap-[var(--space-2)] px-[var(--space-5)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-sm)] font-medium transition-colors"
            style={{ background: "var(--brand-navy)", color: "#fff" }}
          >
            <Sparkles size={14} />
            生成理解框架
          </button>
        </div>
      </section>
    );
  }

  return (
    <section className="space-y-[var(--space-3)]">
      {/* 标题 + 重新生成 */}
      <div className="flex items-center justify-between">
        <h3
          className="text-[var(--text-sm)] font-semibold"
          style={{ color: "var(--text-primary)" }}
        >
          理解框架
        </h3>
        {(status === "done" || explanation) && (
          <button
            type="button"
            onClick={onRegenerate}
            disabled={status === "streaming"}
            className="flex items-center gap-1 text-[var(--text-xs)] transition-colors"
            style={{ color: "var(--text-tertiary)", opacity: status === "streaming" ? 0.5 : 1 }}
          >
            <RefreshCw size={11} />
            重新生成
          </button>
        )}
      </div>

      {/* 内容区 */}
      <ExplanationContent status={status} buffer={buffer} />
    </section>
  );
}

function ExplanationContent({
  status,
  buffer,
}: {
  status: StreamingStatus;
  buffer: string;
}) {
  const explanation = useKnowledgeUnderstandingStore((s) => s.explanation);

  if (status === "error") {
    return (
      <div
        className="px-[var(--space-3)] py-[var(--space-3)] rounded-[var(--radius-md)] text-[var(--text-sm)]"
        style={{
          background: "rgba(239,68,68,0.06)",
          border: "1px solid rgba(239,68,68,0.15)",
          color: "var(--text-secondary)",
        }}
      >
        生成失败，请检查网络或重试
      </div>
    );
  }

  if (status === "streaming") {
    return (
      <div
        className="px-[var(--space-3)] py-[var(--space-3)] rounded-[var(--radius-md)] text-[var(--text-sm)] leading-relaxed space-y-2"
        style={{
          background: "var(--surface-secondary)",
          border: "1px solid var(--border-primary)",
          color: "var(--text-secondary)",
        }}
      >
        {buffer ? (
          <p className="whitespace-pre-wrap">{buffer}</p>
        ) : (
          <StreamingSkeleton />
        )}
        <div className="flex items-center gap-1.5">
          <Loader2 size={11} className="animate-spin" style={{ color: "var(--text-tertiary)" }} />
          <span className="text-[var(--text-xs)]" style={{ color: "var(--text-tertiary)" }}>
            正在生成理解框架…
          </span>
        </div>
      </div>
    );
  }

  if (!explanation) return null;

  return (
    <div className="space-y-[var(--space-4)]">
      {/* 核心机制 */}
      <div className="space-y-[var(--space-2)]">
        <SubTitle>核心机制</SubTitle>
        <ExplanationItemCard item={explanation.mechanism} />
      </div>

      {/* 典型场景 */}
      {explanation.typicalScenarios.length > 0 && (
        <div className="space-y-[var(--space-2)]">
          <SubTitle>典型场景</SubTitle>
          <div className="space-y-[var(--space-2)]">
            {explanation.typicalScenarios.map((scenario, i) => (
              <ExplanationItemCard key={i} item={scenario} />
            ))}
          </div>
        </div>
      )}

      {/* 常见误区（null 或空时不渲染） */}
      {explanation.commonMisconceptions && explanation.commonMisconceptions.length > 0 && (
        <div className="space-y-[var(--space-2)]">
          <SubTitle>常见误区</SubTitle>
          <div className="space-y-[var(--space-2)]">
            {explanation.commonMisconceptions.map((item, i) => (
              <ExplanationItemCard key={i} item={item} />
            ))}
          </div>
        </div>
      )}

      {/* 一句话精华 */}
      {explanation.essenceSentence && (
        <div className="space-y-[var(--space-2)]">
          <SubTitle>一句话精华</SubTitle>
          <div
            className="px-[var(--space-3)] py-[var(--space-3)] rounded-[var(--radius-md)] text-[var(--text-sm)] leading-relaxed"
            style={{
              background: "var(--surface-secondary)",
              border: "1px solid var(--border-primary)",
              color: "var(--text-primary)",
              fontWeight: 500,
            }}
          >
            <p>{explanation.essenceSentence}</p>
            <p
              className="text-[var(--text-xs)] mt-[var(--space-1)]"
              style={{ color: "var(--text-tertiary)", fontWeight: 400 }}
            >
              （根据你的文档总结）
            </p>
          </div>
        </div>
      )}
    </div>
  );
}

function SubTitle({ children }: { children: React.ReactNode }) {
  return (
    <h4
      className="text-[var(--text-xs)] font-medium uppercase tracking-wide"
      style={{ color: "var(--text-tertiary)" }}
    >
      {children}
    </h4>
  );
}

function StreamingSkeleton() {
  return (
    <div className="space-y-[var(--space-2)] animate-pulse">
      {[85, 65, 75].map((w, i) => (
        <div
          key={i}
          className="h-3 rounded"
          style={{ width: `${w}%`, background: "var(--surface-tertiary)" }}
        />
      ))}
    </div>
  );
}
