/**
 * PR-4 task_014/015/016: Prompt 编辑器 store
 */
import { create } from "zustand";
import * as cmd from "../lib/tauri-commands";

type Kind = cmd.PromptInfo["kind"];

interface DraftState {
  user: string;
  output: string;
}

interface PromptStore {
  byKind: Record<Kind, cmd.PromptInfo | null>;
  drafts: Record<Kind, DraftState>;
  dryRun: Record<Kind, cmd.DryRunOutcome | null>;
  loading: boolean;
  error: string | null;

  load: (kind: Kind) => Promise<void>;
  updateDraft: (kind: Kind, field: "user" | "output", text: string) => void;
  save: (kind: Kind, field: "user" | "output") => Promise<void>;
  testDryRun: (kind: Kind) => Promise<cmd.DryRunOutcome>;
  reset: (kind: Kind, field?: "user" | "output") => Promise<void>;
}

const blankDraft = (): DraftState => ({ user: "", output: "" });

export const usePromptStore = create<PromptStore>((set, get) => ({
  byKind: { classify: null, naming: null, tagging: null },
  drafts: { classify: blankDraft(), naming: blankDraft(), tagging: blankDraft() },
  dryRun: { classify: null, naming: null, tagging: null },
  loading: false,
  error: null,

  load: async (kind) => {
    set({ loading: true, error: null });
    try {
      const info = await cmd.getPrompt(kind);
      set((s) => ({
        byKind: { ...s.byKind, [kind]: info },
        drafts: {
          ...s.drafts,
          [kind]: {
            user: info.overrideText ?? info.defaultText,
            output: info.overrideOutput ?? "",
          },
        },
        loading: false,
      }));
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  updateDraft: (kind, field, text) => {
    set((s) => ({
      drafts: { ...s.drafts, [kind]: { ...s.drafts[kind], [field]: text } },
    }));
  },

  save: async (kind, field) => {
    const draft = get().drafts[kind];
    await cmd.savePrompt(kind, field, draft[field]);
    await get().load(kind);
  },

  testDryRun: async (kind) => {
    const draft = get().drafts[kind].user;
    const outcome = await cmd.dryRunPrompt(kind, draft);
    set((s) => ({ dryRun: { ...s.dryRun, [kind]: outcome } }));
    return outcome;
  },

  reset: async (kind, field) => {
    await cmd.resetPrompt(kind, field);
    await get().load(kind);
  },
}));
