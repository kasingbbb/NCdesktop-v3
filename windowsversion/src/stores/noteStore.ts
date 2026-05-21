import { create } from "zustand";
import type { Note } from "../types";
import * as cmd from "../lib/tauri-commands";

interface NoteStore {
  notes: Note[];
  selectedNoteId: string | null;
  isLoading: boolean;
  error: string | null;

  fetchNotes: (projectId: string) => Promise<void>;
  createNote: (params: {
    projectId: string;
    content: string;
    assetId?: string;
    timelineTime?: number;
  }) => Promise<Note>;
  updateNote: (id: string, content: string) => Promise<void>;
  deleteNote: (id: string) => Promise<void>;
  selectNote: (id: string | null) => void;
}

export const useNoteStore = create<NoteStore>((set) => ({
  notes: [],
  selectedNoteId: null,
  isLoading: false,
  error: null,

  fetchNotes: async (projectId) => {
    set({ isLoading: true, error: null });
    try {
      const notes = await cmd.getNotes(projectId);
      set({ notes, isLoading: false });
    } catch (e) {
      set({ error: String(e), isLoading: false });
    }
  },

  createNote: async (params) => {
    const note = await cmd.createNote(params);
    set((s) => ({ notes: [note, ...s.notes] }));
    return note;
  },

  updateNote: async (id, content) => {
    await cmd.updateNote(id, content);
    set((s) => ({
      notes: s.notes.map((n) =>
        n.id === id ? { ...n, content, updatedAt: new Date().toISOString() } : n
      ),
    }));
  },

  deleteNote: async (id) => {
    await cmd.deleteNote(id);
    set((s) => ({
      notes: s.notes.filter((n) => n.id !== id),
      selectedNoteId: s.selectedNoteId === id ? null : s.selectedNoteId,
    }));
  },

  selectNote: (id) => set({ selectedNoteId: id }),
}));
