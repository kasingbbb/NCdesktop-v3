/**
 * WeekHeader — 周视图导航栏
 *
 * 左右箭头切换周 + 日期范围显示 + 「今天」快捷按钮
 */

import { ChevronLeft, ChevronRight } from "lucide-react";

interface WeekHeaderProps {
  weekStart: Date;
  onPrevWeek: () => void;
  onNextWeek: () => void;
  onToday: () => void;
}

export function WeekHeader({ weekStart, onPrevWeek, onNextWeek, onToday }: WeekHeaderProps) {
  const weekEnd = new Date(weekStart);
  weekEnd.setDate(weekEnd.getDate() + 6);

  const fmt = (d: Date): string =>
    d.toLocaleDateString("zh-CN", { month: "long", day: "numeric" });

  return (
    <div
      className="flex items-center justify-between px-[var(--space-4)] py-[var(--space-3)] border-b flex-shrink-0"
      style={{ borderColor: "var(--border-primary)" }}
    >
      <div className="flex items-center gap-[var(--space-2)]">
        <button
          type="button"
          onClick={onPrevWeek}
          className="p-1 rounded-[var(--radius-sm)] transition-colors hover:bg-[var(--surface-secondary)]"
          style={{ color: "var(--text-secondary)" }}
        >
          <ChevronLeft size={16} />
        </button>
        <button
          type="button"
          onClick={onNextWeek}
          className="p-1 rounded-[var(--radius-sm)] transition-colors hover:bg-[var(--surface-secondary)]"
          style={{ color: "var(--text-secondary)" }}
        >
          <ChevronRight size={16} />
        </button>
      </div>

      <span
        className="text-[var(--text-sm)] font-medium"
        style={{ color: "var(--text-primary)" }}
      >
        {fmt(weekStart)} — {fmt(weekEnd)}
      </span>

      <button
        type="button"
        onClick={onToday}
        className="text-[var(--text-xs)] px-[var(--space-2)] py-1 rounded-[var(--radius-sm)] transition-colors"
        style={{
          color: "var(--brand-navy)",
          background: "var(--surface-secondary)",
          border: "1px solid var(--border-primary)",
        }}
      >
        今天
      </button>
    </div>
  );
}
