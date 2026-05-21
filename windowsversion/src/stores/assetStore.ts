import { create } from "zustand";
import type { Asset, AssetViewMode, SortConfig } from "../types";
import type { AIAnalysis, AssetType } from "../types/asset";
import * as cmd from "../lib/tauri-commands";

/**
 * 后端 `get_assets` 现在返回 `WorkspaceAssetView`（task_003 / ADR-002），
 * 而其它视图仍消费 `Asset` 形状。这里把后端 view 映射到 Asset 并并存
 * 工作区派生字段（state / renditionPath 等），保证：
 *  - `asset.type` 由 `assetType` 派生（旧消费者不变）
 *  - `asset.state / renditionPath / sourceMissing / ...` 透传给工作区视图
 *  - `asset.tags / aiAnalysis / source` 不存在于 view，回填默认值
 */
function normalizeAsset(a: Asset): Asset {
  const r = a as Asset & {
    assetType?: string;
    originalName?: string;
    sourceData?: string | null;
  };
  const t = (r.assetType ?? r.type ?? "other") as AssetType;
  const originalName =
    r.originalName && r.originalName.trim().length > 0 ? r.originalName : r.name;
  return {
    ...r,
    type: t,
    originalName,
    sourceData: r.sourceData ?? undefined,
    // WorkspaceAssetView 不带这些字段；为不破坏其它视图的 prop 类型签名，回填默认值
    tags: r.tags ?? [],
    aiAnalysis: r.aiAnalysis ?? null,
    source: r.source ?? { type: "manual_import" },
  };
}

interface AssetStore {
  assets: Asset[];
  /** 项目内各素材的标签名（与 assets 同步于 fetch） */
  assetTagNamesById: Record<string, string[]>;
  selectedAssetId: string | null;
  /** 多选集合 — 框选 / Cmd+Click / Cmd+A 维护 */
  selectedAssetIds: Set<string>;
  viewMode: AssetViewMode;
  sortConfig: SortConfig;
  isLoading: boolean;
  error: string | null;

  fetchAssets: (projectId: string) => Promise<void>;
  fetchAssetsByTag: (projectId: string, tagId: string) => Promise<void>;
  createAsset: (params: {
    projectId: string;
    assetType: string;
    name: string;
    filePath: string;
    fileSize: number;
    mimeType: string;
  }) => Promise<Asset>;
  /** @deprecated rename 场景请使用 {@link renameAsset}（ADR-007 双写 root + derivative）。
   *  保留供 is_starred 等非 rename 的整行 Asset 更新使用。 */
  updateAsset: (asset: Asset) => Promise<void>;
  /** 工作区 rename（ADR-007）：以 asset_id 为目标，后端双写 root.name + markdown 衍生件 .name。 */
  renameAsset: (assetId: string, newDisplayName: string) => Promise<void>;
  deleteAsset: (id: string) => Promise<void>;
  toggleStar: (id: string) => Promise<void>;
  selectAsset: (id: string | null) => void;
  toggleSelectAsset: (id: string) => void;
  setSelectedAssetIds: (ids: Set<string>) => void;
  clearSelection: () => void;
  /** 跨项目移动选中素材（BatchToolbar"移动到"路径）。从当前列表移除并清空选择。 */
  moveAssets: (assetIds: string[], targetProjectId: string) => Promise<void>;
  /** 跨项目复制选中素材（BatchToolbar"复制到"路径）。当前列表不变，仅清空选择。 */
  copyAssets: (assetIds: string[], targetProjectId: string) => Promise<void>;
  setViewMode: (mode: AssetViewMode) => void;
  setSortConfig: (config: SortConfig) => void;
  getSelectedAsset: () => Asset | undefined;
  getAssetAnalysis: (assetId: string) => Promise<AIAnalysis | null>;
}

export const useAssetStore = create<AssetStore>((set, get) => ({
  assets: [],
  assetTagNamesById: {},
  selectedAssetId: null,
  selectedAssetIds: new Set<string>(),
  viewMode: "list",
  sortConfig: { field: "capturedAt", direction: "desc" },
  isLoading: false,
  error: null,

  fetchAssets: async (projectId) => {
    set({ isLoading: true, error: null });
    try {
      const raw = await cmd.getAssets(projectId);
      const assets = raw.map(normalizeAsset);
      const assetTagNamesById = await cmd.getProjectAssetTagMap(projectId);
      set({ assets, assetTagNamesById, isLoading: false });
    } catch (e) {
      set({ error: String(e), isLoading: false });
    }
  },

  fetchAssetsByTag: async (projectId, tagId) => {
    set({ isLoading: true, error: null });
    try {
      const raw = await cmd.getAssetsByTag(projectId, tagId);
      const assets = raw.map(normalizeAsset);
      const assetTagNamesById = await cmd.getProjectAssetTagMap(projectId);
      set({ assets, assetTagNamesById, isLoading: false });
    } catch (e) {
      set({ error: String(e), isLoading: false });
    }
  },

  createAsset: async (params) => {
    const asset = await cmd.createAsset(params);
    set((s) => ({ assets: [asset, ...s.assets] }));
    return asset;
  },

  updateAsset: async (asset) => {
    await cmd.updateAsset(asset);
    set((s) => ({
      assets: s.assets.map((a) => (a.id === asset.id ? asset : a)),
    }));
  },

  renameAsset: async (assetId, newDisplayName) => {
    // 后端双写 root + derivative.name 并返回最新 WorkspaceAssetView；
    // 前端就地 patch name，避免再走 fetchAssets 整列表重拉。
    const view = await cmd.renameAsset(assetId, newDisplayName);
    set((s) => ({
      assets: s.assets.map((a) =>
        a.id === view.id ? { ...a, name: view.name } : a
      ),
    }));
  },

  deleteAsset: async (id) => {
    await cmd.deleteAsset(id);
    set((s) => ({
      assets: s.assets.filter((a) => a.id !== id),
      selectedAssetId: s.selectedAssetId === id ? null : s.selectedAssetId,
    }));
  },

  toggleStar: async (id) => {
    const newStarred = await cmd.toggleAssetStar(id);
    set((s) => ({
      assets: s.assets.map((a) =>
        a.id === id ? { ...a, isStarred: newStarred } : a
      ),
    }));
  },

  selectAsset: (id) => set({ selectedAssetId: id }),

  toggleSelectAsset: (id) =>
    set((s) => {
      const next = new Set(s.selectedAssetIds);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return { selectedAssetIds: next };
    }),

  setSelectedAssetIds: (ids) => set({ selectedAssetIds: ids }),

  clearSelection: () => set({ selectedAssetIds: new Set<string>() }),

  moveAssets: async (assetIds, targetProjectId) => {
    await cmd.moveAssets(assetIds, targetProjectId);
    set((s) => ({
      assets: s.assets.filter((a) => !assetIds.includes(a.id)),
      selectedAssetIds: new Set<string>(),
      selectedAssetId: assetIds.includes(s.selectedAssetId ?? "")
        ? null
        : s.selectedAssetId,
    }));
  },

  copyAssets: async (assetIds, targetProjectId) => {
    await cmd.copyAssets(assetIds, targetProjectId);
    set({ selectedAssetIds: new Set<string>() });
  },

  setViewMode: (mode) => set({ viewMode: mode }),

  setSortConfig: (config) => set({ sortConfig: config }),

  getSelectedAsset: () => {
    const { assets, selectedAssetId } = get();
    return assets.find((a) => a.id === selectedAssetId);
  },

  getAssetAnalysis: async (assetId) => {
    return cmd.getAssetAnalysis(assetId);
  },
}));
