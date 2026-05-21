import type { MouseEvent } from "react";
import { Folder, Clock, Hash, Trash2 } from "lucide-react";
import type { Project } from "../../types";

interface ProjectListItemProps {
  project: Project;
  onClick?: () => void;
  onDelete?: (e: MouseEvent<HTMLButtonElement>) => void;
}

export function ProjectListItem({ project, onClick, onDelete }: ProjectListItemProps) {
  return (
    <div 
      className="flex items-center px-[var(--space-4)] py-[var(--space-2)] border-b cursor-pointer transition-colors"
      style={{ borderColor: "var(--border-primary)" }}
      onClick={onClick}
    >
      <div className="w-8 h-8 rounded-[var(--radius-sm)] shrink-0 flex items-center justify-center mr-[var(--space-3)]" style={{ background: "var(--surface-secondary)" }}>
        <Folder size={16} className="text-gray-500" />
      </div>
      
      <div className="flex-1 min-w-0 mr-[var(--space-4)]">
        <h4 className="text-[var(--text-sm)] font-medium truncate" style={{ color: "var(--text-primary)" }}>
          {project.name || "Untitled Project"}
        </h4>
        <p className="text-[var(--text-xs)] truncate" style={{ color: "var(--text-tertiary)" }}>
          {project.description || "No description"}
        </p>
      </div>
      
      <div className="w-32 text-[var(--text-xs)] shrink-0 flex items-center" style={{ color: "var(--text-secondary)" }}>
        <Clock size={12} className="mr-1" />
        {new Date(project.updatedAt).toLocaleDateString()}
      </div>
      
      <div className="w-24 text-[var(--text-xs)] shrink-0 flex items-center justify-end" style={{ color: "var(--text-tertiary)" }}>
        <Hash size={12} className="mr-1" />
        {project.metadata?.assetCount || 0} Assets
      </div>

      {onDelete ? (
        <button
          type="button"
          aria-label="删除项目"
          className="shrink-0 ml-[var(--space-2)] rounded-[var(--radius-md)] p-2 transition-colors hover:bg-red-500/20 pointer-events-auto relative z-10"
          style={{ color: "var(--text-tertiary)" }}
          onClick={(e) => {
            e.preventDefault();
            e.stopPropagation();
            onDelete(e);
          }}
        >
          <Trash2 size={16} />
        </button>
      ) : null}
    </div>
  );
}
