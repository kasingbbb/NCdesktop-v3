/**
 * MirrorFeedbackDisplay — AI 镜子反馈展示
 *
 * 当 mirrorStatus = 'done' 时渲染结构化反馈。
 * 流式阶段显示 mirrorStreamBuffer 的原始内容。
 */

import { Loader2, CheckCircle2 } from "lucide-react";
import { useKnowledgeUnderstandingStore } from "../../stores/knowledgeUnderstandingStore";
import { SourceEvidence } from "./SourceEvidence";

export function MirrorFeedbackDisplay() {
  const mirrorFeedback = useKnowledgeUnderstandingStore((s) => s.mirrorFeedback);
  const status = useKnowledgeUnderstandingStore((s) => s.mirrorStatus);
  const buffer = useKnowledgeUnderstandingStore((s) => s.mirrorStreamBuffer);

  if (status === "idle") return null;

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
        反馈解析失败，请重试
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
        {buffer && <p className="whitespace-pre-wrap">{buffer}</p>}
        <div className="flex items-center gap-1.5">
          <Loader2 size={11} className="animate-spin" style={{ color: "var(--text-tertiary)" }} />
          <span className="text-[var(--text-xs)]" style={{ color: "var(--text-tertiary)" }}>
            核对中...
          </span>
        </div>
      </div>
    );
  }

  if (!mirrorFeedback) return null;

  return (
    <div
      className="px-[var(--space-4)] py-[var(--space-4)] rounded-[var(--radius-md)] space-y-[var(--space-3)]"
      style={{
        background: "rgba(31, 69, 110, 0.04)",
        border: "1px solid rgba(31, 69, 110, 0.15)",
      }}
    >
      {/* 核心要点数 */}
      <div className="flex items-center gap-[var(--space-2)]">
        <CheckCircle2 size={16} style={{ color: "var(--brand-navy)" }} />
        <span
          className="text-[var(--text-sm)] font-medium"
          style={{ color: "var(--text-primary)" }}
        >
          你的解释捕捉到了 {mirrorFeedback.coveredCount} 个核心要点 ✓
        </span>
      </div>

      {/* 捕捉到的要点 */}
      {mirrorFeedback.coveredPoints.length > 0 && (
        <ul className="space-y-1 pl-[var(--space-5)]">
          {mirrorFeedback.coveredPoints.map((point, i) => (
            <li
              key={i}
              className="text-[var(--text-xs)] leading-relaxed list-disc"
              style={{ color: "var(--text-secondary)" }}
            >
              {point}
            </li>
          ))}
        </ul>
      )}

      {/* 附加视角 */}
      {mirrorFeedback.additionalPerspectives.length > 0 && (
        <div className="space-y-[var(--space-2)]">
          <p
            className="text-[var(--text-sm)]"
            style={{ color: "var(--text-secondary)" }}
          >
            在你的文档里，还有一些关于这个概念的角度你可能感兴趣：
          </p>
          <div className="space-y-[var(--space-2)] pl-[var(--space-3)]">
            {mirrorFeedback.additionalPerspectives.map((perspective, i) => (
              <div key={i} className="space-y-[var(--space-1)]">
                <p
                  className="text-[var(--text-sm)] leading-relaxed"
                  style={{ color: "var(--text-secondary)" }}
                >
                  {perspective.text}
                </p>
                <SourceEvidence source={perspective.source} />
              </div>
            ))}
          </div>
        </div>
      )}

      {/* 差异说明 */}
      {mirrorFeedback.differenceNote && (
        <p
          className="text-[var(--text-sm)] leading-relaxed"
          style={{ color: "var(--text-secondary)" }}
        >
          你的理解和文档的一个细微差异是：{mirrorFeedback.differenceNote}
        </p>
      )}
    </div>
  );
}
