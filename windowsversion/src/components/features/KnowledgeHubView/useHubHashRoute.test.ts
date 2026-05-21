/**
 * useHubHashRoute / migrateLegacyHash / parseHubStep 单元测试
 * 覆盖 PRD AC-12（旧 hash 重定向）+ AC-13（前进/后退）
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { act, renderHook } from "@testing-library/react";
import {
  migrateLegacyHash,
  parseHubStep,
  useHubHashRoute,
} from "./useHubHashRoute";

function setHash(h: string) {
  // 直接改 history 状态，避免触发 hashchange 副作用污染测试
  window.history.replaceState(null, "", h || "/");
}

describe("parseHubStep", () => {
  it("解析 #/knowledge-hub/assets → 'assets'", () => {
    expect(parseHubStep("#/knowledge-hub/assets")).toBe("assets");
  });
  it("解析 #/knowledge-hub/concepts → 'concepts'", () => {
    expect(parseHubStep("#/knowledge-hub/concepts")).toBe("concepts");
  });
  it("空 hash → 默认 'concepts'（v1.3 task_007 KH-01：DEFAULT_HUB_STEP 改为 concepts）", () => {
    expect(parseHubStep("")).toBe("concepts");
  });
  it("非 hub hash → 默认 'concepts'", () => {
    expect(parseHubStep("#/foo")).toBe("concepts");
  });
  it("未知 step → 降级 'concepts'", () => {
    expect(parseHubStep("#/knowledge-hub/bogus")).toBe("concepts");
  });
});

describe("migrateLegacyHash", () => {
  it("#/skills → #/knowledge-hub/skills + applySidebarSection", () => {
    expect(migrateLegacyHash("#/skills")).toEqual({
      nextHash: "#/knowledge-hub/skills",
      applySidebarSection: true,
    });
  });
  it("#/knowledge → #/knowledge-hub/library + applySidebarSection", () => {
    expect(migrateLegacyHash("#/knowledge")).toEqual({
      nextHash: "#/knowledge-hub/library",
      applySidebarSection: true,
    });
  });
  it("#/knowledge-hub (无 step) → 补 concepts（v1.3 task_007 KH-01）", () => {
    expect(migrateLegacyHash("#/knowledge-hub")).toEqual({
      nextHash: "#/knowledge-hub/concepts",
      applySidebarSection: false,
    });
  });
  it("#/knowledge-hub/<bad> → 降级到 concepts + warnReason", () => {
    const r = migrateLegacyHash("#/knowledge-hub/bogus");
    expect(r?.nextHash).toBe("#/knowledge-hub/concepts");
    expect(r?.applySidebarSection).toBe(false);
    expect(r?.warnReason).toMatch(/bogus/);
  });
  it("#/foo → 不动 (null)", () => {
    expect(migrateLegacyHash("#/foo")).toBeNull();
  });
  it("#/knowledge-hub/skills → 已规范，不重复迁移 (null)", () => {
    expect(migrateLegacyHash("#/knowledge-hub/skills")).toBeNull();
  });
});

describe("useHubHashRoute", () => {
  beforeEach(() => {
    setHash("");
  });
  afterEach(() => {
    setHash("");
    vi.restoreAllMocks();
  });

  it("挂载时按当前 hash 解析 step（深链支持 AC-13 / input AC-3）", () => {
    setHash("#/knowledge-hub/concepts");
    const { result } = renderHook(() => useHubHashRoute());
    expect(result.current.step).toBe("concepts");
  });

  it("默认值（hash 为空）→ 'concepts'（v1.3 task_007 KH-01）", () => {
    setHash("");
    const { result } = renderHook(() => useHubHashRoute());
    expect(result.current.step).toBe("concepts");
  });

  it("setStep 触发 pushState 并更新 step", () => {
    setHash("#/knowledge-hub/assets");
    const pushSpy = vi.spyOn(window.history, "pushState");
    const { result } = renderHook(() => useHubHashRoute());
    act(() => {
      result.current.setStep("library");
    });
    expect(result.current.step).toBe("library");
    expect(pushSpy).toHaveBeenCalledWith(null, "", "#/knowledge-hub/library");
    expect(window.location.hash).toBe("#/knowledge-hub/library");
  });

  it("popstate 事件 → step 跟随 history 同步（AC-13 后退）", () => {
    setHash("#/knowledge-hub/assets");
    const { result } = renderHook(() => useHubHashRoute());

    act(() => {
      // 模拟用户后退到 skills
      window.history.replaceState(null, "", "#/knowledge-hub/skills");
      window.dispatchEvent(new PopStateEvent("popstate"));
    });
    expect(result.current.step).toBe("skills");
  });

  it("旧 hash #/skills 启动时 replaceState 重定向 + 触发 onLegacyMigrated", () => {
    setHash("#/skills");
    const onLegacyMigrated = vi.fn();
    const replaceSpy = vi.spyOn(window.history, "replaceState");
    const { result } = renderHook(() => useHubHashRoute({ onLegacyMigrated }));
    expect(window.location.hash).toBe("#/knowledge-hub/skills");
    expect(result.current.step).toBe("skills");
    expect(onLegacyMigrated).toHaveBeenCalledTimes(1);
    expect(replaceSpy).toHaveBeenCalled();
  });

  it("旧 hash #/knowledge → #/knowledge-hub/library", () => {
    setHash("#/knowledge");
    const onLegacyMigrated = vi.fn();
    const { result } = renderHook(() => useHubHashRoute({ onLegacyMigrated }));
    expect(window.location.hash).toBe("#/knowledge-hub/library");
    expect(result.current.step).toBe("library");
    expect(onLegacyMigrated).toHaveBeenCalledTimes(1);
  });

  it("非 hub hash 不被迁移（不污染其他视图路由）", () => {
    setHash("#/foo");
    const onLegacyMigrated = vi.fn();
    renderHook(() => useHubHashRoute({ onLegacyMigrated }));
    expect(window.location.hash).toBe("#/foo");
    expect(onLegacyMigrated).not.toHaveBeenCalled();
  });

  it("卸载后 popstate 不再更新内部 state（AC-6 cleanup）", () => {
    setHash("#/knowledge-hub/assets");
    const { result, unmount } = renderHook(() => useHubHashRoute());
    const stepBefore = result.current.step;
    unmount();
    act(() => {
      window.history.replaceState(null, "", "#/knowledge-hub/skills");
      window.dispatchEvent(new PopStateEvent("popstate"));
    });
    // 已卸载：返回的 result 仍是最后一次的 step（assets），不会变成 skills
    expect(result.current.step).toBe(stepBefore);
  });
});
