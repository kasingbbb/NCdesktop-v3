import { create } from 'zustand';
import type { SyncProgress } from '../types';

interface SyncStore {
  isTFCardConnected: boolean;
  isSyncing: boolean;
  progress: SyncProgress | null;
  error: string | null;

  setTFCardConnected: (connected: boolean) => void;
  startSync: () => Promise<void>;
  cancelSync: () => Promise<void>;
}

export const useSyncStore = create<SyncStore>((set) => ({
  isTFCardConnected: false,
  isSyncing: false,
  progress: null,
  error: null,

  setTFCardConnected: (connected) => set({ isTFCardConnected: connected }),
  startSync: async () => {
    set({ isSyncing: true, error: null });
    // mock sync
  },
  cancelSync: async () => {
    set({ isSyncing: false });
  },
}));
