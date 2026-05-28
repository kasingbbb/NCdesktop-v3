/**
 * task_019_doc_viewer_render — DocumentViewer.TextContent 单元测试
 *
 * 覆盖 AC-4 + AC-5：
 *  1. doc_viewer_renders_markdown_with_table        — KC v6 表格 → <table>
 *  2. doc_viewer_renders_anchor_links_with_fragment — [text](#paragraph-0) 锚点
 *  3. doc_viewer_renders_frontmatter_card_for_kc_md — frontmatter 摘要 + 标签
 *  4. doc_viewer_falls_back_to_pre_on_invalid_markdown — 无 frontmatter → <pre>
 *  5. doc_viewer_kc_enriched_partial_shows_amber_label — partial → amber dot + 文案
 *  6. doc_viewer_kc_enriched_null_hides_row         — null → 不渲染该行
 *
 * 设计要点：
 *  - mock `tauri-commands.getFileContent` 直接返回 MD 字符串（不打 IPC）
 *  - mock `@tauri-apps/api/core.convertFileSrc` 避免 jsdom 无 Tauri runtime
 *  - 通过 `useAssetStore` 注入 fake asset（DocumentViewer 用 assetId 索引 store）
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import type { Asset } from "../../../../types";
import { useAssetStore } from "../../../../stores/assetStore";

// mock tauri-commands.getFileContent — 让每个 test 自己设返回值
const mockGetFileContent = vi.fn();

vi.mock("../../../../lib/tauri-commands", async () => {
  const actual = await vi.importActual<typeof import("../../../../lib/tauri-commands")>(
    "../../../../lib/tauri-commands",
  );
  return {
    ...actual,
    getFileContent: (...args: unknown[]) => mockGetFileContent(...args),
  };
});

// mock @tauri-apps/api/core — jsdom 无 Tauri runtime
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (p: string) => `mock://${p}`,
  invoke: vi.fn(),
}));

import { DocumentViewer } from "../DocumentViewer";

function makeMdAsset(overrides: Partial<Asset> = {}): Asset {
  return {
    id: "a-md-1",
    projectId: "p1",
    type: "markdown",
    name: "demo.md",
    filePath: "/tmp/demo.md",
    fileSize: 1,
    mimeType: "text/markdown",
    tags: [],
    capturedAt: "2026-01-01",
    importedAt: "2026-01-01",
    source: { type: "manual_import" },
    aiAnalysis: null,
    isStarred: false,
    assetType: "markdown",
    ...overrides,
  } as Asset;
}

function primeAsset(asset: Asset) {
  useAssetStore.setState({ ...useAssetStore.getState(), assets: [asset] });
}

const KC_MD_WITH_TABLE = [
  "---",
  "kc_doc_id: doc-1",
  "kc_version: '1.0'",
  "kc_enriched: 'true'",
  "ai_summary: 这是一份测试摘要。",
  "ai_tags:",
  "  - 机器学习",
  "  - PDF 抽取",
  "rule_tags:",
  "  - '2026'",
  "paragraph_count: 3",
  "---",
  "# KC 详细索引",
  "",
  "| 段落 | 起始位置 | 关键词 |",
  "|------|----------|--------|",
  "| 0 | 0 | hello |",
  "| 1 | 120 | world |",
  "",
  "正文：跳转到 [第 0 段](#paragraph-0)",
].join("\n");

const PLAIN_MD_NO_FRONTMATTER = [
  "# Plain Markdown",
  "",
  "这是一份历史 MD，没有 frontmatter。",
  "下方是表格，但因为无 frontmatter 应进入 <pre> fallback。",
].join("\n");

beforeEach(() => {
  vi.clearAllMocks();
  // reset asset store
  useAssetStore.setState({ ...useAssetStore.getState(), assets: [] });
});

describe("DocumentViewer.TextContent — task_019 AC-4/5", () => {
  it("doc_viewer_renders_markdown_with_table — KC v6 表格渲染为 <table>", async () => {
    const asset = makeMdAsset();
    primeAsset(asset);
    mockGetFileContent.mockResolvedValueOnce(KC_MD_WITH_TABLE);

    render(<DocumentViewer assetId={asset.id} onClose={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByTestId("doc-viewer-markdown-body")).toBeInTheDocument();
    });
    const body = screen.getByTestId("doc-viewer-markdown-body");
    // remark-gfm 把 markdown 表格转成 <table>
    expect(body.querySelector("table")).not.toBeNull();
    expect(body.querySelector("thead th")).not.toBeNull();
    expect(body.querySelectorAll("tbody tr").length).toBeGreaterThanOrEqual(2);
  });

  it("doc_viewer_renders_anchor_links_with_fragment — 段落锚点用 fragment href", async () => {
    const asset = makeMdAsset();
    primeAsset(asset);
    mockGetFileContent.mockResolvedValueOnce(KC_MD_WITH_TABLE);

    render(<DocumentViewer assetId={asset.id} onClose={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByTestId("doc-viewer-markdown-body")).toBeInTheDocument();
    });
    const body = screen.getByTestId("doc-viewer-markdown-body");
    // react-markdown 默认把 [text](#paragraph-0) 渲染成 <a href="#paragraph-0">
    const anchor = body.querySelector('a[href="#paragraph-0"]');
    expect(anchor).not.toBeNull();
    expect(anchor?.textContent).toBe("第 0 段");
  });

  it("doc_viewer_renders_frontmatter_card_for_kc_md — frontmatter 摘要 + 标签出现", async () => {
    const asset = makeMdAsset();
    primeAsset(asset);
    mockGetFileContent.mockResolvedValueOnce(KC_MD_WITH_TABLE);

    render(<DocumentViewer assetId={asset.id} onClose={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByTestId("doc-viewer-frontmatter-card")).toBeInTheDocument();
    });
    const card = screen.getByTestId("doc-viewer-frontmatter-card");
    expect(card).toHaveTextContent("这是一份测试摘要。");
    expect(card).toHaveTextContent("机器学习");
    expect(card).toHaveTextContent("PDF 抽取");
    expect(card).toHaveTextContent("2026");
  });

  it("doc_viewer_falls_back_to_pre_on_invalid_markdown — 无 frontmatter → <pre>", async () => {
    const asset = makeMdAsset();
    primeAsset(asset);
    mockGetFileContent.mockResolvedValueOnce(PLAIN_MD_NO_FRONTMATTER);

    render(<DocumentViewer assetId={asset.id} onClose={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByTestId("doc-viewer-pre-fallback")).toBeInTheDocument();
    });
    expect(screen.getByTestId("doc-viewer-pre-fallback").textContent).toContain(
      "# Plain Markdown",
    );
    // 不应渲染 markdown view / frontmatter card
    expect(screen.queryByTestId("doc-viewer-markdown-body")).not.toBeInTheDocument();
    expect(screen.queryByTestId("doc-viewer-frontmatter-card")).not.toBeInTheDocument();
  });

  // AC-5（TD-4）追加测试：partial → amber，null → 整行隐藏
  it("doc_viewer_kc_enriched_partial_shows_amber_label — partial 显示 amber dot + LLM 不可用文案", async () => {
    const PARTIAL_MD = [
      "---",
      "kc_doc_id: doc-2",
      "kc_enriched: 'partial'",
      "ai_summary: partial summary",
      "---",
      "正文",
    ].join("\n");
    const asset = makeMdAsset({ id: "a-partial" });
    primeAsset(asset);
    mockGetFileContent.mockResolvedValueOnce(PARTIAL_MD);

    render(<DocumentViewer assetId={asset.id} onClose={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByTestId("doc-viewer-kc-enriched-label")).toBeInTheDocument();
    });
    expect(screen.getByTestId("doc-viewer-kc-enriched-label")).toHaveTextContent(
      "AI 增强：仅规则标签（LLM 不可用）",
    );
    // amber dot 渲染（partial tone）
    expect(screen.getByTestId("doc-viewer-kc-dot-partial")).toBeInTheDocument();
    // 不应有 success/inactive tone
    expect(screen.queryByTestId("doc-viewer-kc-dot-success")).not.toBeInTheDocument();
    expect(screen.queryByTestId("doc-viewer-kc-dot-inactive")).not.toBeInTheDocument();
  });

  it("doc_viewer_kc_enriched_null_hides_row — kc_enriched 缺失时整行隐藏", async () => {
    // frontmatter 存在但没有 kc_enriched 字段
    const MD_NO_KC = [
      "---",
      "ai_summary: 仅摘要，无 kc_enriched",
      "ai_tags:",
      "  - 测试",
      "---",
      "正文",
    ].join("\n");
    const asset = makeMdAsset({ id: "a-nokc" });
    primeAsset(asset);
    mockGetFileContent.mockResolvedValueOnce(MD_NO_KC);

    render(<DocumentViewer assetId={asset.id} onClose={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByTestId("doc-viewer-frontmatter-card")).toBeInTheDocument();
    });
    // 摘要 / 标签仍然在
    expect(screen.getByTestId("doc-viewer-frontmatter-card")).toHaveTextContent(
      "仅摘要，无 kc_enriched",
    );
    // kc_enriched 行整行不应渲染
    expect(screen.queryByTestId("doc-viewer-kc-enriched-row")).not.toBeInTheDocument();
    expect(screen.queryByTestId("doc-viewer-kc-enriched-label")).not.toBeInTheDocument();
  });
});
