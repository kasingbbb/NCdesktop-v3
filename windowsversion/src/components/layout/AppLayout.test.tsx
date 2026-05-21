import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { AppLayout } from "./AppLayout";
import { logger } from "../../utils/logger";

vi.mock("./TitleBar", () => ({ TitleBar: () => <div data-testid="title-bar" /> }));
vi.mock("./Sidebar", () => ({ Sidebar: () => <div data-testid="sidebar" /> }));
vi.mock("./ContentArea", () => ({ ContentArea: () => <div data-testid="content-area" /> }));
vi.mock("./Inspector", () => ({ Inspector: () => <div data-testid="inspector" /> }));
vi.mock("./ResizeHandle", () => ({ ResizeHandle: () => <div data-testid="resize-handle" /> }));

vi.mock("../../hooks/useResizable", () => ({
  useResizable: () => ({
    width: 200,
    isResizing: false,
    handleMouseDown: vi.fn(),
  }),
}));

vi.spyOn(logger, "info");

describe("AppLayout Component", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders TitleBar, Sidebar, and ContentArea on wide screens", () => {
    Object.defineProperty(window, "innerWidth", { writable: true, configurable: true, value: 1200 });
    
    render(<AppLayout />);
    expect(screen.getByTestId("title-bar")).toBeInTheDocument();
    expect(screen.getByTestId("sidebar")).toBeInTheDocument();
    expect(screen.getByTestId("content-area")).toBeInTheDocument();
    expect(screen.getByTestId("inspector")).toBeInTheDocument();
  });

  it("changes layout to two-column when screen width strictly between 700 and 1200", () => {
    Object.defineProperty(window, "innerWidth", { writable: true, configurable: true, value: 800 });
    render(<AppLayout />);
    // Initial state sets it properly based on window size
    expect(screen.getByTestId("sidebar")).toBeInTheDocument();
    expect(screen.queryByTestId("inspector")).not.toBeInTheDocument();
  });

  it("hides sidebar on narrow screens (single-column)", () => {
    Object.defineProperty(window, "innerWidth", { writable: true, configurable: true, value: 800 });
    render(<AppLayout />);
    expect(screen.getByTestId("sidebar")).toBeInTheDocument();

    Object.defineProperty(window, "innerWidth", { writable: true, configurable: true, value: 500 });
    fireEvent(window, new Event("resize"));

    expect(screen.queryByTestId("sidebar")).not.toBeInTheDocument();
    expect(screen.queryByTestId("inspector")).not.toBeInTheDocument();
    expect(screen.getByTestId("content-area")).toBeInTheDocument();
    expect(logger.info).toHaveBeenCalledWith("AppLayout", "Layout changed", { mode: "single-column" });
  });
});
