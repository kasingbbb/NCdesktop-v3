import { useCallback } from "react";
import { useTimelineStore } from "../stores";

interface UseKeyframeDropOptions {
  containerRef: React.RefObject<HTMLElement | null>;
  timelineId: string | null;
}

interface UseKeyframeDropReturn {
  handleDragOver: (e: React.DragEvent) => void;
  handleDrop: (e: React.DragEvent) => void;
}

export function useKeyframeDrop({
  containerRef,
  timelineId,
}: UseKeyframeDropOptions): UseKeyframeDropReturn {
  const handleDragOver = useCallback((e: React.DragEvent) => {
    if (e.dataTransfer.types.includes("application/x-asset-id")) {
      e.preventDefault();
      e.dataTransfer.dropEffect = "link";
    }
  }, []);

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      const assetId = e.dataTransfer.getData("application/x-asset-id");
      if (!assetId || !timelineId) return;

      const container = containerRef.current;
      if (!container) return;

      const rect = container.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const ratio = x / rect.width;

      const { viewport } = useTimelineStore.getState();
      const anchorTime = viewport.startTime + ratio * (viewport.endTime - viewport.startTime);

      useTimelineStore.getState().createKeyframe({
        timelineId,
        assetId,
        anchorTime,
        source: "manual",
      });
    },
    [containerRef, timelineId]
  );

  return { handleDragOver, handleDrop };
}
