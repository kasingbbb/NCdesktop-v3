/**
 * v2 Sidebar Redesign — TitleBar 视图层单元测试（task_004 / F-P0-7 / AC-3 / AC-6）。
 *
 * 覆盖：
 *   - 右侧增加 ⌘K 按钮，aria-label="打开搜索 (⌘K)"
 *   - 点击 ⌘K 按钮触发 onSearchOpen
 *   - 设置齿轮按钮仍存在并触发 onSettingsOpen
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";

vi.mock("../../../stores/projectStore", () => ({
  useProjectStore: <T,>(selector: (s: { getActiveProject: () => null }) => T) =>
    selector({ getActiveProject: () => null }),
}));

import { TitleBar } from "../TitleBar";

beforeEach(() => {
  vi.clearAllMocks();
});

describe("TitleBar — ⌘K 按钮（AC-3 / AC-6）", () => {
  it("当传入 onSearchOpen 时，⌘K 按钮可见，aria-label = 打开搜索 (⌘K)", () => {
    render(<TitleBar onSearchOpen={() => {}} />);
    const btn = screen.getByLabelText("打开搜索 (⌘K)");
    expect(btn).toBeInTheDocument();
  });

  it("点击 ⌘K 按钮触发 onSearchOpen", () => {
    const onSearchOpen = vi.fn();
    render(<TitleBar onSearchOpen={onSearchOpen} />);
    fireEvent.click(screen.getByLabelText("打开搜索 (⌘K)"));
    expect(onSearchOpen).toHaveBeenCalledTimes(1);
  });

  it("点击设置齿轮触发 onSettingsOpen", () => {
    const onSettingsOpen = vi.fn();
    render(<TitleBar onSettingsOpen={onSettingsOpen} />);
    fireEvent.click(screen.getByLabelText("打开设置"));
    expect(onSettingsOpen).toHaveBeenCalledTimes(1);
  });

  it("未传 onSearchOpen 时不渲染 ⌘K 按钮", () => {
    render(<TitleBar onSettingsOpen={() => {}} />);
    expect(screen.queryByLabelText("打开搜索 (⌘K)")).not.toBeInTheDocument();
  });
});
