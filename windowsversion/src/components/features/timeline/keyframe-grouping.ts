import type { Keyframe } from "../../../types";

/** 将关键帧按时间聚合分组（默认阈值 ±2s） */
export function groupKeyframesByTime(
  keyframes: Keyframe[],
  thresholdMs: number = 2000
): Keyframe[][] {
  if (keyframes.length === 0) return [];

  const sorted = [...keyframes].sort((a, b) => a.anchorTime - b.anchorTime);
  const groups: Keyframe[][] = [[sorted[0]]];

  for (let i = 1; i < sorted.length; i++) {
    const lastGroup = groups[groups.length - 1];
    const lastKf = lastGroup[lastGroup.length - 1];
    const diff = (sorted[i].anchorTime - lastKf.anchorTime) * 1000;

    if (diff <= thresholdMs) {
      lastGroup.push(sorted[i]);
    } else {
      groups.push([sorted[i]]);
    }
  }

  return groups;
}

