import { act, render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { DropzoneApp } from "./DropzoneApp";
import type { DropzoneStore } from "../../../stores/dropzoneStore";
import { useKcQueueStore } from "../../../stores/kcQueueStore";
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
    // task_025：每个 case 复位 KC 队列 store（真实 store，不 mock）
    useKcQueueStore.getState().reset();
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

  // task_025 AC-3 / AC-5 #1：KC 队列非空时显示 "AI 增强中 N…" toast
  it("shows kc queue toast when queue length > 0", async () => {
    render(<DropzoneApp />);
    // 初始无 toast
    expect(screen.queryByTestId("kc-queue-toast")).toBeNull();
    // 模拟收到 kc-queued 事件 → 队列 +1
    act(() => {
      useKcQueueStore.getState().enqueue("asset-1");
      useKcQueueStore.getState().enqueue("asset-2");
    });
    await waitFor(() => {
      const toast = screen.getByTestId("kc-queue-toast");
      expect(toast).toBeInTheDocument();
      expect(toast.getAttribute("data-kind")).toBe("running");
      expect(toast.textContent).toContain("2");
      expect(toast.textContent).toContain("AI 增强中");
    });
  });

  // task_025 AC-3 / AC-5 #2：队列清空后 toast 切换 "完成"；超过 5s 窗口后 toast 隐藏
  it("hides kc queue toast when queue empty and last completion > 5s ago", async () => {
    render(<DropzoneApp />);

    // 第一步：队列非空 → running toast
    act(() => {
      useKcQueueStore.getState().enqueue("asset-1");
    });
    await waitFor(() => {
      const t = screen.getByTestId("kc-queue-toast");
      expect(t.getAttribute("data-kind")).toBe("running");
    });

    // 第二步：出队 → done toast（lastCompletedAt 设为 now）
    act(() => {
      useKcQueueStore.getState().dequeue("asset-1");
    });
    await waitFor(() => {
      const t = screen.getByTestId("kc-queue-toast");
      expect(t.getAttribute("data-kind")).toBe("done");
      expect(t.textContent).toContain("AI 增强完成");
    });

    // 第三步：模拟 5s 窗口已过 —— 直接把 lastCompletedAt 推到 10s 之前，
    // 让 50ms tick 触发 re-render 后派生逻辑返回 null。
    act(() => {
      // 不可变设置：单独覆盖 lastCompletedAt 字段（不动 pendingAssetIds）
      useKcQueueStore.setState({ lastCompletedAt: Date.now() - 10000 });
    });
    await waitFor(
      () => {
        expect(screen.queryByTestId("kc-queue-toast")).toBeNull();
      },
      { timeout: 2000 },
    );
  });
});
