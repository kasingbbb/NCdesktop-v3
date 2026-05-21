// v2.1 — 课程日历 & 预习相关类型

/** 已持久化的课程事件（来自数据库） */
export interface CourseEvent {
  id: string;
  libraryId: string;
  projectId: string | null;
  title: string;
  courseCode: string | null;
  instructor: string | null;
  location: string | null;
  startTime: string; // RFC3339
  endTime: string;   // RFC3339
  recurrenceRule: string | null;
  dayOfWeek: number[];
  description: string | null;
  calendarSource: "ics_file" | "ics_url";
  sourceUrl: string | null;
  sourceUid: string | null;
  lastSynced: string | null;
  createdAt: string;
}

/** 解析中的课程事件（来自 ics_parser，尚未写入 DB） */
export interface ParsedEvent {
  tempId: string;
  title: string;
  courseCode: string | null;
  instructor: string | null;
  location: string | null;
  startTime: string;
  endTime: string;
  recurrenceRule: string | null;
  dayOfWeek: number[];
  description: string | null;
  sourceUid: string | null;
}

/** 解析结果（import_ics_file / import_ics_url 的返回值） */
export interface ImportIcsResult {
  events: ParsedEvent[];
  totalParsed: number;
  duplicatesSkipped: number;
}

/** AI 预习内容（持久化） */
export interface CoursePreview {
  id: string;
  courseEventId: string;
  content: string;       // Markdown
  userNotes: string | null;
  model: string | null;
  generatedAt: string;
  createdAt: string;
}
