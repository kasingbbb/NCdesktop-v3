import { useCallback, useRef, useState } from "react";
import { useTimelineStore } from "../stores";
import type { Keyframe } from "../types";

interface UseKeyframeDragOptions {
  containerRef: React.RefObject<HTMLElement | null>;
  onDragEnd: (keyframeId: string, newAnchorTime: number) => void;
}

interface UseKeyframeDragReturn {
  isDragging: boolean;
  dragKeyframeId: string | null;
  dragX: number;
  startDrag: (e: React.MouseEvent, keyframe: Keyframe) => void;
}

export function useKeyframeDrag({
  containerRef,
  onDragEnd,
}: UseKeyframeDragOptions): UseKeyframeDragReturn {
  const [isDragging, setIsDragging] = useState(false);
  const [dragKeyframeId, setDragKeyframeId] = useState<string | null>(null);
  const [dragX, setDragX] = useState(0);
  const startXRef = useRef(0);
  const startAnchorRef = useRef(0);

  const startDrag = useCallback(
    (e: React.MouseEvent, keyframe: Keyframe) => {
      e.stopPropagation();
      setIsDragging(true);
      setDragKeyframeId(keyframe.id);
      startXRef.current = e.clientX;
      startAnchorRef.current = keyframe.anchorTime;

      const handleMouseMove = (me: MouseEvent): void => {
        const container = containerRef.current;
        if (!container) return;

        const deltaX = me.clientX - startXRef.current;
        setDragX(deltaX);
      };

      const handleMouseUp = (me: MouseEvent): void => {
        const container = containerRef.current;
        if (!container) return;

        const { viewport } = useTimelineStore.getState();
        const containerWidth = container.clientWidth;
        const visibleDuration = viewport.endTime - viewport.startTime;
        const pxPerSec = containerWidth / visibleDuration;

        const deltaX = me.clientX - startXRef.current;
        const deltaTime = deltaX / pxPerSec;
        const newAnchor = Math.max(0, startAnchorRef.current + deltaTime);

        onDragEnd(keyframe.id, newAnchor);

        setIsDragging(false);
        setDragKeyframeId(null);
        setDragX(0);

        window.removeEventListener("mousemove", handleMouseMove);
        window.removeEventListener("mouseup", handleMouseUp);
      };

      window.addEventListener("mousemove", handleMouseMove);
      window.addEventListener("mouseup", handleMouseUp);
    },
    [containerRef, onDragEnd]
  );

  return { isDragging, dragKeyframeId, dragX, startDrag };
}
