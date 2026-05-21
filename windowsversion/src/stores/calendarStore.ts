/**
 * calendarStore — 课程日历前端状态管理
 *
 * 职责：
 *   - 持有已持久化的 CourseEvent[]
 *   - 管理待确认导入的 ParsedEvent[]（预览阶段）
 *   - 提供按日分组、今日课程等派生数据
 *
 * 约束（宪章 A4）：不 import 其他 Store，跨 Store 数据在组件层组合
 */

import { create } from "zustand";
import type { CourseEvent, ImportIcsResult } from "../types/calendar";
import * as cmd from "../lib/tauri-commands";

// ─── 状态结构 ─────────────────────────────────────────────────────────────────

interface CalendarStore {
  /** 已持久化的课程事件 */
  events: CourseEvent[];
  /** 当前选中的课程事件 ID */
  selectedEventId: string | null;
  /** 导入预览：待用户确认的 ParsedEvent 列表 */
  pendingImportResult: ImportIcsResult | null;
  /** 已勾选的 temp_id 集合（空 = 全选） */
  pendingSelectedIds: Set<string>;
  /** 上一次导入时的 source 信息，供确认时使用 */
  pendingSource: { type: "ics_file" | "ics_url"; url?: string } | null;
  isLoading: boolean;
  error: string | null;

  /** v2.2: 周视图当前显示的周一日期（YYYY-MM-DD） */
  activeWeekStart: string | null;

  // ── 动作 ──────────────────────────────────────────────────────────────────

  /** 从数据库加载课程事件 */
  fetchEvents: (libraryId: string, startAfter?: string, endBefore?: string) => Promise<void>;
  /** 选中某个课程事件 */
  selectEvent: (id: string | null) => void;
  /** v2.2: 设置周视图当前周 */
  setActiveWeekStart: (date: string) => void;
  /** v2.2: 加载指定周的课程事件 */
  fetchWeekEvents: (libraryId: string, weekStart: string) => Promise<void>;
  /** v2.2: 向前/向后切换 N 周 */
  navigateWeek: (libraryId: string, offset: number) => void;

  /** 解析本地 .ics 文件，进入预览模式 */
  importFromFile: (libraryId: string, filePath: string) => Promise<void>;
  /** 从 iCal URL 导入，进入预览模式 */
  importFromUrl: (libraryId: string, url: string) => Promise<void>;

  /** 切换预览列表中某条事件的勾选状态 */
  togglePendingSelect: (tempId: string) => void;
  /** 全选 / 全不选预览列表 */
  selectAllPending: (allSelected: boolean) => void;

  /** 确认写入数据库，并刷新事件列表 */
  confirmImport: (libraryId: string) => Promise<number>;
  /** 取消预览，清空待导入状态 */
  cancelImport: () => void;

  /** 删除某个日历来源 */
  deleteSource: (
    libraryId: string,
    calendarSource: "ics_file" | "ics_url",
    sourceUrl?: string
  ) => Promise<void>;

  /** 刷新订阅日历 */
  refreshSubscription: (libraryId: string, sourceUrl: string) => Promise<void>;

  // ── 派生数据（getter，避免存储冗余） ────────────────────────────────────────

  /** 将 events 按日期字符串（YYYY-MM-DD）分组 */
  getEventsGroupedByDay: () => Map<string, CourseEvent[]>;
  /** 今日课程 */
  getTodayEvents: () => CourseEvent[];
  /** 明日课程 */
  getTomorrowEvents: () => CourseEvent[];
  /** 本周剩余课程（不含今明两天） */
  getThisWeekEvents: () => CourseEvent[];
}

// ─── 实现 ─────────────────────────────────────────────────────────────────────

export const useCalendarStore = create<CalendarStore>((set, get) => ({
  events: [],
  selectedEventId: null,
  pendingImportResult: null,
  pendingSelectedIds: new Set(),
  pendingSource: null,
  isLoading: false,
  error: null,
  activeWeekStart: null,

  // ── fetchEvents ──────────────────────────────────────────────────────────

  fetchEvents: async (libraryId, startAfter, endBefore) => {
    set({ isLoading: true, error: null });
    try {
      const events = await cmd.getCourseEvents(libraryId, startAfter, endBefore);
      set({ events, isLoading: false });
    } catch (e) {
      set({ error: String(e), isLoading: false });
    }
  },

  // ── selectEvent ──────────────────────────────────────────────────────────

  selectEvent: (id) => set({ selectedEventId: id }),

  // ── v2.2: 周导航 ──────────────────────────────────────────────────────

  setActiveWeekStart: (date) => set({ activeWeekStart: date }),

  fetchWeekEvents: async (libraryId, weekStart) => {
    const start = new Date(weekStart);
    const end = new Date(weekStart);
    end.setDate(end.getDate() + 7);
    set({ activeWeekStart: weekStart });
    await get().fetchEvents(libraryId, start.toISOString(), end.toISOString());
  },

  navigateWeek: (libraryId, offset) => {
    const current = get().activeWeekStart ?? getMondayKey(new Date());
    const d = new Date(current);
    d.setDate(d.getDate() + offset * 7);
    const next = localDateKey(d.toISOString());
    void get().fetchWeekEvents(libraryId, next);
  },

  // ── importFromFile ───────────────────────────────────────────────────────

  importFromFile: async (libraryId, filePath) => {
    set({ isLoading: true, error: null });
    try {
      const result = await cmd.importIcsFile(libraryId, filePath);
      // 初始全选
      const allIds = new Set(result.events.map((e) => e.tempId));
      set({
        pendingImportResult: result,
        pendingSelectedIds: allIds,
        pendingSource: { type: "ics_file" },
        isLoading: false,
      });
    } catch (e) {
      set({ error: String(e), isLoading: false });
    }
  },

  // ── importFromUrl ────────────────────────────────────────────────────────

  importFromUrl: async (libraryId, url) => {
    set({ isLoading: true, error: null });
    try {
      const result = await cmd.importIcsUrl(libraryId, url);
      const allIds = new Set(result.events.map((e) => e.tempId));
      set({
        pendingImportResult: result,
        pendingSelectedIds: allIds,
        pendingSource: { type: "ics_url", url },
        isLoading: false,
      });
    } catch (e) {
      set({ error: String(e), isLoading: false });
    }
  },

  // ── togglePendingSelect ──────────────────────────────────────────────────

  togglePendingSelect: (tempId) =>
    set((s) => {
      const next = new Set(s.pendingSelectedIds);
      if (next.has(tempId)) {
        next.delete(tempId);
      } else {
        next.add(tempId);
      }
      return { pendingSelectedIds: next };
    }),

  // ── selectAllPending ─────────────────────────────────────────────────────

  selectAllPending: (allSelected) =>
    set((s) => ({
      pendingSelectedIds: allSelected
        ? new Set(s.pendingImportResult?.events.map((e) => e.tempId) ?? [])
        : new Set<string>(),
    })),

  // ── confirmImport ────────────────────────────────────────────────────────

  confirmImport: async (libraryId) => {
    const { pendingImportResult, pendingSelectedIds, pendingSource } = get();
    if (!pendingImportResult || !pendingSource) return 0;

    set({ isLoading: true, error: null });
    try {
      const inserted = await cmd.confirmImportEvents(
        libraryId,
        pendingImportResult.events,
        Array.from(pendingSelectedIds),
        pendingSource.type,
        pendingSource.url
      );
      // 清空预览状态
      set({
        pendingImportResult: null,
        pendingSelectedIds: new Set(),
        pendingSource: null,
        isLoading: false,
      });
      // 刷新事件列表
      await get().fetchEvents(libraryId);
      return inserted;
    } catch (e) {
      set({ error: String(e), isLoading: false });
      return 0;
    }
  },

  // ── cancelImport ─────────────────────────────────────────────────────────

  cancelImport: () =>
    set({
      pendingImportResult: null,
      pendingSelectedIds: new Set(),
      pendingSource: null,
      error: null,
    }),

  // ── deleteSource ─────────────────────────────────────────────────────────

  deleteSource: async (libraryId, calendarSource, sourceUrl) => {
    set({ isLoading: true, error: null });
    try {
      await cmd.deleteCalendarSource(libraryId, calendarSource, sourceUrl);
      await get().fetchEvents(libraryId);
      set({ isLoading: false });
    } catch (e) {
      set({ error: String(e), isLoading: false });
    }
  },

  // ── refreshSubscription ──────────────────────────────────────────────────

  refreshSubscription: async (libraryId, sourceUrl) => {
    set({ isLoading: true, error: null });
    try {
      await cmd.refreshIcsSubscription(libraryId, sourceUrl);
      await get().fetchEvents(libraryId);
      set({ isLoading: false });
    } catch (e) {
      set({ error: String(e), isLoading: false });
    }
  },

  // ── 派生：getEventsGroupedByDay ──────────────────────────────────────────

  getEventsGroupedByDay: () => {
    const { events } = get();
    const map = new Map<string, CourseEvent[]>();
    for (const ev of events) {
      // startTime 是 RFC3339，取日期部分 YYYY-MM-DD（本地）
      const day = localDateKey(ev.startTime);
      const bucket = map.get(day) ?? [];
      bucket.push(ev);
      map.set(day, bucket);
    }
    return map;
  },

  // ── 派生：getTodayEvents ─────────────────────────────────────────────────

  getTodayEvents: () => {
    const today = todayKey();
    return get().getEventsGroupedByDay().get(today) ?? [];
  },

  // ── 派生：getTomorrowEvents ──────────────────────────────────────────────

  getTomorrowEvents: () => {
    const tomorrow = offsetDayKey(1);
    return get().getEventsGroupedByDay().get(tomorrow) ?? [];
  },

  // ── 派生：getThisWeekEvents ──────────────────────────────────────────────

  getThisWeekEvents: () => {
    const today = todayKey();
    const tomorrow = offsetDayKey(1);
    const weekEnd = thisWeekEndKey();
    const grouped = get().getEventsGroupedByDay();
    const result: CourseEvent[] = [];
    for (const [day, evs] of grouped) {
      if (day > tomorrow && day <= weekEnd) {
        result.push(...evs);
      }
    }
    result.sort((a, b) => a.startTime.localeCompare(b.startTime));
    return result;
    void today; // 消除 unused warning（today 用于语义对比）
  },
}));

// ─── 日期工具（仅模块内用） ──────────────────────────────────────────────────

/** 将 RFC3339 字符串转为本地 YYYY-MM-DD */
function localDateKey(rfc3339: string): string {
  const d = new Date(rfc3339);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** 今日 YYYY-MM-DD */
function todayKey(): string {
  return localDateKey(new Date().toISOString());
}

/** offset 天后的 YYYY-MM-DD */
function offsetDayKey(offset: number): string {
  const d = new Date();
  d.setDate(d.getDate() + offset);
  return localDateKey(d.toISOString());
}

/** 本周（周日为最后一天）的最后一天 YYYY-MM-DD */
function thisWeekEndKey(): string {
  const d = new Date();
  const dow = d.getDay(); // 0=Sun
  const daysToEnd = dow === 0 ? 0 : 7 - dow;
  d.setDate(d.getDate() + daysToEnd);
  return localDateKey(d.toISOString());
}

/** 获取给定日期所在周的周一 YYYY-MM-DD */
export function getMondayKey(d: Date): string {
  const clone = new Date(d);
  const dow = clone.getDay(); // 0=Sun
  const diff = dow === 0 ? -6 : 1 - dow;
  clone.setDate(clone.getDate() + diff);
  return localDateKey(clone.toISOString());
}

function pad(n: number): string {
  return String(n).padStart(2, "0");
}
