/**
 * PR-3 task_009/012: 分类对象 store
 * 副作用集中在 action；组件只 dispatch / 读取
 */
import { create } from "zustand";
import * as cmd from "../lib/tauri-commands";

interface CategoryStore {
  libraryId: string | null;
  categories: cmd.Category[];
  activeSlug: string | null;
  loading: boolean;
  error: string | null;

  setLibrary: (id: string | null) => void;
  fetch: (includeDisabled?: boolean) => Promise<void>;
  setActive: (slug: string | null) => void;
  create: (slug: string, label: string, sortOrder?: number) => Promise<void>;
  rename: (slug: string, label: string) => Promise<void>;
  setDisabled: (slug: string, disabled: boolean) => Promise<void>;
  remove: (slug: string) => Promise<void>;
}

export const useCategoryStore = create<CategoryStore>((set, get) => ({
  libraryId: null,
  categories: [],
  activeSlug: null,
  loading: false,
  error: null,

  setLibrary: (id) => {
    set({ libraryId: id, categories: [], activeSlug: null });
  },

  fetch: async (includeDisabled = false) => {
    const lib = get().libraryId;
    if (!lib) return;
    set({ loading: true, error: null });
    try {
      const cats = await cmd.listCategories(lib, includeDisabled);
      set({ categories: cats, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  setActive: (slug) => set({ activeSlug: slug }),

  create: async (slug, label, sortOrder) => {
    const lib = get().libraryId;
    if (!lib) throw new Error("libraryId 未设置");
    await cmd.createCategory(lib, slug, label, sortOrder);
    await get().fetch();
  },

  rename: async (slug, label) => {
    const lib = get().libraryId;
    if (!lib) throw new Error("libraryId 未设置");
    await cmd.renameCategory(lib, slug, label);
    await get().fetch();
  },

  setDisabled: async (slug, disabled) => {
    const lib = get().libraryId;
    if (!lib) throw new Error("libraryId 未设置");
    await cmd.setCategoryDisabled(lib, slug, disabled);
    await get().fetch(true);
  },

  remove: async (slug) => {
    const lib = get().libraryId;
    if (!lib) throw new Error("libraryId 未设置");
    await cmd.deleteCategory(lib, slug);
    await get().fetch();
  },
}));
