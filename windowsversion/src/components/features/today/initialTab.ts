/**
 * task_010 / ADR-006 — TodayView 内部 Tab 初始策略（纯函数）。
 *
 * 三态行为矩阵（与 input.md AC-1 一致）：
 *
 * | lastTab           | justEnabled | 返回                |
 * | ----------------- | ----------- | ------------------- |
 * | null              | false       | "course-prep"       |
 * | null              | true        | "course-prep"       |
 * | "course-prep"     | false       | "course-prep"       |
 * | "daily-review"    | false       | "daily-review"      |
 * | "course-prep"     | true        | "course-prep"       |
 * | "daily-review"    | true        | "course-prep"（强制重置） |
 *
 * 不可妥协：
 *   - 仅纯函数，不读 store、不写 store、不触发副作用。
 *   - JustEnabled 信号优先级高于 lastTab；信号消费在调用方（TodayView mount effect）完成。
 *   - 未来若 TodayTab union 扩展，新增成员的默认行为应在此处显式处理，不要默默 fallback。
 */
import type { TodayTab } from "../../../types";

export function computeInitialTodayTab(
  lastTab: TodayTab | null,
  justEnabled: boolean,
): TodayTab {
  if (justEnabled) return "course-prep";
  return lastTab ?? "course-prep";
}
