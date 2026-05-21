/** 布局模式 */
export type LayoutMode = "three-column" | "two-column" | "single-column";

/** 侧边栏导航项（v2 重定义 — ADR-001）
 *  v1 的 "search" 已删除；"knowledge"/"skills" 合并为 "knowledge-hub"；
 *  "today" / "calendar" 仅在学习模式开启时由视图层条件渲染。
 */
export type SidebarSection =
  | "recent"
  | "starred"
  | "projects"
  | "tags"
  | "knowledge-hub"
  | "today"
  | "calendar";

/** TodayView 内部 Tab */
export type TodayTab = "course-prep" | "daily-review";

/** 素材视图模式（对应访达：图标 / 列表） */
export type AssetViewMode = "grid" | "list";

/** 右栏：素材详情 Inspector 与时间流预览 */
export type RightPanelMode =
  | "inspector"
  | "timeline-flow"
  | "course_preview"
  | "knowledge_association";

/** 课程预习空间的 Back 回退目标 */
export interface CoursePreviewReturnTo {
  section: SidebarSection;
  weekStart?: string;
}

/** 排序方式 */
export interface SortConfig {
  field: "name" | "createdAt" | "updatedAt" | "capturedAt" | "fileSize";
  direction: "asc" | "desc";
}

/** 时间轴播放状态 */
export interface PlaybackState {
  isPlaying: boolean;
  currentTime: number;
  duration: number;
  playbackSpeed: number;
  volume: number;
  isMuted: boolean;
}

/** 时间轴视口 */
export interface TimelineViewport {
  startTime: number;
  endTime: number;
  zoomLevel: number;
}

/** 全局搜索结果 */
export interface SearchResult {
  id: string;
  type: "project" | "asset" | "transcription" | "note";
  title: string;
  snippet: string;
  projectId: string;
  assetId: string | null;
  highlightRanges: Array<{ start: number; end: number }>;
  score: number;
}

/** 模态框枚举 */
export type ModalType =
  | "settings"
  | "export"
  | "import_progress"
  | "asset_detail"
  | "confirm_delete"
  | "about"
  | null;

/** 通知项 */
export interface Notification {
  id: string;
  type: "info" | "success" | "warning" | "error";
  title: string;
  message: string;
  duration: number;
  createdAt: string;
  /**
   * task_011 AC-5：可选去重键。同一 dedupeKey 在 `addNotification` 3 秒
   * 滑动窗口内合并/替换为最新一条（用于快速重复拖拽 / 重试场景，避免 toast 堆积）。
   */
  dedupeKey?: string;
}

/** Dropzone 悬浮窗状态 */
export interface DropzoneState {
  isVisible: boolean;
  isDragOver: boolean;
  isProcessing: boolean;
  recentItems: DropzoneItem[];
}

/** Dropzone 接收项 */
export interface DropzoneItem {
  id: string;
  fileName: string;
  fileType: string;
  status: "pending" | "classifying" | "done" | "error";
  targetProjectId: string | null;
  addedAt: string;
  /** 第二行说明：如 AI 状态 */
  detail?: string;
}
