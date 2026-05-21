import { useCallback, useRef, useState } from "react";
import { useTimelineStore } from "../../../stores";

interface SelectionOverlayProps {
  containerWidth: number;
  height: number;
  onSelect?: (startTime: number, endTime: number) => void;
}

export function SelectionOverlay({
  containerWidth,
  height,
  onSelect,
}: SelectionOverlayProps) {
  const viewport = useTimelineStore((s) => s.viewport);
  const [selection, setSelection] = useState<{ start: number; end: number } | null>(null);
  const isDragging = useRef(false);
  const startX = useRef(0);

  const xToTime = useCallback(
    (x: number): number => {
      const ratio = x / containerWidth;
      return viewport.startTime + ratio * (viewport.endTime - viewport.startTime);
    },
    [containerWidth, viewport]
  );

  const timeToX = useCallback(
    (time: number): number => {
      const ratio = (time - viewport.startTime) / (viewport.endTime - viewport.startTime);
      return ratio * containerWidth;
    },
    [containerWidth, viewport]
  );

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      if (!e.altKey) return;
      isDragging.current = true;
      startX.current = e.nativeEvent.offsetX;
      setSelection({ start: xToTime(startX.current), end: xToTime(startX.current) });
    },
    [xToTime]
  );

  const handleMouseMove = useCallback(
    (e: React.MouseEvent) => {
      if (!isDragging.current) return;
      const currentX = e.nativeEvent.offsetX;
      const t1 = xToTime(startX.current);
      const t2 = xToTime(currentX);
      setSelection({ start: Math.min(t1, t2), end: Math.max(t1, t2) });
    },
    [xToTime]
  );

  const handleMouseUp = useCallback(() => {
    isDragging.current = false;
    if (selection && onSelect && selection.end - selection.start > 0.1) {
      onSelect(selection.start, selection.end);
    }
  }, [selection, onSelect]);

  return (
    <div
      className="absolute inset-0"
      style={{ height, zIndex: 10, pointerEvents: "auto" }}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
    >
      {selection && (
        <div
          className="absolute top-0 bottom-0"
          style={{
            left: timeToX(selection.start),
            width: timeToX(selection.end) - timeToX(selection.start),
            backgroundColor: "rgba(31, 69, 110, 0.10)",
            borderLeft: "1px solid rgba(31, 69, 110, 0.3)",
            borderRight: "1px solid rgba(31, 69, 110, 0.3)",
          }}
        />
      )}
    </div>
  );
}
