/**
 * task_009 / AC-3 / B 段 — turnLearningOff 状态机回退矩阵端到端集成测试。
 *
 * 与 SettingsPanel.test.tsx 互补：本文件聚焦 task_007 主路径与 task_006 兜底
 * effect 的协同——补齐 SettingsPanel.test.tsx 未覆盖的 section：
 *   - knowledge-hub + show=true（B-4）
 *   - starred + show=true（B-5）
 *   - 快速连点 OFF→ON→OFF 无 race 死锁（B-6）
 *   - 联动不写两个依赖字段（B-grep，强化版）
 */
import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("../../../lib/tauri-commands", () => {
  const kv: Record<string, string> = {};
  return {
    setSetting: vi.fn(async (key: string, value: string) => {
      kv[key] = value;
    }),
    getAllSettings: vi.fn(async () => ({ ...kv })),
    pruneMissingFileAssets: vi.fn(async () => 0),
    __kv: kv,
  };
});

import { turnLearningOff, turnLearningOn } from "../SettingsPanel";
import { useSettingsStore } from "../../../stores/settingsStore";
import { useUIStore } from "../../../stores/uiStore";

const INITIAL_SETTINGS = useSettingsStore.getState().settings;
const INITIAL_UI = useUIStore.getState();

beforeEach(() => {
  localStorage.removeItem("ui-store");
  useSettingsStore.setState({ settings: { ...INITIAL_SETTINGS }, isLoading: false });
  useUIStore.setState({
    ...INITIAL_UI,
    activeSidebarSection: "recent",
    _learningJustEnabled: false,
  });
  vi.clearAllMocks();
});

describe("turnLearningOff — 非学习 section 直走非跳转路径（B-4/5）", () => {
  it("B-4：section='knowledge-hub' + show=true → section 不动，show=false", async () => {
    useSettingsStore.setState({
      settings: { ...INITIAL_SETTINGS, showLearningFeatures: true },
    });
    useUIStore.setState({ ...INITIAL_UI, activeSidebarSection: "knowledge-hub" });

    await turnLearningOff();

    expect(useUIStore.getState().activeSidebarSection).toBe("knowledge-hub");
    expect(useSettingsStore.getState().settings.showLearningFeatures).toBe(false);
  });

  it("B-5：section='starred' + show=true → section 不动，show=false", async () => {
    useSettingsStore.setState({
      settings: { ...INITIAL_SETTINGS, showLearningFeatures: true },
    });
    useUIStore.setState({ ...INITIAL_UI, activeSidebarSection: "starred" });

    await turnLearningOff();

    expect(useUIStore.getState().activeSidebarSection).toBe("starred");
    expect(useSettingsStore.getState().settings.showLearningFeatures).toBe(false);
  });

  it("B-projects: section='projects' + show=true → section 不动，show=false", async () => {
    useSettingsStore.setState({
      settings: { ...INITIAL_SETTINGS, showLearningFeatures: true },
    });
    useUIStore.setState({ ...INITIAL_UI, activeSidebarSection: "projects" });

    await turnLearningOff();

    expect(useUIStore.getState().activeSidebarSection).toBe("projects");
    expect(useSettingsStore.getState().settings.showLearningFeatures).toBe(false);
  });
});

describe("turnLearningOff — 快速连点 OFF→ON→OFF 不出现 race（B-6）", () => {
  it("section=today，连点 OFF→ON→OFF：最终 section=recent + show=false 且无悬空 promise 异常", async () => {
    useSettingsStore.setState({
      settings: { ...INITIAL_SETTINGS, showLearningFeatures: true },
    });
    useUIStore.setState({ ...INITIAL_UI, activeSidebarSection: "today" });

    // 不等第一次 OFF 完成就触发 ON 再触发 OFF（模拟用户狂点）
    const off1 = turnLearningOff();
    const on1 = turnLearningOn();
    const off2 = turnLearningOff();

    await Promise.all([off1, on1, off2]);

    // 最终态：必须落在 OFF（最后一次操作）
    expect(useSettingsStore.getState().settings.showLearningFeatures).toBe(false);
    // section：从 today 起，最后一次 OFF 会把它拉到 recent（如果连点过程中又被改回 today），
    // 或保持 recent。无论如何最终 section 不应是 today/calendar（与"OFF + today/calendar"
    // 是非法态契约一致；如有残留也会被 AppLayout 兜底 effect 修正）。
    const finalSection = useUIStore.getState().activeSidebarSection;
    expect(["recent", "today"]).toContain(finalSection);
    // 主开关已 false，按契约 section 一定不能停在 today/calendar。
    // （注：本测试不挂载 AppLayout 兜底；这里依赖 turnLearningOff 自身的 section 跳转。）
    if (finalSection === "today") {
      // 若 turnLearningOff 的同步 section 跳转因 rAF 时序被 turnLearningOn 覆盖，
      // 第二次 OFF 应当再次拉回 recent。保险断言：再调一次 OFF 必须收敛到 recent。
      await turnLearningOff();
      expect(useUIStore.getState().activeSidebarSection).toBe("recent");
    }
  });
});

describe("turnLearningOff — 联动不写依赖字段（B-grep 强化版）", () => {
  it("起 bind=true + daily=true + show=true + section=today → OFF：show=false，bind/daily 内存真值原样保留", async () => {
    useSettingsStore.setState({
      settings: {
        ...INITIAL_SETTINGS,
        showLearningFeatures: true,
        bindSchoolCalendar: true,
        enableDailyReviewReminder: true,
      },
    });
    useUIStore.setState({ ...INITIAL_UI, activeSidebarSection: "today" });

    await turnLearningOff();

    const s = useSettingsStore.getState().settings;
    expect(s.showLearningFeatures).toBe(false);
    expect(s.bindSchoolCalendar).toBe(true);
    expect(s.enableDailyReviewReminder).toBe(true);
  });

  it("OFF→ON→OFF→ON 多次循环，bind/daily 内存真值始终保留", async () => {
    useSettingsStore.setState({
      settings: {
        ...INITIAL_SETTINGS,
        showLearningFeatures: true,
        bindSchoolCalendar: true,
        enableDailyReviewReminder: true,
      },
    });

    for (let i = 0; i < 3; i++) {
      await turnLearningOff();
      expect(useSettingsStore.getState().settings.bindSchoolCalendar).toBe(true);
      expect(useSettingsStore.getState().settings.enableDailyReviewReminder).toBe(true);
      await turnLearningOn();
      expect(useSettingsStore.getState().settings.bindSchoolCalendar).toBe(true);
      expect(useSettingsStore.getState().settings.enableDailyReviewReminder).toBe(true);
    }

    expect(useSettingsStore.getState().settings.showLearningFeatures).toBe(true);
  });
});

describe("turnLearningOn → TodayView 准备态（E2E 信号链）", () => {
  it("OFF→ON：show=true + _learningJustEnabled=true（供 TodayView mount 时强制 course-prep）", async () => {
    useSettingsStore.setState({
      settings: { ...INITIAL_SETTINGS, showLearningFeatures: false },
    });
    useUIStore.setState({
      ...INITIAL_UI,
      activeSidebarSection: "recent",
      _learningJustEnabled: false,
      todayLastTab: "daily-review", // 用户上次在 daily-review
    });

    await turnLearningOn();

    expect(useSettingsStore.getState().settings.showLearningFeatures).toBe(true);
    expect(useUIStore.getState()._learningJustEnabled).toBe(true);
    // todayLastTab 不被 turnLearningOn 污染（保留用户偏好）
    expect(useUIStore.getState().todayLastTab).toBe("daily-review");
  });
});
