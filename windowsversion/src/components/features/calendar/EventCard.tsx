/**
 * EventCard — 周视图中的课程卡片
 *
 * 通过 absolute positioning 在 TimeGrid 的日列容器中定位，
 * top/height 由 startTime/endTime 相对于当天 08:00 计算。
 */

import type { CourseEvent } from "../../../types/calendar";

interface EventCardProps {
  event: CourseEvent;
  onClick: (event: CourseEvent) => void;
}

const HOUR_START = 8;
const HOUR_HEIGHT_PX = 60;

export function EventCard({ event, onClick }: EventCardProps) {
  const start = new Date(event.startTime);
  const end = new Date(event.endTime);

  const startMinutes = start.getHours() * 60 + start.getMinutes() - HOUR_START * 60;
  const endMinutes = end.getHours() * 60 + end.getMinutes() - HOUR_START * 60;
  const durationMinutes = endMinutes - startMinutes;

  const top = (startMinutes / 60) * HOUR_HEIGHT_PX;
  const height = Math.max((durationMinutes / 60) * HOUR_HEIGHT_PX, 20);

  const startTimeStr = start.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  const endTimeStr = end.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });

  const displayName = event.courseCode || event.title;
  const isCompact = height < 50;

  return (
    <button
      type="button"
      onClick={() => onClick(event)}
      className="absolute left-[2px] right-[2px] rounded-[var(--radius-sm)] px-[var(--space-1)] overflow-hidden text-left transition-all cursor-pointer group"
      style={{
        top: `${top}px`,
        height: `${height}px`,
        background: "var(--brand-navy)",
        color: "#fff",
        opacity: 0.9,
        fontSize: "var(--text-xs)",
        lineHeight: 1.3,
      }}
      title={`${event.title}\n${startTimeStr}–${endTimeStr}${event.instructor ? `\n${event.instructor}` : ""}${event.location ? `\n${event.location}` : ""}`}
    >
      <div
        className="absolute inset-0 rounded-[var(--radius-sm)] opacity-0 group-hover:opacity-100 transition-opacity"
        style={{ background: "rgba(255,255,255,0.12)" }}
      />
      <span className="relative z-10 font-medium truncate block pt-px">
        {displayName}
      </span>
      {!isCompact && event.location && (
        <span className="relative z-10 truncate block opacity-70" style={{ fontSize: 10 }}>
          {event.location}
        </span>
      )}
    </button>
  );
}

export { HOUR_START, HOUR_HEIGHT_PX };
