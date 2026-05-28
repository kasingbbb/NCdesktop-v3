/**
 * task_019 / TD-4 shared helper —— kcEnrichedLabel 单元测试
 *
 * 覆盖：
 *   - "true"     → success tone + 中文完整文案
 *   - "partial"  → partial tone + LLM 不可用文案
 *   - "false"    → inactive tone + 未启用文案
 *   - null/undefined/未识别 → 返回 null（整行隐藏）
 */
import { describe, it, expect } from "vitest";
import { mapKcEnrichedToLabel } from "../kcEnrichedLabel";

describe("mapKcEnrichedToLabel", () => {
  it("returns success tone + 完整文案 for 'true'", () => {
    expect(mapKcEnrichedToLabel("true")).toEqual({
      label: "AI 增强：完整",
      tone: "success",
    });
  });

  it("returns partial tone + LLM 不可用文案 for 'partial'", () => {
    expect(mapKcEnrichedToLabel("partial")).toEqual({
      label: "AI 增强：仅规则标签（LLM 不可用）",
      tone: "partial",
    });
  });

  it("returns inactive tone + 未启用文案 for 'false'", () => {
    expect(mapKcEnrichedToLabel("false")).toEqual({
      label: "未启用 AI 增强",
      tone: "inactive",
    });
  });

  it("returns null for null/undefined (历史数据整行隐藏)", () => {
    expect(mapKcEnrichedToLabel(null)).toBeNull();
    expect(mapKcEnrichedToLabel(undefined)).toBeNull();
  });

  it("returns null for 未识别字面值 (脏数据 fail-safe)", () => {
    expect(mapKcEnrichedToLabel("yes")).toBeNull();
    expect(mapKcEnrichedToLabel("")).toBeNull();
    expect(mapKcEnrichedToLabel("TRUE")).toBeNull();
  });
});
