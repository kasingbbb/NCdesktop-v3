import { useEffect, useRef, useState } from "react";
import { useTimelineStore } from "../../../stores";
import { WaveformRenderer } from "./WaveformRenderer";
import { Playhead } from "./Playhead";
import { TimeRuler } from "./TimeRuler";
import { PlaybackControls } from "./PlaybackControls";
import { SelectionOverlay } from "./SelectionOverlay";
import { KeyframeTrack } from "./KeyframeTrack";
import { useTimelineZoom } from "../../../hooks/useTimelineZoom";
import { useTimelineDrag } from "../../../hooks/useTimelineDrag";
import { useTimelineShortcuts } from "../../../hooks/useTimelineShortcuts";
import { useMagicMoment } from "../../../hooks/useMagicMoment";
import type { WaveformDataResult } from "../../../lib/tauri-commands";
import type { Keyframe } from "../../../types";
import { logger } from "../../../utils/logger";

interface TimelineViewProps {
  waveformData: WaveformDataResult | null;
  audioUrl: string | null;
  onSeek?: (time: number) => void;
  onKeyframeClick?: (keyframe: Keyframe) => void;
  onPreviewAssetChange?: (assetId: string | null) => void;
}

const KEYFRAME_TRACK_HEIGHT = 72;
const WAVEFORM_HEIGHT = 64;

export function TimelineView({
  waveformData,
  onSeek,
  onKeyframeClick,
  onPreviewAssetChange,
}: TimelineViewProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [containerWidth, setContainerWidth] = useState(0);

  const timeline = useTimelineStore((s) => s.timeline);
  const seek = useTimelineStore((s) => s.seek);

  const { seekToKeyframe, previewAssetId } = useMagicMoment();

  useTimelineZoom(containerRef);
  useTimelineDrag(containerRef);
  useTimelineShortcuts();

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setContainerWidth(entry.contentRect.width);
      }
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    onPreviewAssetChange?.(previewAssetId);
  }, [previewAssetId, onPreviewAssetChange]);

  const handleSeek = (time: number): void => {
    seek(time);
    onSeek?.(time);
  };

  const handleKeyframeClick = (kf: Keyframe): void => {
    logger.info("TimelineView", "Seeking to keyframe", { id: kf.id });
    seekToKeyframe(kf);
    onKeyframeClick?.(kf);
  };

  if (!timeline) {
    return (
      <div className="flex items-center justify-center h-full" style={{ color: "var(--text-tertiary)" }}>
        <span className="text-[var(--text-sm)]">暂无时间轴数据</span>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* 关键帧轨道（波形上方） */}
      <KeyframeTrack
        containerWidth={containerWidth}
        waveformTop={KEYFRAME_TRACK_HEIGHT}
        onKeyframeClick={handleKeyframeClick}
      />

      {/* 波形区域 */}
      <div ref={containerRef} className="relative flex-shrink-0 overflow-hidden cursor-grab">
        <WaveformRenderer
          waveformData={waveformData}
          height={WAVEFORM_HEIGHT}
          onSeek={handleSeek}
        />
        <Playhead containerWidth={containerWidth} />
        <SelectionOverlay containerWidth={containerWidth} height={WAVEFORM_HEIGHT} />
      </div>

      {/* 时间刻度尺 */}
      <TimeRuler width={containerWidth} />

      {/* 播放控制 */}
      <PlaybackControls />
    </div>
  );
}
