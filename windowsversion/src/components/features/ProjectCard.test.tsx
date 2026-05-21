import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { ProjectCard } from "./ProjectCard";
import { logger } from "../../utils/logger";
import type { Project } from "../../types";

vi.mock("lucide-react", () => ({
  Clock: () => <div data-testid="icon-clock" />,
  HardDrive: () => <div data-testid="icon-harddrive" />,
  Tag: () => <div data-testid="icon-tag" />,
  Trash2: () => <div data-testid="icon-trash" />,
}));

vi.spyOn(logger, "info");

const mockProject: Project = {
  id: "p1",
  name: "Test Project",
  createdAt: "2023-01-01T00:00:00Z",
  updatedAt: "2023-01-01T00:00:00Z",
  libraryId: "lib1",
  tags: [{ id: "t1", name: "Tag 1" }],
  metadata: { assetCount: 5 }
} as any;

describe("ProjectCard Component", () => {
  it("renders project name and metadata", () => {
    render(<ProjectCard project={mockProject} />);
    expect(screen.getByText("Test Project")).toBeInTheDocument();
    expect(screen.getByText("5 items")).toBeInTheDocument();
  });

  it("calls onClick and logs event when clicked", () => {
    const onClick = vi.fn();
    render(<ProjectCard project={mockProject} onClick={onClick} />);
    
    fireEvent.click(screen.getByText("Test Project").parentElement!);
    
    expect(onClick).toHaveBeenCalled();
    expect(logger.info).toHaveBeenCalledWith("ProjectCard", "Project clicked", { id: "p1", name: "Test Project" });
  });

  it("renders tags properly", () => {
    render(<ProjectCard project={mockProject} />);
    expect(screen.getByText("Tag 1")).toBeInTheDocument();
  });
});
