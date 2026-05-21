import { useCallback, useEffect, useRef, useState } from "react";
import { Search, X, Loader2 } from "lucide-react";
import { useSearchStore } from "../../stores";
import {
  SearchResultItem,
  type SearchResultData,
} from "./SearchResultItem";
import { logger } from "../../utils/logger";

interface SearchPanelProps {
  isOpen: boolean;
  onClose: () => void;
  onNavigate?: (result: SearchResultData) => void;
}

export function SearchPanel({ isOpen, onClose, onNavigate }: SearchPanelProps) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const [results, setResults] = useState<SearchResultData[]>([]);
  const [isSearching, setIsSearching] = useState(false);

  const { performSearch } = useSearchStore();

  useEffect(() => {
    if (isOpen) {
      setQuery("");
      setResults([]);
      setActiveIndex(0);
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [isOpen]);

  useEffect(() => {
    if (!query.trim()) {
      setResults([]);
      return;
    }

    const timer = setTimeout(async () => {
      setIsSearching(true);
      logger.debug("SearchPanel", "Performing search", { query });
      try {
        const raw = await performSearch(query);
        const mapped: SearchResultData[] = raw.map((r) => ({
          id: r.id,
          type: r.type as SearchResultData["type"],
          title: r.title,
          snippet: r.snippet,
          projectName: r.projectId ?? null,
          score: r.score,
        }));
        setResults(mapped);
        setActiveIndex(0);
      } catch (e) {
        logger.error("SearchPanel", "Search failed", { query, error: e });
        setResults([]);
      } finally {
        setIsSearching(false);
      }
    }, 200);

    return () => clearTimeout(timer);
  }, [query, performSearch]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
        return;
      }
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setActiveIndex((i) => Math.min(i + 1, results.length - 1));
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setActiveIndex((i) => Math.max(i - 1, 0));
      }
      if (e.key === "Enter" && results[activeIndex]) {
        logger.info("SearchPanel", "Result selected (Enter)", {
          id: results[activeIndex].id,
          title: results[activeIndex].title,
        });
        onNavigate?.(results[activeIndex]);
        onClose();
      }
    },
    [results, activeIndex, onClose, onNavigate],
  );

  if (!isOpen) return null;

  return (
    <>
      {/* 半透明遮罩 + 毛玻璃 */}
      <div
        className="fixed inset-0 z-50"
        style={{
          background: "rgba(0,0,0,0.35)",
          backdropFilter: "blur(4px)",
          WebkitBackdropFilter: "blur(4px)",
        }}
        onClick={onClose}
      />

      {/* Command Palette */}
      <div
        className="fixed z-50 left-1/2 -translate-x-1/2 w-[540px] max-w-[90vw] overflow-hidden"
        style={{
          top: "100px",
          background: "var(--surface-primary)",
          border: "1px solid var(--border-primary)",
          borderRadius: "var(--radius-2xl)",
          boxShadow: "var(--shadow-lg)",
          animation: "cmdEnter var(--duration-normal) var(--ease-out-expo)",
        }}
      >
        {/* 搜索输入 */}
        <div
          className="flex items-center gap-[10px] px-[16px] py-[12px] border-b"
          style={{ borderColor: "var(--border-primary)" }}
        >
          <Search size={15} style={{ color: "var(--text-tertiary)", flexShrink: 0 }} />
          <input
            ref={inputRef}
            type="text"
            className="flex-1 bg-transparent border-none outline-none text-[15px]"
            style={{ color: "var(--text-primary)", fontFamily: "inherit" }}
            placeholder="搜索笔记、项目、标签…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleKeyDown}
          />
          {isSearching && <Loader2 size={14} className="animate-spin" style={{ color: "var(--text-tertiary)" }} />}
          <span
            className="text-[11px] px-[6px] py-[2px] rounded-[4px] whitespace-nowrap"
            style={{
              background: "var(--surface-tertiary)",
              border: "1px solid var(--border-primary)",
              color: "var(--text-tertiary)",
              fontFamily: "var(--font-mono)",
            }}
          >
            Esc
          </span>
        </div>

        {/* 结果列表 */}
        <div className="max-h-[360px] overflow-y-auto">
          {results.length > 0 ? (
            <div className="py-[6px]">
              {results.map((result, index) => (
                <SearchResultItem
                  key={result.id}
                  result={result}
                  isActive={index === activeIndex}
                  onSelect={(r) => {
                    logger.info("SearchPanel", "Result selected (Click)", {
                      id: r.id,
                      title: r.title,
                    });
                    onNavigate?.(r);
                    onClose();
                  }}
                />
              ))}
            </div>
          ) : query.trim() && !isSearching ? (
            <div className="flex flex-col items-center py-[var(--space-8)]">
              <Search size={32} style={{ color: "var(--text-tertiary)", opacity: 0.3 }} />
              <p className="text-[var(--text-sm)] mt-[var(--space-2)]" style={{ color: "var(--text-tertiary)" }}>
                无匹配结果
              </p>
            </div>
          ) : null}
        </div>

        {/* 底部快捷键提示 */}
        <div
          className="flex gap-[12px] px-[14px] py-[8px] border-t"
          style={{ borderColor: "var(--border-primary)" }}
        >
          <FooterHint keys="↑↓" label="导航" />
          <FooterHint keys="↵" label="打开" />
          <FooterHint keys="Esc" label="关闭" />
        </div>
      </div>
    </>
  );
}

function FooterHint({ keys, label }: { keys: string; label: string }) {
  return (
    <span className="text-[11px] flex items-center gap-[4px]" style={{ color: "var(--text-tertiary)" }}>
      <span
        className="px-[5px] py-[1px] rounded-[3px]"
        style={{
          background: "var(--surface-tertiary)",
          border: "1px solid var(--border-primary)",
          fontFamily: "var(--font-mono)",
          fontSize: "10px",
        }}
      >
        {keys}
      </span>
      {label}
    </span>
  );
}
