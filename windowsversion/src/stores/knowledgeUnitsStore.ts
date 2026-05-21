/**
 * knowledgeUnitsStore — 知识进化系统前端状态管理
 *
 * 职责：
 *   - 持有知识单元列表（知识库主视图）
 *   - 管理当前选中的知识单元详情
 *   - 合成管道进度
 *   - 搜索过滤
 *
 * 约束（宪章 A4）：不 import 其他 Store，跨 Store 数据在组件层组合
 */

import { create } from "zustand";
import type {
  KnowledgeUnit,
  KnowledgeUnitSummary,
  UnderstandingSnapshot,
  CreateSnapshot,
} from "../types/knowledge-units";
import * as cmd from "../lib/tauri-commands";

// ─── 状态结构 ─────────────────────────────────────────────────────────────────

interface KnowledgeUnitsStore {
  units: KnowledgeUnitSummary[];
  selectedUnitId: string | null;
  unitDetail: KnowledgeUnit | null;
  snapshots: UnderstandingSnapshot[];

  synthesisStage: string | null; // null=idle | "clustering" | "naming" | "completed" | "error"
  synthesisGroupsFound: number;
  synthesisUnitsWritten: number;

  searchQuery: string;
  isLoading: boolean;
  isLoadingDetail: boolean;
  error: string | null;

  // ── 动作 ──────────────────────────────────────────────────────────────────

  fetchUnits: (libraryId: string) => Promise<void>;
  selectUnit: (id: string | null) => void;
  loadDetail: (id: string) => Promise<void>;
  loadSnapshots: (id: string) => Promise<void>;

  startSynthesis: (libraryId: string, force?: boolean) => Promise<void>;

  updateNote: (id: string, userNote: string) => Promise<void>;
  updateMirrorFeedback: (id: string, feedbackJson: string) => Promise<void>;
  updateStatus: (id: string, status: string) => Promise<void>;
  updateReviewSchedule: (id: string, nextReviewDue: string | null, depthLevel: number) => Promise<void>;
  createSnapshot: (snapshot: CreateSnapshot) => Promise<void>;
  deleteUnit: (id: string) => Promise<void>;

  setSearchQuery: (q: string) => void;
  setSynthesisProgress: (stage: string, groupsFound: number, unitsWritten: number) => void;
  getFilteredUnits: () => KnowledgeUnitSummary[];
}

// ─── 实现 ──────────────────────────────────────────────────────────────────────

export const useKnowledgeUnitsStore = create<KnowledgeUnitsStore>((set, get) => ({
  units: [],
  selectedUnitId: null,
  unitDetail: null,
  snapshots: [],
  synthesisStage: null,
  synthesisGroupsFound: 0,
  synthesisUnitsWritten: 0,
  searchQuery: "",
  isLoading: false,
  isLoadingDetail: false,
  error: null,

  fetchUnits: async (libraryId) => {
    set({ isLoading: true });
    try {
      const units = await cmd.kuGetList(libraryId);
      set({ units, isLoading: false });
    } catch (e) {
      set({ isLoading: false });
    }
  },

  selectUnit: (id) => {
    set({ selectedUnitId: id, unitDetail: null, snapshots: [] });
    if (id) {
      get().loadDetail(id);
      get().loadSnapshots(id);
    }
  },

  loadDetail: async (id) => {
    set({ isLoadingDetail: true });
    try {
      const detail = await cmd.kuGetDetail(id);
      // Parse explanation JSON if present
      let parsed: KnowledgeUnit | null = detail;
      if (parsed && typeof parsed.explanation === "string") {
        try {
          parsed = { ...parsed, explanation: JSON.parse(parsed.explanation as unknown as string) };
        } catch {
          // leave as-is
        }
      }
      if (parsed && typeof parsed.lastMirrorFeedback === "string") {
        try {
          parsed = { ...parsed, lastMirrorFeedback: JSON.parse(parsed.lastMirrorFeedback as unknown as string) };
        } catch {
          // leave as-is
        }
      }
      set({ unitDetail: parsed, isLoadingDetail: false });
    } catch (e) {
      set({ error: String(e), isLoadingDetail: false });
    }
  },

  loadSnapshots: async (id) => {
    try {
      const snapshots = await cmd.kuGetSnapshots(id);
      set({ snapshots });
    } catch {
      // non-fatal
    }
  },

  startSynthesis: async (libraryId, force = false) => {
    set({ synthesisStage: "clustering", synthesisGroupsFound: 0, synthesisUnitsWritten: 0 });
    try {
      const units = await cmd.synthesizeKnowledgeUnits(libraryId, force);
      set({ units, synthesisStage: "completed" });
    } catch (e) {
      set({ synthesisStage: "error", error: String(e) });
    }
  },

  updateNote: async (id, userNote) => {
    await cmd.kuUpdateNote(id, userNote);
    // Optimistic: update local summary status
    set((s) => ({
      units: s.units.map((u) =>
        u.id === id ? { ...u, status: u.status === "raw" || u.status === "synthesized" || u.status === "understood" ? "articulated" : u.status } as KnowledgeUnitSummary : u
      ),
      unitDetail: s.unitDetail?.id === id
        ? { ...s.unitDetail, userNote, status: "articulated" }
        : s.unitDetail,
    }));
  },

  updateMirrorFeedback: async (id, feedbackJson) => {
    await cmd.kuUpdateMirrorFeedback(id, feedbackJson);
    set((s) => ({
      units: s.units.map((u) =>
        u.id === id ? { ...u, status: "validated" } as KnowledgeUnitSummary : u
      ),
    }));
  },

  updateStatus: async (id, status) => {
    await cmd.kuUpdateStatus(id, status);
    set((s) => ({
      units: s.units.map((u) => (u.id === id ? { ...u, status } as KnowledgeUnitSummary : u)),
      unitDetail: s.unitDetail?.id === id ? { ...s.unitDetail, status } as KnowledgeUnit : s.unitDetail,
    }));
  },

  updateReviewSchedule: async (id, nextReviewDue, depthLevel) => {
    await cmd.kuUpdateReviewSchedule(id, nextReviewDue, depthLevel);
    set((s) => ({
      units: s.units.map((u) =>
        u.id === id ? { ...u, nextReviewDue, depthLevel } as KnowledgeUnitSummary : u
      ),
    }));
  },

  createSnapshot: async (snapshot) => {
    await cmd.kuCreateSnapshot(snapshot);
    // Append to local snapshots
    set((s) => ({ snapshots: [...s.snapshots, snapshot as UnderstandingSnapshot] }));
  },

  deleteUnit: async (id) => {
    await cmd.kuDelete(id);
    set((s) => ({
      units: s.units.filter((u) => u.id !== id),
      selectedUnitId: s.selectedUnitId === id ? null : s.selectedUnitId,
      unitDetail: s.unitDetail?.id === id ? null : s.unitDetail,
    }));
  },

  setSearchQuery: (q) => set({ searchQuery: q }),

  setSynthesisProgress: (stage, groupsFound, unitsWritten) =>
    set({ synthesisStage: stage, synthesisGroupsFound: groupsFound, synthesisUnitsWritten: unitsWritten }),

  getFilteredUnits: () => {
    const { units, searchQuery } = get();
    if (!searchQuery.trim()) return units;
    const q = searchQuery.toLowerCase();
    return units.filter(
      (u) =>
        u.title.toLowerCase().includes(q) ||
        u.coreInsight.toLowerCase().includes(q)
    );
  },
}));
