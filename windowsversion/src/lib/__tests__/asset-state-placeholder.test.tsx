/**
 * task_014 Fix-A4 AC-6：AssetStateBadge 区分 "占位 MD" vs "已就绪"。
 *
 * - extractor_type 以 `placeholder_` 开头 + state="done" → 渲染"占位 MD" 黄色
 *   （`data-placeholder="true"`）。
 * - extractor_type = "text_passthrough" / "markitdown" 等真 extractor →
 *   渲染"已就绪" 绿色（`data-placeholder="false"`）。
 */
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { AssetStateBadge, isPlaceholderExtractor } from "../asset-state";

describe("isPlaceholderExtractor", () => {
  it("识别 placeholder_ 前缀", () => {
    expect(isPlaceholderExtractor("placeholder_unsupported")).toBe(true);
    expect(isPlaceholderExtractor("placeholder_extract_failed")).toBe(true);
    expect(isPlaceholderExtractor("placeholder_read_failed")).toBe(true);
  });
  it("真 extractor 返回 false", () => {
    expect(isPlaceholderExtractor("text_passthrough")).toBe(false);
    expect(isPlaceholderExtractor("markitdown")).toBe(false);
    expect(isPlaceholderExtractor("audio_asr_iflytek")).toBe(false);
    expect(isPlaceholderExtractor(null)).toBe(false);
    expect(isPlaceholderExtractor(undefined)).toBe(false);
    expect(isPlaceholderExtractor("")).toBe(false);
  });
});

describe("AssetStateBadge placeholder vs 真 MD（task_014 AC-6）", () => {
  it("placeholder_unsupported_mime + done → 占位 MD 黄色徽章", () => {
    render(
      <AssetStateBadge
        state="done"
        assetId="a1"
        extractorType="placeholder_unsupported_mime"
      />
    );
    const badge = screen.getByTestId("asset-state-badge");
    expect(badge).toHaveAttribute("data-placeholder", "true");
    expect(badge.textContent).toContain("占位 MD");
    expect(badge).toHaveAttribute("title", "未配置该格式的提取器，仅写占位");
  });

  it("text_passthrough + done → 已就绪绿色徽章", () => {
    render(
      <AssetStateBadge
        state="done"
        assetId="a2"
        extractorType="text_passthrough"
      />
    );
    const badge = screen.getByTestId("asset-state-badge");
    expect(badge).toHaveAttribute("data-placeholder", "false");
    expect(badge.textContent).toContain("已就绪");
  });

  it("markitdown + done → 已就绪", () => {
    render(
      <AssetStateBadge state="done" assetId="a3" extractorType="markitdown" />
    );
    const badge = screen.getByTestId("asset-state-badge");
    expect(badge).toHaveAttribute("data-placeholder", "false");
    expect(badge.textContent).toContain("已就绪");
  });

  it("无 extractorType + done → 已就绪（不显示占位）", () => {
    render(<AssetStateBadge state="done" assetId="a4" />);
    const badge = screen.getByTestId("asset-state-badge");
    expect(badge).toHaveAttribute("data-placeholder", "false");
    expect(badge.textContent).toContain("已就绪");
  });

  it("failed 态不受 placeholder 影响：仍是失败徽章", () => {
    render(
      <AssetStateBadge
        state="failed"
        assetId="a5"
        extractorType="placeholder_extract_failed"
        reason="timeout"
      />
    );
    const badge = screen.getByTestId("asset-state-badge");
    // failed != done → 不命中 isPlaceholder 分支
    expect(badge).toHaveAttribute("data-placeholder", "false");
    expect(badge.textContent).toContain("失败");
  });
});
