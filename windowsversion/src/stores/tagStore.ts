import { create } from "zustand";
import type { Tag } from "../types";
import * as cmd from "../lib/tauri-commands";

interface TagStore {
  tags: Tag[];
  isLoading: boolean;
  error: string | null;

  fetchTags: () => Promise<void>;
  createTag: (name: string, color: string, source: string) => Promise<Tag>;
  deleteTag: (id: string) => Promise<void>;
  linkTagToAsset: (assetId: string, tagId: string) => Promise<void>;
  getAssetTags: (assetId: string) => Promise<Tag[]>;
}

export const useTagStore = create<TagStore>((set) => ({
  tags: [],
  isLoading: false,
  error: null,

  fetchTags: async () => {
    set({ isLoading: true, error: null });
    try {
      const tags = await cmd.getTags();
      set({ tags, isLoading: false });
    } catch (e) {
      set({ error: String(e), isLoading: false });
    }
  },

  createTag: async (name, color, source) => {
    const tag = await cmd.createTag(name, color, source);
    set((s) => ({ tags: [tag, ...s.tags] }));
    return tag;
  },

  deleteTag: async (id) => {
    await cmd.deleteTag(id);
    set((s) => ({ tags: s.tags.filter((t) => t.id !== id) }));
  },

  linkTagToAsset: async (assetId, tagId) => {
    await cmd.linkTagToAsset(assetId, tagId);
  },

  getAssetTags: async (assetId) => {
    return cmd.getAssetTags(assetId);
  },
}));
