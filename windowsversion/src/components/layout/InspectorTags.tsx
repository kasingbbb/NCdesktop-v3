import { useCallback, useEffect, useState } from "react";
import { Tag } from "lucide-react";
import type { Asset } from "../../types";
import type { Tag as TagModel } from "../../types/common";
import { TagEditor } from "../common/TagEditor";
import {
  ensureAssetTagByName,
  getAssetSuggestedTagNames,
  getAssetTags,
  unlinkTagFromAsset,
} from "../../lib/tauri-commands";
import { useTagStore } from "../../stores/tagStore";
import { useUIStore } from "../../stores/uiStore";

interface InspectorTagsProps {
  asset: Asset;
}

export function InspectorTags({ asset }: InspectorTagsProps) {
  const [linkedTags, setLinkedTags] = useState<TagModel[]>([]);
  const [suggested, setSuggested] = useState<string[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const fetchTags = useTagStore((s) => s.fetchTags);
  const addNotification = useUIStore((s) => s.addNotification);

  const reload = useCallback(async (): Promise<void> => {
    setLoadError(null);
    try {
      const [tags, names] = await Promise.all([
        getAssetTags(asset.id),
        getAssetSuggestedTagNames(asset.id),
      ]);
      setLinkedTags(tags);
      const linkedNames = new Set(tags.map((t) => t.name));
      setSuggested(names.filter((n) => !linkedNames.has(n)));
    } catch (e) {
      setLoadError(String(e));
    }
  }, [asset.id]);

  useEffect(() => {
    void reload();
  }, [reload]);

  const handleAddSuggested = useCallback(
    async (name: string): Promise<void> => {
      setBusy(true);
      try {
        await ensureAssetTagByName(asset.id, name);
        await fetchTags();
        await reload();
        addNotification({
          type: "success",
          title: "标签",
          message: `已添加「${name}」`,
          duration: 2200,
        });
      } catch (e) {
        addNotification({
          type: "error",
          title: "标签",
          message: String(e),
          duration: 4000,
        });
      } finally {
        setBusy(false);
      }
    },
    [addNotification, asset.id, fetchTags, reload],
  );

  const handleAddManual = useCallback(
    async (name: string): Promise<void> => {
      await handleAddSuggested(name);
    },
    [handleAddSuggested],
  );

  const handleRemove = useCallback(
    async (tag: TagModel): Promise<void> => {
      setBusy(true);
      try {
        await unlinkTagFromAsset(asset.id, tag.id);
        await fetchTags();
        await reload();
      } catch (e) {
        addNotification({
          type: "error",
          title: "标签",
          message: String(e),
          duration: 4000,
        });
      } finally {
        setBusy(false);
      }
    },
    [addNotification, asset.id, fetchTags, reload],
  );

  return (
    <div className="mb-[var(--space-4)]">
      <h3
        className="text-[var(--text-sm)] uppercase tracking-[0.08em] mb-[var(--space-2)] flex items-center gap-1"
        style={{ color: "var(--text-tertiary)" }}
      >
        <Tag size={14} /> Tags
      </h3>

      {loadError ? (
        <p className="text-[var(--text-xs)] mb-[var(--space-2)]" style={{ color: "#FF3B30" }}>
          {loadError}
        </p>
      ) : null}

      <div className="mb-[var(--space-3)]">
        <TagEditor
          tags={linkedTags}
          onRemoveTag={(t) => void handleRemove(t)}
          onAddTag={(n) => handleAddManual(n)}
          addDisabled={busy}
        />
      </div>

      {suggested.length > 0 ? (
        <div>
          <h4
            className="text-[10px] uppercase tracking-[0.05em] mb-1 pl-1"
            style={{ color: "var(--text-tertiary)" }}
          >
            建议（点击添加）
          </h4>
          <div className="flex flex-wrap gap-1">
            {suggested.map((tagName) => (
              <button
                key={tagName}
                type="button"
                disabled={busy}
                className="text-[10px] px-2 py-0.5 rounded-md border border-gray-200 bg-gray-50 text-gray-600 hover:bg-gray-100 hover:border-gray-300 transition-colors disabled:opacity-40"
                onClick={() => void handleAddSuggested(tagName)}
              >
                + {tagName}
              </button>
            ))}
          </div>
        </div>
      ) : null}
    </div>
  );
}
