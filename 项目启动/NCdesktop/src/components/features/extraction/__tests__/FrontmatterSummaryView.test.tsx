/**
 * task_017_frontmatter_renderer_dep — FrontmatterSummaryView 单元测试
 *
 * 覆盖 AC-5：FrontmatterSummaryView_renders_text
 */
import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { FrontmatterSummaryView } from "../FrontmatterSummaryView";

vi.mock("lucide-react", () => ({
  FileText: () => <div data-testid="icon-summary" />,
  Sparkles: () => <div data-testid="icon-ai" />,
}));

describe("FrontmatterSummaryView", () => {
  it("FrontmatterSummaryView_renders_text — 渲染 AI 摘要文本和 AI 图标", () => {
    render(<FrontmatterSummaryView summary="本文介绍了人工智能" isAi />);
    expect(screen.getByTestId("frontmatter-summary-text")).toHaveTextContent("本文介绍了人工智能");
    expect(screen.getByText("AI 摘要")).toBeInTheDocument();
    expect(screen.getByTestId("icon-ai")).toBeInTheDocument();
    expect(screen.queryByTestId("icon-summary")).toBeNull();
  });

  it("FrontmatterSummaryView_renders_text — 非 AI 时显示普通摘要图标", () => {
    render(<FrontmatterSummaryView summary="某段摘要" isAi={false} />);
    expect(screen.getByText("摘要")).toBeInTheDocument();
    expect(screen.queryByText("AI 摘要")).toBeNull();
    expect(screen.getByTestId("icon-summary")).toBeInTheDocument();
  });

  it("FrontmatterSummaryView_renders_text — summary=undefined → 显示 (无摘要)", () => {
    render(<FrontmatterSummaryView summary={undefined} isAi />);
    expect(screen.getByTestId("frontmatter-summary-empty")).toBeInTheDocument();
    expect(screen.getByText("（无摘要）")).toBeInTheDocument();
  });

  it("FrontmatterSummaryView_renders_text — summary 为空白字符串 → 显示 (无摘要)", () => {
    render(<FrontmatterSummaryView summary="   " isAi />);
    expect(screen.getByText("（无摘要）")).toBeInTheDocument();
  });
});
