import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { SearchPanel } from "./SearchPanel";
import { logger } from "../../utils/logger";

// vi.useFakeTimers(); // Removing fake timers as it might cause hangs in some environments

const searchHoisted = vi.hoisted(() => ({
  performSearch: vi.fn(async (q) => [
    { id: "1", title: "Result 1", type: "project", snippet: "desc", score: 1 }
  ]),
}));

vi.mock("../../stores", () => ({
  useSearchStore: () => ({
    performSearch: searchHoisted.performSearch
  }),
}));

vi.mock("./SearchResultItem", () => ({
  SearchResultItem: ({ result, isActive, onSelect }: any) => (
    <div 
      data-testid="search-result-item" 
      data-active={isActive} 
      onClick={() => onSelect(result)}
    >
      {result.title}
    </div>
  )
}));

vi.spyOn(logger, "info");
vi.spyOn(logger, "debug");

describe("SearchPanel Component", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("does not render when closed", () => {
    const { container } = render(<SearchPanel isOpen={false} onClose={vi.fn()} />);
    expect(container.firstChild).toBeNull();
  });

  it("renders when open and displays input", () => {
    render(<SearchPanel isOpen={true} onClose={vi.fn()} />);
    expect(screen.getByPlaceholderText(/搜索/)).toBeInTheDocument();
  });

  it("performs search after internal debounce", async () => {
    render(<SearchPanel isOpen={true} onClose={vi.fn()} />);
    const input = screen.getByPlaceholderText(/搜索/);
    
    fireEvent.change(input, { target: { value: "test" } });
    
    // We wait for the 200ms debounce
    const item = await screen.findByTestId("search-result-item", {}, { timeout: 2000 });
    expect(item).toHaveTextContent("Result 1");
    expect(searchHoisted.performSearch).toHaveBeenCalledWith("test");
    expect(logger.debug).toHaveBeenCalledWith("SearchPanel", "Performing search", { query: "test" });
  });

  it("calls onNavigate and logs when item is selected", async () => {
    const onNavigate = vi.fn();
    render(<SearchPanel isOpen={true} onClose={vi.fn()} onNavigate={onNavigate} />);
    
    const input = screen.getByPlaceholderText(/搜索/);
    fireEvent.change(input, { target: { value: "test" } });
    
    const item = await screen.findByTestId("search-result-item", {}, { timeout: 2000 });
    fireEvent.click(item);

    expect(onNavigate).toHaveBeenCalledWith(expect.objectContaining({ id: "1" }));
    expect(logger.info).toHaveBeenCalledWith("SearchPanel", "Result selected (Click)", expect.any(Object));
  });

  it("navigates with keyboard Enter", async () => {
    const onNavigate = vi.fn();
    render(<SearchPanel isOpen={true} onClose={vi.fn()} onNavigate={onNavigate} />);
    
    const input = screen.getByPlaceholderText(/搜索/);
    fireEvent.change(input, { target: { value: "test" } });
    
    await screen.findByTestId("search-result-item", {}, { timeout: 2000 });

    fireEvent.keyDown(input, { key: "Enter" });
    expect(onNavigate).toHaveBeenCalled();
    expect(logger.info).toHaveBeenCalledWith("SearchPanel", "Result selected (Enter)", expect.any(Object));
  });
});
