/**
 * SourceEvidence — 「查看依据」展开原文段落
 *
 * 点击后折叠/展开显示对应原文引用。
 * 数据通过 props 传入（来自 concept_cases.excerpt 或 viewpoints.summary）。
 */

import { useState } from "react";
import { ChevronDown, ChevronRight, FileText } from "lucide-react";

interface SourceEvidenceProps {
  source: string;
  excerpt?: string;
}

export function SourceEvidence({ source, excerpt }: SourceEvidenceProps) {
  const [expanded, setExpanded] = useState(false);

  if (!excerpt) {
    return (
      <span
        className="inline-flex items-center gap-1 text-[var(--text-xs)]"
        style={{ color: "var(--text-tertiary)" }}
      >
        <FileText size={10} />
        {source}
      </span>
    );
  }

  return (
    <div className="mt-[var(--space-1)]">
      <button
        type="button"
        onClick={() => setExpanded(!expanded)}
        className="inline-flex items-center gap-1 text-[var(--text-xs)] transition-colors"
        style={{ color: "var(--brand-navy)" }}
      >
        {expanded ? <ChevronDown size={10} /> : <ChevronRight size={10} />}
        <FileText size={10} />
        {source}
      </button>
      {expanded && (
        <div
          className="mt-[var(--space-1)] ml-[var(--space-4)] px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-sm)] text-[var(--text-xs)] leading-relaxed"
          style={{
            background: "var(--surface-tertiary)",
            color: "var(--text-secondary)",
            borderLeft: "2px solid var(--brand-navy)",
          }}
        >
          {excerpt}
        </div>
      )}
    </div>
  );
}
