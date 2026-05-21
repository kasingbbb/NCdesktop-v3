import { useCallback, useEffect, useState } from "react";
import { ChevronDown, ChevronRight, ExternalLink, FolderOpen } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { SidebarSection } from "../layout/SidebarItem";
import { useProjectStore } from "../../stores/projectStore";
import { useUIStore } from "../../stores/uiStore";
import {
  listProjectWorkspaceFolders,
  revealProjectWorkspaceFolder,
} from "../../lib/tauri-commands";
import type { WorkspaceFolderEntry } from "../../types";
import { workspaceFolderKindBadge } from "../../lib/workspace-folder-badges";

export function ProjectTree() {
  const { projects, activeProjectId, setActiveProject } = useProjectStore();
  const workspaceFolderRelativePath = useUIStore((s) => s.workspaceFolderRelativePath);
  const setWorkspaceFolderRelativePath = useUIStore(
    (s) => s.setWorkspaceFolderRelativePath
  );
  const setSidebarSection = useUIStore((s) => s.setSidebarSection);

  const [expandedIds, setExpandedIds] = useState<string[]>([]);
  const [foldersByProject, setFoldersByProject] = useState<
    Record<string, WorkspaceFolderEntry[]>
  >({});
  const [loadingProjectId, setLoadingProjectId] = useState<string | null>(null);
  const [folderRefreshTick, setFolderRefreshTick] = useState(0);

  const loadFolders = useCallback(async (projectId: string) => {
    setLoadingProjectId(projectId);
    try {
      const list = await listProjectWorkspaceFolders(projectId);
      setFoldersByProject((prev) => ({ ...prev, [projectId]: list }));
    } catch {
      setFoldersByProject((prev) => ({ ...prev, [projectId]: [] }));
    } finally {
      setLoadingProjectId((cur) => (cur === projectId ? null : cur));
    }
  }, []);

  const toggleExpand = useCallback(
    (projectId: string) => {
      setExpandedIds((prev) => {
        if (prev.includes(projectId)) {
          return prev.filter((id) => id !== projectId);
        }
        return [...prev, projectId];
      });
    },
    []
  );

  useEffect(() => {
    expandedIds.forEach((id) => {
      void loadFolders(id);
    });
  }, [expandedIds, folderRefreshTick, loadFolders]);

  useEffect(() => {
    let cancelled = false;
    let unlistenImport: (() => void) | undefined;
    let unlistenAi: (() => void) | undefined;

    void listen("notecapt/import-drop-finished", () => {
      if (!cancelled) setFolderRefreshTick((t) => t + 1);
    }).then((fn) => {
      if (!cancelled) unlistenImport = fn;
    });

    void listen<{ projectId: string }>("notecapt/dropzone-ai-finished", () => {
      if (!cancelled) setFolderRefreshTick((t) => t + 1);
    }).then((fn) => {
      if (!cancelled) unlistenAi = fn;
    });

    return () => {
      cancelled = true;
      unlistenImport?.();
      unlistenAi?.();
    };
  }, []);

  const openProject = useCallback(
    (projectId: string, folderPath: string | null) => {
      setActiveProject(projectId);
      setWorkspaceFolderRelativePath(folderPath);
      setSidebarSection("projects");
    },
    [setActiveProject, setWorkspaceFolderRelativePath, setSidebarSection]
  );

  return (
    <SidebarSection title="Projects">
      {projects.length === 0 ? (
        <div className="px-5 py-2 text-[var(--text-xs)]" style={{ color: "var(--text-tertiary)" }}>
          暂无项目
        </div>
      ) : (
        projects.map((project) => {
          const expanded = expandedIds.includes(project.id);
          const folders = foldersByProject[project.id] ?? [];
          const isLoading = loadingProjectId === project.id;
          const projectHighlighted = activeProjectId === project.id;

          return (
            <div key={project.id} className="mb-1">
              <div className="flex items-stretch gap-0.5 pr-1">
                <button
                  type="button"
                  className="shrink-0 w-6 flex items-center justify-center rounded-[var(--radius-sm)]"
                  style={{ color: "var(--text-tertiary)" }}
                  aria-expanded={expanded}
                  title={expanded ? "收起子文件夹" : "展开工作区子文件夹"}
                  onClick={(e) => {
                    e.stopPropagation();
                    toggleExpand(project.id);
                  }}
                >
                  {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                </button>
                <button
                  type="button"
                  className={`sidebar-item flex-1 min-w-0 flex items-center mb-0 ${
                    projectHighlighted ? "active" : ""
                  }`}
                  onClick={() => openProject(project.id, null)}
                >
                  <span className="sidebar-item-icon mr-2 shrink-0">
                    <FolderOpen size={16} />
                  </span>
                  <span className="flex-1 truncate text-left">
                    {project.name || "Untitled Project"}
                  </span>
                </button>
              </div>
              {expanded ? (
                <div
                  className="pl-7 mt-0.5 mb-1 space-y-0.5 border-l ml-3"
                  style={{ borderColor: "var(--border-primary)" }}
                >
                  {isLoading && folders.length === 0 ? (
                    <span className="text-[10px] px-2" style={{ color: "var(--text-tertiary)" }}>
                      加载中…
                    </span>
                  ) : null}
                  {folders.map((f) => {
                    const folderActive =
                      projectHighlighted && workspaceFolderRelativePath === f.relativePath;
                    const badge = workspaceFolderKindBadge(f.kind);
                    return (
                      <div key={f.relativePath} className="flex items-center gap-0.5 group">
                        <button
                          type="button"
                          className={`flex-1 text-left text-[11px] px-2 py-1 rounded-[var(--radius-md)] truncate sidebar-item mb-0 ${
                            folderActive ? "active" : ""
                          }`}
                          style={{
                            color: folderActive ? "var(--sidebar-active-fg)" : "var(--text-secondary)",
                          }}
                          onClick={() => openProject(project.id, f.relativePath)}
                        >
                          {badge ? <span className="opacity-60 mr-1">{badge}</span> : null}
                          {f.displayLabel}
                        </button>
                        <button
                          type="button"
                          className="p-0.5 opacity-70 hover:opacity-100 shrink-0"
                          style={{ color: "var(--text-tertiary)" }}
                          title="在访达中打开"
                          onClick={(e) => {
                            e.stopPropagation();
                            void revealProjectWorkspaceFolder(project.id, f.relativePath).catch(
                              () => {
                                /* 忽略 */
                              }
                            );
                          }}
                        >
                          <ExternalLink size={12} />
                        </button>
                      </div>
                    );
                  })}
                </div>
              ) : null}
            </div>
          );
        })
      )}
    </SidebarSection>
  );
}
