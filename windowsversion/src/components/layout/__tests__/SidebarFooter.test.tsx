/**
 * v2 Sidebar Redesign — SidebarFooter 视图层单元测试（task_004 / F-P0-6 / AC-9）。
 *
 * 覆盖：
 *   - AC-5/AC-9：footer DOM 行数 = 2
 *   - AC-9：未插入 TF 时显示小圆点（降级），插入时显示 TF 徽章
 *   - 「悬浮导入」点击仍触发 toggle_dropzone_window
 *   - 「设置」点击仍触发 onSettingsOpen
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";

const invokeMock = vi.fn().mockResolvedValue(undefined);
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

import { SidebarFooter } from "../SidebarFooter";
import { useSyncStore } from "../../../stores/syncStore";

const INITIAL_SYNC = useSyncStore.getState();

beforeEach(() => {
  useSyncStore.setState({ ...INITIAL_SYNC, isTFCardConnected: false });
  invokeMock.mockClear();
});

describe("SidebarFooter — 2 行结构（AC-5 / AC-9）", () => {
  it("DOM 行数 = 2（设置 + 悬浮导入合并行）", () => {
    render(<SidebarFooter />);
    const footer = screen.getByTestId("sidebar-footer");
    // 直接子元素：两个 SidebarItem button
    const buttons = footer.querySelectorAll("button.sidebar-item");
    expect(buttons.length).toBe(2);
    expect(screen.getByRole("button", { name: /设置/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /悬浮导入/ })).toBeInTheDocument();
  });

  it("未插入 TF 时显示小圆点（降级，无 TF 文字行）", () => {
    render(<SidebarFooter />);
    expect(screen.getByTestId("sidebar-footer-tf-dot")).toBeInTheDocument();
    expect(screen.queryByTestId("sidebar-footer-tf-badge")).not.toBeInTheDocument();
    expect(screen.queryByText(/未插入 TF 卡/)).not.toBeInTheDocument();
  });

  it("插入 TF 时显示 TF 徽章替换小圆点", () => {
    useSyncStore.setState({ isTFCardConnected: true });
    render(<SidebarFooter />);
    expect(screen.getByTestId("sidebar-footer-tf-badge")).toBeInTheDocument();
    expect(screen.queryByTestId("sidebar-footer-tf-dot")).not.toBeInTheDocument();
  });

  it("点击「悬浮导入」触发 toggle_dropzone_window", () => {
    render(<SidebarFooter />);
    fireEvent.click(screen.getByRole("button", { name: /悬浮导入/ }));
    expect(invokeMock).toHaveBeenCalledWith("toggle_dropzone_window");
  });

  it("点击「设置」触发 onSettingsOpen", () => {
    const onSettingsOpen = vi.fn();
    render(<SidebarFooter onSettingsOpen={onSettingsOpen} />);
    fireEvent.click(screen.getByRole("button", { name: /设置/ }));
    expect(onSettingsOpen).toHaveBeenCalledTimes(1);
  });
});
