/**
 * v2 Sidebar Redesign — uiStore 单元测试（task_002）。
 *
 * 覆盖：
 *   - migrateLegacySection 全矩阵（ADR-001）
 *   - setSidebarSection setter 入口拦截幂等（AC-3）
 *   - persist 反序列化 round-trip smoke（AC-4：旧值进 LS → rehydrate → 规范化）
 *   - DEV warn 行为（仅 DEV 触发）
 *
 * 完整 LocalStorage round-trip 矩阵（≥5 用例）由 task_009 接管。
 */
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { migrateLegacySection, useUIStore } from "../uiStore";

// 保存初始默认值，便于每个用例后回滚。
const INITIAL_SECTION = useUIStore.getState().activeSidebarSection;
const INITIAL_TODAY = useUIStore.getState().todayLastTab;
const INITIAL_JE = useUIStore.getState()._learningJustEnabled;

beforeEach(() => {
  // 清掉 LocalStorage 持久化痕迹，避免测试间相互污染。
  localStorage.removeItem("ui-store");
  useUIStore.setState({
    activeSidebarSection: INITIAL_SECTION,
    todayLastTab: INITIAL_TODAY,
    _learningJustEnabled: INITIAL_JE,
  });
});

describe("migrateLegacySection (ADR-001 矩阵)", () => {
  it("合法新值原样返回（recent）", () => {
    expect(migrateLegacySection("recent")).toBe("recent");
  });

  it("合法新值原样返回（starred）", () => {
    expect(migrateLegacySection("starred")).toBe("starred");
  });

  it("合法新值原样返回（projects / tags / today / calendar）", () => {
    expect(migrateLegacySection("projects")).toBe("projects");
    expect(migrateLegacySection("tags")).toBe("tags");
    expect(migrateLegacySection("today")).toBe("today");
    expect(migrateLegacySection("calendar")).toBe("calendar");
  });

  it("合法新值原样返回（knowledge-hub）", () => {
    expect(migrateLegacySection("knowledge-hub")).toBe("knowledge-hub");
  });

  it("已弃用旧值 'knowledge' → 'knowledge-hub'", () => {
    expect(migrateLegacySection("knowledge")).toBe("knowledge-hub");
  });

  it("已弃用旧值 'skills' → 'knowledge-hub'", () => {
    expect(migrateLegacySection("skills")).toBe("knowledge-hub");
  });

  it("已删除值 'search' → 'recent'", () => {
    expect(migrateLegacySection("search")).toBe("recent");
  });

  it("null → 'recent'", () => {
    expect(migrateLegacySection(null)).toBe("recent");
  });

  it("undefined → 'recent'", () => {
    expect(migrateLegacySection(undefined)).toBe("recent");
  });

  it("未知字符串 → 'recent'", () => {
    expect(migrateLegacySection("totally-unknown-section")).toBe("recent");
    expect(migrateLegacySection("")).toBe("recent");
  });

  it("非 string 类型（number / object / array / boolean）→ 'recent'", () => {
    expect(migrateLegacySection(42)).toBe("recent");
    expect(migrateLegacySection({})).toBe("recent");
    expect(migrateLegacySection([])).toBe("recent");
    expect(migrateLegacySection(true)).toBe("recent");
    expect(migrateLegacySection(false)).toBe("recent");
  });
});

describe("DEV warn 行为", () => {
  let warnSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
  });

  afterEach(() => {
    warnSpy.mockRestore();
  });

  it("已弃用值 'knowledge' 触发 DEV warn", () => {
    migrateLegacySection("knowledge");
    expect(warnSpy).toHaveBeenCalled();
    const msg = String(warnSpy.mock.calls[0]?.[0] ?? "");
    expect(msg).toContain("[uiStore]");
    expect(msg).toContain("knowledge");
  });

  it("未知字符串触发 DEV warn", () => {
    migrateLegacySection("xxx-unknown");
    expect(warnSpy).toHaveBeenCalled();
  });

  it("非 string 触发 DEV warn", () => {
    migrateLegacySection(42);
    expect(warnSpy).toHaveBeenCalled();
  });

  it("null / undefined 不触发 warn（合法初始）", () => {
    migrateLegacySection(null);
    migrateLegacySection(undefined);
    expect(warnSpy).not.toHaveBeenCalled();
  });

  it("合法新值不触发 warn", () => {
    migrateLegacySection("recent");
    migrateLegacySection("knowledge-hub");
    expect(warnSpy).not.toHaveBeenCalled();
  });
});

describe("setSidebarSection setter 入口拦截 (AC-3)", () => {
  it("传入合法新值 → 写入原值（幂等）", () => {
    useUIStore.getState().setSidebarSection("knowledge-hub");
    expect(useUIStore.getState().activeSidebarSection).toBe("knowledge-hub");
  });

  it("传入旧值（如 Dev 误传）→ 自动规范化到新值", () => {
    // @ts-expect-error 模拟运行时 Dev 误传旧值
    useUIStore.getState().setSidebarSection("knowledge");
    expect(useUIStore.getState().activeSidebarSection).toBe("knowledge-hub");
  });

  it("传入已删除值 'search' → 降级到 'recent'", () => {
    // @ts-expect-error 模拟旧 LS / 旧调用面误传
    useUIStore.getState().setSidebarSection("search");
    expect(useUIStore.getState().activeSidebarSection).toBe("recent");
  });
});

describe("setTodayLastTab / setLearningJustEnabled (AC-4b)", () => {
  it("setTodayLastTab 写入 todayLastTab", () => {
    useUIStore.getState().setTodayLastTab("daily-review");
    expect(useUIStore.getState().todayLastTab).toBe("daily-review");
  });

  it("setLearningJustEnabled 写入 _learningJustEnabled", () => {
    useUIStore.getState().setLearningJustEnabled(true);
    expect(useUIStore.getState()._learningJustEnabled).toBe(true);
    useUIStore.getState().setLearningJustEnabled(false);
    expect(useUIStore.getState()._learningJustEnabled).toBe(false);
  });
});

describe("persist 默认值 (AC-4)", () => {
  it("初始 activeSidebarSection = 'recent'", () => {
    expect(useUIStore.getState().activeSidebarSection).toBe("recent");
  });

  it("初始 todayLastTab = null", () => {
    expect(useUIStore.getState().todayLastTab).toBe(null);
  });

  it("初始 _learningJustEnabled = false", () => {
    expect(useUIStore.getState()._learningJustEnabled).toBe(false);
  });
});

describe("persist round-trip smoke (AC-4 完整矩阵在 task_009)", () => {
  it("partialize 出口字段严格只含 {activeSidebarSection, todayLastTab, tagsExpanded}", () => {
    useUIStore.getState().setSidebarSection("knowledge-hub");
    useUIStore.getState().setTodayLastTab("course-prep");
    useUIStore.getState().setLearningJustEnabled(true);

    // 触发持久化（zustand persist 在 set 时同步写 LS）
    const raw = localStorage.getItem("ui-store");
    expect(raw).not.toBeNull();

    const parsed = JSON.parse(raw!);
    // zustand v5 persist 形态：{ state: {...partialize 出口...}, version: N }
    expect(parsed).toHaveProperty("state");
    expect(parsed.state).toEqual({
      activeSidebarSection: "knowledge-hub",
      todayLastTab: "course-prep",
      tagsExpanded: false,
    });
    // _learningJustEnabled 必须不在白名单内
    expect(parsed.state).not.toHaveProperty("_learningJustEnabled");
  });

  it("migrate 选项规范化旧值（'knowledge' → 'knowledge-hub'）", async () => {
    // 模拟 v0/老 LS（无 version 或 version 不一致），写入旧值
    localStorage.setItem(
      "ui-store",
      JSON.stringify({
        state: { activeSidebarSection: "knowledge", todayLastTab: "course-prep" },
        version: 0,
      }),
    );
    // 触发 rehydrate
    await useUIStore.persist.rehydrate();

    const s = useUIStore.getState();
    expect(s.activeSidebarSection).toBe("knowledge-hub");
    expect(s.todayLastTab).toBe("course-prep");
  });

  it("migrate 选项把非法 todayLastTab 降级为 null", async () => {
    localStorage.setItem(
      "ui-store",
      JSON.stringify({
        state: { activeSidebarSection: "recent", todayLastTab: "garbage-tab" },
        version: 0,
      }),
    );
    await useUIStore.persist.rehydrate();

    expect(useUIStore.getState().todayLastTab).toBe(null);
  });

  it("无 LS 时跳过 migrate，初始走默认 'recent' / null（AC-10）", async () => {
    localStorage.removeItem("ui-store");
    // rehydrate 在无存档时是 no-op
    await useUIStore.persist.rehydrate();

    expect(useUIStore.getState().activeSidebarSection).toBe("recent");
    expect(useUIStore.getState().todayLastTab).toBe(null);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// v1.3 task_002：tagsExpanded 字段（SB-07 / ADR-002）
// ─────────────────────────────────────────────────────────────────────────────

describe("tagsExpanded (v1.3 task_002 SB-07)", () => {
  it("AC-1 默认值为 false", () => {
    expect(useUIStore.getState().tagsExpanded).toBe(false);
  });

  it("AC-2 setTagsExpanded(true/false) toggle 正确", () => {
    useUIStore.getState().setTagsExpanded(true);
    expect(useUIStore.getState().tagsExpanded).toBe(true);
    useUIStore.getState().setTagsExpanded(false);
    expect(useUIStore.getState().tagsExpanded).toBe(false);
  });

  it("AC-3 partialize 出口包含 tagsExpanded", () => {
    useUIStore.getState().setTagsExpanded(true);
    const parsed = JSON.parse(localStorage.getItem("ui-store")!);
    expect(parsed.state).toHaveProperty("tagsExpanded", true);
  });

  it("AC-4 migrate 旧 LS（无 tagsExpanded 字段）→ rehydrate 后默认 false", async () => {
    localStorage.setItem(
      "ui-store",
      JSON.stringify({
        state: { activeSidebarSection: "recent" },
        version: 0,
      }),
    );
    await useUIStore.persist.rehydrate();
    expect(useUIStore.getState().tagsExpanded).toBe(false);
  });

  it("AC-4b migrate 'search' 老用户 → activeSidebarSection=recent 且 tagsExpanded=false", async () => {
    localStorage.setItem(
      "ui-store",
      JSON.stringify({
        state: { activeSidebarSection: "search" },
        version: 0,
      }),
    );
    await useUIStore.persist.rehydrate();
    const s = useUIStore.getState();
    expect(s.activeSidebarSection).toBe("recent");
    expect(s.tagsExpanded).toBe(false);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// task_006 T4：工作区文件夹列表编辑态 5 字段（PRD §5.2 / ADR-009）
// ─────────────────────────────────────────────────────────────────────────────

describe("workspace folder list edit state (task_006 T4)", () => {
  beforeEach(() => {
    // 复位 5 字段到初始值
    useUIStore.setState({
      editingFolderPath: null,
      pendingNewFolder: false,
      pendingRenameIds: new Set<string>(),
      dragOverPath: null,
    });
  });

  it("初始值：editingFolderPath=null / pendingNewFolder=false / pendingRenameIds=空 / dragOverPath=null", () => {
    const s = useUIStore.getState();
    expect(s.editingFolderPath).toBeNull();
    expect(s.pendingNewFolder).toBe(false);
    expect(s.pendingRenameIds.size).toBe(0);
    expect(s.dragOverPath).toBeNull();
  });

  it("startCreating 把 pendingNewFolder 设为 true 且清掉 editingFolderPath", () => {
    useUIStore.setState({ editingFolderPath: "foo" });
    useUIStore.getState().startCreating();
    expect(useUIStore.getState().pendingNewFolder).toBe(true);
    expect(useUIStore.getState().editingFolderPath).toBeNull();
  });

  it("cancelCreating 把 pendingNewFolder 设回 false", () => {
    useUIStore.getState().startCreating();
    useUIStore.getState().cancelCreating();
    expect(useUIStore.getState().pendingNewFolder).toBe(false);
  });

  it("startRenaming(path) 加入 pendingRenameIds 且返回新 Set 实例（zustand 浅比较）", () => {
    const before = useUIStore.getState().pendingRenameIds;
    useUIStore.getState().startRenaming("参考资料");
    const after = useUIStore.getState().pendingRenameIds;
    expect(after.has("参考资料")).toBe(true);
    expect(after).not.toBe(before); // 必须是新实例
    expect(useUIStore.getState().editingFolderPath).toBe("参考资料");
  });

  it("finishRename(path) 从 pendingRenameIds 移除并清 editing（若匹配）", () => {
    useUIStore.getState().startRenaming("foo");
    useUIStore.getState().startRenaming("bar");
    const mid = useUIStore.getState().pendingRenameIds;
    expect(mid.has("foo")).toBe(true);
    expect(mid.has("bar")).toBe(true);

    useUIStore.getState().finishRename("foo");
    const after = useUIStore.getState().pendingRenameIds;
    expect(after.has("foo")).toBe(false);
    expect(after.has("bar")).toBe(true);
    expect(after).not.toBe(mid);
  });

  it("finishRename 幂等：不存在的 path 也不抛错", () => {
    expect(() => useUIStore.getState().finishRename("ghost")).not.toThrow();
    expect(useUIStore.getState().pendingRenameIds.size).toBe(0);
  });

  it("setDragOverPath 覆盖并可清回 null", () => {
    useUIStore.getState().setDragOverPath("参考");
    expect(useUIStore.getState().dragOverPath).toBe("参考");
    useUIStore.getState().setDragOverPath(null);
    expect(useUIStore.getState().dragOverPath).toBeNull();
  });

  it("5 新字段**不进** partialize 白名单（持久化只含 activeSidebarSection / todayLastTab）", () => {
    useUIStore.getState().startCreating();
    useUIStore.getState().startRenaming("foo");
    useUIStore.getState().setDragOverPath("bar");

    const raw = localStorage.getItem("ui-store");
    expect(raw).not.toBeNull();
    const parsed = JSON.parse(raw!);
    expect(parsed.state).not.toHaveProperty("editingFolderPath");
    expect(parsed.state).not.toHaveProperty("pendingNewFolder");
    expect(parsed.state).not.toHaveProperty("pendingRenameIds");
    expect(parsed.state).not.toHaveProperty("dragOverPath");
  });
});
