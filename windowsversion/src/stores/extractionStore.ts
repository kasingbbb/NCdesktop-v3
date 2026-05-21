import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import type {
  ExtractedContent,
  PipelineProgress,
  ExtractionProgressEvent,
  ExtractionCompletedEvent,
  ExtractionFailedEvent,
} from "../types/extraction";
import * as cmd from "../lib/tauri-commands";
import type { ConversionMetaRow } from "../lib/tauri-commands";

interface ExtractionStore {
  contentCache: Record<string, ExtractedContent>;
  statusCache: Record<string, string>;
  conversionMetaCache: Record<string, ConversionMetaRow[]>;
  pipelineProgress: PipelineProgress | null;
  isExtracting: boolean;

  extractAsset: (assetId: string) => Promise<void>;
  extractProjectAssets: (projectId: string) => Promise<void>;
  retryExtraction: (assetId: string) => Promise<void>;
  fetchExtractionStatus: (assetId: string) => Promise<ExtractedContent | null>;
  fetchExtractedContent: (assetId: string) => Promise<ExtractedContent | null>;
  fetchConversionMeta: (assetId: string) => Promise<void>;
  fetchPipelineProgress: () => Promise<void>;
  initEventListeners: () => Promise<void>;
}

export const useExtractionStore = create<ExtractionStore>((set, get) => ({
  contentCache: {},
  statusCache: {},
  conversionMetaCache: {},
  pipelineProgress: null,
  isExtracting: false,

  extractAsset: async (assetId: string) => {
    try {
      await cmd.extractAsset(assetId);
      set((state) => ({
        statusCache: { ...state.statusCache, [assetId]: "extracting" },
        isExtracting: true,
      }));
    } catch (err) {
      console.error("提取素材失败:", err);
    }
  },

  extractProjectAssets: async (projectId: string) => {
    try {
      await cmd.extractProjectAssets(projectId);
      set({ isExtracting: true });
    } catch (err) {
      console.error("批量提取失败:", err);
    }
  },

  retryExtraction: async (assetId: string) => {
    // task_011 AC-3：统一走 `retrigger_extraction`，后端负责幂等检查、
    // 重置 extracted_content / pipeline_tasks 并唤醒调度器。
    try {
      await cmd.retriggerExtraction(assetId);
      set((state) => ({
        statusCache: { ...state.statusCache, [assetId]: "queued" },
      }));
      // 拉一次最新状态（后端可能因幂等 noop，此时 status 保持原值）
      void get().fetchExtractedContent(assetId);
    } catch (err) {
      console.error("重试提取失败:", err);
    }
  },

  fetchExtractionStatus: async (assetId: string) => {
    try {
      const content = await cmd.getExtractionStatus(assetId);
      if (content) {
        set((state) => ({
          contentCache: { ...state.contentCache, [assetId]: content },
          statusCache: { ...state.statusCache, [assetId]: content.status },
        }));
      }
      return content;
    } catch (err) {
      console.error("获取提取状态失败:", err);
      return null;
    }
  },

  fetchExtractedContent: async (assetId: string) => {
    try {
      const content = await cmd.getExtractedContent(assetId);
      if (content) {
        set((state) => ({
          contentCache: { ...state.contentCache, [assetId]: content },
          statusCache: { ...state.statusCache, [assetId]: content.status },
        }));
        // 拉取最新的转换元数据，失败仅 warn，不阻塞内容展示
        void get().fetchConversionMeta(assetId);
      }
      return content;
    } catch (err) {
      console.error("获取提取内容失败:", err);
      return null;
    }
  },

  fetchConversionMeta: async (assetId: string) => {
    try {
      const rows = await cmd.getConversionMeta(assetId);
      set((state) => ({
        conversionMetaCache: { ...state.conversionMetaCache, [assetId]: rows },
      }));
    } catch (err) {
      console.warn("获取转换元数据失败:", err);
    }
  },

  fetchPipelineProgress: async () => {
    try {
      const progress = await cmd.getPipelineProgress();
      const active = progress.queued + progress.running;
      set({ pipelineProgress: progress, isExtracting: active > 0 });
    } catch (err) {
      console.error("获取管道进度失败:", err);
    }
  },

  initEventListeners: async () => {
    await listen<ExtractionProgressEvent>("extraction:progress", (event) => {
      const { assetId, status } = event.payload;
      set((state) => ({
        statusCache: { ...state.statusCache, [assetId]: status },
        isExtracting: true,
      }));
      get().fetchPipelineProgress();
    });

    await listen<ExtractionCompletedEvent>("extraction:completed", (event) => {
      const { assetId } = event.payload;
      set((state) => ({
        statusCache: { ...state.statusCache, [assetId]: "extracted" },
      }));
      get().fetchExtractedContent(assetId);
      get().fetchPipelineProgress();
    });

    await listen<ExtractionFailedEvent>("extraction:failed", (event) => {
      const { assetId } = event.payload;
      set((state) => ({
        statusCache: { ...state.statusCache, [assetId]: "failed" },
      }));
      get().fetchPipelineProgress();
    });
  },
}));
