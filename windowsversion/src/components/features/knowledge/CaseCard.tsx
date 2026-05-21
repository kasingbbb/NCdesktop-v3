/**
 * CaseCard — 概念案例卡片
 *
 * 样式：左边框 text-tertiary + 斜体引文 + 来源 + 查看原文按钮
 * 约束（宪章 A1/A2）：named export，CSS 变量
 */

import { ExternalLink } from "lucide-react";
import type { ConceptCase } from "../../../types/knowledge";

interface Props {
  conceptCase: ConceptCase;
}

export function CaseCard({ conceptCase }: Props) {
  return (
    <div
      className="pl-[var(--space-3)] py-[var(--space-2)] pr-[var(--space-3)] rounded-r-[var(--radius-md)]"
      style={{
        borderLeft: "3px solid var(--text-tertiary)",
        background: "var(--surface-secondary)",
      }}
    >
      {/* 引文（斜体） */}
      <p
        className="text-[var(--text-sm)] leading-relaxed italic mb-[var(--space-2)]"
        style={{ color: "var(--text-primary)" }}
      >
        "{conceptCase.excerpt}"
      </p>

      {/* 底部：来源 + 查看原文 */}
      <div className="flex items-center justify-between">
        <span
          className="text-[var(--text-xs)]"
          style={{ color: "var(--text-tertiary)" }}
        >
          — {conceptCase.title}
        </span>

        {conceptCase.sourceAssetId && (
          <button
            type="button"
            className="flex items-center gap-1 text-[10px] transition-colors"
            style={{ color: "var(--brand-navy)" }}
            title="查看原始素材"
            onClick={() => {
              // 暂时仅提示；完整跳转到 DocumentViewer 将在后续接入
              console.info("查看原文:", conceptCase.sourceAssetId);
            }}
          >
            <ExternalLink size={11} />
            查看原文
          </button>
        )}
      </div>

      {/* 相关性说明（可选） */}
      {conceptCase.relevanceNote && (
        <p
          className="text-[10px] mt-[var(--space-1)]"
          style={{ color: "var(--text-tertiary)" }}
        >
          {conceptCase.relevanceNote}
        </p>
      )}
    </div>
  );
}
