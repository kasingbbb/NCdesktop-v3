/**
 * task_009 / AC-1b — uiStore persist LocalStorage round-trip 集成矩阵。
 *
 * 与 uiStore.test.ts 的 smoke 互补：本文件聚焦"LS 写入旧值 → 重建 store → rehydrate
 * → 规范化后写回 LS"的端到端 round-trip（≥7 用例），并覆盖 task_002 reviewer 标注的
 * setter 入口 DEV warn 缺漏（C 段）。
 *
 * 实现要点：
 *   - 用 vi.resetModules() + 动态 import('../uiStore') 强制每个用例从零创建 store，
 *     从而真正触发 persist `migrate` 路径（而非复用模块级单例）。
 *   - 断言"LS 已被规范化"：rehydrate 后再读 LS，活跃 section 必须是新值。
 *     由于 zustand persist 的写回是异步的，rehydrate 完成后我们主动 set 一次以触发持久化。
 */
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

async function loadFreshStore() {
  vi.resetModules();
  const mod = await import("../uiStore");
  // 等 rehydrate 完成（zustand persist 的 hasHydrated 在挂载后异步置位）
  await mod.useUIStore.persist.rehydrate();
  return mod;
}

beforeEach(() => {
  localStorage.clear();
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("uiStore persist round-trip — 旧值兼容矩阵 (AC-1b)", () => {
  it("用例 1：旧值 'knowledge' → rehydrate 后 state='knowledge-hub'，LS 写回规范化", async () => {
    localStorage.setItem(
      "ui-store",
      JSON.stringify({ state: { activeSidebarSection: "knowledge" }, version: 0 }),
    );
    const { useUIStore } = await loadFreshStore();
    expect(useUIStore.getState().activeSidebarSection).toBe("knowledge-hub");

    // 触发一次写回，验证 LS 被规范化（而非保留 'knowledge'）。
    useUIStore.getState().setSidebarSection("knowledge-hub");
    const raw = JSON.parse(localStorage.getItem("ui-store")!);
    expect(raw.state.activeSidebarSection).toBe("knowledge-hub");
  });

  it("用例 2：旧值 'skills' → 'knowledge-hub'", async () => {
    localStorage.setItem(
      "ui-store",
      JSON.stringify({ state: { activeSidebarSection: "skills" }, version: 0 }),
    );
    const { useUIStore } = await loadFreshStore();
    expect(useUIStore.getState().activeSidebarSection).toBe("knowledge-hub");
  });

  it("用例 3：已删除值 'search' → 'recent'", async () => {
    localStorage.setItem(
      "ui-store",
      JSON.stringify({ state: { activeSidebarSection: "search" }, version: 0 }),
    );
    const { useUIStore } = await loadFreshStore();
    expect(useUIStore.getState().activeSidebarSection).toBe("recent");
  });

  it("用例 4：未知字符串 'unknown_xxx' → 'recent' + DEV warn", async () => {
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    localStorage.setItem(
      "ui-store",
      JSON.stringify({ state: { activeSidebarSection: "unknown_xxx" }, version: 0 }),
    );
    const { useUIStore } = await loadFreshStore();
    expect(useUIStore.getState().activeSidebarSection).toBe("recent");
    expect(warnSpy).toHaveBeenCalled();
    const msgs = warnSpy.mock.calls.map((c) => String(c[0] ?? ""));
    expect(msgs.some((m) => m.includes("unknown_xxx"))).toBe(true);
  });

  it("用例 5：类型错误（数字 / 布尔 / 对象）→ 'recent'", async () => {
    for (const bad of [42, true, { foo: "bar" }]) {
      localStorage.setItem(
        "ui-store",
        JSON.stringify({ state: { activeSidebarSection: bad }, version: 0 }),
      );
      const { useUIStore } = await loadFreshStore();
      expect(useUIStore.getState().activeSidebarSection).toBe("recent");
      localStorage.clear();
    }
  });

  it("用例 6：todayLastTab 非法值 'garbage' → null（同时 section 仍正确）", async () => {
    localStorage.setItem(
      "ui-store",
      JSON.stringify({
        state: { activeSidebarSection: "recent", todayLastTab: "garbage" },
        version: 0,
      }),
    );
    const { useUIStore } = await loadFreshStore();
    expect(useUIStore.getState().todayLastTab).toBeNull();
    expect(useUIStore.getState().activeSidebarSection).toBe("recent");
  });

  it("用例 7：完全没有 LS → 默认值（recent / null / false 且 _learningJustEnabled 不持久）", async () => {
    localStorage.removeItem("ui-store");
    const { useUIStore } = await loadFreshStore();
    const s = useUIStore.getState();
    expect(s.activeSidebarSection).toBe("recent");
    expect(s.todayLastTab).toBeNull();
    expect(s._learningJustEnabled).toBe(false);
  });

  it("用例 8：合法 round-trip（recent + course-prep）保持原样", async () => {
    localStorage.setItem(
      "ui-store",
      JSON.stringify({
        state: { activeSidebarSection: "recent", todayLastTab: "course-prep" },
        version: 0,
      }),
    );
    const { useUIStore } = await loadFreshStore();
    expect(useUIStore.getState().activeSidebarSection).toBe("recent");
    expect(useUIStore.getState().todayLastTab).toBe("course-prep");
  });

  it("用例 9：partialize 白名单严格（_learningJustEnabled 永不进 LS）", async () => {
    const { useUIStore } = await loadFreshStore();
    useUIStore.getState().setLearningJustEnabled(true);
    useUIStore.getState().setSidebarSection("recent"); // 触发一次写
    const parsed = JSON.parse(localStorage.getItem("ui-store")!);
    expect(parsed.state).not.toHaveProperty("_learningJustEnabled");
    expect(parsed.state).not.toHaveProperty("inspectorOpen");
    expect(parsed.state).not.toHaveProperty("layoutMode");
  });
});

describe("setSidebarSection setter 入口 DEV warn（C 段，task_002 reviewer 标注 MINOR）", () => {
  it("setter 传入旧值 'knowledge' 走 migrate 路径，DEV warn 触发", async () => {
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    const { useUIStore } = await loadFreshStore();
    // @ts-expect-error 模拟运行时误传旧值
    useUIStore.getState().setSidebarSection("knowledge");
    expect(useUIStore.getState().activeSidebarSection).toBe("knowledge-hub");
    expect(warnSpy).toHaveBeenCalled();
  });

  it("setter 传入 'search' 触发 DEV warn + 降级 'recent'", async () => {
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    const { useUIStore } = await loadFreshStore();
    // @ts-expect-error 模拟运行时误传已删除值 'search'
    useUIStore.getState().setSidebarSection("search");
    expect(useUIStore.getState().activeSidebarSection).toBe("recent");
    expect(warnSpy).toHaveBeenCalled();
  });

  it("setter 传入未知值触发 DEV warn", async () => {
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    const { useUIStore } = await loadFreshStore();
    // @ts-expect-error 模拟运行时误传未知值，验证降级到 recent + DEV warn
    useUIStore.getState().setSidebarSection("totally-unknown");
    expect(useUIStore.getState().activeSidebarSection).toBe("recent");
    expect(warnSpy).toHaveBeenCalled();
  });

  it("setter 传入合法新值不 warn", async () => {
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    const { useUIStore } = await loadFreshStore();
    useUIStore.getState().setSidebarSection("knowledge-hub");
    useUIStore.getState().setSidebarSection("today");
    expect(warnSpy).not.toHaveBeenCalled();
  });
});
