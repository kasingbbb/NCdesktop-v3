import { Sparkles, FileText } from "lucide-react";
import type { Asset } from "../../types";

interface InspectorAIProps {
  asset: Asset;
}

export function InspectorAI({ asset }: InspectorAIProps) {
  const analysis = asset.aiAnalysis;

  if (!analysis) return null;

  return (
    <div className="mb-[var(--space-4)]">
      <h3 className="text-[var(--text-sm)] uppercase tracking-[0.08em] mb-[var(--space-2)] flex items-center gap-1" style={{ color: "var(--text-tertiary)" }}>
        <Sparkles size={14} className="text-gray-500" />
        AI Analysis
      </h3>
      
      {analysis.summary && (
        <div
          className="rounded-[var(--radius-md)] p-[var(--space-3)] mb-[var(--space-2)] border"
          style={{ background: "var(--ai-surface)", borderColor: "var(--ai-border)" }}
        >
          <p className="text-[var(--text-sm)] leading-relaxed" style={{ color: "var(--text-primary)" }}>
            {analysis.summary}
          </p>
        </div>
      )}

      {asset.type === 'scan_text' && analysis.ocrText && (
        <div className="rounded-[var(--radius-md)] p-[var(--space-3)]" style={{ background: "var(--surface-secondary)" }}>
          <h4 className="text-[var(--text-xs)] uppercase tracking-[0.05em] mb-1 flex items-center gap-1" style={{ color: "var(--text-tertiary)" }}>
            <FileText size={12} /> OCR Extracted
          </h4>
          <p className="text-[var(--text-xs)] line-clamp-3 overflow-hidden text-ellipsis" style={{ color: "var(--text-secondary)" }}>
            {analysis.ocrText}
          </p>
        </div>
      )}
    </div>
  );
}
