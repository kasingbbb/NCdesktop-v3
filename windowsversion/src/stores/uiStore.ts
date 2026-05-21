import { create } from "zustand";
import { persist } from "zustand/middleware";
import type {
  LayoutMode,
  SidebarSection,
  ModalType,
  Notification,
  DropzoneState,
  RightPanelMode,
  CoursePreviewReturnTo,
  TodayTab,
} from "../types";

interface MagicMomentState {
  activeKeyframeId: string | null;
  highlightedKeyframeId: string | null;
  previewAssetId: string | null;
  isAnimating: boolean;
}

// ── SidebarSection 兼容迁移（ADR-001） ───────────────────────────
const VALID_SECTIONS: readonly SidebarSection[] = [
  "recent",
  "starred",
  "projects",
  "tags",
  "knowledge-hub",
  "today",
  "calendar",
] as const;

// 编译期断言：VALID_SECTIONS 与 union 保持同步
type _AssertCovers = Exclude<SidebarSection, (typeof VALID_SECTIONS)[number]> extends never
  ? true
  : false;
const _typeCheck: _AssertCovers = true;
void _typeCheck;

function devWarn(...args: unknown[]): void {
  if (import.meta.env.DEV) {
    console.warn(...args);
  }
}

/** 把任意旧值/未知值/坏类型映射到合法新 SidebarSection（ADR-001 矩阵）。 */
export function migrateLegacySection(raw: unknown): SidebarSection {
  if (raw === null || raw === undefined) return "recent";
  if (typeof raw !== "string") {
    devWarn(`[uiStore] migrateLegacySection 非 string 输入 → recent:`, raw);
    return "recent";
  }
  if ((VALID_SECTIONS as readonly string[]).includes(raw)) {
    return raw as SidebarSection;
  }
  if (raw === "knowledge" || raw === "skills") {
    devWarn(`[uiStore] migrateLegacySection 旧值 "${raw}" → knowledge-hub`);
    return "knowledge-hub";
  }
  if (raw === "search") {
    devWarn(`[uiStore] migrateLegacySection 已删除值 "search" → recent`);
    return "recent";
  }
  devWarn(`[uiStore] migrateLegacySection 未知值 "${raw}" → recent`);
  return "recent";
}

function migrateLegacyTodayTab(raw: unknown): TodayTab | null {
  if (raw === "course-prep" || raw === "daily-review") return raw;
  return null;
}

interface UIStore {
  layoutMode: LayoutMode;
  activeSidebarSection: SidebarSection;
  inspectorOpen: boolean;
  rightPanelMode: RightPanelMode;
  sidebarWidth: number;
  activeModal: ModalType;
  notifications: Notification[];
  dropzone: DropzoneState;
  magicMoment: MagicMomentState;
  assetTagFilterId: string | null;
  workspaceFolderRelativePath: string | null;
  activeCourseEventId: string | null;
  coursePreviewReturnTo: CoursePreviewReturnTo | null;
  /** TodayView 上次活跃 Tab（持久化） */
  todayLastTab: TodayTab | null;
  /** 学习模式刚由 OFF→ON 的瞬态信号（不持久化） */
  _learningJustEnabled: boolean;

  // ── 工作区文件夹列表编辑态（task_006 T4，瞬态，不进 partialize） ──
  editingFolderPath: string | null;
  pendingNewFolder: boolean;
  pendingRenameIds: Set<string>;
  dragOverPath: string | null;

  /** TagTree 展开状态（v1.3 SB-05，默认 false，持久化） */
  tagsExpanded: boolean;

  setLayoutMode: (mode: LayoutMode) => void;
  setSidebarSection: (section: SidebarSection) => void;
  toggleInspector: () => void;
  setInspectorOpen: (open: boolean) => void;
  setRightPanelMode: (mode: RightPanelMode) => void;
  setSidebarWidth: (width: number) => void;
  openModal: (modal: ModalType) => void;
  closeModal: () => void;
  addNotification: (n: Omit<Notification, "id" | "createdAt">) => void;
  removeNotification: (id: string) => void;
  setDropzone: (partial: Partial<DropzoneState>) => void;
  setMagicMoment: (partial: Partial<MagicMomentState>) => void;
  setAssetTagFilterId: (tagId: string | null) => void;
  setWorkspaceFolderRelativePath: (path: string | null) => void;
  setActiveCourseEventId: (id: string | null) => void;
  setCoursePreviewReturnTo: (target: CoursePreviewReturnTo | null) => void;
  setTodayLastTab: (tab: TodayTab | null) => void;
  setLearningJustEnabled: (flag: boolean) => void;
  startCreating: () => void;
  cancelCreating: () => void;
  startRenaming: (path: string) => void;
  finishRename: (path: string) => void;
  setDragOverPath: (path: string | null) => void;
  setTagsExpanded: (expanded: boolean) => void;
}

let notificationId = 0;

/**
 * task_011 AC-5：dedupeKey 滑动窗口（毫秒）。
 * 相同 key 在窗口内的新 toast 会**替换**已存在的同 key 条目（保留最新文案），
 * 而不是再 push 一条。窗口外则照常新增。
 */
const DEDUPE_WINDOW_MS = 3000;
/** key → 最近一次该 key 命中的时间戳（毫秒） */
const dedupeLastSeen = new Map<string, number>();

export const useUIStore = create<UIStore>()(
  persist(
    (set) => ({
      layoutMode: "three-column",
      activeSidebarSection: "recent",
      inspectorOpen: false,
      rightPanelMode: "inspector",
      sidebarWidth: 220,
      activeModal: null,
      notifications: [],
      dropzone: {
        isVisible: false,
        isDragOver: false,
        isProcessing: false,
        recentItems: [],
      },
      magicMoment: {
        activeKeyframeId: null,
        highlightedKeyframeId: null,
        previewAssetId: null,
        isAnimating: false,
      },
      assetTagFilterId: null,
      workspaceFolderRelativePath: null,
      activeCourseEventId: null,
      coursePreviewReturnTo: null,
      todayLastTab: null,
      _learningJustEnabled: false,
      editingFolderPath: null,
      pendingNewFolder: false,
      pendingRenameIds: new Set<string>(),
      dragOverPath: null,
      tagsExpanded: false,

      setLayoutMode: (mode) => set({ layoutMode: mode }),

      // setter 入口拦截：任何写入都先走 migrateLegacySection（防 Dev 误传 / 旧 LS）
      setSidebarSection: (section) =>
        set({ activeSidebarSection: migrateLegacySection(section) }),

      toggleInspector: () => set((s) => ({ inspectorOpen: !s.inspectorOpen })),

      setInspectorOpen: (open) => set({ inspectorOpen: open }),

      setRightPanelMode: (mode) => set({ rightPanelMode: mode }),

      setSidebarWidth: (width) => set({ sidebarWidth: width }),

      openModal: (modal) => set({ activeModal: modal }),

      closeModal: () => set({ activeModal: null }),

      addNotification: (n) => {
        const id = String(++notificationId);
        const notification: Notification = {
          ...n,
          id,
          createdAt: new Date().toISOString(),
        };
        // task_011 AC-5：dedupeKey 命中（3s 窗口内）→ 替换已存在的同 key 条目
        const key = n.dedupeKey;
        const now = Date.now();
        if (key) {
          const last = dedupeLastSeen.get(key);
          const inWindow = typeof last === "number" && now - last < DEDUPE_WINDOW_MS;
          dedupeLastSeen.set(key, now);
          if (inWindow) {
            set((s) => {
              const existingIdx = s.notifications.findIndex(
                (item) => item.dedupeKey === key
              );
              if (existingIdx >= 0) {
                const next = s.notifications.slice();
                next[existingIdx] = notification;
                return { notifications: next };
              }
              return { notifications: [...s.notifications, notification] };
            });
            if (n.duration > 0) {
              setTimeout(() => {
                set((s) => ({
                  notifications: s.notifications.filter((item) => item.id !== id),
                }));
              }, n.duration);
            }
            return;
          }
        }
        set((s) => ({
          notifications: [...s.notifications, notification],
        }));
        if (n.duration > 0) {
          setTimeout(() => {
            set((s) => ({
              notifications: s.notifications.filter((item) => item.id !== id),
            }));
          }, n.duration);
        }
      },

      removeNotification: (id) =>
        set((s) => ({
          notifications: s.notifications.filter((n) => n.id !== id),
        })),

      setDropzone: (partial) =>
        set((s) => ({
          dropzone: { ...s.dropzone, ...partial },
        })),

      setMagicMoment: () => {},

      setAssetTagFilterId: (tagId) => set({ assetTagFilterId: tagId }),

      setWorkspaceFolderRelativePath: (path) =>
        set({ workspaceFolderRelativePath: path }),

      setActiveCourseEventId: (id) => set({ activeCourseEventId: id }),

      setCoursePreviewReturnTo: (target) => set({ coursePreviewReturnTo: target }),

      setTodayLastTab: (tab) => set({ todayLastTab: tab }),

      setLearningJustEnabled: (flag) => set({ _learningJustEnabled: flag }),

      startCreating: () =>
        set({ pendingNewFolder: true, editingFolderPath: null }),

      cancelCreating: () => set({ pendingNewFolder: false }),

      startRenaming: (path) =>
        set((s) => {
          const next = new Set(s.pendingRenameIds);
          next.add(path);
          return { pendingRenameIds: next, editingFolderPath: path };
        }),

      finishRename: (path) =>
        set((s) => {
          const next = new Set(s.pendingRenameIds);
          next.delete(path);
          return {
            pendingRenameIds: next,
            editingFolderPath:
              s.editingFolderPath === path ? null : s.editingFolderPath,
          };
        }),

      setDragOverPath: (path) => set({ dragOverPath: path }),

      setTagsExpanded: (expanded) => set({ tagsExpanded: expanded }),
    }),
    {
      name: "ui-store",
      version: 1,
      partialize: (s) => ({
        activeSidebarSection: s.activeSidebarSection,
        todayLastTab: s.todayLastTab,
        tagsExpanded: s.tagsExpanded,
      }),
      migrate: (persisted) => {
        const raw = (persisted as { activeSidebarSection?: unknown } | undefined)
          ?.activeSidebarSection;
        const rawTab = (persisted as { todayLastTab?: unknown } | undefined)
          ?.todayLastTab;
        const rawTagsExpanded = (persisted as { tagsExpanded?: unknown } | undefined)
          ?.tagsExpanded;
        return {
          activeSidebarSection: migrateLegacySection(raw),
          todayLastTab: migrateLegacyTodayTab(rawTab),
          tagsExpanded: typeof rawTagsExpanded === "boolean" ? rawTagsExpanded : false,
        };
      },
    },
  ),
);
