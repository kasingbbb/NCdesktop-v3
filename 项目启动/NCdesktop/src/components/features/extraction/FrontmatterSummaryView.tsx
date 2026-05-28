/**
 * task_017_frontmatter_renderer_dep — KC AI 摘要展示组件
 *
 * 接收 frontmatter.aiSummary + isAi 标志，渲染：
 *   图标 + 灰底文本块 + "AI 摘要" 标签（或 "（无摘要）"）
 *
 * 设计要点：
 * - 纯展示组件，无副作用。
 * - isAi=true 时显示 Sparkles 图标 + "AI 摘要"；isAi=false 时显示 FileText 图标 + "摘要"。
 * - summary 为空字符串/undefined → 渲染 "（无摘要）"。
 *
 * 不接入 Inspector/DocumentViewer（task_018/019 负责接入）。
 */
import { FileText, Sparkles } from "lucide-react";

interface FrontmatterSummaryViewProps {
  summary: string | undefined;
  isAi: boolean;
}

export function FrontmatterSummaryView({ summary, isAi }: FrontmatterSummaryViewProps) {
  const hasContent = typeof summary === "string" && summary.trim().length > 0;
  const Icon = isAi ? Sparkles : FileText;
  const label = isAi ? "AI 摘要" : "摘要";

  return (
    // task_018 AC-5 (TD-2)：根元素 aria-label="AI 摘要" / "摘要"，区分 AI 摘要 vs 普通摘要。
    // isAi=true 时额外给一个 visible "AI" badge（与 task_017 reviewer TD-2 要求一致）。
    <div
      className="rounded-[var(--radius-md)] p-[var(--space-3)] bg-[var(--surface-tertiary)]"
      style={{ color: "var(--text-secondary)" }}
      data-testid="frontmatter-summary"
      aria-label={label}
      role="region"
    >
      <div
        className="flex items-center gap-[var(--space-1)] mb-[var(--space-1)] text-[var(--text-xs)]"
        style={{ color: "var(--text-tertiary)" }}
      >
        <Icon size={12} data-testid={isAi ? "icon-ai" : "icon-summary"} />
        <span>{label}</span>
        {isAi && (
          <span
            className="ml-[var(--space-1)] px-1.5 py-0.5 rounded-[var(--radius-sm)] text-[10px] uppercase tracking-wide"
            style={{
              background: "var(--surface-secondary)",
              color: "var(--text-tertiary)",
            }}
            data-testid="ai-badge"
            aria-hidden="true"
          >
            AI
          </span>
        )}
      </div>

      {hasContent ? (
        <div
          className="text-[var(--text-sm)] leading-relaxed whitespace-pre-wrap"
          style={{ color: "var(--text-primary)" }}
          data-testid="frontmatter-summary-text"
        >
          {summary}
        </div>
      ) : (
        <div
          className="text-[var(--text-sm)] italic"
          style={{ color: "var(--text-tertiary)" }}
          data-testid="frontmatter-summary-empty"
        >
          （无摘要）
        </div>
      )}
    </div>
  );
}
