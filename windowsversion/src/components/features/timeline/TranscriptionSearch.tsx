import { useCallback, useMemo, useState } from "react";
import type { ReactElement } from "react";
import { Search, ChevronUp, ChevronDown, X } from "lucide-react";
import type { TranscriptionSegment } from "../../../types";

interface TranscriptionSearchProps {
  segments: TranscriptionSegment[];
  onJumpToSegment: (segmentIndex: number) => void;
}

export interface SearchMatch {
  segmentIndex: number;
  ranges: Array<{ start: number; end: number }>;
}

export function TranscriptionSearch({
  segments,
  onJumpToSegment,
}: TranscriptionSearchProps): {
  element: ReactElement;
  matches: SearchMatch[];
  currentMatchIndex: number;
} {
  const [query, setQuery] = useState("");
  const [currentMatchIndex, setCurrentMatchIndex] = useState(0);

  const matches = useMemo((): SearchMatch[] => {
    if (!query.trim()) return [];
    const q = query.toLowerCase();
    const result: SearchMatch[] = [];

    segments.forEach((seg, segIdx) => {
      const text = seg.text.toLowerCase();
      const ranges: Array<{ start: number; end: number }> = [];
      let pos = 0;
      while (true) {
        const idx = text.indexOf(q, pos);
        if (idx === -1) break;
        ranges.push({ start: idx, end: idx + q.length });
        pos = idx + 1;
      }
      if (ranges.length > 0) {
        result.push({ segmentIndex: segIdx, ranges });
      }
    });

    return result;
  }, [segments, query]);

  const goToMatch = useCallback(
    (index: number) => {
      if (matches.length === 0) return;
      const clamped = ((index % matches.length) + matches.length) % matches.length;
      setCurrentMatchIndex(clamped);
      onJumpToSegment(matches[clamped].segmentIndex);
    },
    [matches, onJumpToSegment]
  );

  const handleNext = useCallback(() => goToMatch(currentMatchIndex + 1), [goToMatch, currentMatchIndex]);
  const handlePrev = useCallback(() => goToMatch(currentMatchIndex - 1), [goToMatch, currentMatchIndex]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        if (e.shiftKey) handlePrev();
        else handleNext();
      }
    },
    [handleNext, handlePrev]
  );

  const element = (
    <div className="flex items-center gap-[var(--space-1)] px-[var(--space-2)] py-[var(--space-1)]">
      <Search size={14} style={{ color: "var(--text-tertiary)" }} />
      <input
        type="text"
        className="input-glass flex-1 text-[var(--text-xs)] py-0.5 px-[var(--space-2)]"
        placeholder="搜索转录文本..."
        value={query}
        onChange={(e) => {
          setQuery(e.target.value);
          setCurrentMatchIndex(0);
        }}
        onKeyDown={handleKeyDown}
      />
      {query && (
        <>
          <span className="text-[10px] tabular-nums" style={{ color: "var(--text-tertiary)" }}>
            {matches.length > 0 ? `${currentMatchIndex + 1}/${matches.length}` : "0"}
          </span>
          <button onClick={handlePrev} className="btn-glass p-0.5" title="上一个">
            <ChevronUp size={12} />
          </button>
          <button onClick={handleNext} className="btn-glass p-0.5" title="下一个">
            <ChevronDown size={12} />
          </button>
          <button onClick={() => setQuery("")} className="btn-glass p-0.5" title="清除">
            <X size={12} />
          </button>
        </>
      )}
    </div>
  );

  return { element, matches, currentMatchIndex };
}
