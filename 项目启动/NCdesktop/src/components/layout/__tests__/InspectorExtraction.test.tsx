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
