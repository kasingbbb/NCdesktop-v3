/**
 * task_010 / ADR-006 — computeInitialTodayTab 三态行为矩阵单元测试。
 *
 * 覆盖 input.md AC-1 全部 6 个组合。
 */
import { describe, it, expect } from "vitest";
import { computeInitialTodayTab } from "../initialTab";

describe("computeInitialTodayTab (ADR-006 三态)", () => {
  it("首次：lastTab=null, justEnabled=false → 'course-prep'", () => {
    expect(computeInitialTodayTab(null, false)).toBe("course-prep");
  });

  it("首次 + JustEnabled：lastTab=null, justEnabled=true → 'course-prep'", () => {
    expect(computeInitialTodayTab(null, true)).toBe("course-prep");
  });

  it("后续 course-prep：lastTab='course-prep', justEnabled=false → 'course-prep'", () => {
    expect(computeInitialTodayTab("course-prep", false)).toBe("course-prep");
  });

  it("后续 daily-review：lastTab='daily-review', justEnabled=false → 'daily-review'", () => {
    expect(computeInitialTodayTab("daily-review", false)).toBe("daily-review");
  });

  it("JustEnabled 不影响 course-prep：lastTab='course-prep', justEnabled=true → 'course-prep'", () => {
    expect(computeInitialTodayTab("course-prep", true)).toBe("course-prep");
  });

  it("JustEnabled 强制重置：lastTab='daily-review', justEnabled=true → 'course-prep'", () => {
    expect(computeInitialTodayTab("daily-review", true)).toBe("course-prep");
  });

  it("纯函数：多次调用结果一致，无副作用", () => {
    const a = computeInitialTodayTab("daily-review", false);
    const b = computeInitialTodayTab("daily-review", false);
    expect(a).toBe(b);
    expect(a).toBe("daily-review");
  });
});
