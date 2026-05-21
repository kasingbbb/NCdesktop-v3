import { X, MousePointerClick } from "lucide-react";
import { useUIStore } from "../../stores/uiStore";
import { useAssetStore } from "../../stores/assetStore";
import { InspectorDetails } from "./InspectorDetails";
import { InspectorAI } from "./InspectorAI";
import { InspectorTags } from "./InspectorTags";
import { InspectorExtraction } from "./InspectorExtraction";
import { TimelineFlowView } from "../features/timeline-flow/TimelineFlowView";
import { KnowledgeAssociationView } from "../features/knowledge/KnowledgeAssociationView";

interface InspectorProps {
  width?: number;
}

const TABS: { key: "inspector" | "timeline-flow" | "knowledge_association"; label: string }[] = [
  { key: "inspector", label: "详情" },
  { key: "knowledge_association", label: "知识关联" },
  { key: "timeline-flow", label: "时间流" },
];

export function Inspector({ width = 320 }: InspectorProps) {
  const { inspectorOpen, toggleInspector, rightPanelMode, setRightPanelMode } = useUIStore();
  const { selectedAssetId, assets } = useAssetStore();

  if (!inspectorOpen) return null;
  if (rightPanelMode === "course_preview") return null;

  const activeAsset = assets.find((a) => a.id === selectedAssetId);

  return (
    <aside
      className="h-full shrink-0 border-l flex flex-col relative min-w-0"
      style={{
        width,
        borderColor: "var(--border-primary)",
        background: "var(--surface-primary)",
      }}
    >
      {/* Header: Segmented Control + 关闭 */}
      <div
        className="h-[48px] flex items-center justify-between px-[var(--space-3)] border-b shrink-0 gap-[var(--space-2)]"
        style={{ borderColor: "var(--border-primary)", background: "var(--surface-primary)" }}
      >
        <div
          className="flex rounded-[var(--radius-full)] p-[3px] gap-[2px] flex-1 min-w-0"
          style={{
            background: "var(--surface-tertiary)",
            border: "1px solid var(--border-primary)",
          }}
        >
          {TABS.map((tab) => {
            const isActive = rightPanelMode === tab.key;
            return (
              <button
                key={tab.key}
                type="button"
                className="flex-1 px-[var(--space-2)] py-[4px] rounded-[var(--radius-full)] text-[11px] font-medium transition-all truncate"
                style={{
                  background: isActive ? "var(--surface-primary)" : "transparent",
                  color: isActive ? "var(--text-primary)" : "var(--text-tertiary)",
                  boxShadow: isActive ? "var(--shadow-sm)" : "none",
                  transitionDuration: "var(--duration-fast)",
                  transitionTimingFunction: "var(--ease-out-expo)",
                }}
                onClick={() => setRightPanelMode(tab.key)}
                aria-pressed={isActive}
              >
                {tab.label}
              </button>
            );
          })}
        </div>

        <button
          type="button"
          onClick={toggleInspector}
          className="w-[24px] h-[24px] flex items-center justify-center rounded-[var(--radius-sm)] transition-colors shrink-0"
          style={{ color: "var(--text-tertiary)" }}
          onMouseEnter={(e) =>
            ((e.currentTarget as HTMLButtonElement).style.background = "var(--surface-tertiary)")
          }
          onMouseLeave={(e) =>
            ((e.currentTarget as HTMLButtonElement).style.background = "transparent")
          }
          aria-label="关闭右栏"
        >
          <X size={14} />
        </button>
      </div>

      {/* Body */}
      <div
        className={`flex-1 min-h-0 overflow-hidden flex flex-col pb-0 ${
          rightPanelMode === "timeline-flow" ? "p-[var(--space-3)]" : "p-[var(--space-4)]"
        }`}
      >
        {rightPanelMode === "knowledge_association" ? (
          <div className="flex-1 min-h-0 flex flex-col min-w-0 -m-[var(--space-4)]">
            <KnowledgeAssociationView />
          </div>
        ) : rightPanelMode === "timeline-flow" ? (
          <div className="flex-1 min-h-0 flex flex-col min-w-0">
            <TimelineFlowView />
          </div>
        ) : activeAsset ? (
          <div className="overflow-y-auto h-full min-h-0 space-y-[var(--space-4)]">
            <InspectorDetails asset={activeAsset} />
            <InspectorAI asset={activeAsset} />
            <InspectorExtraction asset={activeAsset} />
            <InspectorTags asset={activeAsset} />
          </div>
        ) : (
          <div className="h-full flex flex-col items-center justify-center gap-[var(--space-3)] px-[var(--space-4)]">
            <div
              className="w-12 h-12 rounded-full flex items-center justify-center"
              style={{ background: "var(--surface-tertiary)", color: "var(--text-tertiary)" }}
            >
              <MousePointerClick size={22} />
            </div>
            <p
              className="text-[var(--text-sm)] text-center font-medium"
              style={{ color: "var(--text-secondary)" }}
            >
              未选中素材
            </p>
            <p
              className="text-[var(--text-xs)] text-center leading-relaxed max-w-[200px]"
              style={{ color: "var(--text-tertiary)" }}
            >
              在列表中选择一项，即可查看详情、AI 摘要与标签。
            </p>
          </div>
        )}
      </div>
    </aside>
  );
}
