import { useEffect, useMemo, useState } from "react";
import { Tag as TagIcon, ChevronRight, ChevronDown } from "lucide-react";
import { SidebarItem } from "../layout/SidebarItem";
import { useTagStore } from "../../stores/tagStore";
import { useUIStore } from "../../stores/uiStore";

export function TagTree() {
  const tags = useTagStore((s) => s.tags);
  const fetchTags = useTagStore((s) => s.fetchTags);
  const filterId = useUIStore((s) => s.assetTagFilterId);
  const setFilterId = useUIStore((s) => s.setAssetTagFilterId);
  const expanded = useUIStore((s) => s.tagsExpanded);
  const setExpanded = useUIStore((s) => s.setTagsExpanded);

  const [filterText, setFilterText] = useState("");

  useEffect(() => {
    void fetchTags();
  }, [fetchTags]);

  const filteredTags = useMemo(() => {
    const q = filterText.trim().toLowerCase();
    if (!q) return tags;
    return tags.filter((t) => t.name.toLowerCase().includes(q));
  }, [tags, filterText]);

  return (
    <div className="mb-[var(--space-2)]">
      <button
        type="button"
        aria-expanded={expanded}
        aria-controls="tag-tree-list"
        onClick={() => setExpanded(!expanded)}
        className="w-full flex items-center justify-between px-[var(--space-3)] py-[var(--space-1)] text-[var(--text-xs)] uppercase tracking-[0.08em]"
        style={{ color: "var(--text-tertiary)" }}
      >
        <span className="flex items-center gap-[var(--space-1)]">
          {expanded ? <ChevronDown size={10} /> : <ChevronRight size={10} />}
          Tags
          <span className="ml-1 normal-case tracking-normal tabular-nums" style={{ color: "var(--text-tertiary)" }}>
            {tags.length}
          </span>
        </span>
        {expanded && filterId && (
          <span
            role="button"
            tabIndex={0}
            onClick={(e) => {
              e.stopPropagation();
              setFilterId(null);
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.stopPropagation();
                setFilterId(null);
              }
            }}
            className="text-[10px] uppercase tracking-wide px-1 py-0.5 rounded text-gray-600"
          >
            清除筛选
          </span>
        )}
      </button>

      {expanded && (
        <div id="tag-tree-list">
          <input
            type="text"
            value={filterText}
            onChange={(e) => setFilterText(e.target.value)}
            placeholder="过滤标签"
            className="w-[calc(100%-var(--space-6))] mx-[var(--space-3)] mb-[var(--space-2)] px-[var(--space-2)] py-[var(--space-1)] text-[var(--text-sm)] rounded-[var(--radius-sm)]"
            style={{
              border: "1px solid var(--border-primary)",
              background: "var(--surface-secondary)",
              color: "var(--text-primary)",
            }}
          />
          {tags.length === 0 ? (
            <p
              className="px-[var(--space-3)] text-[var(--text-xs)]"
              style={{ color: "var(--text-tertiary)" }}
            >
              暂无标签；在 Inspector 中为素材添加标签后将显示于此。
            </p>
          ) : filteredTags.length === 0 ? (
            <p
              className="px-[var(--space-3)] text-[var(--text-xs)]"
              style={{ color: "var(--text-tertiary)" }}
            >
              无匹配标签
            </p>
          ) : (
            filteredTags.map((tag) => (
              <SidebarItem
                key={tag.id}
                icon={<TagIcon size={16} />}
                label={tag.name}
                badge={tag.usageCount ?? 0}
                active={filterId === tag.id}
                onClick={() => setFilterId(filterId === tag.id ? null : tag.id)}
              />
            ))
          )}
        </div>
      )}
    </div>
  );
}
