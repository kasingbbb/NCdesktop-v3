/**
 * TransparencyBanner — AI 生成内容透明度声明横幅
 */

import { AlertTriangle } from "lucide-react";

export function TransparencyBanner() {
  return (
    <div
      className="flex items-start gap-[var(--space-2)] px-[var(--space-4)] py-[var(--space-3)] text-[var(--text-xs)] leading-relaxed flex-shrink-0"
      style={{
        background: "rgba(255, 192, 0, 0.08)",
        border: "1px solid rgba(255, 192, 0, 0.25)",
        color: "var(--text-secondary)",
      }}
    >
      <AlertTriangle
        size={14}
        className="flex-shrink-0 mt-0.5"
        style={{ color: "var(--brand-gold, #FFC000)" }}
      />
      <span>
        以下解释基于你的文档由 AI 生成，AI 可能有理解偏差——点击来源链接查看原文对照
      </span>
    </div>
  );
}
