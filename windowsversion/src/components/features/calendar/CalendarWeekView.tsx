/**
 * CalendarWeekView — 课程日历周视图容器
 *
 * 组合 WeekHeader + TimeGrid，管理周导航和事件加载。
 * 点击课程卡片时记录 returnTo 并切换到预习空间。
 */

import { useEffect } from "react";
import { WeekHeader } from "./WeekHeader";
import { TimeGrid } from "./TimeGrid";
import { useCalendarStore, getMondayKey } from "../../../stores/calendarStore";
import { useUIStore } from "../../../stores/uiStore";
import { useLibraryStore } from "../../../stores/libraryStore";
import type { CourseEvent } from "../../../types/calendar";

function pad2(n: number): string {
  return String(n).padStart(2, "0");
}

function dateKey(d: Date): string {
  return `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}`;
}

export function CalendarWeekView() {
  const libraryId = useLibraryStore((s) => s.activeLibraryId);
  const {
    events,
    activeWeekStart,
    fetchWeekEvents,
    navigateWeek,
    setActiveWeekStart,
    selectEvent,
  } = useCalendarStore();
  const {
    setActiveCourseEventId,
    setRightPanelMode,
    setCoursePreviewReturnTo,
  } = useUIStore();

  const mondayKey = activeWeekStart ?? getMondayKey(new Date());
  const weekStartDate = new Date(mondayKey);

  useEffect(() => {
    if (!libraryId) return;
    if (!activeWeekStart) {
      setActiveWeekStart(getMondayKey(new Date()));
    }
    void fetchWeekEvents(libraryId, mondayKey);
  }, [libraryId, mondayKey]);

  const handlePrevWeek = (): void => {
    if (!libraryId) return;
    navigateWeek(libraryId, -1);
  };

  const handleNextWeek = (): void => {
    if (!libraryId) return;
    navigateWeek(libraryId, 1);
  };

  const handleToday = (): void => {
    if (!libraryId) return;
    const todayMonday = getMondayKey(new Date());
    void fetchWeekEvents(libraryId, todayMonday);
  };

  const handleEventClick = (event: CourseEvent): void => {
    selectEvent(event.id);
    setCoursePreviewReturnTo({ section: "calendar", weekStart: mondayKey });
    setActiveCourseEventId(event.id);
    setRightPanelMode("course_preview");
  };

  const todayStr = dateKey(new Date());

  return (
    <div className="flex flex-col h-full overflow-hidden bg-[var(--surface-primary)]">
      <WeekHeader
        weekStart={weekStartDate}
        onPrevWeek={handlePrevWeek}
        onNextWeek={handleNextWeek}
        onToday={handleToday}
      />
      <TimeGrid
        weekStart={weekStartDate}
        events={events}
        todayKey={todayStr}
        onEventClick={handleEventClick}
      />
    </div>
  );
}
