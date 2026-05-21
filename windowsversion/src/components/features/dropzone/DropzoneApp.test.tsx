import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { DropzoneApp } from "./DropzoneApp";
import type { DropzoneStore } from "../../../stores/dropzoneStore";
import { logger } from "../../../utils/logger";

const dropzoneHoisted = vi.hoisted(() => {
  const store: DropzoneStore = {
    phase: "idle",
    isExpanded: false,
    recentItems: [],
    processingProgress: 0,
    processingMessage: "",
    show: vi.fn(async () => {}),
    hide: vi.fn(async () => {}),
    toggle: vi.fn(async () => {}),
    setPhase: vi.fn(),
    toggleExpand: vi.fn(),
    setExpanded: vi.fn(),
    setProcessingUI: vi.fn(),
    clearProcessingUI: vi.fn(),
    addItem: vi.fn(),
    updateItemStatus: vi.fn(),
    clearRecentItems: vi.fn(),
  };

  const patchStore = (p: Partial<DropzoneStore>): void => {
    Object.assign(store, p);
  };

  const resetStore = (): void => {
    store.phase = "idle";
    store.isExpanded = false;
    vi.clearAllMocks();
  };

  const useDropzoneStoreMock = Object.assign((): DropzoneStore => store, {
    getState: (): DropzoneStore => store,
  });

  // v1.3 task_011 DZ-01：暴露 onFocusChanged mock 供单测自定义 implementation
  const onFocusChangedMock = vi.fn().mockResolvedValue(() => {});

  return { store, patchStore, resetStore, useDropzoneStoreMock, onFocusChangedMock };
});

vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({
    onDragDropEvent: vi.fn().mockResolvedValue(vi.fn()),
  }),
}));

vi.mock("@tauri-apps/api/window", () => ({
  LogicalSize: class LogicalSize {
    width: number;
    height: number;
    constructor(width: number, height: number) {
      this.width = width;
      this.height = height;
    }
  },
  getCurrentWindow: () => ({
    setSize: vi.fn().mockResolvedValue(undefined),
    startDragging: vi.fn().mockResolvedValue(undefined),
    startResizeDragging: vi.fn().mockResolvedValue(undefined),
    // v1.3 task_011 DZ-01：监听焦点变化（mock 通过 hoisted 暴露以便单测自定义 implementation）
    onFocusChanged: dropzoneHoisted.onFocusChangedMock,
  }),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock("../../../lib/tauri-commands", () => ({}));

vi.mock("./DropzoneIdle", () => ({ DropzoneIdle: () => <div data-testid="dz-idle" /> }));
vi.mock("./DropzoneAttract", () => ({ DropzoneAttract: () => <div data-testid="dz-attract" /> }));
vi.mock("./DropzoneProcessing", () => ({ DropzoneProcessing: () => <div data-testid="dz-processing" /> }));
vi.mock("./DropzoneComplete", () => ({ DropzoneComplete: () => <div data-testid="dz-complete" /> }));
vi.mock("./DropzoneExpanded", () => ({ DropzoneExpanded: () => <div data-testid="dz-expanded" /> }));

vi.mock("../../../stores/dropzoneStore", () => ({
  useDropzoneStore: dropzoneHoisted.useDropzoneStoreMock,
}));

vi.spyOn(logger, "info");

describe("DropzoneApp Component", () => {
  beforeEach(() => {
    dropzoneHoisted.resetStore();
    // v1.3 task_011 DZ-01：每个 case 复位 onFocusChanged 默认 implementation
    dropzoneHoisted.onFocusChangedMock.mockReset();
    dropzoneHoisted.onFocusChangedMock.mockResolvedValue(() => {});
  });

  it("renders DropzoneIdle by default", () => {
    render(<DropzoneApp />);
    expect(screen.getByTestId("dz-idle")).toBeInTheDocument();
    expect(logger.info).toHaveBeenCalledWith("DropzoneApp", "Phase changed", { phase: "idle" });
  });

  it("renders DropzoneAttract when phase is attract", () => {
    dropzoneHoisted.patchStore({ phase: "attract" });
    render(<DropzoneApp />);
    expect(screen.getByTestId("dz-attract")).toBeInTheDocument();
  });

  it("handles standard drag events by preventing default", () => {
    dropzoneHoisted.patchStore({ phase: "idle" });
    render(<DropzoneApp />);

    const idle = screen.getByTestId("dz-idle");
    const dragRegion = idle.parentElement?.parentElement?.parentElement;
    expect(dragRegion).toBeTruthy();

    const dragEnterEvent = new Event("dragenter", { bubbles: true, cancelable: true });
    fireEvent(dragRegion!, dragEnterEvent);
    expect(dragEnterEvent.defaultPrevented).toBe(true);

    const dragOverEvent = new Event("dragover", { bubbles: true, cancelable: true });
    fireEvent(dragRegion!, dragOverEvent);
    expect(dragOverEvent.defaultPrevented).toBe(true);
  });

  // v1.3 task_011 DZ-01：失焦回调触发后 root div 加 dropzone-blurred class
  it("applies dropzone-blurred class when window loses focus", async () => {
    dropzoneHoisted.onFocusChangedMock.mockImplementation((cb: (e: { payload: boolean }) => void) => {
      cb({ payload: false });
      return Promise.resolve(() => {});
    });
    render(<DropzoneApp />);
    await waitFor(() => {
      const root = screen.getByTestId("dropzone-root");
      expect(root.className).toContain("dropzone-blurred");
      expect(root.getAttribute("data-focused")).toBe("false");
    });
  });

  // v1.3 task_011 DZ-01：unmount 时 unlisten 被调用，避免内存泄漏
  it("calls unlisten on unmount", async () => {
    const unlistenSpy = vi.fn();
    dropzoneHoisted.onFocusChangedMock.mockResolvedValue(unlistenSpy);
    const { unmount } = render(<DropzoneApp />);
    await waitFor(() => {
      expect(dropzoneHoisted.onFocusChangedMock).toHaveBeenCalled();
    });
    unmount();
    expect(unlistenSpy).toHaveBeenCalled();
  });
});
