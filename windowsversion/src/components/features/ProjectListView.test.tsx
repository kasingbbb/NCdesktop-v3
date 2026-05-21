import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { ProjectListView } from "./ProjectListView";
import { logger } from "../../utils/logger";

import { useVirtualizer } from "@tanstack/react-virtual";

const projectHoisted = vi.hoisted(() => {
  const store = {
    projects: [] as any[],
    viewMode: 'grid',
    fetchProjects: vi.fn(async () => {}),
  };
  return { 
    store,
    setItems: (items: any[]) => { store.projects = items; },
    setViewMode: (mode: string) => { store.viewMode = mode; }
  };
});

vi.mock("../../stores/projectStore", () => ({
  useProjectStore: () => projectHoisted.store,
}));

vi.mock("../../stores/libraryStore", () => ({
  useLibraryStore: () => ({
    activeLibraryId: "lib-1",
    ensureActiveLibrary: vi.fn().mockResolvedValue("lib-1"),
  }),
}));

const { virtualizerMock } = vi.hoisted(() => ({
  virtualizerMock: vi.fn(() => ({
    getTotalSize: () => 100,
    getVirtualItems: () => [],
  })),
}));

vi.mock("@tanstack/react-virtual", () => ({
  useVirtualizer: virtualizerMock,
}));

vi.mock("./ProjectCard", () => ({ ProjectCard: () => <div data-testid="project-card" /> }));
vi.mock("./ProjectListItem", () => ({ ProjectListItem: () => <div data-testid="project-list-item" /> }));
vi.mock("./EmptyState", () => ({ EmptyState: () => <div data-testid="empty-state" /> }));

vi.spyOn(logger, "info");

describe("ProjectListView Component", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    projectHoisted.setItems([]);
  });

  it("renders EmptyState when there are no projects", () => {
    render(<ProjectListView />);
    expect(screen.getByTestId("empty-state")).toBeInTheDocument();
  });

  it("renders ProjectCard in grid mode when items exist", () => {
    projectHoisted.setItems([{ id: "p1", name: "Project 1" }]);
    projectHoisted.setViewMode("grid");
    render(<ProjectListView />);
    expect(screen.getByTestId("project-card")).toBeInTheDocument();
    expect(logger.info).toHaveBeenCalledWith("ProjectListView", "Fetching projects", { libraryId: "lib-1" });
  });

  it("renders ProjectListItem in list mode (virtualized mock)", () => {
    projectHoisted.setItems([{ id: "p1", name: "Project 1" }]);
    projectHoisted.setViewMode("list");
    
    // Mock virtualizer items
    virtualizerMock.mockReturnValue({
      getTotalSize: () => 64,
      getVirtualItems: () => [{ index: 0, start: 0, size: 64 }],
    } as any);
    
    render(<ProjectListView />);
    expect(screen.getByTestId("project-list-item")).toBeInTheDocument();
  });
});
