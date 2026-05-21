/**
 * Inspector — v1.3 task_008 (IN-01/IN-02) tab 顺序重排单测
 *
 * 覆盖：
 *   - AC-1：底部 tab 顺序 = [Inspector, 知识关联, 时间流]
 *   - AC-3：默认 rightPanelMode === "inspector"
 *   - AC-6：role / aria-pressed 保留
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";

// mock 子视图避免拉起重组件
vi.mock("../InspectorDetails", () => ({
  InspectorDetails: () => <div data-testid="mock-inspector-details" />,
}));
vi.mock("../InspectorAI", () => ({
  InspectorAI: () => <div data-testid="mock-inspector-ai" />,
}));
vi.mock("../InspectorTags", () => ({
  InspectorTags: () => <div data-testid="mock-inspector-tags" />,
}));
vi.mock("../InspectorExtraction", () => ({
  InspectorExtraction: () => <div data-testid="mock-inspector-extraction" />,
}));
vi.mock("../../features/timeline-flow/TimelineFlowView", () => ({
  TimelineFlowView: () => <div data-testid="mock-timeline-flow" />,
}));
vi.mock("../../features/knowledge/KnowledgeAssociationView", () => ({
  KnowledgeAssociationView: () => <div data-testid="mock-knowledge-association" />,
}));

import { Inspector } from "../Inspector";
import { useUIStore } from "../../../stores/uiStore";
import { useAssetStore } from "../../../stores/assetStore";

const INITIAL_UI = useUIStore.getState();

beforeEach(() => {
  useUIStore.setState({
    ...INITIAL_UI,
    inspectorOpen: true,
    rightPanelMode: "inspector",
  });
  useAssetStore.setState({
    assets: [],
    selectedAssetId: null,
    assetTagNamesById: {},
    isLoading: false,
  });
});

describe("Inspector — v1.3 task_008 IN-01/02 底部 tab 重排", () => {
  it("AC-1：底部 tab 顺序 = Inspector / 知识关联 / 时间流", () => {
    render(<Inspector />);
    // 通过查找所有底部 tab 区域的 button（带 aria-pressed），按 DOM 顺序读
    const tabs = screen
      .getAllByRole("button")
      .filter((b) => b.hasAttribute("aria-pressed"));
    expect(tabs.length).toBe(3);
    expect(tabs[0].textContent).toMatch(/Inspector/);
    expect(tabs[1].textContent).toMatch(/知识关联/);
    expect(tabs[2].textContent).toMatch(/时间流/);
  });

  it("AC-3：默认 rightPanelMode === 'inspector'，对应 tab aria-pressed=true", () => {
    render(<Inspector />);
    const inspectorTab = screen
      .getAllByRole("button")
      .find((b) => b.textContent?.includes("Inspector"));
    expect(inspectorTab?.getAttribute("aria-pressed")).toBe("true");
  });

  it("点击「知识关联」tab → rightPanelMode 切换", () => {
    render(<Inspector />);
    const knowledgeTab = screen
      .getAllByRole("button")
      .find((b) => b.textContent?.includes("知识关联"));
    fireEvent.click(knowledgeTab!);
    expect(useUIStore.getState().rightPanelMode).toBe("knowledge_association");
  });

  it("点击「时间流」tab → rightPanelMode 切换", () => {
    render(<Inspector />);
    const timelineTab = screen
      .getAllByRole("button")
      .find((b) => b.textContent?.includes("时间流"));
    fireEvent.click(timelineTab!);
    expect(useUIStore.getState().rightPanelMode).toBe("timeline-flow");
  });
});
