import type { MouseEvent } from "react";
import { Clock, HardDrive, Tag as TagIcon, Trash2 } from "lucide-react";
import type { Project } from "../../types";
import type { Tag } from "../../types/common";
import { logger } from "../../utils/logger";

interface ProjectCardProps {
  project: Project;
  onClick?: () => void;
  onDelete?: (e: MouseEvent<HTMLButtonElement>) => void;
}

export function ProjectCard({ project, onClick, onDelete }: ProjectCardProps) {
  return (
    <div
      className="relative glass-card-elevated rounded-[var(--radius-xl)] p-[var(--space-3)] cursor-pointer"
      style={{ transition: "background-color var(--duration-fast), border-color var(--duration-fast), color var(--duration-fast)" }}
      onClick={() => {
        logger.info("ProjectCard", "Project clicked", { id: project.id, name: project.name });
        onClick?.();
      }}
    >
      {onDelete ? (
        <button
          type="button"
          aria-label="删除项目"
          className="absolute right-[var(--space-2)] top-[var(--space-2)] z-20 rounded-[var(--radius-md)] p-1.5 transition-colors hover:bg-red-500/20 pointer-events-auto"
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
        <div
        className="aspect-video w-full rounded-[var(--radius-lg)] mb-[var(--space-3)] overflow-hidden relative bg-[var(--surface-tertiary)]"
      >
        {/* Placeholder for thumbnail */}
      </div>
      
      <h3 className="text-[var(--text-base)] font-medium mb-[var(--space-1)] truncate" style={{ color: "var(--text-primary)" }}>
        {project.name || "Untitled Project"}
      </h3>
      
      <div className="flex items-center gap-[var(--space-3)] text-[var(--text-xs)] mb-[var(--space-2)]" style={{ color: "var(--text-tertiary)" }}>
        <span className="flex items-center gap-1">
          <Clock size={12} />
          {new Date(project.createdAt).toLocaleDateString()}
        </span>
        <span className="flex items-center gap-1">
          <HardDrive size={12} />
          {project.metadata?.assetCount || 0} items
        </span>
      </div>

      <div className="flex gap-[var(--space-1)] flex-wrap">
        {project.tags?.slice(0, 3).map((tag: Tag) => (
          <span
            key={tag.id || tag.name}
            className="tag-pill"
          >
            <TagIcon size={10} className="mr-1" />
            {tag.name}
          </span>
        ))}
      </div>
    </div>
  );
}
