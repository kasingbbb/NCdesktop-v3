/**
 * ViewpointCard — 概念观点卡片
 *
 * 样式：左边框 brand-navy + 观点标题 + 摘要 + 来源标签
 * 约束（宪章 A1/A2）：named export，CSS 变量
 */

import type { ConceptViewpoint } from "../../../types/knowledge";

interface Props {
  viewpoint: ConceptViewpoint;
}

export function ViewpointCard({ viewpoint }: Props) {
  return (
    <div
      className="pl-[var(--space-3)] py-[var(--space-2)] pr-[var(--space-3)] rounded-r-[var(--radius-md)]"
      style={{
        borderLeft: "3px solid var(--brand-navy)",
        background: "var(--surface-secondary)",
      }}
    >
      {/* 观点标题 */}
      <div className="flex items-center justify-between mb-[var(--space-1)]">
        <span
          className="text-[var(--text-xs)] font-semibold"
          style={{ color: "var(--brand-navy)" }}
        >
          🔹 {viewpoint.perspective}
        </span>
        {viewpoint.sourceContext && (
          <span
            className="text-[10px] px-[var(--space-2)] py-px rounded-full flex-shrink-0 ml-[var(--space-2)]"
            style={{
              background: "var(--surface-tertiary)",
              color: "var(--text-tertiary)",
            }}
          >
            {viewpoint.sourceContext}
          </span>
        )}
      </div>

      {/* 摘要 */}
      <p
        className="text-[var(--text-sm)] leading-relaxed"
        style={{ color: "var(--text-secondary)" }}
      >
        {viewpoint.summary}
      </p>
    </div>
  );
}
