import { useEffect, useState } from "react";
import { X, FileText, Image, Music, FolderOpen, ExternalLink } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { useDropzoneStore } from "../../../stores/dropzoneStore";
import {
  getSetting,
  listProjectWorkspaceFolders,
  revealProjectWorkspaceFolder,
} from "../../../lib/tauri-commands";
import type { WorkspaceFolderEntry } from "../../../types";
import { workspaceFolderKindBadge } from "../../../lib/workspace-folder-badges";

export function DropzoneExpanded() {
  const recentItems = useDropzoneStore((s) => s.recentItems);
  const toggleExpand = useDropzoneStore((s) => s.toggleExpand);
  const [workspaceFolders, setWorkspaceFolders] = useState<WorkspaceFolderEntry[]>([]);
  const [workspaceProjectId, setWorkspaceProjectId] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    const load = async (): Promise<void> => {
      try {
        const id = (await getSetting("ui.active_project_id"))?.trim() ?? "";
        if (!id || cancelled) {
          setWorkspaceProjectId(null);
          setWorkspaceFolders([]);
          return;
        }
        setWorkspaceProjectId(id);
        const list = await listProjectWorkspaceFolders(id);
        if (!cancelled) {
          setWorkspaceFolders(list);
        }
      } catch {
        if (!cancelled) {
          setWorkspaceFolders([]);
        }
      }
    };
    void load();

    let unlistenImport: (() => void) | undefined;
    let unlistenAi: (() => void) | undefined;
    void listen("notecapt/import-drop-finished", () => {
      void load();
    }).then((fn) => {
      unlistenImport = fn;
    });
    void listen("notecapt/dropzone-ai-finished", () => {
      void load();
    }).then((fn) => {
      unlistenAi = fn;
    });

    return () => {
      cancelled = true;
      unlistenImport?.();
      unlistenAi?.();
    };
  }, [recentItems.length]);

  return (
    <div
      className="flex flex-col w-full h-full min-h-0 overflow-hidden pointer-events-auto"
      style={{
        borderRadius: 22,
        padding: "var(--space-3)",
        background: "#1a2233",
        border: "1px solid #2d3a50",
        boxShadow: "0 8px 32px rgba(0,0,0,0.4)",
        animation: "glass-modal-enter var(--duration-normal) var(--ease-out-expo)",
      }}
    >
      {/* 头部 */}
      <div className="flex items-center justify-between mb-[var(--space-2)]">
        <span className="text-[var(--text-sm)] font-medium" style={{ color: "rgba(255,255,255,0.9)" }}>最近导入</span>
        <button
          onClick={toggleExpand}
          className="flex items-center justify-center transition-colors"
          style={{
            width: 24,
            height: 24,
            borderRadius: "var(--radius-sm)",
            color: "rgba(255,255,255,0.4)",
            background: "transparent",
            border: "none",
            cursor: "pointer",
          }}
          onMouseEnter={(e) => {
            (e.currentTarget as HTMLElement).style.background = "rgba(255,255,255,0.1)";
          }}
          onMouseLeave={(e) => {
            (e.currentTarget as HTMLElement).style.background = "transparent";
          }}
        >
          <X size={12} />
        </button>
      </div>

      {/* 当前项目工作区子文件夹 */}
      {workspaceProjectId && workspaceFolders.length > 0 ? (
        <div className="mb-[var(--space-2)] shrink-0">
          <div className="flex items-center gap-1.5 mb-1">
            <FolderOpen size={12} style={{ color: "#3b82f6" }} />
            <span className="text-[10px] font-medium" style={{ color: "rgba(255,255,255,0.5)" }}>
              导入目录子文件夹
            </span>
          </div>
          <div className="flex flex-wrap gap-1 max-h-[72px] overflow-y-auto">
            {workspaceFolders.map((f) => {
              const badge = workspaceFolderKindBadge(f.kind);
              return (
                <div key={f.relativePath} className="inline-flex items-center gap-0.5">
                  <span
                    className="text-[9px] px-1.5 py-0.5 rounded-md truncate max-w-[140px]"
                    style={{
                      background: "rgba(255,255,255,0.06)",
                      color: "rgba(255,255,255,0.6)",
                      border: "1px solid #2d3a50",
                    }}
                    title={f.relativePath}
                  >
                    {badge ? <span className="opacity-60 mr-0.5">{badge}</span> : null}
                    {f.displayLabel}
                  </span>
                  <button
                    type="button"
                    className="p-0.5 shrink-0"
                    style={{ color: "rgba(255,255,255,0.35)", background: "transparent", border: "none", cursor: "pointer" }}
                    title="在访达中打开"
                    onClick={() => {
                      void revealProjectWorkspaceFolder(workspaceProjectId, f.relativePath).catch(
                        () => {
                          /* 忽略 */
                        }
                      );
                    }}
                  >
                    <ExternalLink size={10} />
                  </button>
                </div>
              );
            })}
          </div>
        </div>
      ) : null}

      {/* 列表 */}
      {recentItems.length === 0 ? (
        <div
          className="flex items-center justify-center py-[var(--space-4)]"
        >
          <span className="text-[var(--text-xs)]" style={{ color: "rgba(255,255,255,0.35)" }}>暂无导入记录</span>
        </div>
      ) : (
        <div className="flex flex-col gap-[var(--space-1)] overflow-y-auto flex-1 min-h-0">
          {recentItems.slice(0, 5).map((item) => (
            <div
              key={item.id}
              className="flex items-center gap-[var(--space-2)] py-[var(--space-1)] px-[var(--space-2)] rounded-[var(--radius-md)]"
              style={{ backgroundColor: "rgba(255,255,255,0.06)" }}
            >
              <ItemIcon fileType={item.fileType} />
              <div className="flex-1 min-w-0">
                <div className="text-[var(--text-xs)] truncate" style={{ color: "rgba(255,255,255,0.85)" }}>{item.fileName}</div>
                <div className="text-[10px] line-clamp-2" style={{ color: "rgba(255,255,255,0.4)" }}>
                  {item.detail ??
                    (item.status === "done"
                      ? "已入库"
                      : item.status === "classifying"
                        ? "分类中..."
                        : item.status === "error"
                          ? "失败"
                          : "待处理")}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function ItemIcon({ fileType }: { fileType: string }) {
  const size = 14;
  const color = "rgba(255,255,255,0.5)";
  if (fileType.startsWith("image")) return <Image size={size} style={{ color }} />;
  if (fileType.startsWith("audio")) return <Music size={size} style={{ color }} />;
  return <FileText size={size} style={{ color }} />;
}
