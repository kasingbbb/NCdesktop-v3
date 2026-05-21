/**
 * DeepUnderstandButton — 「深入理解」蓝色高亮入口按钮
 *
 * 位置：ConceptDetailPanel 定义区右上角
 * 约束：只使用 knowledgeUnderstandingStore，概念数据通过 props 传入
 */

import { Sparkles } from "lucide-react";

interface DeepUnderstandButtonProps {
  conceptId: string;
  onEnterUnderstanding: (conceptId: string) => void;
}

export function DeepUnderstandButton({
  conceptId,
  onEnterUnderstanding,
}: DeepUnderstandButtonProps) {
  return (
    <button
      type="button"
      onClick={() => onEnterUnderstanding(conceptId)}
      className="flex items-center gap-1.5 px-[var(--space-3)] py-[var(--space-1)] rounded-[var(--radius-md)] text-[var(--text-xs)] font-medium transition-all"
      style={{
        background: "var(--brand-navy)",
        color: "#fff",
      }}
    >
      <Sparkles size={12} />
      深入理解
    </button>
  );
}
