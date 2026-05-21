/**
 * KnowledgeAssociationView — v1.3 task_009 fix + concept_rescan_perf_v1
 * task_perf_02_frontend 单测覆盖
 *
 * 覆盖：
 *   - v1.3 task_009：toggle / 合并按钮占位
 *   - task_perf_02 AC-1：5 状态进度条（启动中 / 进行中 / 完成 / 错误）
 *   - task_perf_02 AC-2：重新扫描按钮 running 时 disabled + aria-disabled + 文案
 *   - task_perf_02 AC-3：IPC 调用透传 forceFull
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";

// mock Tauri event listen（KnowledgeAssociationView 用它监听 extraction-progress）
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

// mock Tauri invoke（task_perf_02 AC-3：验证 forceFull 透传）
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue({
    totalAssets: 0,
    processed: 0,
    conceptsFound: 0,
    status: "completed",
  }),
}));

// mock 子组件：ConceptList 渲染真实合并按钮以便断言；ConceptDetailPanel 占位
vi.mock("../ConceptList", async () => {
  const actual = await vi.importActual<typeof import("../ConceptList")>("../ConceptList");
  return { ConceptList: actual.ConceptList };
});
vi.mock("../ConceptDetailPanel", () => ({
  ConceptDetailPanel: () => <div data-testid="mock-concept-detail" />,
}));
vi.mock("../../../KnowledgeUnderstanding/KnowledgeUnderstandingPage", () => ({
  KnowledgeUnderstandingPage: () => <div data-testid="mock-understanding-page" />,
}));

import { KnowledgeAssociationView } from "../KnowledgeAssociationView";
import { useKnowledgeStore } from "../../../../stores/knowledgeStore";
import { useLibraryStore } from "../../../../stores/libraryStore";
import { useKnowledgeUnderstandingStore } from "../../../../stores/knowledgeUnderstandingStore";

const INITIAL_KNOWLEDGE = useKnowledgeStore.getState();
const INITIAL_LIBRARY = useLibraryStore.getState();

beforeEach(() => {
  useKnowledgeStore.setState({
    ...INITIAL_KNOWLEDGE,
    concepts: [
      {
        id: "c1",
        libraryId: "lib-1",
        name: "测试概念 A",
        definition: "",
        sourceProjectCount: 0,
        viewpointCount: 0,
        userEdited: false,
      },
      {
        id: "c2",
        libraryId: "lib-1",
        name: "测试概念 B",
        definition: "",
        sourceProjectCount: 0,
        viewpointCount: 0,
        userEdited: false,
      },
    ] as unknown as ReturnType<typeof useKnowledgeStore.getState>["concepts"],
    selectedConceptId: null,
    conceptDetail: null,
    extractionProgress: null,
    searchQuery: "",
    isLoading: false,
    isLoadingDetail: false,
    error: null,
    fetchConcepts: vi.fn().mockResolvedValue(undefined),
    getFilteredConcepts: () =>
      (useKnowledgeStore.getState().concepts as unknown as Array<{ id: string }>),
  } as unknown as ReturnType<typeof useKnowledgeStore.getState>);

  useLibraryStore.setState({
    ...INITIAL_LIBRARY,
    activeLibraryId: "lib-1",
  });

  useKnowledgeUnderstandingStore.setState({
    conceptId: null,
  } as unknown as ReturnType<typeof useKnowledgeUnderstandingStore.getState>);
});

describe("KnowledgeAssociationView — v1.3 task_009 占位闭环", () => {
  it("AC-1：toggle 默认 aria-checked='true'", () => {
    render(<KnowledgeAssociationView />);
    const toggle = screen.getByTestId("knowledge-assoc-linked-toggle");
    expect(toggle.getAttribute("aria-checked")).toBe("true");
  });

  it("AC-7：toggle role='switch'", () => {
    render(<KnowledgeAssociationView />);
    const toggle = screen.getByRole("switch");
    expect(toggle).toBeTruthy();
    expect(toggle.getAttribute("data-testid")).toBe("knowledge-assoc-linked-toggle");
  });

  it("点击 toggle 切换 aria-checked 在 true/false 之间", () => {
    render(<KnowledgeAssociationView />);
    const toggle = screen.getByTestId("knowledge-assoc-linked-toggle");
    expect(toggle.getAttribute("aria-checked")).toBe("true");
    fireEvent.click(toggle);
    expect(toggle.getAttribute("aria-checked")).toBe("false");
    fireEvent.click(toggle);
    expect(toggle.getAttribute("aria-checked")).toBe("true");
  });

  it("AC-5/6：每个概念条目右侧合并按钮 disabled + data-merge-id 非空", () => {
    const { container } = render(<KnowledgeAssociationView />);
    // 直接按 data-merge-id 属性查找真正的 button（避开外层 div role=button 误命中）
    const mergeButtons = container.querySelectorAll("button[data-merge-id]");
    expect(mergeButtons.length).toBeGreaterThanOrEqual(2); // 2 个概念
    mergeButtons.forEach((btn) => {
      expect((btn as HTMLButtonElement).disabled).toBe(true);
      expect(btn.textContent).toMatch(/合并/);
      expect(btn.getAttribute("data-merge-id")).toBeTruthy();
      expect(btn.getAttribute("title")).toBe("v1.4 合并 modal 待开");
    });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// concept_rescan_perf_v1 / task_perf_02_frontend
// AC-1：5 状态进度条
// AC-2：按钮 running 态
// AC-3：IPC forceFull 透传
// ─────────────────────────────────────────────────────────────────────────────

describe("KnowledgeAssociationView — task_perf_02 进度条 5 状态", () => {
  it("AC-1 启动中：processed=0 且 totalAssets>0 时显示脉冲 + '正在处理首批文档' + 预估分钟数", () => {
    useKnowledgeStore.setState({
      extractionProgress: {
        totalAssets: 87,
        processed: 0,
        conceptsFound: 0,
        status: "running",
      },
    });
    render(<KnowledgeAssociationView />);
    const bar = screen.getByTestId("extraction-progress-bar");
    expect(bar.getAttribute("data-phase")).toBe("starting");
    // 容器加 animate-pulse 类名
    expect(bar.className).toMatch(/animate-pulse/);
    expect(bar.textContent).toMatch(/正在处理首批文档/);
    expect(bar.textContent).toMatch(/每篇约 60 秒/);
    // 87 文档，每篇 60 秒，4 路并发 → Math.ceil(87 * 60 / 4 / 60) = 22 分钟
    expect(bar.textContent).toMatch(/预估全量约 22 分钟/);
    expect(bar.textContent).toMatch(/4 路并发/);
  });

  it("AC-1 进行中：processed>0 显示真值进度 + 已发现概念数", () => {
    useKnowledgeStore.setState({
      extractionProgress: {
        totalAssets: 87,
        processed: 12,
        conceptsFound: 38,
        status: "running",
      },
    });
    render(<KnowledgeAssociationView />);
    const bar = screen.getByTestId("extraction-progress-bar");
    expect(bar.getAttribute("data-phase")).toBe("running");
    expect(bar.className).not.toMatch(/animate-pulse/);
    expect(bar.textContent).toMatch(/已处理 12\/87 个文档/);
    expect(bar.textContent).toMatch(/发现 38 个概念/);
  });

  it("AC-1 完成：status=completed 显示 '扫描完成 · 共发现 N 个概念'，不再显示进度条轨道", () => {
    useKnowledgeStore.setState({
      extractionProgress: {
        totalAssets: 87,
        processed: 87,
        conceptsFound: 42,
        status: "completed",
      },
    });
    const { container } = render(<KnowledgeAssociationView />);
    const bar = screen.getByTestId("extraction-progress-bar");
    expect(bar.getAttribute("data-phase")).toBe("completed");
    expect(bar.textContent).toMatch(/扫描完成/);
    expect(bar.textContent).toMatch(/共发现 42 个概念/);
    // 完成态不再渲染进度轨道（仅文案通知）
    const track = container.querySelector(
      '[data-testid="extraction-progress-bar"] .h-1\\.5'
    );
    expect(track).toBeNull();
  });

  it("AC-1 错误：status=error 显示 '扫描出错：{error}' + 红色提示", () => {
    useKnowledgeStore.setState({
      extractionProgress: {
        totalAssets: 0,
        processed: 0,
        conceptsFound: 0,
        status: "error",
        error: "LLM 调用失败：超时",
      },
    });
    render(<KnowledgeAssociationView />);
    const bar = screen.getByTestId("extraction-progress-bar");
    expect(bar.getAttribute("data-phase")).toBe("error");
    expect(bar.textContent).toMatch(/扫描出错：LLM 调用失败：超时/);
    // 错误态背景含红色 inline style
    expect(bar.getAttribute("style") || "").toMatch(/239,\s*68,\s*68/);
  });

  it("AC-1 错误兜底：error 字段缺失时显示 '扫描出错：未知错误'", () => {
    useKnowledgeStore.setState({
      extractionProgress: {
        totalAssets: 0,
        processed: 0,
        conceptsFound: 0,
        status: "error",
      },
    });
    render(<KnowledgeAssociationView />);
    expect(screen.getByTestId("extraction-progress-bar").textContent).toMatch(
      /扫描出错：未知错误/
    );
  });

  it("AC-1 preboot：status=running 但 totalAssets=0 时显示 '正在准备文档列表…'", () => {
    useKnowledgeStore.setState({
      extractionProgress: {
        totalAssets: 0,
        processed: 0,
        conceptsFound: 0,
        status: "running",
      },
    });
    render(<KnowledgeAssociationView />);
    const bar = screen.getByTestId("extraction-progress-bar");
    expect(bar.getAttribute("data-phase")).toBe("preboot");
    expect(bar.className).toMatch(/animate-pulse/);
    expect(bar.textContent).toMatch(/正在准备文档列表/);
  });
});

describe("KnowledgeAssociationView — task_perf_02 AC-2 按钮态", () => {
  it("running 时按钮 disabled + aria-disabled + 文案 '扫描中…' + title", () => {
    useKnowledgeStore.setState({
      extractionProgress: {
        totalAssets: 87,
        processed: 0,
        conceptsFound: 0,
        status: "running",
      },
    });
    render(<KnowledgeAssociationView />);
    const btn = screen.getByTestId("knowledge-assoc-rescan-button") as HTMLButtonElement;
    expect(btn.disabled).toBe(true);
    expect(btn.getAttribute("aria-disabled")).toBe("true");
    expect(btn.textContent).toMatch(/扫描中…/);
    expect(btn.getAttribute("title")).toBe("已有扫描任务在执行，请等待完成");
  });

  it("idle 时按钮可点击 + 文案 '重新扫描'", () => {
    useKnowledgeStore.setState({ extractionProgress: null });
    render(<KnowledgeAssociationView />);
    const btn = screen.getByTestId("knowledge-assoc-rescan-button") as HTMLButtonElement;
    expect(btn.disabled).toBe(false);
    expect(btn.getAttribute("aria-disabled")).toBe("false");
    expect(btn.textContent).toMatch(/重新扫描/);
    expect(btn.textContent).not.toMatch(/扫描中…/);
  });

  it("completed 后按钮恢复可点击", () => {
    useKnowledgeStore.setState({
      extractionProgress: {
        totalAssets: 87,
        processed: 87,
        conceptsFound: 42,
        status: "completed",
      },
    });
    render(<KnowledgeAssociationView />);
    const btn = screen.getByTestId("knowledge-assoc-rescan-button") as HTMLButtonElement;
    expect(btn.disabled).toBe(false);
    expect(btn.textContent).toMatch(/重新扫描/);
  });
});

describe("KnowledgeAssociationView — task_perf_04 IPC forceFull 透传（增量默认 + Shift 全量）", () => {
  it("普通点击重新扫描时，invoke 被以 forceFull=false 调用（增量）", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    const invokeMock = vi.mocked(invoke);
    invokeMock.mockClear();
    invokeMock.mockResolvedValue({
      totalAssets: 0,
      processed: 0,
      conceptsFound: 0,
      status: "completed",
    });

    useKnowledgeStore.setState({
      startExtraction: INITIAL_KNOWLEDGE.startExtraction,
      extractionProgress: null,
    });

    render(<KnowledgeAssociationView />);
    const btn = screen.getByTestId("knowledge-assoc-rescan-button");

    await act(async () => {
      fireEvent.click(btn);
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(invokeMock).toHaveBeenCalled();
    const [cmdName, payload] = invokeMock.mock.calls[0];
    expect(cmdName).toBe("start_concept_extraction");
    expect(payload).toEqual({
      libraryId: "lib-1",
      forceFull: false,
    });
  });

  it("Shift+点击重新扫描时，invoke 被以 forceFull=true 调用（强制全量重扫）", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    const invokeMock = vi.mocked(invoke);
    invokeMock.mockClear();
    invokeMock.mockResolvedValue({
      totalAssets: 0,
      processed: 0,
      conceptsFound: 0,
      status: "completed",
    });

    useKnowledgeStore.setState({
      startExtraction: INITIAL_KNOWLEDGE.startExtraction,
      extractionProgress: null,
    });

    render(<KnowledgeAssociationView />);
    const btn = screen.getByTestId("knowledge-assoc-rescan-button");

    await act(async () => {
      fireEvent.click(btn, { shiftKey: true });
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(invokeMock).toHaveBeenCalled();
    const [cmdName, payload] = invokeMock.mock.calls[0];
    expect(cmdName).toBe("start_concept_extraction");
    expect(payload).toEqual({
      libraryId: "lib-1",
      forceFull: true,
    });
  });
});
