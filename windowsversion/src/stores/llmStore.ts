import { create } from 'zustand';
import type { ChatMessage, LLMConfig } from '../types';

interface LLMStore {
  history: ChatMessage[];
  config: LLMConfig | null;
  isGenerating: boolean;
  error: string | null;

  sendMessage: (content: string) => Promise<void>;
  clearHistory: () => void;
  updateConfig: (config: Partial<LLMConfig>) => void;
}

export const useLLMStore = create<LLMStore>((set) => ({
  history: [],
  config: null,
  isGenerating: false,
  error: null,

  sendMessage: async () => {
    set({ isGenerating: true, error: null });
    try {
      // Mock LLM call
      set({ isGenerating: false });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : 'Unknown error', isGenerating: false });
    }
  },
  clearHistory: () => set({ history: [], error: null }),
  updateConfig: (config) => set((state) => ({ 
    config: state.config ? { ...state.config, ...config } : config as LLMConfig 
  })),
}));
