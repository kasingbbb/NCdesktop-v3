/**
 * task_019 / TD-4 共享 helper —— kc_enriched 字面 → 用户可见文案 + 视觉色调
 *
 * **由来与归属说明**：
 * - task_018 (InspectorExtraction) 与 task_019 (DocumentViewer) 都需要把 YAML
 *   字面值 `"true" | "partial" | "false" | null` 翻译成中文文案与视觉 dot。
 * - task_019 input.md AC-5 (TD-4) 建议抽出 `mapKcEnrichedToLabel`，避免 DRY 重写。
 * - **本 helper 是业务表层翻译**，不复用 task_021 的 KcStatusBadge：
 *     - KcStatusBadge 关心 UX 状态 "success" / "failed" / "loading" / "idle"，
 *       与 YAML 字面 "true" / "partial" / "false" 解耦，便于未来调度态 / 队列态复用。
 *     - 这里负责"DB 字面值 → 用户中文文案 + 色调 token"。
 *
 * 返回 null 表示"该行不渲染"（历史数据 kc_enriched = null / 任意未识别字面）。
 *
 * tone 取值：
 *   - "success"  → 完整 KC 增强（green dot）
 *   - "partial"  → 仅规则标签（amber dot，LLM 不可用回退）
 *   - "inactive" → 未启用 AI 增强（grey dot）
 */
export type KcEnrichedTone = "success" | "partial" | "inactive";

export interface KcEnrichedLabelResult {
  label: string;
  tone: KcEnrichedTone;
}

/**
 * 把 `kc_enriched` 字面映射为 { label, tone }。
 *
 * @param value YAML 字面值（来自 frontmatter / DB 列）
 * @returns 显示用结果；null 表示整行隐藏
 */
export function mapKcEnrichedToLabel(
  value: string | null | undefined,
): KcEnrichedLabelResult | null {
  switch (value) {
    case "true":
      return { label: "AI 增强：完整", tone: "success" };
    case "partial":
      return { label: "AI 增强：仅规则标签（LLM 不可用）", tone: "partial" };
    case "false":
      return { label: "未启用 AI 增强", tone: "inactive" };
    default:
      // null / undefined / 任何未识别字面 —— 历史数据或脏数据，整行不显示
      return null;
  }
}
