/**
 * useDragAssets 单测（task_008 AC-6）。
 *
 * 覆盖核心 AC-3 失败路径：
 *  - mock invoke('prepare_outbound_payload') 返回 `StateNotDone` JSON（reject）
 *  - 模拟用户按下卡片 + 跨过 5px 阈值
 *  - 断言：startDrag **未** 被调用 + toast (`useUIStore.addNotification`) 被触发，
 *    且文案为「非 done 态资产无法拖出」
 *
 * 同时回归一条成功路径：invoke 返回 OutboundEntry[] → startDrag 调用 paths。
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import React from "react";

const invokeMock = vi.fn();
const startDragMock = vi.fn().mockResolvedValue(undefined);

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));
vi.mock("@crabnebula/tauri-plugin-drag", () => ({
  startDrag: (...args: unknown[]) => startDragMock(...args),
}));

import { useDragAssets } from "./useDragAssets";
import { useUIStore } from "../stores/uiStore";
import type { Asset } from "../types";

const A1: Asset = {
  id: "a1",
  projectId: "p1",
  type: "markdown",
  name: "a1",
  filePath: "/tmp/a1.md",
  fileSize: 1,
  mimeType: "text/markdown",
  tags: [],
  capturedAt: "2025-01-01T00:00:00Z",
  importedAt: "2025-01-01T00:00:00Z",
  source: { type: "manual_import" },
  aiAnalysis: null,
  isStarred: false,
};

function flushMicrotasks() {
  return new Promise<void>((resolve) => setTimeout(resolve, 0));
}

beforeEach(() => {
  invokeMock.mockReset();
  startDragMock.mockClear();
  // 重置 toast 队列
  useUIStore.setState({ notifications: [] });
  // 默认让 get_drag_icon_path 成功（构造期 useEffect 会调一次）
  invokeMock.mockImplementation(async (cmd: string) => {
    if (cmd === "get_drag_icon_path") return "/tmp/icon.png";
    throw new Error(`unexpected invoke ${cmd}`);
  });
});

function setupHook() {
  const selection = new Set<string>();
  const hookRender = renderHook(() => useDragAssets(selection, [A1]));
  return hookRender;
}

function makeMouseEvent(
  type: "mousedown",
  clientX: number,
  clientY: number
): React.MouseEvent<HTMLElement> {
  // 构造一个最小可用的 React 合成事件对象（hook 只用 button / clientX / clientY / preventDefault）
  return {
    button: 0,
    clientX,
    clientY,
    preventDefault: vi.fn(),
  } as unknown as React.MouseEvent<HTMLElement>;
}

describe("useDragAssets — AC-3 OutboundError.StateNotDone", () => {
  it("invoke 返回 StateNotDone → startDrag 未调用 + toast 触发", async () => {
    const errPayload = JSON.stringify({
      kind: "stateNotDone",
      assetId: "a1",
      state: "converting",
      message: "asset still converting",
    });
    // 覆盖默认：prepare_outbound_payload reject 为 OutboundError JSON 字符串
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "get_drag_icon_path") return "/tmp/icon.png";
      if (cmd === "prepare_outbound_payload") {
        // Tauri 把后端 Err 序列化成字符串抛出
        return Promise.reject(errPayload);
      }
      throw new Error(`unexpected invoke ${cmd}`);
    });

    const { result } = setupHook();
    await flushMicrotasks();

    const props = result.current.makeDragProps("a1");
    // mousedown 起点 (0,0)
    act(() => {
      props.onMouseDown(makeMouseEvent("mousedown", 0, 0));
    });
    // 跨过 5px 阈值 → 触发 startDrag 准备路径
    act(() => {
      window.dispatchEvent(
        new MouseEvent("mousemove", { clientX: 20, clientY: 20 })
      );
    });
    // 等 .catch 落进 toast
    await flushMicrotasks();
    await flushMicrotasks();

    expect(startDragMock).not.toHaveBeenCalled();
    const notifs = useUIStore.getState().notifications;
    expect(notifs.length).toBeGreaterThan(0);
    expect(notifs[0].title).toBe("无法拖出");
    expect(notifs[0].message).toContain("非 done");
    expect(notifs[0].message).toContain("converting");
  });
});

describe("useDragAssets — AC-3 成功路径", () => {
  it("invoke 返回 OutboundEntry[] → startDrag 用 entries.path 调用", async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "get_drag_icon_path") return "/tmp/icon.png";
      if (cmd === "prepare_outbound_payload") {
        return [
          { assetId: "a1", path: "/cache/a1.md", displayName: "a1.md" },
        ];
      }
      throw new Error(`unexpected invoke ${cmd}`);
    });

    const { result } = setupHook();
    await flushMicrotasks();

    const props = result.current.makeDragProps("a1");
    act(() => {
      props.onMouseDown(makeMouseEvent("mousedown", 0, 0));
    });
    act(() => {
      window.dispatchEvent(
        new MouseEvent("mousemove", { clientX: 20, clientY: 20 })
      );
    });
    await flushMicrotasks();
    await flushMicrotasks();

    expect(startDragMock).toHaveBeenCalledTimes(1);
    expect(startDragMock).toHaveBeenCalledWith(
      expect.objectContaining({
        item: ["/cache/a1.md"],
        mode: "copy",
      })
    );
  });
});

/**
 * task_009 顺手补 task_008 MAJOR：参数化覆盖剩余 3 个 OutboundError 变体。
 * 每个用例断言：startDrag 未被调用 + toast 含对应中文文案。
 */
describe("useDragAssets — task_009 AC-Frontend 其余 OutboundError 变体", () => {
  type Case = {
    name: string;
    payload: Record<string, unknown>;
    expectedTitle: string;
    expectedMessageFragment: string;
  };

  const CASES: Case[] = [
    {
      name: "MixedStates → 多选含非 done 态 toast",
      payload: {
        kind: "mixedStates",
        offending: ["a1", "a2"],
        message: "mixed",
      },
      expectedTitle: "无法拖出",
      expectedMessageFragment: "多选包含非 done 态资产",
    },
    {
      name: "RenditionMissing → 未找到 MD toast",
      payload: {
        kind: "renditionMissing",
        assetId: "a1",
        message: "rendition gone",
      },
      expectedTitle: "无法拖出",
      expectedMessageFragment: "未找到转化后的 MD 文件",
    },
    {
      name: "IoFailed → 拖拽准备失败 toast",
      payload: {
        kind: "ioFailed",
        assetId: "a1",
        detail: "EXDEV cross-device link",
        message: "io",
      },
      expectedTitle: "拖拽准备失败",
      expectedMessageFragment: "EXDEV",
    },
  ];

  it.each(CASES)("$name", async ({ payload, expectedTitle, expectedMessageFragment }) => {
    const errJson = JSON.stringify(payload);
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "get_drag_icon_path") return "/tmp/icon.png";
      if (cmd === "prepare_outbound_payload") return Promise.reject(errJson);
      throw new Error(`unexpected invoke ${cmd}`);
    });

    const { result } = setupHook();
    await flushMicrotasks();

    const props = result.current.makeDragProps("a1");
    act(() => {
      props.onMouseDown(makeMouseEvent("mousedown", 0, 0));
    });
    act(() => {
      window.dispatchEvent(
        new MouseEvent("mousemove", { clientX: 20, clientY: 20 })
      );
    });
    await flushMicrotasks();
    await flushMicrotasks();

    expect(startDragMock).not.toHaveBeenCalled();
    const notifs = useUIStore.getState().notifications;
    expect(notifs.length).toBeGreaterThan(0);
    expect(notifs[0].title).toBe(expectedTitle);
    expect(notifs[0].message).toContain(expectedMessageFragment);
  });
});

/**
 * task_011 AC-5：相同 OutboundError 类型在 3s 滑动窗口内合并 → 仅保留最新一条。
 *
 * 复用 stateNotDone 变体：连续触发 3 次拖拽（同一 errorKind），断言：
 *  - notifications 长度 == 1（合并）
 *  - dedupeKey === "outbound:stateNotDone"
 */
describe("useDragAssets — task_011 AC-5 toast dedupe", () => {
  it("快速多次 stateNotDone → toast 合并为 1 条（dedupeKey=outbound:stateNotDone）", async () => {
    const errPayload = JSON.stringify({
      kind: "stateNotDone",
      assetId: "a1",
      state: "converting",
      message: "x",
    });
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "get_drag_icon_path") return "/tmp/icon.png";
      if (cmd === "prepare_outbound_payload") return Promise.reject(errPayload);
      throw new Error(`unexpected invoke ${cmd}`);
    });

    const { result } = setupHook();
    await flushMicrotasks();

    const props = result.current.makeDragProps("a1");

    for (let i = 0; i < 3; i++) {
      act(() => {
        props.onMouseDown(makeMouseEvent("mousedown", 0, 0));
      });
      act(() => {
        window.dispatchEvent(
          new MouseEvent("mousemove", { clientX: 20, clientY: 20 })
        );
      });
      await flushMicrotasks();
      await flushMicrotasks();
    }

    const notifs = useUIStore.getState().notifications;
    expect(notifs.length).toBe(1);
    expect(notifs[0].dedupeKey).toBe("outbound:stateNotDone");
    expect(notifs[0].title).toBe("无法拖出");
  });
});
