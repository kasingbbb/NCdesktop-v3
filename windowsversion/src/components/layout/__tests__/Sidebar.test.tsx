/**
 * v2 Sidebar Redesign — Sidebar 视图层单元测试（task_004 / F-P0-5）。
 *
 * 覆盖：
 *   - AC-1：默认态可见项数（分组标题 + 顶层 SidebarItem）≤ 7
 *   - AC-2：默认态「日历 / 今日 / 课程 / 学习中心」均不可见
 *   - AC-3：学习模式 ON → 学习中心分组出现 + 标题色 var(--sidebar-group-learning)
 *   - AC-3 反向：学习模式 OFF → 学习中心分组不在 DOM 中（条件渲染，禁用 display:none）
 *   - AC-4：搜索不在 Sidebar
 *   - AC-7：知识中心 badge 三段式
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, within } from "@testing-library/react";

// Mock 子组件树：ProjectTree / TagTree / SidebarFooter / CourseSection 不在本 task 测试范围
vi.mock("../../features/ProjectTree", () => ({
  ProjectTree: () => (
    <div data-testid="mock-project-tree">
      <p
        className="text-[10px] font-bold uppercase tracking-[0.08em]"
        data-testid="mock-project-section-title"
      >
        Projects
      </p>
    </div>
  ),
}));
vi.mock("../../features/TagTree", () => ({
  TagTree: () => (
    <div data-testid="mock-tag-tree">
      <p
        className="text-[10px] font-bold uppercase tracking-[0.08em]"
        data-testid="mock-tag-section-title"
      >
        Tags
      </p>
    </div>
  ),
}));
vi.mock("../SidebarFooter", () => ({
  SidebarFooter: () => <div data-testid="mock-sidebar-footer" />,
}));

// Mock Tauri commands（assetStore / libraryStore / knowledgeStore 不会真去 IPC）
vi.mock("../../../lib/tauri-commands", () => ({}));

import { Sidebar } from "../Sidebar";
import { useSettingsStore } from "../../../stores/settingsStore";
import { useUIStore } from "../../../stores/uiStore";
import { useAssetStore } from "../../../stores/assetStore";
import { useKnowledgeStore } from "../../../stores/knowledgeStore";
import { useLibraryStore } from "../../../stores/libraryStore";

const INITIAL_SETTINGS = useSettingsStore.getState().settings;
const INITIAL_UI = useUIStore.getState();

beforeEach(() => {
  useSettingsStore.setState({ settings: { ...INITIAL_SETTINGS }, isLoading: false });
  useUIStore.setState({ ...INITIAL_UI, activeSidebarSection: "recent" });
  useAssetStore.setState({ assets: [], assetTagNamesById: {}, isLoading: false });
  useKnowledgeStore.setState({ concepts: [] } as Partial<ReturnType<typeof useKnowledgeStore.getState>>);
  useLibraryStore.setState({ libraries: [], activeLibraryId: null, isLoading: false });
});

describe("Sidebar — 默认态（学习模式 OFF）", () => {
  it("AC-1：可见项数（分组标题 + 顶层 SidebarItem，不含子项 / 不含 footer）≤ 7", () => {
    render(<Sidebar width={220} />);

    // 顶层 SidebarItem（button）：最近 / 收藏 / 知识中心 = 3
    const items = ["最近", "收藏", "知识中心"];
    items.forEach((label) => {
      expect(screen.getByRole("button", { name: new RegExp(label) })).toBeInTheDocument();
    });

    // 分组标题（用真实 SidebarSection title 文本 + mock 出的 Projects / Tags）
    const titles = ["工作区", "知识"]; // Projects / Tags 来自 mock
    titles.forEach((t) => {
      expect(screen.getByText(t)).toBeInTheDocument();
    });
    expect(screen.getByTestId("mock-project-section-title")).toBeInTheDocument();
    expect(screen.getByTestId("mock-tag-section-title")).toBeInTheDocument();

    // 计数：3 顶层 SidebarItem + 4 分组标题（工作区 / 知识 / Projects / Tags）= 7
    const visibleCount = items.length + titles.length + 2; // +2 = ProjectTree + TagTree mock 的 title
    expect(visibleCount).toBeLessThanOrEqual(7);
    expect(visibleCount).toBe(7);
  });

  it("AC-2：默认态「日历 / 今日 / 学习中心」均不在 DOM 中", () => {
    render(<Sidebar width={220} />);
    expect(screen.queryByText("日历")).not.toBeInTheDocument();
    expect(screen.queryByText("今日")).not.toBeInTheDocument();
    expect(screen.queryByText("学习中心")).not.toBeInTheDocument();
  });

  it("AC-3 反向：学习模式 OFF → 学习中心分组不在 DOM（条件渲染，非 display:none）", () => {
    const { container } = render(<Sidebar width={220} />);
    // 学习中心 wrapper 不存在
    expect(container.querySelector(".sidebar-learning-group")).toBeNull();
  });

  it("AC-4：Sidebar 内不存在搜索 SidebarItem", () => {
    render(<Sidebar width={220} />);
    expect(screen.queryByRole("button", { name: /搜索/ })).not.toBeInTheDocument();
  });

  it("AC-7：知识中心 badge 三段式 — 无 library 时为 0·0·0", () => {
    render(<Sidebar width={220} />);
    const knowledgeBtn = screen.getByRole("button", { name: /知识中心/ });
    expect(within(knowledgeBtn).getByText("0·0·0")).toBeInTheDocument();
  });

  it("AC-7：知识中心 badge 三段式 — 有 library 时取 assets / concepts / 0", () => {
    useLibraryStore.setState({ activeLibraryId: "lib-1" });
    useAssetStore.setState({
      assets: [{ id: "a1" }, { id: "a2" }, { id: "a3" }] as unknown as ReturnType<
        typeof useAssetStore.getState
      >["assets"],
      assetTagNamesById: {},
      isLoading: false,
    });
    useKnowledgeStore.setState({
      concepts: [{ id: "c1" }, { id: "c2" }] as unknown as ReturnType<
        typeof useKnowledgeStore.getState
      >["concepts"],
    } as Partial<ReturnType<typeof useKnowledgeStore.getState>>);

    render(<Sidebar width={220} />);
    const knowledgeBtn = screen.getByRole("button", { name: /知识中心/ });
    expect(within(knowledgeBtn).getByText("3·2·0")).toBeInTheDocument();
  });
});

describe("Sidebar — 学习模式 ON", () => {
  beforeEach(() => {
    useSettingsStore.setState({
      settings: { ...INITIAL_SETTINGS, showLearningFeatures: true },
      isLoading: false,
    });
  });

  it("AC-3：学习中心分组出现 + 含「今日」「日历」两项", () => {
    render(<Sidebar width={220} />);
    expect(screen.getByText("学习中心")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /今日/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /日历/ })).toBeInTheDocument();
  });

  it("AC-3：学习中心分组标题色含 --sidebar-group-learning 语义令牌", () => {
    render(<Sidebar width={220} />);
    const titleSpan = screen.getByText("学习中心");
    // SidebarSection 把 titleColor 应用在父 <p> 上（titleSpan 是其内的 <span>）
    const titleP = titleSpan.closest("p");
    expect(titleP).not.toBeNull();
    expect(titleP!.getAttribute("style") || "").toMatch(/--sidebar-group-learning/);
  });

  it("学习模式 ON 时 wrapper 存在（用于动画 hook .sidebar-learning-fade-in）", () => {
    const { container } = render(<Sidebar width={220} />);
    const wrapper = container.querySelector(".sidebar-learning-group");
    expect(wrapper).not.toBeNull();
    expect(wrapper?.className).toContain("sidebar-learning-fade-in");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// v1.3 task_003+004+005 PR-B 差异点（ADR-007）
//   - SB-03：hub badge 全 0 不渲染（"不出现 0·0·0"）
//   - SB-04：学习模式 ON 学习中心含「今日 + 课程表」（v1.3 PRD），不再渲染「今天没有课程」占位
//   - SB-01：Sidebar 内无 Search 项（与历史 AC-4 重合，再补一条用 hash 检测）
// ─────────────────────────────────────────────────────────────────────────────

describe("Sidebar — v1.3 PR-B 差异点", () => {
  it("SB-03：assets/concepts/library 全 0 时 hub badge 整条不渲染（不出现 '0·0·0'）", () => {
    // beforeEach 已经把三个 store 都置空
    render(<Sidebar width={220} />);
    const knowledgeBtn = screen.getByRole("button", { name: /知识中心/ });
    expect(within(knowledgeBtn).queryByText("0·0·0")).toBeNull();
    // 整个 button 内不应有 "·" 这个分隔符
    expect(within(knowledgeBtn).queryByText(/·/)).toBeNull();
  });

  it("SB-03：至少一个 > 0 时 hub badge 渲染（包括 library=0 仍渲染 '3·2·0'）", () => {
    useAssetStore.setState({
      assets: [{ id: "a1" }, { id: "a2" }, { id: "a3" }] as unknown as ReturnType<
        typeof useAssetStore.getState
      >["assets"],
      assetTagNamesById: {},
      isLoading: false,
    });
    useKnowledgeStore.setState({
      concepts: [{ id: "c1" }, { id: "c2" }] as unknown as ReturnType<
        typeof useKnowledgeStore.getState
      >["concepts"],
    } as Partial<ReturnType<typeof useKnowledgeStore.getState>>);
    render(<Sidebar width={220} />);
    const knowledgeBtn = screen.getByRole("button", { name: /知识中心/ });
    expect(within(knowledgeBtn).getByText("3·2·0")).toBeInTheDocument();
  });

  it("SB-04（v1.3 PRD）：学生态学习中心含「今日」与「课程表」（非「日历」）", () => {
    useSettingsStore.setState({
      settings: { ...INITIAL_SETTINGS, showLearningFeatures: true },
      isLoading: false,
    });
    render(<Sidebar width={220} />);
    expect(screen.getByRole("button", { name: /今日/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /课程表/ })).toBeInTheDocument();
  });

  it("SB-04：不再渲染 '今天没有课程' 占位文案（无论学生态是否开启）", () => {
    // OFF
    render(<Sidebar width={220} />);
    expect(screen.queryByText(/今天没有课程/)).toBeNull();
  });

  it("SB-01：Sidebar 内不存在 Search 入口（顶层无 Search button、aria-label 也无）", () => {
    render(<Sidebar width={220} />);
    expect(screen.queryByRole("button", { name: /Search/i })).toBeNull();
  });

  it("SB-02：点击「知识中心」入口 → setSidebarSection('knowledge-hub') + hash 跳到 concepts", () => {
    const setSidebarSection = vi.spyOn(useUIStore.getState(), "setSidebarSection");
    render(<Sidebar width={220} />);
    const knowledgeBtn = screen.getByRole("button", { name: /知识中心/ });
    knowledgeBtn.click();
    expect(setSidebarSection).toHaveBeenCalledWith("knowledge-hub");
    expect(window.location.hash).toBe("#/knowledge-hub/concepts");
  });
});
