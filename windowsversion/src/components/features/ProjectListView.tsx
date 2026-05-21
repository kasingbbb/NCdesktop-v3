import { useRef, useEffect, useCallback, useState, type MouseEvent } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useProjectStore } from "../../stores/projectStore";
import { useLibraryStore } from "../../stores/libraryStore";
import { ProjectCard } from "./ProjectCard";
import { ProjectListItem } from "./ProjectListItem";
import { EmptyState } from "./EmptyState";
import { logger } from "../../utils/logger";

export function ProjectListView() {
  const {
    projects: items,
    viewMode,
    fetchProjects: fetchItems,
    setActiveProject,
    deleteProject,
  } = useProjectStore();
  const { activeLibraryId, ensureActiveLibrary } = useLibraryStore();
  const parentRef = useRef<HTMLDivElement>(null);
  /** Tauri WKWebView 下 window.confirm 常不弹出或恒为 false，改用应用内确认 */
  const [deleteTarget, setDeleteTarget] = useState<{ id: string; name: string } | null>(null);
  const [deleteBusy, setDeleteBusy] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);

  const openDeleteConfirm = useCallback(
    (e: MouseEvent<HTMLButtonElement>, projectId: string, projectName: string) => {
      e.preventDefault();
      e.stopPropagation();
      setDeleteError(null);
      setDeleteTarget({ id: projectId, name: projectName });
    },
    [],
  );

  const handleConfirmDelete = useCallback(async () => {
    if (!deleteTarget) {
      return;
    }
    setDeleteBusy(true);
    setDeleteError(null);
    try {
      await deleteProject(deleteTarget.id);
      setDeleteTarget(null);
      const libId = activeLibraryId ?? (await ensureActiveLibrary());
      await fetchItems(libId);
    } catch (err) {
      setDeleteError(String(err));
    } finally {
      setDeleteBusy(false);
    }
  }, [activeLibraryId, deleteProject, deleteTarget, ensureActiveLibrary, fetchItems]);

  useEffect(() => {
    (async () => {
      const libId = activeLibraryId ?? (await ensureActiveLibrary());
      logger.info("ProjectListView", "Fetching projects", { libraryId: libId });
      await fetchItems(libId);
    })();
  }, [fetchItems, activeLibraryId, ensureActiveLibrary]);

  const rowVirtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => (viewMode === 'list' ? 64 : 240),
  });

  const deleteModal =
    deleteTarget !== null ? (
      <div
        className="fixed inset-0 z-[300] flex items-center justify-center p-[var(--space-4)]"
        style={{ background: "rgba(0,0,0,0.45)" }}
        role="presentation"
        onClick={() => {
          if (!deleteBusy) {
            setDeleteTarget(null);
          }
        }}
      >
        <div
          className="glass-popover w-full max-w-md p-[var(--space-4)] pointer-events-auto"
          role="dialog"
          aria-modal="true"
          aria-labelledby="delete-project-title"
          onClick={(ev) => ev.stopPropagation()}
        >
          <h2
            id="delete-project-title"
            className="text-[var(--text-base)] font-semibold mb-[var(--space-2)]"
            style={{ color: "var(--text-primary)" }}
          >
            确认删除项目？
          </h2>
          <p className="text-[var(--text-sm)] leading-relaxed mb-[var(--space-3)]" style={{ color: "var(--text-secondary)" }}>
            将删除「{deleteTarget.name}」及其下素材、时间轴等数据，并移除磁盘上的项目资产目录。此操作不可恢复。
          </p>
          {deleteError ? (
            <p className="text-[var(--text-sm)] mb-[var(--space-3)]" style={{ color: "#FF3B30" }}>
              {deleteError}
            </p>
          ) : null}
          <div className="flex justify-end gap-[var(--space-2)]">
            <button
              type="button"
              disabled={deleteBusy}
              className="px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-sm)]"
              style={{ color: "var(--text-secondary)", border: "1px solid var(--border-primary)" }}
              onClick={() => setDeleteTarget(null)}
            >
              取消
            </button>
            <button
              type="button"
              disabled={deleteBusy}
              className="px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-sm)] font-medium"
              style={{ background: "rgba(255, 59, 48, 0.15)", color: "#FF3B30" }}
              onClick={() => void handleConfirmDelete()}
            >
              {deleteBusy ? "删除中…" : "删除"}
            </button>
          </div>
        </div>
      </div>
    ) : null;

  if (items.length === 0) {
    return <EmptyState />;
  }

  // Simplified Grid fallback until virtualized grid is fully implemented
  if (viewMode === 'grid') {
    return (
      <>
        <div ref={parentRef} className="flex-1 overflow-y-auto p-[var(--space-4)]">
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-[var(--space-4)]">
            {items.map((project) => (
              <ProjectCard
                key={project.id}
                project={project}
                onClick={() => setActiveProject(project.id)}
                onDelete={(e) => openDeleteConfirm(e, project.id, project.name)}
              />
            ))}
          </div>
        </div>
        {deleteModal}
      </>
    );
  }

  return (
    <>
      <div ref={parentRef} className="flex-1 overflow-y-auto">
        <div
          style={{
            height: `${rowVirtualizer.getTotalSize()}px`,
            width: "100%",
            position: "relative",
          }}
        >
          {rowVirtualizer.getVirtualItems().map((virtualRow) => {
            const project = items[virtualRow.index];
            return (
              <div
                key={virtualRow.index}
                style={{
                  position: "absolute",
                  top: 0,
                  left: 0,
                  width: "100%",
                  height: `${virtualRow.size}px`,
                  transform: `translateY(${virtualRow.start}px)`,
                }}
              >
                <ProjectListItem
                  project={project}
                  onClick={() => setActiveProject(project.id)}
                  onDelete={(e) => openDeleteConfirm(e, project.id, project.name)}
                />
              </div>
            );
          })}
        </div>
      </div>
      {deleteModal}
    </>
  );
}
