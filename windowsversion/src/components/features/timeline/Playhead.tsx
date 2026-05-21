import { useMemo } from "react";
import { useTimelineStore } from "../../../stores";
import { useUIStore } from "../../../stores/uiStore";

interface PlayheadProps {
  containerWidth: number;
}

export function Playhead({ containerWidth }: PlayheadProps) {
  const currentTime = useTimelineStore((s) => s.playback.currentTime);
  const viewport = useTimelineStore((s) => s.viewport);
  const keyframes = useTimelineStore((s) => s.keyframes);
  const isAnimating = useUIStore((s) => s.magicMoment.isAnimating);
  const highlightedKeyframeId = useUIStore((s) => s.magicMoment.highlightedKeyframeId);

  const nearKeyframe = useMemo(() => {
    if (!highlightedKeyframeId) return false;
    const kf = keyframes.find((k) => k.id === highlightedKeyframeId);
    if (!kf) return false;
    return Math.abs(currentTime - kf.anchorTime) < 0.5;
  }, [currentTime, keyframes, highlightedKeyframeId]);

  const { startTime, endTime } = viewport;
  const visibleDuration = endTime - startTime;
  if (visibleDuration <= 0) return null;

  const ratio = (currentTime - startTime) / visibleDuration;
  if (ratio < 0 || ratio > 1) return null;

  const x = ratio * containerWidth;

  return (
    <div
      className="absolute top-0 bottom-0 pointer-events-none"
      style={{
        left: `${x}px`,
        zIndex: 20,
        transition: isAnimating ? "left var(--duration-normal) var(--ease-out-expo)" : "none",
      }}
    >
      {/* Magic Moment 近关键帧时略放大 */}
      <div
        className="absolute -translate-x-1/2 rounded-full"
        style={{
          width: nearKeyframe ? 12 : 8,
          height: nearKeyframe ? 12 : 8,
          top: nearKeyframe ? -3 : -1,
          left: 1,
          backgroundColor: "#1d1d1f",
          transition: "all var(--duration-fast) var(--ease-out-expo)",
          boxShadow: nearKeyframe ? "0 0 0 2px rgba(0,0,0,0.12), var(--shadow-sm)" : "none",
          animation: nearKeyframe ? "magic-pulse 1.5s ease-in-out infinite" : "none",
        }}
      />
      {/* 播放头线 */}
      <div
        className="absolute top-0 bottom-0"
        style={{
          width: 2,
          backgroundColor: "#1d1d1f",
        }}
      />
    </div>
  );
}
