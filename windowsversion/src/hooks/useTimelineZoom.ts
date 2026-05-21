import { useCallback, useEffect, useRef } from "react";
import { useTimelineStore } from "../stores";

export function useTimelineZoom(containerRef: React.RefObject<HTMLElement | null>): void {
  const lastZoomRef = useRef<number>(0);

  const handleWheel = useCallback(
    (e: WheelEvent) => {
      if (!e.ctrlKey && !e.metaKey) return;
      e.preventDefault();

      const now = Date.now();
      if (now - lastZoomRef.current < 50) return;
      lastZoomRef.current = now;

      if (e.deltaY < 0) {
        useTimelineStore.getState().zoomIn();
      } else {
        useTimelineStore.getState().zoomOut();
      }
    },
    []
  );

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    el.addEventListener("wheel", handleWheel, { passive: false });
    return () => el.removeEventListener("wheel", handleWheel);
  }, [containerRef, handleWheel]);
}
