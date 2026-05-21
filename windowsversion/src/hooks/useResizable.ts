import { useState, useCallback, useRef, useEffect } from "react";

interface UseResizableOptions {
  initialWidth: number;
  minWidth: number;
  maxWidth: number;
  direction?: "left" | "right";
}

interface UseResizableReturn {
  width: number;
  isResizing: boolean;
  handleMouseDown: (e: React.MouseEvent) => void;
}

export function useResizable({
  initialWidth,
  minWidth,
  maxWidth,
  direction = "right",
}: UseResizableOptions): UseResizableReturn {
  const [width, setWidth] = useState(initialWidth);
  const [isResizing, setIsResizing] = useState(false);
  const startXRef = useRef(0);
  const startWidthRef = useRef(0);

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      setIsResizing(true);
      startXRef.current = e.clientX;
      startWidthRef.current = width;
    },
    [width],
  );

  useEffect(() => {
    if (!isResizing) return;

    const handleMouseMove = (e: MouseEvent): void => {
      const delta =
        direction === "right"
          ? e.clientX - startXRef.current
          : startXRef.current - e.clientX;
      const newWidth = Math.min(
        maxWidth,
        Math.max(minWidth, startWidthRef.current + delta),
      );
      setWidth(newWidth);
    };

    const handleMouseUp = (): void => {
      setIsResizing(false);
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";

    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
  }, [isResizing, minWidth, maxWidth, direction]);

  return { width, isResizing, handleMouseDown };
}
