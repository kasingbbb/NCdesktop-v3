/**
 * AssetContextMenu — task_011 AC-1 / AC-6 单测。
 *
 * 覆盖：
 *  - 查看原文件 enabled / disabled 切换（sourceMissing）。
 *  - 点击查看原文件 → revealSourceFile(sourcePath) 调用。
 *  - 点击重命名 → onRequestRename 回调被触发（替代 window.prompt）。
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";

const revealSourceFileMock = vi.fn().mockResolvedValue(undefined);
const moveAssetMock = vi.fn().mockResolvedValue(undefined);
const revealProjMock = vi.fn().mockResolvedValue(undefined);

vi.mock("../../../lib/tauri-commands", () => ({
  revealSourceFile: (...args: unknown[]) => revealSourceFileMock(...args),
  moveAssetToWorkspaceFolder: (...args: unknown[]) => moveAssetMock(...args),
  revealProjectWorkspaceFolder: (...args: unknown[]) => revealProjMock(...args),
}));

// useAssetStore.getState() 在删除 / rename 路径里被读，给个最小桩
vi.mock("../../../stores/assetStore", () => ({
  useAssetStore: {
    getState: () => ({
      assets: [{ id: "a1", name: "Demo" }],
      renameAsset: vi.fn().mockResolvedValue(undefined),
      deleteAsset: vi.fn().mockResolvedValue(undefined),
    }),
  },
}));

import { AssetContextMenu } from "../AssetContextMenu";

beforeEach(() => {
  revealSourceFileMock.mockClear();
  moveAssetMock.mockClear();
  revealProjMock.mockClear();
});

function renderMenu(overrides: Partial<React.ComponentProps<typeof AssetContextMenu>> = {}) {
  const onClose = vi.fn();
  const onMoved = vi.fn();
  const onRequestRename = vi.fn();
  const props: React.ComponentProps<typeof AssetContextMenu> = {
    x: 100,
    y: 100,
    assetId: "a1",
    pane: "left",
    selectedAssetIds: new Set(["a1"]),
    workspaceFolders: [],
    projectId: "p1",
    currentFilePath: "/tmp/p1/a.md",
    sourcePath: "/orig/source.pdf",
    sourceMissing: false,
    onClose,
    onMoved,
    onRequestRename,
    ...overrides,
  };
  render(<AssetContextMenu {...props} />);
  return { onClose, onMoved, onRequestRename };
}

describe("AssetContextMenu — task_011 AC-1 查看原文件", () => {
  it("sourceMissing=false → 「查看原文件」enabled，点击调 revealSourceFile", async () => {
    renderMenu({ sourceMissing: false, sourcePath: "/orig/source.pdf" });
    const btn = screen.getByTestId("ctx-reveal-source");
    expect(btn).toHaveTextContent("查看原文件");
    expect(btn).toHaveAttribute("data-disabled", "false");
    expect(btn).not.toBeDisabled();
    fireEvent.click(btn);
    await Promise.resolve();
    expect(revealSourceFileMock).toHaveBeenCalledWith("/orig/source.pdf");
  });

  it("sourceMissing=true → 文案改为「原文件已不存在」+ disabled + 不调用 revealSourceFile", () => {
    renderMenu({ sourceMissing: true, sourcePath: "/orig/source.pdf" });
    const btn = screen.getByTestId("ctx-reveal-source");
    expect(btn).toHaveTextContent("原文件已不存在");
    expect(btn).toHaveAttribute("data-disabled", "true");
    expect(btn).toBeDisabled();
    fireEvent.click(btn);
    expect(revealSourceFileMock).not.toHaveBeenCalled();
  });

  it("sourcePath 空 → disabled（即便 sourceMissing=false）", () => {
    renderMenu({ sourcePath: null });
    const btn = screen.getByTestId("ctx-reveal-source");
    expect(btn).toHaveAttribute("data-disabled", "true");
  });
});

describe("AssetContextMenu — task_011 AC-6 重命名走回调", () => {
  it("点击「重命名」→ 调用 onRequestRename(assetId)，不再 window.prompt", () => {
    const promptSpy = vi.spyOn(window, "prompt").mockReturnValue("ignored");
    const { onRequestRename, onClose } = renderMenu();
    fireEvent.click(screen.getByRole("button", { name: "重命名" }));
    expect(onRequestRename).toHaveBeenCalledWith("a1");
    expect(promptSpy).not.toHaveBeenCalled();
    expect(onClose).toHaveBeenCalled();
    promptSpy.mockRestore();
  });
});
