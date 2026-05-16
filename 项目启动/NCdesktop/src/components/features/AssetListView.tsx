import { useCallback, useEffect, useMemo, useState } from "react";
import { FileText, Image, Music, File, FolderOpen, AlertTriangle } from "lucide-react";
import { useAssetStore } from "../../stores/assetStore";
import { useProjectStore } from "../../stores/projectStore";
import { useTagStore } from "../../stores/tagStore";
import { useUIStore } from "../../stores/uiStore";
import { useResizable } from "../../hooks/useResizable";
import { useDragAssets } from "../../hooks/useDragAssets";
import { ResizeHandle } from "../layout/ResizeHandle";
import { WorkspaceFolderStrip } from "./WorkspaceFolderStrip";
import { AssetContextMenu } from "./AssetContextMenu";
import { RenameAssetModal } from "./RenameAssetModal";
import { AssetStateBadge } from "../../lib/asset-state";
import type { Asset, WorkspaceFolderEntry } from "../../types";
import {
  getProjectWorkspaceRoot,
  listProjectWorkspaceFolders,
  revealProjectWorkspaceFolder,
} from "../../lib/tauri-commands";

/** 后端 JSON 为 assetType，与 types/asset 的 type 对齐 */
function assetKind(a: Asset): string {
  const r = a as Asset & { assetType?: string };
  return r.assetType ?? r.type ?? "other";
}

function kindLabel(kind: string): string {
  const map: Record<string, string> = {
    image: "图像",
    photo: "照片",
    audio_clip: "音频",
    markdown: "Markdown",
    scan_text: "扫描文本",
    pdf: "PDF",
    webpage: "网页",
    other: "其他",
  };
  return map[kind] ?? kind;
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

function formatDateLabel(iso: string): string {
  try {
    const d = new Date(iso);
    return `${d.getMonth() + 1}月${d.getDate()}日`;
  } catch {
    return iso;
  }
}

function groupAssetsByDate(assets: Asset[]): { label: string; items: Asset[] }[] {
  const groups: Record<string, Asset[]> = {};
  const order: string[] = [];
  for (const a of assets) {
    const key = formatDateLabel(a.importedAt);
    if (!(key in groups)) {
      groups[key] = [];
      order.push(key);
    }
    groups[key].push(a);
  }
  return order.map((k) => ({ label: k, items: groups[k] }));
}

function formatImportTime(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleString(undefined, {
      month: "numeric",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  } catch {
    return iso;
  }
}

/** AI 整理后的子目录 slug，用于「主题/归类」提示 */
function inferOrganizedCategory(filePath: string): string | null {
  const u = filePath.replace(/\\/g, "/");
  const m = u.match(/\/organized\/([^/]+)\//);
  return m?.[1] ?? null;
}

function originalDisplayName(a: Asset): string {
  return (a.originalName && a.originalName.trim().length > 0 ? a.originalName : a.name).trim();
}

function sourcePathHint(a: Asset): string | undefined {
  const raw = (a as Asset & { sourceData?: string | null }).sourceData;
  if (raw && raw.trim().length > 0) return raw;
  return undefined;
}

function assetIcon(a: Asset, size: number) {
  const t = assetKind(a);
  const color = "var(--text-secondary)";
  if (t === "image" || t === "photo") return <Image size={size} style={{ color }} />;
  if (t === "audio_clip") return <Music size={size} style={{ color }} />;
  if (t === "markdown" || t === "scan_text") return <FileText size={size} style={{ color }} />;
  return <File size={size} style={{ color }} />;
}

/** 按导入时间新→旧（双栏对齐） */
function sortByImportedAtDesc(assets: Asset[]): Asset[] {
  const list = [...assets];
  list.sort(
    (a, b) => new Date(b.importedAt).getTime() - new Date(a.importedAt).getTime()
  );
  return list;
}

/** 判断素材文件是否落在当前工作区子目录（与 Rust 侧路径一致，正斜杠规范化） */
function assetMatchesWorkspaceFolder(
  filePath: string,
  workspaceRoot: string,
  relativePath: string
): boolean {
  const fp = filePath.replace(/\\/g, "/");
  const root = workspaceRoot.replace(/\\/g, "/").replace(/\/$/, "");
  if (!root) {
    return true;
  }
  if (relativePath === "__ROOT__") {
    const last = fp.lastIndexOf("/");
    const parent = last <= 0 ? fp : fp.slice(0, last);
    return parent === root;
  }
  const prefix = `${root}/${relativePath.replace(/^\/+/, "")}`;
  return fp === prefix || fp.startsWith(`${prefix}/`);
}

export function AssetListView() {
  const {
    assets,
    assetTagNamesById,
    isLoading,
    error,
    selectAsset,
    selectedAssetId,
    selectedAssetIds,
    toggleSelectAsset,
    setSelectedAssetIds,
    viewMode,
  } = useAssetStore();
  const activeProject = useProjectStore((s) => s.getActiveProject());
  const assetTagFilterId = useUIStore((s) => s.assetTagFilterId);
  const setAssetTagFilterId = useUIStore((s) => s.setAssetTagFilterId);
  const workspaceFolderRelativePath = useUIStore((s) => s.workspaceFolderRelativePath);
  const setWorkspaceFolderRelativePath = useUIStore(
    (s) => s.setWorkspaceFolderRelativePath
  );
  const tags = useTagStore((s) => s.tags);
  const fetchTags = useTagStore((s) => s.fetchTags);
  const filterTagName = assetTagFilterId
    ? tags.find((t) => t.id === assetTagFilterId)?.name ?? null
    : null;

  const orderedAssets = useMemo(() => sortByImportedAtDesc(assets), [assets]);

  const [workspaceRoot, setWorkspaceRoot] = useState<string>("");
  const [workspaceFolders, setWorkspaceFolders] = useState<WorkspaceFolderEntry[]>([]);
  const [foldersLoading, setFoldersLoading] = useState(false);

  const loadWorkspaceFolders = useCallback(async () => {
    const pid = activeProject?.id;
    if (!pid) {
      setWorkspaceRoot("");
      setWorkspaceFolders([]);
      return;
    }
    setFoldersLoading(true);
    try {
      const [root, list] = await Promise.all([
        getProjectWorkspaceRoot(pid),
        listProjectWorkspaceFolders(pid),
      ]);
      setWorkspaceRoot(root);
      setWorkspaceFolders(list);
    } catch {
      setWorkspaceFolders([]);
    } finally {
      setFoldersLoading(false);
    }
  }, [activeProject?.id]);

  useEffect(() => {
    void loadWorkspaceFolders();
  }, [loadWorkspaceFolders, assets.length]);

  const displayAssets = useMemo(() => {
    if (!workspaceFolderRelativePath || !workspaceRoot) {
      return orderedAssets;
    }
    return orderedAssets.filter((a) =>
      assetMatchesWorkspaceFolder(
        a.filePath,
        workspaceRoot,
        workspaceFolderRelativePath
      )
    );
  }, [orderedAssets, workspaceRoot, workspaceFolderRelativePath]);

  const groupedDisplayAssets = useMemo(
    () => groupAssetsByDate(displayAssets),
    [displayAssets]
  );

  // 文件转换 v1.1：衍生 .md 通过 sourceAssetId 反查原件名，渲染「转换自 xxx」标记
  const assetsById = useMemo(() => {
    const m = new Map<string, typeof displayAssets[number]>();
    for (const a of displayAssets) m.set(a.id, a);
    return m;
  }, [displayAssets]);

  const addNotification = useUIStore((s) => s.addNotification);
  const setInspectorOpen = useUIStore((s) => s.setInspectorOpen);

  const handleRetried = useCallback(
    (assetId: string) => {
      // 触发列表刷新，让新的 state（converting → done/failed）落到 UI 上
      const pid = activeProject?.id;
      if (pid) {
        void useAssetStore.getState().fetchAssets(pid);
      }
      addNotification({
        type: "info",
        title: "已加入重试队列",
        message: `资产 ${assetId.slice(0, 8)}… 将重新转化`,
        duration: 2500,
      });
    },
    [activeProject?.id, addNotification]
  );

  const handleRetryError = useCallback(
    (msg: string) => {
      addNotification({
        type: "error",
        title: "重试失败",
        message: msg,
        duration: 4000,
      });
    },
    [addNotification]
  );

  const folderFilterLabel = workspaceFolderRelativePath
    ? workspaceFolders.find((f) => f.relativePath === workspaceFolderRelativePath)
        ?.displayLabel ?? workspaceFolderRelativePath
    : null;

  const leftPane = useResizable({
    initialWidth: 360,
    minWidth: 260,
    maxWidth: 560,
    direction: "right",
  });

  const [leftPaneFocused, setLeftPaneFocused] = useState(false);
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    assetId: string;
    pane: "left" | "right";
  } | null>(null);

  // task_011 AC-6：rename Modal 状态（替代 window.prompt）。
  const [renameTarget, setRenameTarget] = useState<{
    assetId: string;
    initialName: string;
  } | null>(null);
  const [renameBusy, setRenameBusy] = useState(false);
  // task_011 AC-7：删除确认对话框状态（中文文案，替代 window.confirm）。
  const [deleteTarget, setDeleteTarget] = useState<{ ids: string[] } | null>(null);
  const [deleteBusy, setDeleteBusy] = useState(false);

  const openRenameModal = useCallback(
    (assetId: string) => {
      const current = useAssetStore.getState().assets.find((a) => a.id === assetId);
      if (!current) return;
      setRenameTarget({ assetId, initialName: current.name ?? "" });
    },
    []
  );

  const openDeleteModal = useCallback((ids: string[]) => {
    if (ids.length === 0) return;
    setDeleteTarget({ ids });
  }, []);

  const { makeDragProps } = useDragAssets(selectedAssetIds, assets);

  const handleCardClick = useCallback(
    (e: React.MouseEvent, assetId: string) => {
      if (e.metaKey || e.ctrlKey) {
        toggleSelectAsset(assetId);
      } else {
        selectAsset(assetId);
        setSelectedAssetIds(new Set([assetId]));
        setInspectorOpen(true);
      }
    },
    [selectAsset, toggleSelectAsset, setSelectedAssetIds, setInspectorOpen]
  );

  const handleCardContextMenu = useCallback(
    (e: React.MouseEvent, assetId: string, pane: "left" | "right") => {
      e.preventDefault();
      setContextMenu({ x: e.clientX, y: e.clientY, assetId, pane });
    },
    []
  );

  useEffect(() => {
    if (assetTagFilterId) {
      void fetchTags();
    }
  }, [assetTagFilterId, fetchTags]);

  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement | null)?.tagName;
      // 输入框 / 文本域 / 任何 Modal 内：不拦截
      if (tag === "INPUT" || tag === "TEXTAREA") return;
      if (renameTarget || deleteTarget) return;

      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "a") {
        e.preventDefault();
        const ids = new Set(displayAssets.map((a) => a.id));
        setSelectedAssetIds(ids);
        return;
      }
      // task_011 AC-7：Enter / F2 → 重命名 Modal（仅单选生效）
      if ((e.key === "Enter" || e.key === "F2") && !e.metaKey && !e.ctrlKey) {
        const ids = Array.from(selectedAssetIds);
        if (ids.length === 1) {
          e.preventDefault();
          openRenameModal(ids[0]);
        }
        return;
      }
      // task_011 AC-7：Backspace / Delete → 删除确认（中文）
      if (e.key === "Backspace" || e.key === "Delete") {
        const ids = Array.from(selectedAssetIds);
        if (ids.length > 0) {
          e.preventDefault();
          openDeleteModal(ids);
        }
        return;
      }
    };
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [
    displayAssets,
    leftPaneFocused,
    setSelectedAssetIds,
    selectedAssetIds,
    renameTarget,
    deleteTarget,
    openRenameModal,
    openDeleteModal,
  ]);

  // task_011 AC-6：Modal 提交 → renameAsset；失败用 toast，不再 window.alert
  const handleRenameSubmit = useCallback(
    async (newName: string) => {
      if (!renameTarget) return;
      setRenameBusy(true);
      try {
        await useAssetStore.getState().renameAsset(renameTarget.assetId, newName);
        setRenameTarget(null);
      } catch (err) {
        console.error("[AssetListView] renameAsset failed:", err);
        addNotification({
          type: "error",
          title: "重命名失败",
          message: String(err),
          duration: 4000,
          dedupeKey: "rename_asset:err",
        });
      } finally {
        setRenameBusy(false);
      }
    },
    [renameTarget, addNotification]
  );

  // task_011 AC-7：删除确认 Modal 提交
  const handleDeleteConfirm = useCallback(async () => {
    if (!deleteTarget) return;
    setDeleteBusy(true);
    const ids = deleteTarget.ids;
    try {
      for (const id of ids) {
        await useAssetStore.getState().deleteAsset(id);
      }
      setDeleteTarget(null);
    } catch (err) {
      console.error("[AssetListView] deleteAsset failed:", err);
      addNotification({
        type: "error",
        title: "删除失败",
        message: String(err),
        duration: 4000,
        dedupeKey: "delete_asset:err",
      });
    } finally {
      setDeleteBusy(false);
    }
  }, [deleteTarget, addNotification]);

  if (isLoading) {
    return (
      <div className="flex-1 flex items-center justify-center p-[var(--space-6)]">
        <p className="text-[var(--text-sm)]" style={{ color: "var(--text-tertiary)" }}>
          加载素材中…
        </p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex-1 flex items-center justify-center p-[var(--space-6)]">
        <p className="text-[var(--text-sm)]" style={{ color: "#FF3B30" }}>
          {error}
        </p>
      </div>
    );
  }

  const filterBanner =
    filterTagName ? (
      <div
        className="mb-[var(--space-3)] flex items-center justify-between gap-[var(--space-2)] px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-lg)] text-[var(--text-xs)] shrink-0 border border-app bg-[var(--surface-tertiary)]"
      >
        <span style={{ color: "var(--text-secondary)" }}>
          按标签筛选：<strong className="font-semibold" style={{ color: "var(--text-primary)" }}>{filterTagName}</strong>（{assets.length} 个素材）
        </span>
        <button
          type="button"
          className="shrink-0 px-2 py-1 rounded-[var(--radius-sm)]"
          style={{ color: "var(--text-tertiary)" }}
          onClick={() => setAssetTagFilterId(null)}
        >
          清除
        </button>
      </div>
    ) : null;

  const folderFilterBanner =
    folderFilterLabel && workspaceFolderRelativePath ? (
      <div
        className="mb-[var(--space-3)] flex items-center justify-between gap-[var(--space-2)] px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-xs)] shrink-0"
        style={{ background: "rgba(31,69,110,0.06)", border: "1px solid var(--border-primary)" }}
      >
        <span style={{ color: "var(--text-secondary)" }}>
          文件夹：<strong className="font-semibold" style={{ color: "var(--text-primary)" }}>{folderFilterLabel}</strong>（
          {displayAssets.length} 个素材）
        </span>
        <button
          type="button"
          className="shrink-0 px-2 py-1 rounded-[var(--radius-sm)]"
          style={{ color: "var(--text-tertiary)" }}
          onClick={() => setWorkspaceFolderRelativePath(null)}
        >
          清除
        </button>
      </div>
    ) : null;

  const emptyCopy = (
    <div className="flex-1 flex flex-col items-center justify-center gap-[var(--space-2)] p-[var(--space-6)] min-h-[200px]">
      <p className="text-[var(--text-base)] font-medium" style={{ color: "var(--text-secondary)" }}>
        {assets.length === 0
          ? "该项目暂无素材"
          : workspaceFolderRelativePath
            ? "当前文件夹筛选下暂无素材"
            : "暂无素材"}
      </p>
      <p className="text-[var(--text-sm)] text-center max-w-md" style={{ color: "var(--text-tertiary)" }}>
        拖入文件会<strong style={{ color: "var(--text-secondary)" }}>复制</strong>到「下载」文件夹下的{" "}
        <code className="text-[11px]">NoteCaptWorkPlace</code> 中本项目目录，原件不会被修改。当前项目：「
        {activeProject?.name ?? "…"}」。
      </p>
    </div>
  );

  return (
    <div className="flex-1 min-h-0 flex flex-col overflow-hidden p-[var(--space-4)]">
      {filterBanner}
      {folderFilterBanner}

      <WorkspaceFolderStrip
        folders={workspaceFolders}
        workspaceRootHint={workspaceRoot}
        selectedRelativePath={workspaceFolderRelativePath}
        loading={foldersLoading}
        onSelect={(path) => setWorkspaceFolderRelativePath(path)}
        onReveal={(relativePath) => {
          const pid = activeProject?.id;
          if (!pid) {
            return;
          }
          void revealProjectWorkspaceFolder(pid, relativePath).catch(() => {
            /* 非 Tauri 环境或路径不存在 */
          });
        }}
        onRefresh={() => void loadWorkspaceFolders()}
      />

      {displayAssets.length === 0 ? (
        emptyCopy
      ) : (
      <div
        className="flex flex-1 min-h-0 gap-0 overflow-hidden rounded-[var(--radius-xl)] border border-app bg-[var(--surface-primary)]"
        style={{ boxShadow: "var(--shadow-float)" }}
      >
        {/* 左：导入原件 */}
        <div
          className="flex flex-col min-h-0 min-w-0 border-r shrink-0"
          style={{ width: leftPane.width, borderColor: "var(--border-primary)" }}
          onMouseEnter={() => setLeftPaneFocused(true)}
          onMouseLeave={() => setLeftPaneFocused(false)}
        >
          <div className="px-3 py-2 border-b shrink-0 border-app bg-[var(--surface-tertiary)]">
            <p className="text-[var(--text-sm)] font-semibold" style={{ color: "var(--text-primary)" }}>
              导入原件
            </p>
            <p className="text-[10px] mt-0.5" style={{ color: "var(--text-tertiary)" }}>
              拖入时的文件名 · 按导入时间新→旧 · 与访达原件一致
            </p>
          </div>
          <div className="flex-1 min-h-0 overflow-y-auto bg-[var(--surface-primary)]">
            {viewMode === "list" ? (
              <ul className="flex flex-col gap-1 p-2">
                {displayAssets.map((a) => {
                  const active = selectedAssetId === a.id;
                  const hint = sourcePathHint(a);
                  return (
                    <li key={a.id}>
                      <button
                        type="button"
                        onClick={(e) => handleCardClick(e, a.id)}
                        onContextMenu={(e) => handleCardContextMenu(e, a.id, "left")}
                        {...makeDragProps(a.id)}
                        className="w-full text-left px-2.5 py-1.5 flex items-start gap-1.5 rounded-[var(--radius-md)] border border-app transition-colors hover:border-[var(--border-hover)] focus:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--border-active)] bg-[var(--surface-primary)]"
                        style={{
                          background: selectedAssetIds.has(a.id)
                            ? "var(--brand-navy-10)"
                            : active
                            ? "var(--sidebar-active-bg)"
                            : undefined,
                          outline: selectedAssetIds.has(a.id)
                            ? "2px solid var(--brand-navy)"
                            : undefined,
                        }}
                        title={hint ? `原件路径：${hint}` : originalDisplayName(a)}
                      >
                        <span className="shrink-0 mt-0.5">{assetIcon(a, 18)}</span>
                        <span className="min-w-0 flex-1">
                          <span className="text-[var(--text-sm)] font-medium truncate block leading-[1.3]" style={{ color: "var(--text-primary)" }} title={originalDisplayName(a)}>
                            {originalDisplayName(a)}
                          </span>
                          <span className="text-[9.5px] font-mono tabular-nums mt-0.5 block" style={{ color: "var(--text-secondary)" }}>
                            导入 {formatImportTime(a.importedAt)}
                          </span>
                        </span>
                      </button>
                    </li>
                  );
                })}
              </ul>
            ) : (
              <div className="p-2 grid grid-cols-2 gap-2">
                {displayAssets.map((a) => {
                  const active = selectedAssetId === a.id;
                  const hint = sourcePathHint(a);
                  return (
                    <button
                      key={a.id}
                      type="button"
                      onClick={(e) => handleCardClick(e, a.id)}
                      onContextMenu={(e) => handleCardContextMenu(e, a.id, "left")}
                      {...makeDragProps(a.id)}
                      className="flex flex-col items-center gap-1.5 rounded-[var(--radius-md)] border border-app p-2 transition-colors hover:border-[var(--border-hover)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--border-active)] bg-[var(--surface-primary)]"
                      style={{
                        background: selectedAssetIds.has(a.id)
                          ? "var(--brand-navy-10)"
                          : active
                          ? "var(--sidebar-active-bg)"
                          : undefined,
                        outline: selectedAssetIds.has(a.id)
                          ? "2px solid var(--brand-navy)"
                          : undefined,
                      }}
                      title={hint ? `原件：${hint}` : undefined}
                    >
                      <div className="w-14 h-14 rounded-[var(--radius-md)] flex items-center justify-center shrink-0 bg-[var(--surface-tertiary)]">
                        {assetIcon(a, 22)}
                      </div>
                      <p className="w-full text-[11px] font-medium truncate text-center leading-snug" style={{ color: "var(--text-primary)" }} title={originalDisplayName(a)}>
                        {originalDisplayName(a)}
                      </p>
                    </button>
                  );
                })}
              </div>
            )}
          </div>
        </div>

        <ResizeHandle onMouseDown={leftPane.handleMouseDown} isResizing={leftPane.isResizing} />

        {/* 右：工作区（重命名 + 标签 + 归类目录） */}
        <div className="flex-1 min-w-0 flex flex-col min-h-0">
          <div className="px-3 py-2 border-b shrink-0 border-app bg-[var(--surface-tertiary)] flex items-start justify-between">
            <div>
              <p className="text-[var(--text-sm)] font-semibold" style={{ color: "var(--text-primary)" }}>
                工作区
              </p>
              <p className="text-[10px] mt-0.5" style={{ color: "var(--text-tertiary)" }}>
                AI 整理后的 .md 文件 · 可拖拽到 Claude / ChatGPT
              </p>
            </div>
            {selectedAssetId && (
              <span
                className="text-[10px] inline-flex items-center gap-1 px-1.5 py-0.5 rounded-[var(--radius-full)]"
                style={{
                  color: "var(--color-accent-dark)",
                  background: "var(--color-accent-soft)",
                }}
              >
                已定位对应文件 ↑
              </span>
            )}
          </div>
          <div className="flex-1 min-h-0 overflow-y-auto bg-[var(--surface-primary)]">
            {viewMode === "list" ? (
              <div className="flex flex-col p-2 gap-2">
                {groupedDisplayAssets.map((group) => (
                  <div key={group.label}>
                    {/* 日期分组头 */}
                    <div className="flex items-center justify-between px-1 mb-1.5">
                      <span
                        className="text-[11px] font-semibold"
                        style={{ color: "var(--text-secondary)" }}
                      >
                        {group.label}{activeProject ? ` ${activeProject.name}` : ""}
                      </span>
                      <span
                        className="text-[10px] tabular-nums"
                        style={{ color: "var(--text-tertiary)" }}
                      >
                        {group.items.length} 个 ↑
                      </span>
                    </div>
                    <ul className="flex flex-col gap-1">
                      {group.items.map((a) => {
                        const active = selectedAssetId === a.id;
                        const tagNames = assetTagNamesById[a.id] ?? [];
                        const cat = inferOrganizedCategory(a.filePath);
                        const renamed = a.name.trim() !== originalDisplayName(a).trim();
                        const state = a.state;
                        // task_011 AC-2 / AC-4
                        const sourceMissing = a.sourceMissing === true;
                        return (
                          <li
                            key={a.id}
                            data-asset-id={a.id}
                            data-state={state ?? "unknown"}
                            data-source-missing={sourceMissing ? "true" : "false"}
                          >
                            <button
                              type="button"
                              onClick={(e) => handleCardClick(e, a.id)}
                              onContextMenu={(e) => handleCardContextMenu(e, a.id, "right")}
                              {...makeDragProps(a.id)}
                              className="w-full text-left px-2.5 py-1.5 flex items-start gap-1.5 rounded-[var(--radius-md)] border border-app transition-colors hover:border-[var(--border-hover)] focus:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--border-active)] bg-[var(--surface-primary)]"
                              style={{
                                background: selectedAssetIds.has(a.id)
                                  ? "var(--brand-navy-10)"
                                  : active
                                  ? "var(--sidebar-active-bg)"
                                  : undefined,
                                outline: selectedAssetIds.has(a.id)
                                  ? "2px solid var(--brand-navy)"
                                  : undefined,
                              }}
                            >
                              <span className="shrink-0 mt-0.5">{assetIcon(a, 18)}</span>
                              <span className="min-w-0 flex-1">
                                <span className="flex items-center justify-between gap-2 min-w-0">
                                  <span className="text-[var(--text-md)] font-medium truncate flex-1 min-w-0 leading-[1.3]" style={{ color: "var(--text-primary)" }} title={a.name}>
                                    {a.name}
                                  </span>
                                  {/* task_011 AC-2：源文件缺失角标 */}
                                  {sourceMissing ? (
                                    <span
                                      data-testid="source-missing-badge"
                                      className="shrink-0 inline-flex items-center gap-0.5 text-[10px] font-medium px-1.5 py-0.5 rounded-[var(--radius-md)]"
                                      style={{
                                        color: "#b45309",
                                        background: "rgba(245,158,11,0.14)",
                                        border: "1px solid rgba(245,158,11,0.35)",
                                      }}
                                      title="源文件不在原位置，rendition 仍可拖出"
                                    >
                                      <AlertTriangle size={10} aria-hidden />
                                      <span>原件丢失</span>
                                    </span>
                                  ) : null}
                                  {renamed ? (
                                    <span className="shrink-0 text-[10px] font-normal px-1.5 py-0.5 rounded-[var(--radius-md)] bg-[var(--surface-tertiary)]" style={{ color: "var(--text-secondary)" }}>
                                      已重命名
                                    </span>
                                  ) : null}
                                  {state ? (
                                    <span className="shrink-0">
                                      <AssetStateBadge
                                        state={state}
                                        assetId={a.id}
                                        reason={a.stateReason ?? null}
                                        extractorType={
                                          (a as typeof a & {
                                            extractorType?: string | null;
                                          }).extractorType ?? null
                                        }
                                        failureCode={
                                          (a as typeof a & {
                                            extractionFailureCode?: string | null;
                                          }).extractionFailureCode ?? null
                                        }
                                        onRetry={() => handleRetried(a.id)}
                                        onError={handleRetryError}
                                      />
                                    </span>
                                  ) : null}
                                </span>
                                <span className="flex flex-wrap items-center gap-1.5 mt-1">
                                  {cat ? (
                                    <span className="inline-flex items-center gap-0.5 text-[10px] px-1.5 py-0.5 rounded-[var(--radius-full)] bg-[var(--color-accent-soft)] border border-app" style={{ color: "var(--text-secondary)" }}>
                                      <FolderOpen size={10} />
                                      {cat}
                                    </span>
                                  ) : null}
                                  {tagNames.slice(0, 3).map((tn) => (
                                    <span
                                      key={tn}
                                      className="tag-pill !text-[10px] !px-1.5 !py-0.5"
                                    >
                                      {tn}
                                    </span>
                                  ))}
                                  {tagNames.length > 3 && (
                                    <span
                                      className="text-[10px] px-1 py-0.5 rounded-[var(--radius-full)]"
                                      style={{ color: "var(--text-tertiary)" }}
                                    >
                                      +{tagNames.length - 3}
                                    </span>
                                  )}
                                </span>
                                <span className="text-[9.5px] font-mono tabular-nums mt-1 block truncate" style={{ color: "var(--text-secondary)" }} title={a.filePath}>
                                  {kindLabel(assetKind(a))} · {formatBytes(a.fileSize)} · {formatImportTime(a.importedAt)}
                                </span>
                                {(() => {
                                  const sourceId = a.sourceAssetId;
                                  const source = sourceId ? assetsById.get(sourceId) : null;
                                  const label = source
                                    ? `转换自 ${originalDisplayName(source)}`
                                    : "来源：1 个原件";
                                  return (
                                    <span
                                      className="text-[9.5px] mt-0.5 block truncate"
                                      style={{ color: "var(--text-tertiary)" }}
                                      title={label}
                                    >
                                      {label}
                                    </span>
                                  );
                                })()}
                              </span>
                            </button>
                          </li>
                        );
                      })}
                    </ul>
                  </div>
                ))}
              </div>
            ) : (
              <div className="p-2 grid grid-cols-2 sm:grid-cols-3 gap-2">
                {displayAssets.map((a) => {
                  const active = selectedAssetId === a.id;
                  const tagNames = assetTagNamesById[a.id] ?? [];
                  const cat = inferOrganizedCategory(a.filePath);
                  const renamed = a.name.trim() !== originalDisplayName(a).trim();
                  return (
                    <button
                      key={a.id}
                      type="button"
                      onClick={(e) => handleCardClick(e, a.id)}
                      onContextMenu={(e) => handleCardContextMenu(e, a.id, "right")}
                      {...makeDragProps(a.id)}
                      className="flex flex-col items-center gap-1 rounded-[var(--radius-md)] border border-app p-2 transition-colors hover:border-[var(--border-hover)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--border-active)] bg-[var(--surface-primary)]"
                      style={{
                        background: selectedAssetIds.has(a.id)
                          ? "var(--brand-navy-10)"
                          : active
                          ? "var(--sidebar-active-bg)"
                          : undefined,
                        outline: selectedAssetIds.has(a.id)
                          ? "2px solid var(--brand-navy)"
                          : undefined,
                      }}
                    >
                      <div className="w-14 h-14 rounded-[var(--radius-md)] flex items-center justify-center shrink-0 bg-[var(--surface-tertiary)]">
                        {assetIcon(a, 22)}
                      </div>
                      <p className="w-full text-[11px] font-medium truncate text-center leading-snug" style={{ color: "var(--text-primary)" }} title={a.name}>
                        {a.name}
                      </p>
                      {renamed ? (
                        <span className="text-[9px]" style={{ color: "var(--text-secondary)" }}>
                          已重命名
                        </span>
                      ) : null}
                      {cat ? (
                        <span className="text-[9px] line-clamp-1 w-full text-center" style={{ color: "var(--text-tertiary)" }}>
                          {cat}
                        </span>
                      ) : null}
                      {tagNames.length > 0 ? (
                        <span className="text-[9px] line-clamp-1 w-full text-center" style={{ color: "var(--text-tertiary)" }}>
                          {tagNames.slice(0, 2).join(" · ")}
                          {tagNames.length > 2 ? "…" : ""}
                        </span>
                      ) : null}
                    </button>
                  );
                })}
              </div>
            )}
          </div>
        </div>
      </div>
      )}

      {contextMenu && activeProject ? (() => {
        const target = assets.find((a) => a.id === contextMenu.assetId);
        const srcRaw = (target as (Asset & { sourceData?: string | null }) | undefined)?.sourceData;
        return (
          <AssetContextMenu
            x={contextMenu.x}
            y={contextMenu.y}
            assetId={contextMenu.assetId}
            pane={contextMenu.pane}
            selectedAssetIds={selectedAssetIds}
            workspaceFolders={workspaceFolders}
            projectId={activeProject.id}
            currentFilePath={target?.filePath ?? ""}
            sourcePath={srcRaw ?? null}
            sourceMissing={target?.sourceMissing === true}
            onRequestRename={(id) => openRenameModal(id)}
            onClose={() => setContextMenu(null)}
            onMoved={() => {
              void loadWorkspaceFolders();
              void useAssetStore.getState().fetchAssets(activeProject.id);
            }}
          />
        );
      })() : null}

      {/* task_011 AC-6：rename Modal */}
      {renameTarget ? (
        <RenameAssetModal
          initialName={renameTarget.initialName}
          busy={renameBusy}
          onCancel={() => {
            if (!renameBusy) setRenameTarget(null);
          }}
          onSubmit={(name) => void handleRenameSubmit(name)}
        />
      ) : null}

      {/* task_011 AC-7：删除确认 Modal（中文） */}
      {deleteTarget ? (
        <div
          data-testid="asset-delete-modal"
          role="dialog"
          aria-modal="true"
          style={{
            position: "fixed",
            inset: 0,
            background: "rgba(0,0,0,0.35)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            zIndex: 9998,
          }}
          onClick={() => {
            if (!deleteBusy) setDeleteTarget(null);
          }}
        >
          <div
            onClick={(e) => e.stopPropagation()}
            style={{
              minWidth: 360,
              maxWidth: 480,
              background: "var(--surface-primary)",
              border: "1px solid var(--border-primary)",
              borderRadius: 8,
              boxShadow: "0 8px 32px rgba(0,0,0,0.2)",
              padding: 20,
              color: "var(--text-primary)",
            }}
          >
            <div style={{ fontSize: 14, marginBottom: 16, lineHeight: 1.5 }}>
              {deleteTarget.ids.length === 1
                ? "确认删除此文件？此操作不可撤销。"
                : `确认删除选中的 ${deleteTarget.ids.length} 个文件？此操作不可撤销。`}
            </div>
            <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
              <button
                type="button"
                data-testid="asset-delete-cancel"
                disabled={deleteBusy}
                onClick={() => setDeleteTarget(null)}
                className="text-[13px] px-3 py-1 rounded-[var(--radius-sm)]"
                style={{
                  border: "1px solid var(--border-primary)",
                  background: "var(--surface-primary)",
                  color: "var(--text-primary)",
                }}
              >
                取消
              </button>
              <button
                type="button"
                data-testid="asset-delete-confirm"
                disabled={deleteBusy}
                onClick={() => void handleDeleteConfirm()}
                className="text-[13px] px-3 py-1 rounded-[var(--radius-sm)]"
                style={{
                  background: "#ef4444",
                  color: "#fff",
                  border: "none",
                  opacity: deleteBusy ? 0.6 : 1,
                }}
              >
                {deleteBusy ? "删除中…" : "删除"}
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}
