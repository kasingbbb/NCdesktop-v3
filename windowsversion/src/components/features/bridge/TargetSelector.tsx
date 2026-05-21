import { BookOpen, MessageSquare, Copy, Brain } from "lucide-react";

export type ExportTarget = "notebooklm" | "chatgpt" | "claude" | "clipboard";

interface TargetSelectorProps {
  selected: ExportTarget;
  onSelect: (target: ExportTarget) => void;
}

const TARGETS: Array<{
  id: ExportTarget;
  label: string;
  icon: typeof BookOpen;
  description: string;
}> = [
  {
    id: "notebooklm",
    label: "NotebookLM",
    icon: BookOpen,
    description: "导出 .md 文件并打开浏览器",
  },
  {
    id: "chatgpt",
    label: "ChatGPT",
    icon: MessageSquare,
    description: "复制到剪贴板并打开 ChatGPT",
  },
  {
    id: "claude",
    label: "Claude",
    icon: Brain,
    description: "复制到剪贴板并打开 Claude",
  },
  {
    id: "clipboard",
    label: "剪贴板",
    icon: Copy,
    description: "仅复制 Markdown 到剪贴板",
  },
];

export function TargetSelector({
  selected,
  onSelect,
}: TargetSelectorProps) {
  return (
    <div className="grid grid-cols-2 gap-[var(--space-2)]">
      {TARGETS.map((target) => {
        const Icon = target.icon;
        const isSelected = selected === target.id;
        return (
          <button
            key={target.id}
            className="flex flex-col items-center gap-[var(--space-1)] p-[var(--space-3)] rounded-[var(--radius-md)] transition-all"
            style={{
              backgroundColor: isSelected
                ? "var(--sidebar-active-bg)"
                : "var(--surface-secondary)",
              border: isSelected
                ? "1px solid var(--border-active)"
                : "1px solid var(--border-primary)",
            }}
            onClick={() => onSelect(target.id)}
          >
            <Icon
              size={20}
              className={isSelected ? "text-gray-900" : "text-gray-500"}
            />
            <span
              className="text-[var(--text-sm)] font-medium"
              style={{
                color: "var(--text-primary)",
                fontWeight: isSelected ? 600 : 500,
              }}
            >
              {target.label}
            </span>
            <span
              className="text-[10px] text-center"
              style={{ color: "var(--text-tertiary)" }}
            >
              {target.description}
            </span>
          </button>
        );
      })}
    </div>
  );
}
