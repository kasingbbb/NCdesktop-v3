import { useCallback, useEffect, useRef, useState } from "react";

export interface SelectionRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

const DRAG_THRESHOLD = 5;

interface UseRubberBandSelectOptions {
  containerRef: React.RefObject<HTMLElement | null>;
  getItemRects: () => Array<{ id: string; rect: DOMRect }>;
  onSelectionChange: (ids: Set<string>) => void;
}

export function useRubberBandSelect({
  containerRef,
  getItemRects,
  onSelectionChange,
}: UseRubberBandSelectOptions) {
  const [selectionRect, setSelectionRect] = useState<SelectionRect | null>(null);
  const isSelectingRef = useRef(false);
  const pendingRef = useRef(false);
  const startPosRef = useRef({ x: 0, y: 0 });
  const startClientRef = useRef({ x: 0, y: 0 });

  const rectsIntersect = (a: SelectionRect, b: DOMRect): boolean => {
    return (
      a.x < b.right &&
      a.x + a.width > b.left &&
      a.y < b.bottom &&
      a.y + a.height > b.top
    );
  };

  const handleMouseDown = useCallback(
    (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      if (
        e.button !== 0 ||
        target.closest("a") ||
        target.closest("[data-no-rubber]")
      ) return;

      const onCard = !!target.closest("[data-asset-card]");
      if (onCard) return;

      const container = containerRef.current;
      if (!container) return;

      const containerRect = container.getBoundingClientRect();
      startPosRef.current = {
        x: e.clientX - containerRect.left + container.scrollLeft,
        y: e.clientY - containerRect.top + container.scrollTop,
      };
      startClientRef.current = { x: e.clientX, y: e.clientY };
      pendingRef.current = true;
      isSelectingRef.current = false;
      e.preventDefault();
    },
    [containerRef]
  );

  const handleMouseMove = useCallback(
    (e: MouseEvent) => {
      if (!pendingRef.current && !isSelectingRef.current) return;
      const container = containerRef.current;
      if (!container) return;

      if (pendingRef.current && !isSelectingRef.current) {
        const dx = e.clientX - startClientRef.current.x;
        const dy = e.clientY - startClientRef.current.y;
        if (Math.abs(dx) < DRAG_THRESHOLD && Math.abs(dy) < DRAG_THRESHOLD) return;
        pendingRef.current = false;
        isSelectingRef.current = true;
        onSelectionChange(new Set());
      }

      const containerRect = container.getBoundingClientRect();
      const currentX = e.clientX - containerRect.left + container.scrollLeft;
      const currentY = e.clientY - containerRect.top + container.scrollTop;

      const rect: SelectionRect = {
        x: Math.min(startPosRef.current.x, currentX),
        y: Math.min(startPosRef.current.y, currentY),
        width: Math.abs(currentX - startPosRef.current.x),
        height: Math.abs(currentY - startPosRef.current.y),
      };

      const viewportRect: SelectionRect = {
        x: rect.x - container.scrollLeft + containerRect.left,
        y: rect.y - container.scrollTop + containerRect.top,
        width: rect.width,
        height: rect.height,
      };

      setSelectionRect(rect);

      const itemRects = getItemRects();
      const selected = new Set<string>();
      for (const { id, rect: itemRect } of itemRects) {
        if (rectsIntersect(viewportRect, itemRect)) {
          selected.add(id);
        }
      }
      onSelectionChange(selected);
    },
    [containerRef, getItemRects, onSelectionChange]
  );

  const handleMouseUp = useCallback(() => {
    pendingRef.current = false;
    if (!isSelectingRef.current) return;
    isSelectingRef.current = false;
    setSelectionRect(null);
  }, []);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    container.addEventListener("mousedown", handleMouseDown);
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);

    return () => {
      container.removeEventListener("mousedown", handleMouseDown);
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [containerRef, handleMouseDown, handleMouseMove, handleMouseUp]);

  return { selectionRect, isSelecting: isSelectingRef.current };
}
