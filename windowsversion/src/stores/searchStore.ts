import { create } from "zustand";
import type { SearchResult } from "../types";
import * as cmd from "../lib/tauri-commands";

interface SearchStore {
  query: string;
  results: SearchResult[];
  isSearching: boolean;
  hasSearched: boolean;

  setQuery: (query: string) => void;
  search: (query: string) => Promise<void>;
  performSearch: (query: string) => Promise<SearchResult[]>;
  clearSearch: () => void;
}

export const useSearchStore = create<SearchStore>((set) => ({
  query: "",
  results: [],
  isSearching: false,
  hasSearched: false,

  setQuery: (query) => set({ query }),

  search: async (query) => {
    if (!query.trim()) {
      set({ results: [], hasSearched: false });
      return;
    }
    set({ isSearching: true, query });
    try {
      const results = await cmd.searchAll(query, 50);
      set({ results, isSearching: false, hasSearched: true });
    } catch (e) {
      console.error("搜索失败:", e);
      set({ results: [], isSearching: false, hasSearched: true });
    }
  },

  performSearch: async (query) => {
    if (!query.trim()) return [];
    set({ isSearching: true, query });
    try {
      const results = await cmd.searchAll(query, 50);
      set({ results, isSearching: false, hasSearched: true });
      return results;
    } catch (e) {
      console.error("搜索失败:", e);
      set({ results: [], isSearching: false, hasSearched: true });
      return [];
    }
  },

  clearSearch: () => set({ query: "", results: [], hasSearched: false }),
}));
