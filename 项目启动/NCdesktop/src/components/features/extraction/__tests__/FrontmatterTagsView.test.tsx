/**
 * task_017_frontmatter_renderer_dep — FrontmatterTagsView 单元测试
 *
 * 覆盖 AC-5：
 * - FrontmatterTagsView_renders_ai_and_rule_tags
 * - FrontmatterTagsView_handles_empty
 */
import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { FrontmatterTagsView } from "../FrontmatterTagsView";

vi.mock("lucide-react", () => ({
  Info: () => <div data-testid="icon-info" />,
}));

describe("FrontmatterTagsView", () => {
  it("FrontmatterTagsView_renders_ai_and_rule_tags — 渲染 AI + 规则标签", () => {
    render(
      <FrontmatterTagsView
        aiTags={["AI", "机器学习", "深度学习"]}
        ruleTags={["AI", "ML"]}
      />,
    );

    // AI 标签全部渲染
    expect(screen.getByText("#AI")).toBeInTheDocument();
    expect(screen.getByText("#机器学习")).toBeInTheDocument();
    expect(screen.getByText("#深度学习")).toBeInTheDocument();

    // 规则标签去重：'AI' 已存在 AI 标签里，不重复；'ML' 独立显示
    expect(screen.getByText("#ML")).toBeInTheDocument();
    expect(screen.getAllByText("#AI")).toHaveLength(1);

    // AI 角标存在
    expect(screen.getByTestId("ai-marker")).toBeInTheDocument();
    expect(screen.getByTestId("ai-marker").getAttribute("title")).toMatch(/AI|LLM/);
  });

  it("FrontmatterTagsView_renders_ai_and_rule_tags — 只有 AI 标签时仍有 AI 角标", () => {
    render(<FrontmatterTagsView aiTags={["foo"]} />);
    expect(screen.getByText("#foo")).toBeInTheDocument();
    expect(screen.getByTestId("ai-marker")).toBeInTheDocument();
  });

  it("FrontmatterTagsView_renders_ai_and_rule_tags — 只有规则标签时不显示 AI 角标", () => {
    render(<FrontmatterTagsView ruleTags={["foo"]} />);
    expect(screen.getByText("#foo")).toBeInTheDocument();
    expect(screen.queryByTestId("ai-marker")).toBeNull();
  });

  it("FrontmatterTagsView_handles_empty — undefined props → (无标签)", () => {
    render(<FrontmatterTagsView />);
    expect(screen.getByTestId("frontmatter-tags-empty")).toBeInTheDocument();
    expect(screen.getByText("（无标签）")).toBeInTheDocument();
  });

  it("FrontmatterTagsView_handles_empty — 空数组 → (无标签)", () => {
    render(<FrontmatterTagsView aiTags={[]} ruleTags={[]} />);
    expect(screen.getByText("（无标签）")).toBeInTheDocument();
  });
});
