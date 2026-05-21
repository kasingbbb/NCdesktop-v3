import { useCallback, useEffect, useRef } from "react";
import { useTimelineStore } from "../../../stores";
import type { WaveformDataResult } from "../../../lib/tauri-commands";

interface WaveformRendererProps {
  waveformData: WaveformDataResult | null;
  height?: number;
  onSeek?: (time: number) => void;
}

/** 高对比度单色方案 — 数据可视化本质 */
const PLAYED_COLOR = "#3D3D40";
const UNPLAYED_COLOR = "#D0D0D5";

export function WaveformRenderer({
  waveformData,
  height = 64,
  onSeek,
}: WaveformRendererProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const rafRef = useRef<number>(0);

  const viewport = useTimelineStore((s) => s.viewport);
  const currentTime = useTimelineStore((s) => s.playback.currentTime);

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas || !waveformData) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const rect = canvas.getBoundingClientRect();
    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    ctx.scale(dpr, dpr);

    const w = rect.width;
    const h = rect.height;
    ctx.clearRect(0, 0, w, h);

    const { startTime, endTime } = viewport;
    const visibleDuration = endTime - startTime;
    if (visibleDuration <= 0) return;

    const peaksPerSec = waveformData.peaksPerSecond;
    const startIdx = Math.floor(startTime * peaksPerSec);
    const endIdx = Math.ceil(endTime * peaksPerSec);
    const visiblePeaks = waveformData.peaks.slice(
      Math.max(0, startIdx),
      Math.min(waveformData.peaks.length, endIdx)
    );

    const pxPerPeak = w / (endIdx - startIdx);
    const centerY = h / 2;

    const playedPeakIdx = Math.floor((currentTime - startTime) * peaksPerSec);

    for (let i = 0; i < visiblePeaks.length; i++) {
      const peak = visiblePeaks[i];
      const x = i * pxPerPeak;
      const minH = peak.min * centerY;
      const maxH = peak.max * centerY;

      ctx.fillStyle = i < playedPeakIdx ? PLAYED_COLOR : UNPLAYED_COLOR;
      ctx.fillRect(x, centerY - maxH, Math.max(1, pxPerPeak - 0.5), maxH - minH);
    }
  }, [waveformData, viewport, currentTime]);

  useEffect(() => {
    rafRef.current = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(rafRef.current);
  }, [draw]);

  const handleClick = useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      const canvas = canvasRef.current;
      if (!canvas || !onSeek) return;

      const rect = canvas.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const ratio = x / rect.width;
      const { startTime, endTime } = viewport;
      const time = startTime + ratio * (endTime - startTime);
      onSeek(time);
    },
    [viewport, onSeek]
  );

  return (
    <div ref={containerRef} className="relative w-full" style={{ height }}>
      <canvas
        ref={canvasRef}
        className="w-full h-full cursor-crosshair"
        onClick={handleClick}
      />
    </div>
  );
}
