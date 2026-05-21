import { useCallback, useEffect, useRef, useMemo } from "react";
import { useTimelineStore } from "../../../stores";
import { useTranscriptionSync } from "../../../hooks/useTranscriptionSync";
import { TranscriptionSegmentRow } from "./TranscriptionSegment";
import { TranscriptionSearch, type SearchMatch } from "./TranscriptionSearch";
import type { TranscriptionSegment } from "../../../types";

interface TranscriptionPanelProps {
  segments: TranscriptionSegment[];
  onSegmentEdit?: (index: number, newText: string) => void;
}

export function TranscriptionPanel({
  segments,
  onSegmentEdit,
}: TranscriptionPanelProps) {
  const listRef = useRef<HTMLDivElement>(null);
  const seek = useTimelineStore((s) => s.seek);
  const isPlaying = useTimelineStore((s) => s.playback.isPlaying);
  const { activeSegmentIndex, isSegmentActive } = useTranscriptionSync(segments);

  // 播放时自动滚动到当前句子
  useEffect(() => {
    if (!isPlaying || activeSegmentIndex < 0) return;
    const container = listRef.current;
    if (!container) return;

    const segEl = container.children[activeSegmentIndex] as HTMLElement | undefined;
    if (segEl) {
      segEl.scrollIntoView({ behavior: "smooth", block: "center" });
    }
  }, [activeSegmentIndex, isPlaying]);

  const handleSeek = useCallback(
    (time: number) => {
      seek(time);
    },
    [seek]
  );

  const handleJumpToSegment = useCallback(
    (segmentIndex: number) => {
      const seg = segments[segmentIndex];
      if (seg) {
        seek(seg.startTime);
        const container = listRef.current;
        const segEl = container?.children[segmentIndex] as HTMLElement | undefined;
        if (segEl) {
          segEl.scrollIntoView({ behavior: "smooth", block: "center" });
        }
      }
    },
    [segments, seek]
  );

  const { element: searchBar, matches } = TranscriptionSearch({
    segments,
    onJumpToSegment: handleJumpToSegment,
  });

  const matchMap = useMemo(() => {
    const map = new Map<number, SearchMatch>();
    for (const m of matches) {
      map.set(m.segmentIndex, m);
    }
    return map;
  }, [matches]);

  if (segments.length === 0) {
    return (
      <div className="flex items-center justify-center h-full" style={{ color: "var(--text-tertiary)" }}>
        <span className="text-[var(--text-sm)]">暂无转录数据</span>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* 搜索栏 */}
      {searchBar}

      {/* 转录列表 */}
      <div
        ref={listRef}
        className="flex-1 overflow-y-auto px-[var(--space-2)] py-[var(--space-1)]"
      >
        {segments.map((seg, idx) => (
          <TranscriptionSegmentRow
            key={`${seg.startTime}-${idx}`}
            segment={seg}
            index={idx}
            isActive={isSegmentActive(seg)}
            searchHighlights={matchMap.get(idx)?.ranges}
            onSeek={handleSeek}
            onEdit={onSegmentEdit}
          />
        ))}
      </div>
    </div>
  );
}
