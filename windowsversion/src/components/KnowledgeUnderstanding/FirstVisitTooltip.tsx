/**
 * FirstVisitTooltip — 一次性引导 Tooltip
 *
 * 首次进入知识关联视图时显示，指向「深入理解」按钮。
 * 使用 localStorage 记录已展示状态，确保后续不重复显示。
 */

import { useState, useEffect } from "react";
import { X } from "lucide-react";

const STORAGE_KEY = "nc_knowledge_tooltip_shown";

export function FirstVisitTooltip() {
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    const shown = localStorage.getItem(STORAGE_KEY);
    if (!shown) {
      setVisible(true);
    }
  }, []);

  const dismiss = () => {
    setVisible(false);
    localStorage.setItem(STORAGE_KEY, "1");
  };

  if (!visible) return null;

  return (
    <div
      className="absolute top-full right-0 mt-2 z-50 px-3 py-2.5 rounded-[var(--radius-md)] text-[var(--text-xs)] shadow-lg"
      style={{
        background: "var(--brand-navy)",
        color: "#fff",
        width: 240,
      }}
    >
      <button
        type="button"
        onClick={dismiss}
        className="absolute top-1.5 right-1.5 p-0.5 rounded-sm opacity-60 hover:opacity-100 transition-opacity"
      >
        <X size={10} />
      </button>
      <p className="pr-4 leading-relaxed">
        点击「深入理解」，让 AI 基于你的文档帮你真正理解这个概念
      </p>
      {/* 指向上方按钮的小三角 */}
      <div
        className="absolute -top-1.5 right-4 w-3 h-3 rotate-45"
        style={{ background: "var(--brand-navy)" }}
      />
    </div>
  );
}
