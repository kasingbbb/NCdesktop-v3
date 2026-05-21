import { useMemo } from "react";
import { useTimelineStore } from "../stores";
import type { TranscriptionSegment } from "../types";

interface TranscriptionSyncResult {
  activeSegmentIndex: number;
  isSegmentActive: (segment: TranscriptionSegment) => boolean;
}

/**
 * 根据播放时间实时计算当前活跃的转录段落
 */
export function useTranscriptionSync(
  segments: TranscriptionSegment[]
): TranscriptionSyncResult {
  const currentTime = useTimelineStore((s) => s.playback.currentTime);

  const activeSegmentIndex = useMemo(() => {
    for (let i = segments.length - 1; i >= 0; i--) {
      if (currentTime >= segments[i].startTime && currentTime < segments[i].endTime) {
        return i;
      }
    }
    for (let i = segments.length - 1; i >= 0; i--) {
      if (currentTime >= segments[i].startTime) {
        return i;
      }
    }
    return -1;
  }, [segments, currentTime]);

  const isSegmentActive = (segment: TranscriptionSegment): boolean => {
    return currentTime >= segment.startTime && currentTime < segment.endTime;
  };

  return { activeSegmentIndex, isSegmentActive };
}
