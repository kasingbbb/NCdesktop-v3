/**
 * CourseSection — 侧边栏课程分区
 *
 * 按 Today / Tomorrow / This Week 三组展示课程，
 * 点击课程事件触发进入预习空间。
 *
 * - 无课程时不渲染（空状态隐藏）
 * - 约束（宪章 A1/A2/A4）：named export，CSS 变量，不 import 其他 Store
 */

import { useEffect, useState } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";
import { CourseEventItem } from "./CourseEventItem";
import { useCalendarStore } from "../../../stores/calendarStore";
import { useUIStore } from "../../../stores/uiStore";
import { useLibraryStore } from "../../../stores/libraryStore";
import type { CourseEvent } from "../../../types/calendar";

// ─────────────────────────────────────────────────────────────────────────────

export function CourseSection() {
  const libraryId = useLibraryStore((s) => s.activeLibraryId);
  const {
    selectedEventId,
    selectEvent,
    getTodayEvents,
    getTomorrowEvents,
    getThisWeekEvents,
    fetchEvents,
  } = useCalendarStore();
  const {
    activeSidebarSection,
    setRightPanelMode,
    setActiveCourseEventId,
    setCoursePreviewReturnTo,
  } = useUIStore();

  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});

  // 首次挂载时加载本周课程
  useEffect(() => {
    if (!libraryId) return;
    const now = new Date();
    const weekEnd = new Date(now);
    weekEnd.setDate(now.getDate() + 7);
    fetchEvents(libraryId, now.toISOString(), weekEnd.toISOString());
  }, [libraryId]);

  const todayEvents = getTodayEvents();
  const tomorrowEvents = getTomorrowEvents();
  const weekEvents = getThisWeekEvents();

  const allEmpty =
    todayEvents.length === 0 &&
    tomorrowEvents.length === 0 &&
    weekEvents.length === 0;

  const handleSelect = (ev: CourseEvent) => {
    selectEvent(ev.id);
    setCoursePreviewReturnTo({ section: activeSidebarSection });
    setActiveCourseEventId(ev.id);
    setRightPanelMode("course_preview");
  };

  const toggleGroup = (key: string) =>
    setCollapsed((s) => ({ ...s, [key]: !s[key] }));

  // PRD §9.1：TODAY 整组在无课时不渲染（由父调用方决定是否渲染整段课程区）
  if (allEmpty) return null;

  return (
    <div className="mb-[var(--space-1)]">
      <SectionGroup
        label="Today"
        events={todayEvents}
        selectedId={selectedEventId}
        collapsed={!!collapsed["today"]}
        onToggle={() => toggleGroup("today")}
        onSelect={handleSelect}
      />
      <SectionGroup
        label="Tomorrow"
        events={tomorrowEvents}
        selectedId={selectedEventId}
        collapsed={!!collapsed["tomorrow"]}
        onToggle={() => toggleGroup("tomorrow")}
        onSelect={handleSelect}
      />
      <SectionGroup
        label="This Week"
        events={weekEvents}
        selectedId={selectedEventId}
        collapsed={!!collapsed["week"]}
        onToggle={() => toggleGroup("week")}
        onSelect={handleSelect}
      />
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// 子组件：可折叠的分组
// ─────────────────────────────────────────────────────────────────────────────

function SectionGroup({
  label,
  events,
  selectedId,
  collapsed,
  onToggle,
  onSelect,
}: {
  label: string;
  events: CourseEvent[];
  selectedId: string | null;
  collapsed: boolean;
  onToggle: () => void;
  onSelect: (ev: CourseEvent) => void;
}) {
  if (events.length === 0) return null;

  return (
    <div className="mb-[var(--space-1)]">
      {/* 分组标题 */}
      <button
        type="button"
        onClick={onToggle}
        className="w-full flex items-center gap-[var(--space-1)] px-[var(--space-3)] py-[var(--space-1)] transition-colors"
      >
        {collapsed ? (
          <ChevronRight size={11} style={{ color: "var(--sidebar-text-dim)" }} />
        ) : (
          <ChevronDown size={11} style={{ color: "var(--sidebar-text-dim)" }} />
        )}
        <span
          className="text-[10px] uppercase tracking-[0.08em] font-semibold"
          style={{ color: "var(--sidebar-text-dim)" }}
        >
          {label}
        </span>
        <span
          className="ml-auto text-[10px] tabular-nums"
          style={{ color: "var(--sidebar-text-dim)" }}
        >
          {events.length}
        </span>
      </button>

      {/* 事件列表 */}
      {!collapsed && (
        <div className="space-y-[1px]">
          {events.map((ev) => (
            <CourseEventItem
              key={ev.id}
              event={ev}
              isActive={selectedId === ev.id}
              onClick={() => onSelect(ev)}
            />
          ))}
        </div>
      )}
    </div>
  );
}
