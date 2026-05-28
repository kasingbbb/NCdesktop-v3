/**
 * task_021_visual_badge — KC 增强状态视觉标识（4 态）
 *
 * 纯展示组件：接收 `status` props 渲染对应 badge，无副作用（无 store / 无 IPC / 无 useEffect）。
 *
 * ## 4 态来源（与衍生件落地状态对齐）
 *
 * 字面值映射 `src-tauri/src/extracted_content.kc_enriched` 列（v18 schema，ADR-005）：
 *
 * | KcStatus | 衍生件状态 | 后端来源 | 形象 |
 * |----------|----------|---------|------|
 * | `"success"`  | KC 增强完整产出（含 ai_tags + ai_summary） | `KcEnrichmentOutcome::Success` → kc_enriched = "true"   | 绿色 CheckCircle |
 * | `"partial"`  | KC LLM 不可用，仅规则标签             | `KcEnrichmentOutcome::PartialLlmUnavailable` → kc_enriched = "partial" | 黄色 AlertCircle |
 * | `"failed"`   | KC 调用失败，衍生件已落地 MarkItDown 原版 | `KcEnrichmentOutcome::Fallback`（非 Disabled）→ kc_enriched = "false"  | 红色 XCircle |
 * | `"none"`     | 用户禁用 KC 或未经 KC 处理（含历史数据） | `Disabled` / 历史 NULL 行 → kc_enriched = null               | 灰色 Circle |
 *
 * ## 与后端 `KcTagsSource` (`src-tauri/src/kc/errors.rs`) 的对齐说明
 *
 * 后端 `KcTagsSource` 是**仅 2 态**（"ai+rule" / "rule_only"），它对应 KC 调用**成功**路径下
 * 标签来源（决定 partial vs success），并不是 4 态视觉标识的完整覆盖面。
 * 本组件的 4 态等价于"衍生件落地后用户能看到的最终 KC 状态"，
 * 由 `extracted_content.kc_enriched` 列 + `null` 表示历史数据共同提供。
 * 这层映射在 `KcEnrichmentOutcome::resolve_outcome`（task_011）完成，前端只读最终态。
 *
 * ## a11y 设计（TD-2 直接处理点，本组件不能再缺）
 *
 * - `role="img"`：视觉装饰元素，screen reader 通过 `aria-label` 读出语义；
 * - `aria-label` 文本与 4 态一一对应，与 tooltip title 一致；
 * - 形状区分（不只靠颜色）：success=CheckCircle / partial=AlertCircle / failed=XCircle / none=Circle，
 *   照顾红绿色觉障碍用户；
 * - `title` 属性提供原生 tooltip（hover 显示完整状态描述，无需引入新依赖）。
 *
 * 严格不做：
 * - 不接入 AssetListView / Inspector / DocumentViewer（task_018 / task_019 / 后续 task 负责）；
 * - 不依赖 Tauri command / Zustand store（status 由外部传入）；
 * - 不引入新 npm 依赖（沿用 lucide-react + Tailwind）。
 */
import { CheckCircle, AlertCircle, XCircle, Circle } from "lucide-react";

/**
 * KC 增强 4 态字面值。
 *
 * 与后端 v18 schema `extracted_content.kc_enriched` 列字面值同形（成功/部分/失败），
 * 外加 `"none"` 用于表示"未经 KC 处理 / 用户禁用"（对应 DB 中的 NULL 行）。
 */
export type KcStatus = "success" | "partial" | "failed" | "none";

interface KcStatusBadgeProps {
  /**
   * KC 增强状态。
   *
   * 调用方负责把 `extracted_content.kc_enriched` 列字面值翻译到此类型：
   * - `"true"`    → `"success"`
   * - `"partial"` → `"partial"`
   * - `"false"`   → `"failed"`
   * - `null` / undefined → `"none"`
   *
   * 当传入 `undefined` 时按 `"none"` 处理（防御历史数据 / loading 中场景）。
   */
  status: KcStatus | undefined;
  /** badge 尺寸，默认 sm（用于列表项），md 用于详情页。 */
  size?: "sm" | "md";
}

interface StatusConfig {
  /** lucide 图标组件 */
  Icon: typeof CheckCircle;
  /** Tailwind 颜色 class（图标主色） */
  colorClass: string;
  /** screen reader / tooltip 文本（与 PRD §7 视觉标识行 + §4.3 UI 文案一致） */
  label: string;
}

/**
 * 4 态视觉配置。
 *
 * 颜色与 `ExtractionBadge.tsx`（已有兄弟组件）一致：
 * - 绿（success）= `text-green-500`
 * - 黄（partial）= `text-yellow-500`（与原 ExtractionBadge 的红 / 蓝色错开）
 * - 红（failed） = `text-red-500`
 * - 灰（none）   = `text-gray-400`
 *
 * 4 态用 4 个不同图标形状区分（CheckCircle / AlertCircle / XCircle / Circle），
 * 避免红绿色盲用户只能区分 partial vs failed。
 */
const STATUS_CONFIGS: Record<KcStatus, StatusConfig> = {
  success: {
    Icon: CheckCircle,
    colorClass: "text-green-500",
    label: "AI 增强完整（含 ai_tags + ai_summary）",
  },
  partial: {
    Icon: AlertCircle,
    colorClass: "text-yellow-500",
    label: "仅规则标签（KC LLM 不可用）",
  },
  failed: {
    Icon: XCircle,
    colorClass: "text-red-500",
    label: "KC 增强失败（已落地基础 MD）",
  },
  none: {
    Icon: Circle,
    colorClass: "text-gray-400",
    label: "未经 KC 增强",
  },
};

export function KcStatusBadge({ status, size = "sm" }: KcStatusBadgeProps) {
  // 防御：undefined / 未知值都按 "none" 渲染（不抛错，列表场景需要鲁棒）
  const effectiveStatus: KcStatus =
    status && status in STATUS_CONFIGS ? status : "none";
  const { Icon, colorClass, label } = STATUS_CONFIGS[effectiveStatus];

  // 尺寸与 ExtractionBadge 对齐：sm 用列表项，md 用详情页
  const sizeClass = size === "sm" ? "w-3.5 h-3.5" : "w-4 h-4";

  return (
    <span
      role="img"
      aria-label={label}
      title={label}
      data-testid="kc-status-badge"
      data-status={effectiveStatus}
      className="inline-flex items-center"
    >
      <Icon className={`${sizeClass} ${colorClass}`} aria-hidden="true" />
    </span>
  );
}
