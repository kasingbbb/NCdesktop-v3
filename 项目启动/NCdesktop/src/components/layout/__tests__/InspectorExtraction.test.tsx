/**
 * InspectorExtraction — task_026 AC-3 / AC-4 单测
 *
 * 覆盖：
 *   - 按钮显隐：仅 kc_enriched ∈ {"true","partial"} 且 assetType != 'md' 时显示
 *   - asset_type='md' 不显示（避免对 markdown 原件触发 KC）
 *   - 点击触发 retriggerExtraction(asset.id, true)
 *   - 点击中按钮 disabled（reEnriching 状态）
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import type { Asset } from "../../../types";
import type { ExtractedContent } from "../../../types/extraction";
import { useExtractionStore } from "../../../stores/extractionStore";

// mock tauri-commands：retriggerExtraction 是 AC-3 点击的目标命令
vi.mock("../../../lib/tauri-commands", async () => {
  const actual = await vi.importActual<typeof import("../../../lib/tauri-commands")>(
    "../../../lib/tauri-commands",
  );
  return {
    ...actual,
    retriggerExtraction: vi.fn().mockResolvedValue(undefined),
    getExtractedContent: vi.fn().mockResolvedValue(null),
    getConversionMeta: vi.fn().mockResolvedValue([]),
  };
});

// mock ExtractionBadge：减少 store / event 依赖
vi.mock("../../features/extraction/ExtractionBadge", () => ({
  ExtractionBadge: () => <span data-testid="mock-badge" />,
}));

import * as cmd from "../../../lib/tauri-commands";
import { InspectorExtraction } from "../InspectorExtraction";

const INITIAL_STORE = useExtractionStore.getState();

function makeAsset(overrides: Partial<Asset> = {}): Asset {
  return {
    id: "a1",
    projectId: "p1",
    type: "pdf",
    name: "demo.pdf",
    filePath: "/tmp/demo.pdf",
    fileSize: 1,
    mimeType: "application/pdf",
    tags: [],
    capturedAt: "2026-01-01",
    importedAt: "2026-01-01",
    source: "import",
    aiAnalysis: null,
    isStarred: false,
    assetType: "pdf",
    ...overrides,
  } as Asset;
}

function makeContent(
  assetId: string,
  kcEnriched: string | null,
  overrides: Partial<ExtractedContent> = {},
): ExtractedContent {
  return {
    id: `ec-${assetId}`,
    assetId,
    status: "extracted",
    errorMessage: null,
    retryCount: 0,
    rawText: "raw",
    structuredMd: "# heading\nbody",
    qualityLevel: 3,
    extractorType: "markitdown+kc",
    segmentsJson: null,
    kcEnriched,
    createdAt: "2026-01-01",
    updatedAt: "2026-01-01",
    ...overrides,
  };
}

function primeStore(asset: Asset, content: ExtractedContent | null) {
  useExtractionStore.setState({
    ...INITIAL_STORE,
    contentCache: content ? { [asset.id]: content } : {},
    statusCache: content ? { [asset.id]: content.status } : {},
    conversionMetaCache: {},
    pipelineProgress: null,
    isExtracting: false,
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  useExtractionStore.setState(INITIAL_STORE);
});

describe("InspectorExtraction — task_026 AC-3/4 重新增强按钮", () => {
  it("AC-3：kc_enriched='true' && assetType='pdf' → 按钮显示", async () => {
    const asset = makeAsset({ assetType: "pdf" });
    primeStore(asset, makeContent(asset.id, "true"));

    render(<InspectorExtraction asset={asset} />);

    await waitFor(() => {
      expect(screen.getByTestId("re-enrich-button")).toBeInTheDocument();
    });
    expect(screen.getByText("重新增强")).toBeInTheDocument();
  });

  it("AC-3：kc_enriched='partial' → 按钮显示（PartialLlmUnavailable 也允许重跑）", async () => {
    const asset = makeAsset({ assetType: "pdf" });
    primeStore(asset, makeContent(asset.id, "partial"));

    render(<InspectorExtraction asset={asset} />);

    await waitFor(() => {
      expect(screen.getByTestId("re-enrich-button")).toBeInTheDocument();
    });
  });

  it("AC-3 边界：assetType='md' → 按钮不显示（不能对 markdown 原件触发 KC）", () => {
    const asset = makeAsset({ assetType: "md" });
    primeStore(asset, makeContent(asset.id, "true"));

    render(<InspectorExtraction asset={asset} />);

    expect(screen.queryByTestId("re-enrich-button")).not.toBeInTheDocument();
  });

  it("AC-3 边界：kc_enriched=null（未走过 KC）→ 按钮不显示", () => {
    const asset = makeAsset({ assetType: "pdf" });
    primeStore(asset, makeContent(asset.id, null));

    render(<InspectorExtraction asset={asset} />);

    expect(screen.queryByTestId("re-enrich-button")).not.toBeInTheDocument();
  });

  it("AC-3 边界：kc_enriched='false'（enrich 失败）→ 按钮不显示", () => {
    const asset = makeAsset({ assetType: "pdf" });
    primeStore(asset, makeContent(asset.id, "false"));

    render(<InspectorExtraction asset={asset} />);

    expect(screen.queryByTestId("re-enrich-button")).not.toBeInTheDocument();
  });

  it("AC-3：点击按钮触发 retriggerExtraction(asset.id, true)", async () => {
    const asset = makeAsset({ assetType: "pdf" });
    primeStore(asset, makeContent(asset.id, "true"));

    render(<InspectorExtraction asset={asset} />);

    const btn = await screen.findByTestId("re-enrich-button");
    fireEvent.click(btn);

    await waitFor(() => {
      expect(cmd.retriggerExtraction).toHaveBeenCalledWith(asset.id, true);
    });
  });

  it("AC-3：点击中按钮 disabled（reEnriching=true）", async () => {
    const asset = makeAsset({ assetType: "pdf" });
    primeStore(asset, makeContent(asset.id, "true"));

    // 让 retriggerExtraction 永不 resolve，模拟 in-flight
    (cmd.retriggerExtraction as ReturnType<typeof vi.fn>).mockImplementation(
      () => new Promise(() => {}),
    );

    render(<InspectorExtraction asset={asset} />);

    const btn = await screen.findByTestId("re-enrich-button");
    fireEvent.click(btn);

    // 立刻读 disabled 状态
    await waitFor(() => {
      expect(btn).toBeDisabled();
    });
    expect(screen.getByText("重新增强中…")).toBeInTheDocument();
  });
});

/**
 * task_018 AC-4 — Inspector frontmatter 渲染 + react-markdown + kc_enriched 翻译
 *
 * 覆盖：
 *   - inspector_renders_summary_and_tags_for_kc_enriched_md
 *   - inspector_falls_back_to_pre_when_no_frontmatter
 *   - inspector_falls_back_to_pre_on_parse_error
 *   - inspector_displays_kc_enriched_partial_label
 *   - inspector_displays_kc_enriched_none_for_history
 */
const KC_MD = [
  "---",
  "source_asset_id: a1",
  "kc_enriched: true",
  "ai_summary: 本文介绍 NCdesktop 知识进化系统",
  "ai_tags:",
  "  - AI",
  "  - 知识进化",
  "rule_tags:",
  "  - pdf",
  "---",
  "# 正文标题",
  "",
  "| 列 1 | 列 2 |",
  "| --- | --- |",
  "| a | b |",
].join("\n");

describe("InspectorExtraction — task_018 AC-4 frontmatter 渲染 + kc_enriched 文案", () => {
  it("inspector_renders_summary_and_tags_for_kc_enriched_md — frontmatter 解析后渲染 summary/tags/markdown", async () => {
    const asset = makeAsset({ assetType: "pdf" });
    primeStore(asset, makeContent(asset.id, "true", { structuredMd: KC_MD }));

    render(<InspectorExtraction asset={asset} />);

    // frontmatter view 容器存在
    await waitFor(() => {
      expect(screen.getByTestId("frontmatter-view")).toBeInTheDocument();
    });
    // summary 渲染
    expect(screen.getByTestId("frontmatter-summary-text")).toHaveTextContent(
      "本文介绍 NCdesktop 知识进化系统",
    );
    // AI 标签 + 规则标签都渲染
    expect(screen.getByText("#AI")).toBeInTheDocument();
    expect(screen.getByText("#知识进化")).toBeInTheDocument();
    expect(screen.getByText("#pdf")).toBeInTheDocument();
    // markdown body 渲染（react-markdown 把 # 标题转 h1）
    const body = screen.getByTestId("markdown-body");
    expect(body.querySelector("h1")?.textContent).toBe("正文标题");
    // remark-gfm 表格被渲染为 <table>
    expect(body.querySelector("table")).not.toBeNull();
    // 不再走 <pre> fallback
    expect(screen.queryByTestId("pre-fallback")).not.toBeInTheDocument();
  });

  it("inspector_falls_back_to_pre_when_no_frontmatter — 无 frontmatter 的 markdown 走 <pre> fallback", async () => {
    const asset = makeAsset({ assetType: "pdf" });
    const plainMd = "# Plain heading\n\nbody text without frontmatter";
    primeStore(asset, makeContent(asset.id, null, { structuredMd: plainMd }));

    render(<InspectorExtraction asset={asset} />);

    await waitFor(() => {
      expect(screen.getByTestId("pre-fallback")).toBeInTheDocument();
    });
    expect(screen.getByTestId("pre-fallback").textContent).toContain("# Plain heading");
    // frontmatter view 不应出现
    expect(screen.queryByTestId("frontmatter-view")).not.toBeInTheDocument();
  });

  it("inspector_falls_back_to_pre_on_parse_error — YAML 非法时 fallback 到 <pre>", async () => {
    const asset = makeAsset({ assetType: "pdf" });
    // 非法 YAML：tag 值是冒号开头，js-yaml 会抛
    const badMd = [
      "---",
      "ai_tags: : : bad",
      "  - invalid",
      "---",
      "正文",
    ].join("\n");
    primeStore(asset, makeContent(asset.id, "true", { structuredMd: badMd }));

    render(<InspectorExtraction asset={asset} />);

    await waitFor(() => {
      expect(screen.getByTestId("pre-fallback")).toBeInTheDocument();
    });
    // 完整原文（含 ---）都保留在 <pre> 里
    expect(screen.getByTestId("pre-fallback").textContent).toContain("---");
    expect(screen.queryByTestId("frontmatter-view")).not.toBeInTheDocument();
  });

  it("inspector_displays_kc_enriched_partial_label — kc_enriched='partial' 显示 LLM 不可用文案", async () => {
    const asset = makeAsset({ assetType: "pdf" });
    primeStore(asset, makeContent(asset.id, "partial", { structuredMd: KC_MD }));

    render(<InspectorExtraction asset={asset} />);

    await waitFor(() => {
      expect(screen.getByTestId("kc-enriched-label")).toBeInTheDocument();
    });
    expect(screen.getByTestId("kc-enriched-label")).toHaveTextContent(
      "AI 增强：仅规则标签（LLM 不可用）",
    );
  });

  it("inspector_displays_kc_enriched_label_for_true_and_false — kc_enriched='true'/'false' 显示对应文案", async () => {
    // true → 完整
    const asset1 = makeAsset({ assetType: "pdf", id: "atrue" });
    primeStore(asset1, makeContent(asset1.id, "true", { structuredMd: KC_MD }));
    const { unmount } = render(<InspectorExtraction asset={asset1} />);
    await waitFor(() => {
      expect(screen.getByTestId("kc-enriched-label")).toHaveTextContent("AI 增强：完整");
    });
    unmount();

    // false → 未启用
    const asset2 = makeAsset({ assetType: "pdf", id: "afalse" });
    primeStore(asset2, makeContent(asset2.id, "false", { structuredMd: KC_MD }));
    render(<InspectorExtraction asset={asset2} />);
    await waitFor(() => {
      expect(screen.getByTestId("kc-enriched-label")).toHaveTextContent("未启用 AI 增强");
    });
  });

  it("inspector_displays_kc_enriched_none_for_history — kc_enriched=null 历史数据不显示该行", async () => {
    const asset = makeAsset({ assetType: "pdf" });
    primeStore(asset, makeContent(asset.id, null, { structuredMd: "# plain md" }));

    render(<InspectorExtraction asset={asset} />);

    // 等组件 mount
    await waitFor(() => {
      expect(screen.getByTestId("pre-fallback")).toBeInTheDocument();
    });
    // kc_enriched 行不应渲染
    expect(screen.queryByTestId("kc-enriched-label")).not.toBeInTheDocument();
  });
});
