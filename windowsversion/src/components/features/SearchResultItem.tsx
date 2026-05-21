import { FileText, Mic, Camera, Tag, StickyNote, FolderOpen } from "lucide-react";

export type SearchResultType =
  | "project"
  | "asset"
  | "transcription"
  | "note"
  | "tag";

export interface SearchResultData {
  id: string;
  type: SearchResultType;
  title: string;
  snippet: string;
  projectName: string | null;
  score: number;
}

interface SearchResultItemProps {
  result: SearchResultData;
  isActive: boolean;
  onSelect: (result: SearchResultData) => void;
}

const TYPE_ICON: Record<SearchResultType, typeof FileText> = {
  project: FolderOpen,
  asset: Camera,
  transcription: Mic,
  note: StickyNote,
  tag: Tag,
};

const TYPE_LABEL: Record<SearchResultType, string> = {
  project: "项目",
  asset: "素材",
  transcription: "转录",
  note: "笔记",
  tag: "标签",
};

export function SearchResultItem({
  result,
  isActive,
  onSelect,
}: SearchResultItemProps) {
  const Icon = TYPE_ICON[result.type] || FileText;

  return (
    <button
      className="w-full flex items-start gap-[var(--space-3)] px-[var(--space-4)] py-[var(--space-3)] transition-colors text-left"
      style={{
        backgroundColor: isActive ? "rgba(255, 192, 0, 0.08)" : "transparent",
      }}
      onClick={() => onSelect(result)}
    >
      <div
        className="flex-shrink-0 w-8 h-8 rounded-[var(--radius-sm)] flex items-center justify-center mt-0.5"
        style={{
          backgroundColor: "var(--surface-secondary)",
        }}
      >
        <Icon size={14} className="text-gray-500" />
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-[var(--space-2)]">
          <span
            className="text-[var(--text-sm)] font-medium truncate"
            style={{ color: "var(--text-primary)" }}
          >
            {result.title}
          </span>
          <span
            className="text-[10px] px-1.5 py-0.5 rounded-full flex-shrink-0"
            style={{
              backgroundColor: "var(--surface-secondary)",
              color: "var(--text-tertiary)",
            }}
          >
            {TYPE_LABEL[result.type]}
          </span>
        </div>
        <p
          className="text-[var(--text-xs)] mt-0.5 line-clamp-2"
          style={{ color: "var(--text-tertiary)" }}
          dangerouslySetInnerHTML={{ __html: result.snippet }}
        />
        {result.projectName && (
          <span className="text-[10px] mt-1 inline-block" style={{ color: "var(--text-tertiary)" }}>
            {result.projectName}
          </span>
        )}
      </div>
    </button>
  );
}
