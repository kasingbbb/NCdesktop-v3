/**
 * KnowledgeHubView — 4-step 聚合视图入口（v1.3 task_007 / KH-01~05）
 *
 * 4-step：assets → concepts → library → skills。
 * 这是「聚合视图顺序约定」（PRD §10 Glossary），**不是 wizard**：
 *   - 横向 nav 切换，无 prev/next 按钮
 *   - 任意 step 可深链直达
 *
 * 路由：原生 hash route `#/knowledge-hub/:step`
 *   - pushState + popstate 双向同步（前进/后退可用，PRD AC-13）
 *   - 旧 hash `#/skills` `#/knowledge` 自动 replaceState（PRD AC-12）
 *
 * v1.3 task_007 改造：
 *   - DEFAULT_HUB_STEP 改为 concepts（types.ts）
 *   - StepNav 升级为"链条 + counts"：step 间插 chevron `›`，每个 step 右侧显示当前
 *     count（>0 才渲染数字；===0 仅显示 step label）
 *   - 父组件 useMemo 聚合四个 store 长度
 */

import { useCallback, useMemo } from "react";
import { useUIStore } from "../../../stores/uiStore";
import { useHubHashRoute } from "./useHubHashRoute";
import type { HubStep } from "./types";
import { AssetsStep } from "./steps/AssetsStep";
import { ConceptsStep } from "./steps/ConceptsStep";
import { LibraryStep } from "./steps/LibraryStep";
import { SkillsStep } from "./steps/SkillsStep";
import { useAssetStore } from "../../../stores/assetStore";
import { useKnowledgeStore } from "../../../stores/knowledgeStore";
import { useLibraryStore } from "../../../stores/libraryStore";
import { useKnowledgeUnitsStore } from "../../../stores/knowledgeUnitsStore";

interface Props {
  libraryId: string | null;
}

const STEP_LABELS: Record<HubStep, string> = {
  assets: "素材",
  concepts: "概念",
  library: "知识库",
  skills: "技能",
};

export function KnowledgeHubView({ libraryId }: Props) {
  const setSidebarSection = useUIStore((s) => s.setSidebarSection);

  const onLegacyMigrated = useCallback(() => {
    setSidebarSection("knowledge-hub");
  }, [setSidebarSection]);

  const { step, setStep, steps } = useHubHashRoute({ onLegacyMigrated });

  const assetCount = useAssetStore((s) => s.assets.length);
  const conceptCount = useKnowledgeStore((s) => s.concepts.length);
  const libraryCount = useLibraryStore((s) => s.libraries.length);
  const skillCount = useKnowledgeUnitsStore((s) => s.units.length);

  const counts = useMemo<Record<HubStep, number>>(
    () => ({
      assets: assetCount,
      concepts: conceptCount,
      library: libraryCount,
      skills: skillCount,
    }),
    [assetCount, conceptCount, libraryCount, skillCount],
  );

  return (
    <div className="flex flex-col h-full min-h-0">
      <StepNav steps={steps} current={step} onSelect={setStep} counts={counts} />
      <div className="flex-1 min-h-0 overflow-hidden">
        {step === "assets" && <AssetsStep />}
        {step === "concepts" && <ConceptsStep />}
        {step === "library" && <LibraryStep />}
        {step === "skills" && <SkillsStep libraryId={libraryId} />}
      </div>
    </div>
  );
}

interface StepNavProps {
  steps: readonly HubStep[];
  current: HubStep;
  onSelect: (next: HubStep) => void;
  counts?: Partial<Record<HubStep, number>>;
}

function StepNav({ steps, current, onSelect, counts }: StepNavProps) {
  return (
    <nav
      role="tablist"
      aria-label="Knowledge Hub Steps"
      className="flex items-center gap-[var(--space-1)] px-[var(--space-3)] py-[var(--space-3)] border-b"
      style={{ borderColor: "var(--border-primary)" }}
    >
      {steps.map((s, i) => {
        const active = s === current;
        const n = counts?.[s] ?? 0;
        return (
          <span key={s} className="inline-flex items-center gap-[var(--space-1)]">
            {i > 0 && (
              <span
                aria-hidden="true"
                className="text-[12px] select-none"
                style={{ color: "var(--text-tertiary)" }}
              >
                ›
              </span>
            )}
            <button
              type="button"
              role="tab"
              aria-selected={active}
              data-step={s}
              onClick={() => onSelect(s)}
              className="inline-flex items-center gap-[var(--space-1)] px-[var(--space-3)] py-[var(--space-1)] text-[var(--text-sm)] rounded-[var(--radius-sm)] transition-colors"
              style={{
                background: active ? "var(--surface-tertiary)" : "transparent",
                color: active ? "var(--text-primary)" : "var(--text-secondary)",
                fontWeight: active ? 600 : 400,
              }}
            >
              <span>{STEP_LABELS[s]}</span>
              {n > 0 && (
                <span
                  className="step-count font-mono text-[11px] tabular-nums px-[6px] rounded-[8px]"
                  style={{
                    background: "var(--surface-tertiary)",
                    color: "var(--text-tertiary)",
                  }}
                >
                  {n}
                </span>
              )}
            </button>
          </span>
        );
      })}
    </nav>
  );
}
