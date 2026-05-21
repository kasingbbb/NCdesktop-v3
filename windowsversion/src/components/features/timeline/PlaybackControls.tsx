import {
  SkipBack,
  Rewind,
  Play,
  Pause,
  FastForward,
  SkipForward,
  Volume2,
  VolumeX,
} from "lucide-react";
import { useTimelineStore } from "../../../stores";

const SPEED_OPTIONS = [0.5, 0.75, 1, 1.25, 1.5, 2];

export function PlaybackControls() {
  const isPlaying = useTimelineStore((s) => s.playback.isPlaying);
  const currentTime = useTimelineStore((s) => s.playback.currentTime);
  const duration = useTimelineStore((s) => s.playback.duration);
  const speed = useTimelineStore((s) => s.playback.playbackSpeed);
  const volume = useTimelineStore((s) => s.playback.volume);
  const isMuted = useTimelineStore((s) => s.playback.isMuted);

  const play = useTimelineStore((s) => s.play);
  const pause = useTimelineStore((s) => s.pause);
  const seek = useTimelineStore((s) => s.seek);
  const setPlaybackSpeed = useTimelineStore((s) => s.setPlaybackSpeed);
  const setVolume = useTimelineStore((s) => s.setVolume);
  const toggleMute = useTimelineStore((s) => s.toggleMute);

  const handleSkipBack = (): void => seek(0);
  const handleRewind = (): void => seek(Math.max(0, currentTime - 10));
  const handleForward = (): void => seek(Math.min(duration, currentTime + 10));
  const handleSkipForward = (): void => seek(duration);

  const nextSpeed = (): void => {
    const idx = SPEED_OPTIONS.indexOf(speed);
    const next = SPEED_OPTIONS[(idx + 1) % SPEED_OPTIONS.length];
    setPlaybackSpeed(next);
  };

  return (
    <div
      className="flex items-center gap-[var(--space-2)] px-[var(--space-4)] py-[var(--space-2)]"
      style={{ borderTop: "1px solid var(--border-primary)" }}
    >
      {/* 时间显示 */}
      <span className="text-[var(--text-xs)] tabular-nums min-w-[80px]" style={{ color: "var(--text-secondary)" }}>
        {formatTime(currentTime)} / {formatTime(duration)}
      </span>

      {/* 播放控制 */}
      <div className="flex items-center gap-[var(--space-1)]">
        <ControlButton onClick={handleSkipBack} title="回到开头">
          <SkipBack size={14} />
        </ControlButton>
        <ControlButton onClick={handleRewind} title="后退 10 秒">
          <Rewind size={14} />
        </ControlButton>

        <button
          onClick={() => (isPlaying ? pause() : play())}
          className="btn-glass flex items-center justify-center text-gray-900"
          style={{ width: 32, height: 32, borderRadius: "var(--radius-full)" }}
          title={isPlaying ? "暂停" : "播放"}
        >
          {isPlaying ? <Pause size={16} /> : <Play size={16} style={{ marginLeft: 2 }} />}
        </button>

        <ControlButton onClick={handleForward} title="前进 10 秒">
          <FastForward size={14} />
        </ControlButton>
        <ControlButton onClick={handleSkipForward} title="到结尾">
          <SkipForward size={14} />
        </ControlButton>
      </div>

      {/* 速度 */}
      <button
        onClick={nextSpeed}
        className="btn-glass text-[var(--text-xs)] px-[var(--space-2)] py-[var(--space-1)]"
        title="切换播放速度"
      >
        {speed}x
      </button>

      {/* 音量 */}
      <div className="flex items-center gap-[var(--space-1)] ml-auto">
        <ControlButton onClick={toggleMute} title={isMuted ? "取消静音" : "静音"}>
          {isMuted ? <VolumeX size={14} /> : <Volume2 size={14} />}
        </ControlButton>
        <input
          type="range"
          min={0}
          max={1}
          step={0.05}
          value={isMuted ? 0 : volume}
          onChange={(e) => setVolume(Number(e.target.value))}
          className="w-16 h-1 accent-gray-700"
        />
      </div>
    </div>
  );
}

function ControlButton({
  onClick,
  title,
  children,
}: {
  onClick: () => void;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      title={title}
      className="btn-glass flex items-center justify-center"
      style={{ width: 28, height: 28, borderRadius: "var(--radius-md)" }}
    >
      {children}
    </button>
  );
}

function formatTime(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);
  if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  return `${m}:${String(s).padStart(2, "0")}`;
}
