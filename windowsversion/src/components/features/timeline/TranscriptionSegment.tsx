import { useCallback, useState } from "react";
import type { KeyboardEvent, ReactElement } from "react";
import type { TranscriptionSegment as TSegment } from "../../../types";

interface TranscriptionSegmentProps {
  segment: TSegment;
  index: number;
  isActive: boolean;
  searchHighlights?: Array<{ start: number; end: number }>;
  onSeek: (time: number) => void;
  onEdit?: (index: number, newText: string) => void;
}

export function TranscriptionSegmentRow({
  segment,
  index,
  isActive,
  searchHighlights,
  onSeek,
  onEdit,
}: TranscriptionSegmentProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [editText, setEditText] = useState(segment.text);

  const handleClick = useCallback(() => {
    onSeek(segment.startTime);
  }, [segment.startTime, onSeek]);

  const handleDoubleClick = useCallback(() => {
    if (onEdit) {
      setIsEditing(true);
      setEditText(segment.text);
    }
  }, [segment.text, onEdit]);

  const handleEditConfirm = useCallback(() => {
    setIsEditing(false);
    if (editText !== segment.text) {
      onEdit?.(index, editText);
    }
  }, [editText, segment.text, index, onEdit]);

  const handleEditKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        handleEditConfirm();
      } else if (e.key === "Escape") {
        setIsEditing(false);
        setEditText(segment.text);
      }
    },
    [handleEditConfirm, segment.text]
  );

  const renderText = (): ReactElement => {
    if (!searchHighlights || searchHighlights.length === 0) {
      return <span>{segment.text}</span>;
    }

    const parts: ReactElement[] = [];
    let lastEnd = 0;
    for (const hl of searchHighlights || []) {
      if (hl.start > lastEnd) {
        parts.push(<span key={`t-${lastEnd}`}>{segment.text.slice(lastEnd, hl.start)}</span>);
      }
      parts.push(
        <mark
          key={`h-${hl.start}`}
          style={{ backgroundColor: "rgba(255, 192, 0, 0.3)", borderRadius: 2 }}
        >
          {segment.text.slice(hl.start, hl.end)}
        </mark>
      );
      lastEnd = hl.end;
    }
    if (lastEnd < segment.text.length) {
      parts.push(<span key={`t-${lastEnd}`}>{segment.text.slice(lastEnd)}</span>);
    }
    return <>{parts}</>;
  };

  return (
    <div
      className="flex gap-[var(--space-2)] py-[var(--space-1)] px-[var(--space-2)] rounded-[var(--radius-sm)] cursor-pointer transition-colors"
      style={{
        backgroundColor: isActive ? "rgba(255, 192, 0, 0.08)" : "transparent",
      }}
      onClick={handleClick}
      onDoubleClick={handleDoubleClick}
    >
      {/* 时间戳 */}
      <span
        className="flex-shrink-0 text-[var(--text-xs)] tabular-nums"
        style={{
          color: isActive ? "var(--text-primary)" : "var(--text-tertiary)",
          minWidth: 48,
        }}
      >
        {formatSegmentTime(segment.startTime)}
      </span>

      {/* 说话人 */}
      {segment.speaker && (
        <span
          className="flex-shrink-0 text-[var(--text-xs)] font-medium"
          style={{ color: "var(--brand-navy)", minWidth: 24 }}
        >
          {segment.speaker}
        </span>
      )}

      {/* 文本 */}
      {isEditing ? (
        <textarea
          className="flex-1 text-[var(--text-sm)] input-glass resize-none min-h-[24px]"
          value={editText}
          onChange={(e) => setEditText(e.target.value)}
          onBlur={handleEditConfirm}
          onKeyDown={handleEditKeyDown}
          autoFocus
        />
      ) : (
        <span
          className="flex-1 text-[var(--text-sm)] leading-relaxed"
          style={{
            color: "var(--text-primary)",
            textDecoration: isActive ? "underline" : "none",
            textDecorationColor: isActive ? "var(--text-secondary)" : "transparent",
            textUnderlineOffset: 3,
          }}
        >
          {renderText()}
        </span>
      )}
    </div>
  );
}

function formatSegmentTime(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}
