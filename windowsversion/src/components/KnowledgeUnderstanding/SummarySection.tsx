/**
 * SummarySection — 「你的文档怎么说」摘要区域
 *
 * 渲染规范：
 *   - 流式渲染中显示骨架屏 + 已有内容
 *   - 完成后显示摘要文本 + 来源文档标注
 *   - 「重新生成」按钮
 */

import { RefreshCw, Loader2 } from "lucide-react";
import { useKnowledgeUnderstandingStore } from "../../stores/knowledgeUnderstandingStore";
import type { ConceptSummaryResult, StreamingStatus } from "../../types/knowledge-understanding.types";

interface SummarySectionProps {
  onRegenerate: () => void;
}

export function SummarySection({ onRegenerate }: SummarySectionProps) {
  const summary = useKnowledgeUnderstandingStore((s) => s.summary);
  const status = useKnowledgeUnderstandingStore((s) => s.summaryStatus);
  const buffer = useKnowledgeUnderstandingStore((s) => s.summaryStreamBuffer);

  return (
    <section className="space-y-[var(--space-3)]">
      {/* 标题 + 重新生成 */}
      <div className="flex items-center justify-between">
        <h3
          className="text-[var(--text-sm)] font-semibold"
          style={{ color: "var(--text-primary)" }}
        >
          你的文档怎么说
        </h3>
        {(status === "done" || summary) && (
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
      <SummaryContent summary={summary} status={status} buffer={buffer} />
    </section>
  );
}

function SummaryContent({
  summary,
  status,
  buffer,
}: {
  summary: ConceptSummaryResult | null;
  status: StreamingStatus;
  buffer: string;
}) {
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
            正在生成…
          </span>
        </div>
      </div>
    );
  }

  if (status === "idle" && !summary) {
    return (
      <div
        className="px-[var(--space-3)] py-[var(--space-3)] rounded-[var(--radius-md)] text-[var(--text-sm)]"
        style={{
          background: "var(--surface-secondary)",
          border: "1px solid var(--border-primary)",
          color: "var(--text-tertiary)",
        }}
      >
        <StreamingSkeleton />
      </div>
    );
  }

  if (summary) {
    return (
      <div
        className="px-[var(--space-3)] py-[var(--space-3)] rounded-[var(--radius-md)] text-[var(--text-sm)] leading-relaxed"
        style={{
          background: "var(--surface-secondary)",
          border: "1px solid var(--border-primary)",
          color: "var(--text-secondary)",
        }}
      >
        <p className="whitespace-pre-wrap">{summary.summary}</p>
        {summary.sourceAssetIds.length > 0 && (
          <div className="mt-[var(--space-2)] flex flex-wrap gap-[var(--space-1)]">
            {summary.sourceAssetIds.map((id) => (
              <span
                key={id}
                className="text-[10px] px-[var(--space-2)] py-px rounded-full"
                style={{
                  background: "var(--surface-tertiary)",
                  color: "var(--text-tertiary)",
                }}
              >
                {id}
              </span>
            ))}
          </div>
        )}
      </div>
    );
  }

  return null;
}

function StreamingSkeleton() {
  return (
    <div className="space-y-[var(--space-2)] animate-pulse">
      {[90, 70, 85, 60].map((w, i) => (
        <div
          key={i}
          className="h-3 rounded"
          style={{ width: `${w}%`, background: "var(--surface-tertiary)" }}
        />
      ))}
    </div>
  );
}
