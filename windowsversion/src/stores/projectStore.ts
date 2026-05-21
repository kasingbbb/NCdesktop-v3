import { create } from "zustand";
import type { Project } from "../types";
import * as cmd from "../lib/tauri-commands";

interface ProjectStore {
  projects: Project[];
  activeProjectId: string | null;
  viewMode: "grid" | "list";
  isLoading: boolean;
  error: string | null;

  fetchProjects: (libraryId: string) => Promise<void>;
  createProject: (libraryId: string, name: string) => Promise<Project>;
  updateProject: (project: Project) => Promise<void>;
  deleteProject: (id: string) => Promise<void>;
  setActiveProject: (id: string | null) => void;
  setViewMode: (mode: "grid" | "list") => void;
  getActiveProject: () => Project | undefined;
}

export const useProjectStore = create<ProjectStore>((set, get) => ({
  projects: [],
  activeProjectId: null,
  viewMode: "grid",
  isLoading: false,
  error: null,

  fetchProjects: async (libraryId) => {
    set({ isLoading: true, error: null });
    try {
      const projects = await cmd.getProjects(libraryId);
      set({ projects, isLoading: false });
    } catch (e) {
      set({ error: String(e), isLoading: false });
    }
  },

  createProject: async (libraryId, name) => {
    const project = await cmd.createProject(libraryId, name);
    set((s) => ({ projects: [project, ...s.projects] }));
    return project;
  },

  updateProject: async (project) => {
    await cmd.updateProject(project);
    set((s) => ({
      projects: s.projects.map((p) => (p.id === project.id ? project : p)),
    }));
  },

  deleteProject: async (id) => {
    await cmd.deleteProject(id);
    set((s) => ({
      projects: s.projects.filter((p) => p.id !== id),
      activeProjectId: s.activeProjectId === id ? null : s.activeProjectId,
    }));
  },

  setActiveProject: (id) => {
    set({ activeProjectId: id });
    void cmd.setSetting("ui.active_project_id", id ?? "");
  },

  setViewMode: (mode) => set({ viewMode: mode }),

  getActiveProject: () => {
    const { projects, activeProjectId } = get();
    return projects.find((p) => p.id === activeProjectId);
  },
}));
