/**
 * task_010 / ADR-006 — TodayView 内部 Tab 行为集成测试。
 *
 * 覆盖：
 *   - AC-3 首次进入 → Tab = course-prep
 *   - AC-4 切到 daily-review → todayLastTab 持久化为 'daily-review'
 *   - AC-5 再次 mount（_learningJustEnabled=false） → Tab = daily-review（恢复用户上次）
 *   - JustEnabled 路径：mount 时 _learningJustEnabled=true → 强制 course-prep
 *     + 信号被消费写回 false + 不污染 todayLastTab
 *   - AC-2 条件渲染：未激活 panel 不在 DOM 中
 *
 * tauri-commands mock 掉，避免 IPC；CoursePrepPanel 内部 load() 走 resolved Promise。
 */
import { render, screen, fireEvent, act, cleanup } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

vi.mock("../../../../lib/tauri-commands", () => ({
  kuGetList: vi.fn(async () => []),
  kuGetDueForReview: vi.fn(async () => []),
}));

import { TodayView } from "../TodayView";
import { useUIStore } from "../../../../stores/uiStore";

const INITIAL = {
  todayLastTab: useUIStore.getState().todayLastTab,
  _learningJustEnabled: useUIStore.getState()._learningJustEnabled,
};

async function flushAsync() {
  // CoursePrepPanel 内有 load() resolved Promise，需要 microtask 排空。
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
  });
}

beforeEach(() => {
  localStorage.removeItem("ui-store");
  useUIStore.setState({
    todayLastTab: INITIAL.todayLastTab,
    _learningJustEnabled: INITIAL._learningJustEnabled,
  });
});

afterEach(() => {
  cleanup();
});

describe("TodayView Tab — 首次挂载（AC-3）", () => {
  it("todayLastTab=null + _learningJustEnabled=false → 默认渲染 course-prep panel", async () => {
    useUIStore.setState({ todayLastTab: null, _learningJustEnabled: false });

    render(<TodayView libraryId="lib-1" />);
    await flushAsync();

    expect(screen.getByTestId("tdv-panel-course-prep")).toBeTruthy();
    expect(screen.queryByTestId("tdv-panel-daily-review")).toBeNull();

    // 首次路径不应写 todayLastTab（保持 null）。
    expect(useUIStore.getState().todayLastTab).toBeNull();
    // 信号本就是 false，保持 false。
    expect(useUIStore.getState()._learningJustEnabled).toBe(false);
  });
});

describe("TodayView Tab — 切换写入 todayLastTab（AC-4）", () => {
  it("点击 daily-review → todayLastTab='daily-review' + 仅 daily-review panel 渲染", async () => {
    useUIStore.setState({ todayLastTab: null, _learningJustEnabled: false });

    render(<TodayView libraryId="lib-1" />);
    await flushAsync();

    fireEvent.click(screen.getByTestId("tdv-tab-daily-review"));

    expect(useUIStore.getState().todayLastTab).toBe("daily-review");
    expect(screen.getByTestId("tdv-panel-daily-review")).toBeTruthy();
    // AC-2 条件渲染：未激活 panel 不在 DOM 中。
    expect(screen.queryByTestId("tdv-panel-course-prep")).toBeNull();
  });

  it("同 Tab 重复点击不重复写 store（短路）", async () => {
    useUIStore.setState({ todayLastTab: null, _learningJustEnabled: false });

    render(<TodayView libraryId="lib-1" />);
    await flushAsync();

    // 默认就在 course-prep；再点 course-prep。
    fireEvent.click(screen.getByTestId("tdv-tab-course-prep"));
    // 仍然不写（用户没有真正切换）。
    expect(useUIStore.getState().todayLastTab).toBeNull();
  });
});

describe("TodayView Tab — 再次 mount 恢复用户上次（AC-5）", () => {
  it("todayLastTab='daily-review' + _learningJustEnabled=false → 直接渲染 daily-review", async () => {
    useUIStore.setState({ todayLastTab: "daily-review", _learningJustEnabled: false });

    render(<TodayView libraryId="lib-1" />);
    await flushAsync();

    expect(screen.getByTestId("tdv-panel-daily-review")).toBeTruthy();
    expect(screen.queryByTestId("tdv-panel-course-prep")).toBeNull();
    // 后续路径不写 todayLastTab。
    expect(useUIStore.getState().todayLastTab).toBe("daily-review");
  });
});

describe("TodayView Tab — JustEnabled 路径（ADR-006 强制重置 + 一次性消费）", () => {
  it("todayLastTab='daily-review' + _learningJustEnabled=true → 强制 course-prep + 信号被消费 + 不污染 lastTab", async () => {
    useUIStore.setState({ todayLastTab: "daily-review", _learningJustEnabled: true });

    render(<TodayView libraryId="lib-1" />);
    await flushAsync();

    // 强制 course-prep 渲染。
    expect(screen.getByTestId("tdv-panel-course-prep")).toBeTruthy();
    expect(screen.queryByTestId("tdv-panel-daily-review")).toBeNull();
    // 信号被消费写回 false。
    expect(useUIStore.getState()._learningJustEnabled).toBe(false);
    // 不污染 todayLastTab：用户上次原值仍是 daily-review。
    expect(useUIStore.getState().todayLastTab).toBe("daily-review");
  });

  it("JustEnabled 消费后第二次 mount → 走后续路径恢复 daily-review", async () => {
    useUIStore.setState({ todayLastTab: "daily-review", _learningJustEnabled: true });

    const { unmount } = render(<TodayView libraryId="lib-1" />);
    await flushAsync();

    expect(useUIStore.getState()._learningJustEnabled).toBe(false);
    unmount();

    // 第二次 mount：_learningJustEnabled 已是 false，应恢复 daily-review。
    render(<TodayView libraryId="lib-1" />);
    await flushAsync();

    expect(screen.getByTestId("tdv-panel-daily-review")).toBeTruthy();
    expect(screen.queryByTestId("tdv-panel-course-prep")).toBeNull();
  });

  it("todayLastTab=null + _learningJustEnabled=true → course-prep + 信号消费 + lastTab 仍 null", async () => {
    useUIStore.setState({ todayLastTab: null, _learningJustEnabled: true });

    render(<TodayView libraryId="lib-1" />);
    await flushAsync();

    expect(screen.getByTestId("tdv-panel-course-prep")).toBeTruthy();
    expect(useUIStore.getState()._learningJustEnabled).toBe(false);
    expect(useUIStore.getState().todayLastTab).toBeNull();
  });
});

// ─── task_010 review_scorecard MAJOR 2：空状态 + 感性文案守门 ─────────────────
describe("TodayView — 空状态与感性文案守门（task_010 review_scorecard）", () => {
  it("ES-02：allUnits 全 0 时，tdv-stats-row 不渲染", async () => {
    useUIStore.setState({ todayLastTab: null, _learningJustEnabled: false });

    render(<TodayView libraryId="lib-1" />);
    await flushAsync();

    // course-prep panel 已渲染
    expect(await screen.findByTestId("tdv-panel-course-prep")).toBeTruthy();
    // 但统计行因 total=0 / validated=0 / mastered=0 而整行不渲染
    expect(screen.queryByTestId("tdv-stats-row")).toBeNull();
  });

  it("ES-02：prioritized 为空时，mainCard 区域显示 tdv-empty 空状态", async () => {
    useUIStore.setState({ todayLastTab: null, _learningJustEnabled: false });

    render(<TodayView libraryId="lib-1" />);
    await flushAsync();

    const empty = await screen.findByTestId("tdv-empty");
    expect(empty).toBeTruthy();
    expect(empty.textContent ?? "").toContain("今日无待处理");
  });

  it("ES-03：DOM 中不出现感性文案 🎉", async () => {
    useUIStore.setState({ todayLastTab: null, _learningJustEnabled: false });

    const { container } = render(<TodayView libraryId="lib-1" />);
    await flushAsync();

    // 确保异步加载完毕
    expect(await screen.findByTestId("tdv-panel-course-prep")).toBeTruthy();
    expect(container.innerHTML.includes("🎉")).toBe(false);
  });
});
