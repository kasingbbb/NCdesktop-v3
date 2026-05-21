import { FolderOpen, RefreshCw, ExternalLink } from "lucide-react";
import type { WorkspaceFolderEntry } from "../../types";
import { workspaceFolderKindBadge } from "../../lib/workspace-folder-badges";

interface WorkspaceFolderStripProps {
  folders: WorkspaceFolderEntry[];
  workspaceRootHint: string;
  selectedRelativePath: string | null;
  loading: boolean;
  onSelect: (relativePath: string | null) => void;
  onReveal: (relativePath: string) => void;
  onRefresh: () => void;
}

export function WorkspaceFolderStrip({
  folders,
  workspaceRootHint,
  selectedRelativePath,
  loading,
  onSelect,
  onReveal,
  onRefresh,
}: WorkspaceFolderStripProps) {
  return (
    <div
      className="shrink-0 mb-[var(--space-3)] rounded-[var(--radius-lg)] border px-[var(--space-3)] py-[var(--space-2)]"
      style={{
        borderColor: "var(--border-primary)",
        background: "var(--surface-tertiary)",
        boxShadow: "var(--shadow-sm)",
      }}
    >
      <div className="flex items-center gap-2 mb-2">
        <FolderOpen size={16} style={{ color: "var(--text-secondary)" }} />
        <span className="text-[var(--text-sm)] font-semibold" style={{ color: "var(--text-primary)" }}>
          工作区文件夹
        </span>
        <span className="text-[10px] truncate flex-1 min-w-0" style={{ color: "var(--text-tertiary)" }} title={workspaceRootHint}>
          {workspaceRootHint ? `「下载」/${workspaceRootHint.split("/").slice(-2).join("/")}` : "…"}
        </span>
        <button
          type="button"
          className="shrink-0 p-1 rounded-[var(--radius-sm)] hover:opacity-80"
          style={{ color: "var(--text-tertiary)" }}
          title="刷新文件夹列表"
          onClick={() => onRefresh()}
          disabled={loading}
        >
          <RefreshCw size={14} className={loading ? "animate-spin" : ""} />
        </button>
      </div>
      <div className="flex flex-wrap gap-1.5 items-center">
        <button
          type="button"
          className="text-[10px] px-2 py-0.5 rounded-full border transition-colors"
          style={{
            borderColor: selectedRelativePath === null ? "var(--border-active)" : "var(--border-primary)",
            background: selectedRelativePath === null ? "var(--sidebar-active-bg)" : "var(--surface-primary)",
            color: selectedRelativePath === null ? "var(--sidebar-active-fg)" : "var(--text-secondary)",
          }}
          onClick={() => onSelect(null)}
        >
          全部素材
        </button>
        {folders.map((f) => {
          const active = selectedRelativePath === f.relativePath;
          const badge = workspaceFolderKindBadge(f.kind);
          return (
            <div key={f.relativePath} className="inline-flex items-center gap-0.5">
              <button
                type="button"
                className="text-[10px] px-2 py-0.5 rounded-full border transition-colors max-w-[200px] truncate"
                style={{
                  borderColor: active ? "var(--border-active)" : "var(--border-primary)",
                  background: active ? "var(--sidebar-active-bg)" : "var(--surface-primary)",
                  color: active ? "var(--sidebar-active-fg)" : "var(--text-secondary)",
                }}
                title={f.relativePath}
                onClick={() => onSelect(active ? null : f.relativePath)}
              >
                {badge ? (
                  <span className="opacity-70 mr-1">{badge}</span>
                ) : null}
                {f.displayLabel}
              </button>
              <button
                type="button"
                className="p-0.5 rounded-[var(--radius-sm)] shrink-0"
                style={{ color: "var(--text-tertiary)" }}
                title="在访达中打开"
                onClick={(e) => {
                  e.stopPropagation();
                  onReveal(f.relativePath);
                }}
              >
                <ExternalLink size={12} />
              </button>
            </div>
          );
        })}
        {folders.length === 1 && !loading ? (
          <span className="text-[10px]" style={{ color: "var(--text-tertiary)" }}>
            拖入文件或经 AI 整理后，将在此出现子文件夹
          </span>
        ) : null}
      </div>
    </div>
  );
}
