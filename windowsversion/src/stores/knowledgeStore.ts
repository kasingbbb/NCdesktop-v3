/**
 * knowledgeStore — 知识关联前端状态管理
 *
 * 职责：
 *   - 持有知识库概念列表（含统计）
 *   - 管理概念选中、详情加载、提取进度
 *   - 提供搜索过滤派生数据
 *
 * 约束（宪章 A4）：不 import 其他 Store，跨 Store 数据在组件层组合
 */

import { create } from "zustand";
import type {
  ConceptWithStats,
  ConceptDetail,
  ConceptViewpoint,
  ConceptExtension,
  ExtractionProgress,
} from "../types/knowledge";
import * as cmd from "../lib/tauri-commands";

// ─── 状态结构 ─────────────────────────────────────────────────────────────────

interface KnowledgeStore {
  /** 概念列表（含统计，用于左侧面板） */
  concepts: ConceptWithStats[];
  /** 当前选中概念 ID */
  selectedConceptId: string | null;
  /** 当前选中概念的完整详情（含观点/案例/拓展） */
  conceptDetail: ConceptDetail | null;
  /** 概念提取后台任务进度 */
  extractionProgress: ExtractionProgress | null;
  /** 搜索关键词 */
  searchQuery: string;
  /** 按项目筛选（null = 全部） */
  filterProjectId: string | null;
  isLoading: boolean;
  isLoadingDetail: boolean;
  error: string | null;

  // ── 动作 ──────────────────────────────────────────────────────────────────

  /** 加载知识库所有概念 */
  fetchConcepts: (libraryId: string) => Promise<void>;

  /** 选中概念（自动触发 loadDetail） */
  selectConcept: (id: string | null) => void;

  /** 加载概念详情（含观点/案例/拓展） */
  loadDetail: (conceptId: string) => Promise<void>;

  /** 更新概念名称或定义 */
  updateConcept: (
    conceptId: string,
    name?: string,
    definition?: string
  ) => Promise<void>;

  /** 删除概念 */
  deleteConcept: (conceptId: string) => Promise<void>;

  /**
   * 触发概念提取任务。
   * `forceFull` 语义见 `tauri-commands.ts::extractConceptsForLibrary`。
   */
  startExtraction: (libraryId: string, forceFull: boolean) => Promise<void>;

  /** 设置搜索词 */
  setSearchQuery: (q: string) => void;

  /** 设置项目筛选 */
  setFilterProject: (projectId: string | null) => void;

  /** 更新提取进度（由外部事件监听调用） */
  setExtractionProgress: (progress: ExtractionProgress | null) => void;

  /** 为选中概念合成观点（按需触发） */
  synthesizeViewpoints: (conceptId: string) => Promise<void>;

  /** 为选中概念生成知识拓展（按需触发） */
  generateExtensions: (conceptId: string) => Promise<void>;

  // ── 派生数据 ──────────────────────────────────────────────────────────────

  /** 基于 searchQuery + filterProjectId 过滤后的概念列表 */
  getFilteredConcepts: () => ConceptWithStats[];
}

// ─── 实现 ─────────────────────────────────────────────────────────────────────

export const useKnowledgeStore = create<KnowledgeStore>((set, get) => ({
  concepts: [],
  selectedConceptId: null,
  conceptDetail: null,
  extractionProgress: null,
  searchQuery: "",
  filterProjectId: null,
  isLoading: false,
  isLoadingDetail: false,
  error: null,

  // ── fetchConcepts ─────────────────────────────────────────────────────────

  fetchConcepts: async (libraryId) => {
    set({ isLoading: true, error: null });
    try {
      const concepts = await cmd.getConcepts(libraryId);
      set({ concepts, isLoading: false });
    } catch (e) {
      set({ error: String(e), isLoading: false });
    }
  },

  // ── selectConcept ─────────────────────────────────────────────────────────

  selectConcept: (id) => {
    set({ selectedConceptId: id, conceptDetail: null });
    if (id) {
      void get().loadDetail(id);
    }
  },

  // ── loadDetail ────────────────────────────────────────────────────────────

  loadDetail: async (conceptId) => {
    set({ isLoadingDetail: true, error: null });
    try {
      const detail = await cmd.getConceptDetail(conceptId);
      set({ conceptDetail: detail ?? null, isLoadingDetail: false });
    } catch (e) {
      set({ error: String(e), isLoadingDetail: false });
    }
  },

  // ── updateConcept ─────────────────────────────────────────────────────────

  updateConcept: async (conceptId, name, definition) => {
    await cmd.updateConcept(conceptId, name, definition);
    // 乐观更新 concepts 列表
    set((s) => ({
      concepts: s.concepts.map((c) =>
        c.id === conceptId
          ? {
              ...c,
              ...(name !== undefined ? { name } : {}),
              ...(definition !== undefined ? { definition } : {}),
              userEdited: true,
            }
          : c
      ),
      // 同步更新详情中的 concept 字段
      conceptDetail:
        s.conceptDetail?.concept.id === conceptId
          ? {
              ...s.conceptDetail,
              concept: {
                ...s.conceptDetail.concept,
                ...(name !== undefined ? { name } : {}),
                ...(definition !== undefined ? { definition } : {}),
                userEdited: true,
              },
            }
          : s.conceptDetail,
    }));
  },

  // ── deleteConcept ─────────────────────────────────────────────────────────

  deleteConcept: async (conceptId) => {
    await cmd.deleteConcept(conceptId);
    set((s) => ({
      concepts: s.concepts.filter((c) => c.id !== conceptId),
      selectedConceptId:
        s.selectedConceptId === conceptId ? null : s.selectedConceptId,
      conceptDetail:
        s.conceptDetail?.concept.id === conceptId ? null : s.conceptDetail,
    }));
  },

  // ── startExtraction ───────────────────────────────────────────────────────

  startExtraction: async (libraryId, forceFull) => {
    set({
      extractionProgress: {
        totalAssets: 0,
        processed: 0,
        conceptsFound: 0,
        status: "running",
      },
      error: null,
    });
    try {
      const result = await cmd.extractConceptsForLibrary(libraryId, forceFull);
      set({ extractionProgress: result });
      // 提取完成后刷新概念列表
      await get().fetchConcepts(libraryId);
    } catch (e) {
      // 错误态：除了 store.error 外，同步写入 extractionProgress.error，
      // 进度条组件用它渲染"扫描出错：..."（task_perf_02 AC-1）
      const msg = String(e);
      set({
        extractionProgress: {
          totalAssets: 0,
          processed: 0,
          conceptsFound: 0,
          status: "error",
          error: msg,
        },
        error: msg,
      });
    }
  },

  // ── setSearchQuery ────────────────────────────────────────────────────────

  setSearchQuery: (q) => set({ searchQuery: q }),

  // ── setFilterProject ──────────────────────────────────────────────────────

  setFilterProject: (projectId) => set({ filterProjectId: projectId }),

  // ── setExtractionProgress ─────────────────────────────────────────────────

  setExtractionProgress: (progress) => set({ extractionProgress: progress }),

  // ── synthesizeViewpoints ──────────────────────────────────────────────────

  synthesizeViewpoints: async (conceptId) => {
    set({ isLoadingDetail: true });
    try {
      const viewpoints: ConceptViewpoint[] =
        await cmd.synthesizeViewpoints(conceptId);
      set((s) => ({
        conceptDetail: s.conceptDetail
          ? { ...s.conceptDetail, viewpoints }
          : null,
        isLoadingDetail: false,
      }));
    } catch (e) {
      set({ error: String(e), isLoadingDetail: false });
    }
  },

  // ── generateExtensions ────────────────────────────────────────────────────

  generateExtensions: async (conceptId) => {
    set({ isLoadingDetail: true });
    try {
      const extensions: ConceptExtension[] =
        await cmd.generateExtensions(conceptId);
      set((s) => ({
        conceptDetail: s.conceptDetail
          ? { ...s.conceptDetail, extensions }
          : null,
        isLoadingDetail: false,
      }));
    } catch (e) {
      set({ error: String(e), isLoadingDetail: false });
    }
  },

  // ── getFilteredConcepts ───────────────────────────────────────────────────

  getFilteredConcepts: () => {
    const { concepts, searchQuery, filterProjectId } = get();
    let result = concepts;

    // 搜索过滤：模糊匹配 name + aliases
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      result = result.filter(
        (c) =>
          c.name.toLowerCase().includes(q) ||
          c.aliases.some((a) => a.toLowerCase().includes(q))
      );
    }

    // 项目筛选（v2.1 scope：暂时按 sourceProjectCount > 0 过滤，
    // 完整的 projectId 级过滤需要后端支持）
    if (filterProjectId) {
      result = result.filter((c) => c.sourceProjectCount > 0);
    }

    return result;
  },
}));
