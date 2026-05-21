/**
 * CourseEventItem — 侧边栏中单条课程事件展示
 *
 * 约束（宪章 A1/A2）：named export，CSS 变量
 */

import type { CourseEvent } from "../../../types/calendar";

interface Props {
  event: CourseEvent;
  isActive: boolean;
  onClick: () => void;
}

export function CourseEventItem({ event, isActive, onClick }: Props) {
  const start = new Date(event.startTime);
  const timeStr = start.toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });

  return (
    <button
      type="button"
      onClick={onClick}
      className="w-full flex items-start gap-[var(--space-2)] px-[var(--space-3)] py-[var(--space-1)] rounded-[var(--radius-sm)] text-left transition-colors"
      style={{
        background: isActive ? "var(--sidebar-active-bg, var(--surface-tertiary))" : "transparent",
      }}
    >
      {/* 时间戳 */}
      <span
        className="text-[var(--text-xs)] tabular-nums flex-shrink-0 mt-0.5 w-10"
        style={{ color: "var(--sidebar-text-dim)" }}
      >
        {timeStr}
      </span>

      {/* 课程名 */}
      <span
        className="text-[var(--text-xs)] font-medium truncate leading-5"
        style={{
          color: isActive ? "var(--sidebar-active-fg)" : "var(--sidebar-text-muted)",
        }}
      >
        {event.courseCode ?? event.title}
      </span>
    </button>
  );
}
