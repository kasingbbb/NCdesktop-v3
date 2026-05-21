/**
 * KnowledgeHubView — 4-step 聚合视图类型
 *
 * 注意（PRD §10 Glossary）：4-step 是「聚合视图顺序约定」，不是流水线/wizard。
 * 任何 step 都可独立访问；不存在 prev/next 强约束。
 */

export const HUB_STEPS = ["assets", "concepts", "library", "skills"] as const;

export type HubStep = (typeof HUB_STEPS)[number];

export const DEFAULT_HUB_STEP: HubStep = "concepts";

export function isHubStep(value: unknown): value is HubStep {
  return typeof value === "string" && (HUB_STEPS as readonly string[]).includes(value);
}
