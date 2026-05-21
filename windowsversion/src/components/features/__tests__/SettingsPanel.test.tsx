/**
 * v2 Sidebar Redesign — SettingsPanel「学习功能」Tab 单元测试（task_007 / PRD F-P0-9 / F-P0-10）。
 *
 * 覆盖：
 *   - 学习 Tab 渲染：主开关 + 2 依赖开关 + 副文案（AC-1/2/3/4）
 *   - 主开关 OFF 时依赖开关 disabled，但底层真值不变（AC-3 + 不可妥协底线 1）
 *   - 主开关 ON→OFF→ON 来回切，依赖开关底层值保留（值不丢）
 *   - turnLearningOff 主路径：section=today + show=true → 立即 section=recent，rAF 后 show=false（AC-5）
 *   - turnLearningOff: section=recent → 不动 section，rAF 后 show=false
 *   - turnLearningOff: section=calendar → section=recent
 *   - turnLearningOff 不联动写两个依赖字段（PRD 底线 1）
 *   - turnLearningOn → show=true + _learningJustEnabled=true（AC-6）
 */
import { render, screen, fireEvent, act } from "@testing-library/react";
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

// 子 Tab 的重组件 mock 掉，避免拖入无关依赖。
vi.mock("../bridge/LLMSettingsForm", () => ({
  LLMSettingsForm: () => <div data-testid="llm-form-mock" />,
}));
vi.mock("../calendar/CalendarImportTab", () => ({
  CalendarImportTab: () => <div data-testid="calendar-import-mock" />,
}));
vi.mock("../../settings/CategoryManager", () => ({
  CategoryManager: () => <div data-testid="category-manager-mock" />,
}));
vi.mock("../../settings/PromptEditor", () => ({
  PromptEditor: () => <div data-testid="prompt-editor-mock" />,
}));
vi.mock("../../settings/PromptCustomizationPanel", () => ({
  PromptCustomizationPanel: () => (
    <div data-testid="prompt-custom-panel-mock" />
  ),
}));

import { SettingsPanel, turnLearningOff, turnLearningOn } from "../SettingsPanel";
import { useSettingsStore } from "../../../stores/settingsStore";
import { useUIStore } from "../../../stores/uiStore";

const INITIAL_SETTINGS = useSettingsStore.getState().settings;
const INITIAL_UI = useUIStore.getState();

beforeEach(() => {
  useSettingsStore.setState({ settings: { ...INITIAL_SETTINGS }, isLoading: false });
  useUIStore.setState({
    ...INITIAL_UI,
    activeSidebarSection: "recent",
    _learningJustEnabled: false,
  });
  vi.clearAllMocks();
});

/** 等待一帧 rAF（turnLearningOff 时序测试用）。 */
async function nextFrame(): Promise<void> {
  await new Promise<void>((r) => requestAnimationFrame(() => r()));
}

function openLearningTab() {
  render(<SettingsPanel onClose={() => {}} />);
  fireEvent.click(screen.getByRole("button", { name: /学习功能/ }));
}

describe("SettingsPanel — 学习功能 Tab 渲染（AC-1/2/3/4）", () => {
  it("点击侧栏「学习功能」后渲染主开关 + 2 依赖开关 + 副文案", () => {
    openLearningTab();

    expect(screen.getByRole("switch", { name: "启用学习功能" })).toBeInTheDocument();
    expect(screen.getByRole("switch", { name: "绑定校历" })).toBeInTheDocument();
    expect(screen.getByRole("switch", { name: "每日复习提醒" })).toBeInTheDocument();
    expect(screen.getByTestId("learning-tab-helper-text")).toHaveTextContent(
      "关闭不会清除你的复习记录与课程关联",
    );
  });

  it("主开关 OFF 时，2 个依赖开关 disabled + aria-disabled=true，但底层真值显示原样", () => {
    // 注入：主关 OFF + 两个依赖底层为 true（模拟"曾经开过"）。
    useSettingsStore.setState({
      settings: {
        ...INITIAL_SETTINGS,
        showLearningFeatures: false,
        bindSchoolCalendar: true,
        enableDailyReviewReminder: true,
      },
    });

    openLearningTab();

    const bind = screen.getByRole("switch", { name: "绑定校历" });
    const daily = screen.getByRole("switch", { name: "每日复习提醒" });

    expect(bind).toBeDisabled();
    expect(daily).toBeDisabled();
    expect(bind).toHaveAttribute("aria-disabled", "true");
    expect(daily).toHaveAttribute("aria-disabled", "true");

    // 依赖开关 checked 状态显示底层真值（向用户证明"值还在"）。
    expect(bind).toHaveAttribute("aria-checked", "true");
    expect(daily).toHaveAttribute("aria-checked", "true");
  });

  it("主开关 OFF 状态下点击依赖开关不应触发 updateSetting（disabled 拦截）", () => {
    useSettingsStore.setState({
      settings: {
        ...INITIAL_SETTINGS,
        showLearningFeatures: false,
        bindSchoolCalendar: true,
      },
    });
    openLearningTab();

    const bind = screen.getByRole("switch", { name: "绑定校历" });
    fireEvent.click(bind);

    // 底层真值不变（disabled 不写入）。
    expect(useSettingsStore.getState().settings.bindSchoolCalendar).toBe(true);
  });
});

describe("turnLearningOff — rAF 时序（AC-5 / 不可妥协底线 8）", () => {
  it("section=today + show=true → 立即 section=recent，rAF 后 show=false", async () => {
    useSettingsStore.setState({
      settings: { ...INITIAL_SETTINGS, showLearningFeatures: true },
    });
    useUIStore.setState({ ...INITIAL_UI, activeSidebarSection: "today" });

    const p = turnLearningOff();

    // 同步：section 已切换为 recent。
    expect(useUIStore.getState().activeSidebarSection).toBe("recent");
    // 同步：showLearningFeatures 还**没**变（仍 true，待 rAF）。
    expect(useSettingsStore.getState().settings.showLearningFeatures).toBe(true);

    await p;

    expect(useSettingsStore.getState().settings.showLearningFeatures).toBe(false);
    // section 保持 recent，无空白页中间态。
    expect(useUIStore.getState().activeSidebarSection).toBe("recent");
  });

  it("section=calendar + show=true → 立即 section=recent，rAF 后 show=false", async () => {
    useSettingsStore.setState({
      settings: { ...INITIAL_SETTINGS, showLearningFeatures: true },
    });
    useUIStore.setState({ ...INITIAL_UI, activeSidebarSection: "calendar" });

    const p = turnLearningOff();
    expect(useUIStore.getState().activeSidebarSection).toBe("recent");
    await p;
    expect(useSettingsStore.getState().settings.showLearningFeatures).toBe(false);
  });

  it("section=recent + show=true → section 不动，rAF 后 show=false（直走非跳转路径）", async () => {
    useSettingsStore.setState({
      settings: { ...INITIAL_SETTINGS, showLearningFeatures: true },
    });
    useUIStore.setState({ ...INITIAL_UI, activeSidebarSection: "recent" });

    await turnLearningOff();

    expect(useUIStore.getState().activeSidebarSection).toBe("recent");
    expect(useSettingsStore.getState().settings.showLearningFeatures).toBe(false);
  });

  it("turnLearningOff **不**联动清零 bindSchoolCalendar / enableDailyReviewReminder（PRD 底线 1）", async () => {
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
    expect(s.bindSchoolCalendar).toBe(true); // 内存真值保留
    expect(s.enableDailyReviewReminder).toBe(true);
  });
});

describe("turnLearningOn（AC-6）", () => {
  it("OFF → ON：show=true + _learningJustEnabled=true，section 不动", async () => {
    useSettingsStore.setState({
      settings: { ...INITIAL_SETTINGS, showLearningFeatures: false },
    });
    useUIStore.setState({
      ...INITIAL_UI,
      activeSidebarSection: "starred",
      _learningJustEnabled: false,
    });

    await turnLearningOn();

    expect(useSettingsStore.getState().settings.showLearningFeatures).toBe(true);
    expect(useUIStore.getState()._learningJustEnabled).toBe(true);
    expect(useUIStore.getState().activeSidebarSection).toBe("starred");
  });
});

describe("依赖开关真值不丢（OFF→ON→OFF 来回切）", () => {
  it("用户先开主关并打开 bindSchoolCalendar，关主关，再开主关，依赖真值保留", async () => {
    // 起点：主关 OFF。
    useSettingsStore.setState({
      settings: { ...INITIAL_SETTINGS },
    });

    // 1) 主关 ON
    await turnLearningOn();
    expect(useSettingsStore.getState().settings.showLearningFeatures).toBe(true);

    // 2) 打开 bindSchoolCalendar
    await useSettingsStore.getState().updateSetting("bindSchoolCalendar", true);
    expect(useSettingsStore.getState().settings.bindSchoolCalendar).toBe(true);

    // 3) 关主关（在 recent 路径，rAF）
    await turnLearningOff();
    expect(useSettingsStore.getState().settings.showLearningFeatures).toBe(false);
    expect(useSettingsStore.getState().settings.bindSchoolCalendar).toBe(true); // 仍在

    // 4) 再开主关：依赖真值仍是 true，可立即生效。
    await turnLearningOn();
    expect(useSettingsStore.getState().settings.showLearningFeatures).toBe(true);
    expect(useSettingsStore.getState().settings.bindSchoolCalendar).toBe(true);
  });

  it("UI 触发：点击主开关 ON → 再 OFF → bindSchoolCalendar 底层值不丢", async () => {
    useSettingsStore.setState({
      settings: {
        ...INITIAL_SETTINGS,
        showLearningFeatures: true,
        bindSchoolCalendar: true,
      },
    });
    useUIStore.setState({ ...INITIAL_UI, activeSidebarSection: "recent" });

    openLearningTab();

    // 主开关：当前 ON，点击 → 走 turnLearningOff。
    const main = screen.getByRole("switch", { name: "启用学习功能" });
    await act(async () => {
      fireEvent.click(main);
      await nextFrame();
      // 给 rAF 内的 await updateSetting promise 一次解析机会。
      await Promise.resolve();
    });

    expect(useSettingsStore.getState().settings.showLearningFeatures).toBe(false);
    expect(useSettingsStore.getState().settings.bindSchoolCalendar).toBe(true);
  });
});
