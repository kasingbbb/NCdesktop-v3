/**
 * task_006 — ContentArea 学习模式渲染防御（PRD F-P0-10 / AC-4 / 不可妥协底线 8）。
 *
 * 防御目标：即便 activeSidebarSection 滞留在 'today' / 'calendar' 而 settings.showLearningFeatures
 * 已为 false，ContentArea 也**绝对不能**挂载 TodayView / CalendarWeekView，应直接 fallback
 * 到末尾的 AssetPreview 分支（杜绝"颠倒视图悬空"中间帧）。
 *
 * 注：AppLayout 兜底 effect 会把 section 拉回 recent，但 ContentArea 这层渲染防御是独立
 * 不变量 —— 即使 AppLayout 未 mount（直接渲染 ContentArea 的测试 / 未来 SSR）也不能漏。
 */
import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("../../lib/tauri-commands", () => {
  const kv: Record<string, string> = {};
  return {
    setSetting: vi.fn(async (k: string, v: string) => {
      kv[k] = v;
    }),
    getAllSettings: vi.fn(async () => ({ ...kv })),
  };
});

// 子组件全部 mock 成可识别的 testid，便于 DOM 断言。
vi.mock("./Toolbar", () => ({ Toolbar: () => <div data-testid="toolbar" /> }));
vi.mock("../features/ProjectListView", () => ({ ProjectListView: () => <div data-testid="project-list-view" /> }));
vi.mock("../features/AssetListView", () => ({ AssetListView: () => <div data-testid="asset-list-view" /> }));
vi.mock("../features/AssetPreview", () => ({ AssetPreview: () => <div data-testid="asset-preview" /> }));
vi.mock("../features/preview/CoursePreviewSpace", () => ({
  CoursePreviewSpace: () => <div data-testid="course-preview-space" />,
}));
vi.mock("../features/calendar/CalendarWeekView", () => ({
  CalendarWeekView: () => <div data-testid="calendar-week-view" />,
}));
vi.mock("../features/today/TodayView", () => ({
  TodayView: () => <div data-testid="today-view" />,
}));
vi.mock("../features/KnowledgeHubView", () => ({
  KnowledgeHubView: () => <div data-testid="knowledge-hub-view" />,
}));

import { ContentArea } from "./ContentArea";
import { useUIStore } from "../../stores/uiStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { useLibraryStore } from "../../stores/libraryStore";

const INITIAL_SETTINGS = useSettingsStore.getState().settings;
const INITIAL_UI = useUIStore.getState();

describe("ContentArea — 学习模式渲染防御 (task_006 AC-4)", () => {
  beforeEach(() => {
    useSettingsStore.setState({ settings: { ...INITIAL_SETTINGS }, isLoading: false });
    useUIStore.setState({
      activeSidebarSection: "recent",
      rightPanelMode: "inspector",
      activeCourseEventId: null,
      inspectorOpen: INITIAL_UI.inspectorOpen,
    });
    useLibraryStore.setState({ activeLibraryId: "lib-test" });
  });

  it("show=false + section='today' → 不挂载 TodayView", () => {
    useUIStore.setState({ activeSidebarSection: "today" });
    useSettingsStore.setState({
      settings: { ...INITIAL_SETTINGS, showLearningFeatures: false },
    });
    render(<ContentArea />);
    expect(screen.queryByTestId("today-view")).not.toBeInTheDocument();
    // 应 fallback 到末尾 AssetPreview 分支。
    expect(screen.getByTestId("asset-preview")).toBeInTheDocument();
  });

  it("show=false + section='calendar' → 不挂载 CalendarWeekView", () => {
    useUIStore.setState({ activeSidebarSection: "calendar" });
    useSettingsStore.setState({
      settings: { ...INITIAL_SETTINGS, showLearningFeatures: false },
    });
    render(<ContentArea />);
    expect(screen.queryByTestId("calendar-week-view")).not.toBeInTheDocument();
    expect(screen.getByTestId("asset-preview")).toBeInTheDocument();
  });

  it("show=true + section='today' → 正常挂载 TodayView", () => {
    useUIStore.setState({ activeSidebarSection: "today" });
    useSettingsStore.setState({
      settings: { ...INITIAL_SETTINGS, showLearningFeatures: true },
    });
    render(<ContentArea />);
    expect(screen.getByTestId("today-view")).toBeInTheDocument();
  });

  it("show=true + section='calendar' → 正常挂载 CalendarWeekView", () => {
    useUIStore.setState({ activeSidebarSection: "calendar" });
    useSettingsStore.setState({
      settings: { ...INITIAL_SETTINGS, showLearningFeatures: true },
    });
    render(<ContentArea />);
    expect(screen.getByTestId("calendar-week-view")).toBeInTheDocument();
  });

  it("show=false + section='recent' → 走 isLibraryView 分支（不受影响）", () => {
    useUIStore.setState({ activeSidebarSection: "recent" });
    render(<ContentArea />);
    // recent 默认无 activeProjectId → 渲染 ProjectListView。
    expect(screen.getByTestId("project-list-view")).toBeInTheDocument();
  });
});
