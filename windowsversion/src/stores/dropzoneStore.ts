import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { DropzoneItem } from "../types";

/** 悬浮窗状态机：Hidden → Idle → Attract → Processing → Complete */
type DropzonePhase = "hidden" | "idle" | "attract" | "processing" | "complete";

export interface DropzoneStore {
  phase: DropzonePhase;
  isExpanded: boolean;
  recentItems: DropzoneItem[];
  processingProgress: number;
  processingMessage: string;

  show: () => Promise<void>;
  hide: () => Promise<void>;
  toggle: () => Promise<void>;
  setPhase: (phase: DropzonePhase) => void;
  toggleExpand: () => void;
  setExpanded: (isExpanded: boolean) => void;
  setProcessingUI: (message: string, progress: number) => void;
  clearProcessingUI: () => void;
  addItem: (item: DropzoneItem) => void;
  updateItemStatus: (id: string, status: DropzoneItem["status"], projectId?: string) => void;
  updateItemDetail: (id: string, detail: string) => void;
  clearRecentItems: () => void;
}

export const useDropzoneStore = create<DropzoneStore>((set) => ({
  phase: "idle",
  isExpanded: false,
  recentItems: [],
  processingProgress: 0,
  processingMessage: "",

  show: async () => {
    await invoke("create_dropzone_window");
    set({ phase: "idle" });
  },

  hide: async () => {
    await invoke("close_dropzone_window");
    set({ phase: "hidden" });
  },

  toggle: async () => {
    const visible = await invoke<boolean>("toggle_dropzone_window");
    set({ phase: visible ? "idle" : "hidden" });
  },

  setPhase: (phase) => set({ phase }),

  toggleExpand: () => set((s) => ({ isExpanded: !s.isExpanded })),

  setExpanded: (isExpanded) => set({ isExpanded }),

  setProcessingUI: (processingMessage, processingProgress) =>
    set({ processingMessage, processingProgress }),

  clearProcessingUI: () =>
    set({ processingMessage: "", processingProgress: 0 }),

  addItem: (item) =>
    set((s) => ({
      recentItems: [item, ...s.recentItems].slice(0, 10),
    })),

  updateItemStatus: (id, status, projectId) =>
    set((s) => ({
      recentItems: s.recentItems.map((item) =>
        item.id === id
          ? { ...item, status, targetProjectId: projectId ?? item.targetProjectId }
          : item
      ),
    })),

  updateItemDetail: (id, detail) =>
    set((s) => ({
      recentItems: s.recentItems.map((item) =>
        item.id === id ? { ...item, detail } : item
      ),
    })),

  clearRecentItems: () => set({ recentItems: [] }),
}));
