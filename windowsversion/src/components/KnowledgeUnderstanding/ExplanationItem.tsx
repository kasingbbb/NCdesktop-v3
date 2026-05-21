/**
 * ExplanationItem — 单条解释条目（含来源链接 + 查看依据）
 */

import { SourceEvidence } from "./SourceEvidence";
import type { ExplanationItem as ExplanationItemType } from "../../types/knowledge-understanding.types";

interface ExplanationItemProps {
  item: ExplanationItemType;
  excerpt?: string;
}

export function ExplanationItemCard({ item, excerpt }: ExplanationItemProps) {
  return (
    <div
      className="px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-sm)] leading-relaxed"
      style={{
        background: "var(--surface-secondary)",
        border: "1px solid var(--border-primary)",
        color: "var(--text-secondary)",
      }}
    >
      <p>{item.text}</p>
      <SourceEvidence source={item.source} excerpt={excerpt} />
    </div>
  );
}
