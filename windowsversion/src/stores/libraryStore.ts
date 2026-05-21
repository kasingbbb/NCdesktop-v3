import { create } from "zustand";
import type { Library } from "../types";
import * as cmd from "../lib/tauri-commands";

interface LibraryStore {
  libraries: Library[];
  activeLibraryId: string | null;
  isLoading: boolean;
  error: string | null;

  fetchLibraries: () => Promise<void>;
  ensureActiveLibrary: () => Promise<string>;
  createLibrary: (name: string, rootPath: string) => Promise<Library>;
  updateLibrary: (library: Library) => Promise<void>;
  deleteLibrary: (id: string) => Promise<void>;
  setActiveLibrary: (id: string | null) => void;
}

export const useLibraryStore = create<LibraryStore>((set) => ({
  libraries: [],
  activeLibraryId: null,
  isLoading: false,
  error: null,

  fetchLibraries: async () => {
    set({ isLoading: true, error: null });
    try {
      const libraries = await cmd.getLibraries();
      set((s) => ({
        libraries,
        activeLibraryId: s.activeLibraryId ?? (libraries[0]?.id ?? null),
        isLoading: false,
      }));
    } catch (e) {
      set({ error: String(e), isLoading: false });
    }
  },

  ensureActiveLibrary: async () => {
    set({ isLoading: true, error: null });
    try {
      const libraries = await cmd.getLibraries();
      if (libraries.length > 0) {
        const id = libraries[0].id;
        set({ libraries, activeLibraryId: id, isLoading: false });
        return id;
      }

      const created = await cmd.createLibrary("默认知识库", "");
      set({ libraries: [created], activeLibraryId: created.id, isLoading: false });
      return created.id;
    } catch (e) {
      set({ error: String(e), isLoading: false });
      throw e;
    }
  },

  createLibrary: async (name, rootPath) => {
    const library = await cmd.createLibrary(name, rootPath);
    set((s) => ({ libraries: [library, ...s.libraries] }));
    return library;
  },

  updateLibrary: async (library) => {
    await cmd.updateLibrary(library);
    set((s) => ({
      libraries: s.libraries.map((l) => (l.id === library.id ? library : l)),
    }));
  },

  deleteLibrary: async (id) => {
    await cmd.deleteLibrary(id);
    set((s) => ({
      libraries: s.libraries.filter((l) => l.id !== id),
      activeLibraryId: s.activeLibraryId === id ? null : s.activeLibraryId,
    }));
  },

  setActiveLibrary: (id) => set({ activeLibraryId: id }),
}));
