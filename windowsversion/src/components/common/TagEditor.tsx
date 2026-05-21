import { useState, type KeyboardEvent } from "react";
import { Plus, X } from "lucide-react";
import type { Tag } from "../../types/common";

interface TagEditorProps {
  tags: Tag[];
  onRemoveTag?: (tag: Tag) => void;
  onAddTag?: (name: string) => void | Promise<void>;
  addDisabled?: boolean;
}

export function TagEditor({ tags, onRemoveTag, onAddTag, addDisabled }: TagEditorProps) {
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);

  async function submit(): Promise<void> {
    const name = draft.trim();
    if (!name || !onAddTag || busy || addDisabled) {
      return;
    }
    setBusy(true);
    try {
      await onAddTag(name);
      setDraft("");
    } finally {
      setBusy(false);
    }
  }

  function onKeyDown(e: KeyboardEvent<HTMLInputElement>): void {
    if (e.key === "Enter") {
      e.preventDefault();
      void submit();
    }
  }

  return (
    <div className="flex flex-wrap gap-2 items-center">
      {tags.map((tag) => (
        <button
          key={tag.id}
          type="button"
          className="flex items-center text-[10px] px-2 py-1 rounded-md group cursor-pointer border border-gray-200 bg-gray-100 text-gray-700"
          onClick={() => onRemoveTag?.(tag)}
          title="点击移除标签"
        >
          {tag.name}
          <X size={10} className="ml-1 opacity-0 group-hover:opacity-100 transition-opacity" />
        </button>
      ))}
      {onAddTag ? (
        <div className="flex items-center gap-1 min-w-[140px] flex-1">
          <input
            type="text"
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={onKeyDown}
            disabled={busy || addDisabled}
            placeholder="新标签…"
            className="flex-1 min-w-0 text-[10px] px-2 py-1 rounded-[var(--radius-md)] bg-black/20 border border-white/10 outline-none focus:border-gray-500"
            style={{ color: "var(--text-primary)" }}
          />
          <button
            type="button"
            disabled={busy || addDisabled || !draft.trim()}
            className="flex items-center text-[10px] px-2 py-1 rounded-full border border-dashed transition-colors shrink-0"
            style={{ color: "var(--text-secondary)", borderColor: "var(--border-primary)" }}
            onClick={() => void submit()}
          >
            <Plus size={10} className="mr-0.5" /> 添加
          </button>
        </div>
      ) : null}
    </div>
  );
}
