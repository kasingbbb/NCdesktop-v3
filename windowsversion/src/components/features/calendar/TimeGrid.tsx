/**
 * TimeGrid — 时间网格
 *
 * 渲染 7 列（周一 ~ 周日）× N 行（HOUR_START ~ HOUR_END）的时间网格，
 * 在每列中按 absolute positioning 放置 EventCard。
 */

import { useMemo } from "react";
import { EventCard, HOUR_START, HOUR_HEIGHT_PX } from "./EventCard";
import type { CourseEvent } from "../../../types/calendar";

const HOUR_END = 22;
const TOTAL_HOURS = HOUR_END - HOUR_START;
const DAY_LABELS = ["一", "二", "三", "四", "五", "六", "日"];

interface TimeGridProps {
  weekStart: Date;
  events: CourseEvent[];
  todayKey: string;
  onEventClick: (event: CourseEvent) => void;
}

function pad2(n: number): string {
  return String(n).padStart(2, "0");
}

function dateKey(d: Date): string {
  return `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}`;
}

export function TimeGrid({ weekStart, events, todayKey: todayStr, onEventClick }: TimeGridProps) {
  const days = useMemo(() => {
    const result: Array<{ date: Date; key: string; label: string; dayNum: number }> = [];
    for (let i = 0; i < 7; i++) {
      const d = new Date(weekStart);
      d.setDate(d.getDate() + i);
      result.push({
        date: d,
        key: dateKey(d),
        label: DAY_LABELS[i],
        dayNum: d.getDate(),
      });
    }
    return result;
  }, [weekStart]);

  const eventsByDay = useMemo(() => {
    const map = new Map<string, CourseEvent[]>();
    for (const ev of events) {
      const d = new Date(ev.startTime);
      const key = dateKey(d);
      const bucket = map.get(key) ?? [];
      bucket.push(ev);
      map.set(key, bucket);
    }
    return map;
  }, [events]);

  const hours = useMemo(() => {
    const arr: number[] = [];
    for (let h = HOUR_START; h < HOUR_END; h++) arr.push(h);
    return arr;
  }, []);

  return (
    <div className="flex-1 overflow-auto min-h-0">
      <div className="flex min-w-[700px]">
        {/* 左侧时间列 */}
        <div className="flex-shrink-0" style={{ width: 56 }}>
          {/* 顶部对齐星期行 */}
          <div
            className="h-10 border-b flex-shrink-0"
            style={{ borderColor: "var(--border-primary)" }}
          />
          {/* 小时标记 */}
          <div className="relative" style={{ height: `${TOTAL_HOURS * HOUR_HEIGHT_PX}px` }}>
            {hours.map((h) => (
              <div
                key={h}
                className="absolute right-[var(--space-2)] text-[10px]"
                style={{
                  top: `${(h - HOUR_START) * HOUR_HEIGHT_PX}px`,
                  color: "var(--text-tertiary)",
                  transform: "translateY(-6px)",
                }}
              >
                {`${pad2(h)}:00`}
              </div>
            ))}
          </div>
        </div>

        {/* 7 天列 */}
        {days.map((day) => {
          const isToday = day.key === todayStr;
          const dayEvents = eventsByDay.get(day.key) ?? [];

          return (
            <div
              key={day.key}
              className="flex-1 min-w-0 border-l"
              style={{
                borderColor: "var(--border-primary)",
                background: isToday ? "var(--surface-secondary)" : undefined,
              }}
            >
              {/* 星期行 */}
              <div
                className="h-10 flex flex-col items-center justify-center border-b flex-shrink-0"
                style={{ borderColor: "var(--border-primary)" }}
              >
                <span
                  className="text-[10px] uppercase"
                  style={{ color: "var(--text-tertiary)" }}
                >
                  {day.label}
                </span>
                <span
                  className={`text-[var(--text-sm)] font-medium leading-none ${isToday ? "rounded-full px-1.5 py-0.5" : ""}`}
                  style={{
                    color: isToday ? "#fff" : "var(--text-primary)",
                    background: isToday ? "var(--brand-navy)" : undefined,
                  }}
                >
                  {day.dayNum}
                </span>
              </div>

              {/* 时间格子 + 事件卡片 */}
              <div className="relative" style={{ height: `${TOTAL_HOURS * HOUR_HEIGHT_PX}px` }}>
                {/* 网格线 */}
                {hours.map((h) => (
                  <div
                    key={h}
                    className="absolute w-full border-b"
                    style={{
                      top: `${(h - HOUR_START) * HOUR_HEIGHT_PX}px`,
                      height: `${HOUR_HEIGHT_PX}px`,
                      borderColor: "var(--border-primary)",
                    }}
                  />
                ))}
                {/* 课程卡片 */}
                {dayEvents.map((ev) => (
                  <EventCard key={ev.id} event={ev} onClick={onEventClick} />
                ))}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
