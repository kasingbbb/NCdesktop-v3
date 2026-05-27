/**
 * task_023_e2e_integration_tests — KC enrichment 前端集成测试（vitest，2 个场景）。
 *
 * ## 测试范围（AC-2）
 *
 * 后端落地的 .md 文件（含/不含 frontmatter）被前端读到后，Inspector 必须能：
 *   1. **frontend_renders_kc_enriched_md_in_inspector** — 含 frontmatter →
 *      解析 + 渲染 ai_summary / ai_tags / rule_tags + 用 react-markdown 渲染正文（含 GFM 表格）
 *   2. **frontend_falls_back_to_pre_for_legacy_md** — 无 frontmatter（legacy） → 走 `<pre>` fallback
 *
 * ## 测试边界
 *
 * - mock `useExtractionStore` 注入"已抽取且 KC 增强完成"的 ExtractedContent 行（structuredMd 含
 *   完整 frontmatter），等价于"后端 e2e 落盘 + 前端 fetch 拉到"链路的最后一段；
 * - 不依赖真 Tauri runtime（vitest jsdom 环境），不发真 IPC；
 * - 与 lib 内 DocumentViewer / InspectorExtraction 已有单测的关系：本 e2e 串"两个组件 + 同一份
 *   frontmatter MD" 的集成视角；现有单测分别覆盖各组件内部分支。
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import type { Asset } from "../types";
import type { ExtractedContent } from "../types/extraction";
import { useExtractionStore } from "../stores/extractionStore";

// mock tauri-commands 让 InspectorExtraction 的 useEffect 不打真 IPC
vi.mock("../lib/tauri-commands", async () => {
  const actual = await vi.importActual<typeof import("../lib/tauri-commands")>(
    "../lib/tauri-commands",
  );
  return {
    ...actual,
    retriggerExtraction: vi.fn().mockResolvedValue(undefined),
    getExtractedContent: vi.fn().mockResolvedValue(null),
    getConversionMeta: vi.fn().mockResolvedValue([]),
  };
});

// mock ExtractionBadge 避免拉起 event listener 依赖
vi.mock("../components/features/extraction/ExtractionBadge", () => ({
  ExtractionBadge: () => <span data-testid="mock-badge" />,
}));

import { InspectorExtraction } from "../components/layout/InspectorExtraction";

const INITIAL_STORE = useExtractionStore.getState();

function makeAsset(overrides: Partial<Asset> = {}): Asset {
  return {
    id: "asset-e2e-frontend",
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
  structuredMd: string,
  kcEnriched: string | null,
): ExtractedContent {
  return {
    id: `ec-${assetId}`,
    assetId,
    status: "extracted",
    errorMessage: null,
    retryCount: 0,
    rawText: "raw",
    structuredMd,
    qualityLevel: 3,
    extractorType: kcEnriched === "true" ? "markitdown+kc" : "markitdown",
    segmentsJson: null,
    kcEnriched,
    createdAt: "2026-01-01",
    updatedAt: "2026-01-01",
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

// === 后端 e2e #1 (e2e_drag_pdf_to_kc_enriched_md) 落盘格式的等价物 ===
//
// 字面与后端 `build_kc_frontmatter` 一致：5 个 NC schema 字段 + 10 个 KC 扩展字段。
// 正文含 GFM 表格 + 段落锚点，覆盖 react-markdown + remark-gfm 链路。
const KC_ENRICHED_MD = [
  "---",
  'source_asset_id: "asset-e2e-frontend"',
  "derivative_version: 1",
  'extracted_at: "2026-05-27T00:00:00Z"',
  'extractor_type: "markitdown+kc"',
  "quality_level: 3",
  'kc_doc_id: "doc-e2e-1"',
  'kc_generated_at: "2026-05-27T00:00:00Z"',
  'kc_version: "0.9"',
  'kc_tags_source: "ai+rule"',
  'kc_enriched: "true"',
  "ai_tags:",
  '  - "机器学习"',
  '  - "PDF 抽取"',
  "rule_tags:",
  '  - "pdf"',
  'ai_summary: "本文介绍 KC 集成在 NCdesktop 知识进化系统中的落地。"',
  "ai_qa_pairs_count: 2",
  "paragraph_count: 5",
  "---",
  "# KC 增强后的文档正文",
  "",
  "下面是一个 GFM 表格：",
  "",
  "| 列 1 | 列 2 |",
  "| --- | --- |",
  "| a | b |",
  "",
  "跳转到 [第 0 段](#paragraph-0)",
].join("\n");

// === 后端 e2e #3 (e2e_drag_with_kc_disabled_falls_through) 落盘格式的等价物（无 frontmatter） ===
const LEGACY_MD_NO_FRONTMATTER = [
  "# Plain Markdown",
  "",
  "这是一份历史 markitdown 转换产物，没有 frontmatter。",
  "因为无 frontmatter，Inspector 应进入 <pre> fallback。",
].join("\n");

beforeEach(() => {
  vi.clearAllMocks();
  useExtractionStore.setState(INITIAL_STORE);
});

describe("KC enrichment 前端集成测试（task_023 AC-2）", () => {
  it("frontend_renders_kc_enriched_md_in_inspector — 含 frontmatter 的 KC MD 渲染 summary/tags/markdown", async () => {
    const asset = makeAsset();
    primeStore(asset, makeContent(asset.id, KC_ENRICHED_MD, "true"));

    render(<InspectorExtraction asset={asset} />);

    // frontmatter view 容器出现（替代 <pre>）
    await waitFor(() => {
      expect(screen.getByTestId("frontmatter-view")).toBeInTheDocument();
    });

    // AI 摘要正确渲染
    expect(screen.getByTestId("frontmatter-summary-text")).toHaveTextContent(
      "本文介绍 KC 集成在 NCdesktop 知识进化系统中的落地。",
    );

    // AI 标签 + 规则标签都被渲染（含 # 前缀展示）
    expect(screen.getByText("#机器学习")).toBeInTheDocument();
    expect(screen.getByText("#PDF 抽取")).toBeInTheDocument();
    expect(screen.getByText("#pdf")).toBeInTheDocument();

    // markdown body 通过 react-markdown 渲染：h1 + table
    const body = screen.getByTestId("markdown-body");
    expect(body.querySelector("h1")?.textContent).toBe("KC 增强后的文档正文");
    expect(body.querySelector("table")).not.toBeNull();
    expect(body.querySelector("thead th")).not.toBeNull();
    // 段落锚点用 fragment href
    const anchor = body.querySelector('a[href="#paragraph-0"]');
    expect(anchor).not.toBeNull();
    expect(anchor?.textContent).toBe("第 0 段");

    // kc_enriched=true → "完整" 文案
    await waitFor(() => {
      expect(screen.getByTestId("kc-enriched-label")).toBeInTheDocument();
    });
    expect(screen.getByTestId("kc-enriched-label")).toHaveTextContent(
      "AI 增强：完整",
    );

    // 不再走 <pre> fallback
    expect(screen.queryByTestId("pre-fallback")).not.toBeInTheDocument();
  });

  it("frontend_falls_back_to_pre_for_legacy_md — 无 frontmatter 的 legacy MD 走 <pre> fallback", async () => {
    const asset = makeAsset({ id: "asset-e2e-legacy" });
    primeStore(asset, makeContent(asset.id, LEGACY_MD_NO_FRONTMATTER, null));

    render(<InspectorExtraction asset={asset} />);

    // <pre> fallback 出现
    await waitFor(() => {
      expect(screen.getByTestId("pre-fallback")).toBeInTheDocument();
    });
    // 完整原文（标题 + 正文）都保留在 <pre>
    expect(screen.getByTestId("pre-fallback").textContent).toContain(
      "# Plain Markdown",
    );
    expect(screen.getByTestId("pre-fallback").textContent).toContain(
      "这是一份历史 markitdown 转换产物",
    );

    // 没有 frontmatter view / markdown body
    expect(screen.queryByTestId("frontmatter-view")).not.toBeInTheDocument();
    expect(screen.queryByTestId("markdown-body")).not.toBeInTheDocument();

    // kc_enriched=null → 不渲染 kc-enriched-label
    expect(screen.queryByTestId("kc-enriched-label")).not.toBeInTheDocument();
  });
});
