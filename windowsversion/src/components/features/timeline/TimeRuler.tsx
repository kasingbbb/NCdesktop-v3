import { useTimelineStore } from "../../../stores";

interface TimeRulerProps {
  width: number;
  height?: number;
}

export function TimeRuler({ width, height = 24 }: TimeRulerProps) {
  const viewport = useTimelineStore((s) => s.viewport);
  const { startTime, endTime } = viewport;
  const visibleDuration = endTime - startTime;

  const interval = getTickInterval(visibleDuration, width);
  const firstTick = Math.ceil(startTime / interval) * interval;
  const ticks: number[] = [];
  for (let t = firstTick; t <= endTime; t += interval) {
    ticks.push(t);
  }

  return (
    <div
      className="relative w-full border-t"
      style={{
        height,
        borderColor: "var(--border-primary)",
      }}
    >
      {ticks.map((t) => {
        const ratio = (t - startTime) / visibleDuration;
        const x = ratio * width;
        return (
          <div key={t} className="absolute top-0 flex flex-col items-center" style={{ left: x }}>
            <div
              className="w-px"
              style={{ height: 6, backgroundColor: "var(--text-tertiary)" }}
            />
            <span
              className="text-[10px] mt-0.5 select-none"
              style={{ color: "var(--text-tertiary)" }}
            >
              {formatTime(t)}
            </span>
          </div>
        );
      })}
    </div>
  );
}

function getTickInterval(visibleDuration: number, width: number): number {
  const targetTickSpacing = 80;
  const tickCount = Math.max(1, width / targetTickSpacing);
  const rawInterval = visibleDuration / tickCount;

  const steps = [0.1, 0.25, 0.5, 1, 2, 5, 10, 15, 30, 60, 120, 300, 600, 900, 1800, 3600];
  for (const step of steps) {
    if (step >= rawInterval) return step;
  }
  return 3600;
}

function formatTime(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);

  if (h > 0) {
    return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  }
  return `${m}:${String(s).padStart(2, "0")}`;
}
