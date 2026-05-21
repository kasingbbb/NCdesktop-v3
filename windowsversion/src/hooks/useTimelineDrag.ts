import { useCallback, useEffect, useRef, useState } from "react";
import { useTimelineStore } from "../stores";

export function useTimelineDrag(containerRef: React.RefObject<HTMLElement | null>): {
  isDragging: boolean;
} {
  const draggingRef = useRef(false);
  const [isDragging, setIsDragging] = useState(false);
  const startXRef = useRef(0);
  const startTimeRef = useRef(0);
  const velocityRef = useRef(0);

  const handleMouseDown = useCallback((e: MouseEvent) => {
    if (e.button !== 0) return;
    draggingRef.current = true;
    setIsDragging(true);
    startXRef.current = e.clientX;
    startTimeRef.current = useTimelineStore.getState().viewport.startTime;
    velocityRef.current = 0;
    document.body.style.cursor = "grabbing";
    document.body.style.userSelect = "none";
  }, []);

  const handleMouseMove = useCallback((e: MouseEvent) => {
    if (!draggingRef.current) return;
    const container = containerRef.current;
    if (!container) return;

    const { viewport, timeline } = useTimelineStore.getState();
    const containerWidth = container.clientWidth;
    const visibleDuration = (viewport.endTime - viewport.startTime);
    const pxPerSec = containerWidth / visibleDuration;

    const deltaX = e.clientX - startXRef.current;
    const deltaTime = -deltaX / pxPerSec;
    velocityRef.current = deltaTime;

    const newStart = Math.max(0, startTimeRef.current + deltaTime);
    const maxStart = (timeline?.duration ?? 0) - visibleDuration;
    const clampedStart = Math.min(newStart, Math.max(0, maxStart));

    useTimelineStore.getState().setViewport({
      startTime: clampedStart,
      endTime: clampedStart + visibleDuration,
    });
  }, [containerRef]);

  const handleMouseUp = useCallback(() => {
    draggingRef.current = false;
    setIsDragging(false);
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  }, []);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    el.addEventListener("mousedown", handleMouseDown);
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);

    return () => {
      el.removeEventListener("mousedown", handleMouseDown);
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [containerRef, handleMouseDown, handleMouseMove, handleMouseUp]);

  return { isDragging };
}
