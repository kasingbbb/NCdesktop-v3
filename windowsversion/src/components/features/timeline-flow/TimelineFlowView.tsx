import { useState, useCallback, useMemo, type ReactElement, type ReactNode } from "react";
import { FileText, Image, Music, Play, Pause, type LucideIcon } from "lucide-react";
import {
  DEMO_TIMELINE_DAYS,
  type DemoRecordingItem,
  type DemoAnchoredFile,
} from "../../../lib/demo-timeline-data";

type ZoneTone = "audio" | "image" | "doc";

function zoneStripe(tone: ZoneTone): string {
  switch (tone) {
    case "audio":
      return "var(--timeline-zone-audio-stripe)";
    case "image":
      return "var(--timeline-zone-image-stripe)";
    case "doc":
      return "var(--timeline-zone-doc-stripe)";
    default:
      return "var(--border-primary)";
  }
}

function zoneSurface(tone: ZoneTone): string {
  switch (tone) {
    case "audio":
      return "var(--timeline-zone-audio-bg)";
    case "image":
      return "var(--timeline-zone-image-bg)";
    case "doc":
      return "var(--timeline-zone-doc-bg)";
    default:
      return "var(--surface-tertiary)";
  }
}

/** 锚点相对「所属录音段」的 0–1，换算到「当日合并时间轴」上的 0–1 */
function globalAnchorPercent(file: DemoAnchoredFile, recordings: DemoRecordingItem[]): number {
  const total = recordings.reduce((s, r) => s + r.durationSec, 0) || 1;
  let offsetSec = 0;
  for (const r of recordings) {
    if (r.id === file.linkedRecordingId) {
      return (offsetSec + file.anchorX * r.durationSec) / total;
    }
    offsetSec += r.durationSec;
  }
  return 0;
}

function TagStrip({ tags, accent }: { tags: string[]; accent: string }): ReactElement {
  return (
    <div className="flex flex-wrap gap-1.5 mt-1.5">
      {tags.map((t) => (
        <span
          key={t}
          className="text-[10px] px-2 py-0.5 rounded-full font-medium border"
          style={{
            color: "var(--text-secondary)",
            background: "var(--surface-primary)",
            borderColor: accent,
          }}
        >
          {t}
        </span>
      ))}
    </div>
  );
}

/** 与下方录音行同宽：按时长比例划段，显示每段起点与时长 */
function TimeAxisRuler({ recordings }: { recordings: DemoRecordingItem[] }): ReactElement {
  const stripe = zoneStripe("audio");

  return (
    <div
      className="flex w-full min-w-0 border-b"
      style={{ borderColor: "var(--border-primary)", background: "var(--timeline-zone-audio-bg)" }}
    >
      {recordings.map((r) => (
        <div
          key={r.id}
          className="flex flex-col justify-center gap-0.5 px-2 py-2 min-w-0 border-r last:border-r-0"
          style={{
            flex: `${r.durationSec} 1 0`,
            borderColor: "color-mix(in srgb, var(--border-primary) 80%, transparent)",
          }}
        >
          <span className="text-[9px] font-mono font-semibold tabular-nums truncate" style={{ color: stripe }}>
            {r.clockStart}
          </span>
          <span className="text-[9px] tabular-nums truncate" style={{ color: "var(--text-tertiary)" }}>
            {r.durationLabel}
          </span>
        </div>
      ))}
    </div>
  );
}

/** 合并轴上的刻度线（细横条 + 分段底色微差） */
function TimeAxisSpine({ recordings }: { recordings: DemoRecordingItem[] }): ReactElement {
  return (
    <div className="flex h-2 w-full min-w-0 rounded-sm overflow-hidden border" style={{ borderColor: "var(--border-primary)" }}>
      {recordings.map((r, i) => (
        <div
          key={r.id}
          className="h-full min-w-0 border-r last:border-r-0"
          style={{
            flex: `${r.durationSec} 1 0`,
            borderColor: "var(--border-primary)",
            background:
              i % 2 === 0
                ? "color-mix(in srgb, var(--brand-navy) 12%, var(--surface-primary))"
                : "color-mix(in srgb, var(--brand-navy) 6%, var(--surface-primary))",
          }}
        />
      ))}
    </div>
  );
}

function RecordingBlock({
  item,
  playing,
  onTogglePlay,
}: {
  item: DemoRecordingItem;
  playing: boolean;
  onTogglePlay: (id: string) => void;
}): ReactElement {
  const stripe = zoneStripe("audio");

  return (
    <div
      className="h-full min-w-0 rounded-none border-0 border-r last:border-r-0 flex flex-col overflow-hidden bg-[var(--surface-primary)]"
      style={{
        borderColor: "var(--border-primary)",
        borderRightWidth: 1,
        borderRightStyle: "solid",
        boxShadow: "none",
      }}
    >
      <div
        className="flex items-center gap-1.5 px-2 py-2 border-b min-h-[3rem]"
        style={{
          borderColor: "var(--border-primary)",
          background: "var(--timeline-zone-audio-bg)",
        }}
      >
        <Music size={14} className="shrink-0" style={{ color: stripe }} />
        <span
          className="text-[11px] font-semibold truncate flex-1 min-w-0"
          style={{ color: "var(--text-primary)" }}
          title={item.fileName}
        >
          {item.fileName}
        </span>
        <button
          type="button"
          className="shrink-0 rounded-full p-1.5 border transition-colors"
          style={{
            color: "#fff",
            background: "var(--brand-navy)",
            borderColor: "var(--brand-navy-dark)",
          }}
          aria-label={playing ? "暂停" : "播放"}
          onClick={() => {
            onTogglePlay(item.id);
          }}
        >
          {playing ? <Pause size={12} /> : <Play size={12} className="ml-0.5" />}
        </button>
      </div>
      <div className="px-2 py-2 text-[10px] space-y-1 flex-1" style={{ color: "var(--text-secondary)" }}>
        <p>
          开始{" "}
          <strong style={{ color: "var(--text-primary)" }}>{item.clockStart}</strong>
          {" · "}
          时长{" "}
          <strong style={{ color: "var(--text-primary)" }}>{item.durationLabel}</strong>
        </p>
        <TagStrip tags={item.tags} accent={stripe} />
      </div>
    </div>
  );
}

/** 第一行：与录音同宽的合并时间轴 — 多段录音按时长比例横向铺开 */
function RecordingTimelineRow({
  recordings,
  playingId,
  onTogglePlay,
}: {
  recordings: DemoRecordingItem[];
  playingId: string | null;
  onTogglePlay: (id: string) => void;
}): ReactElement {
  return (
    <div
      className="flex w-full min-w-0 rounded-[var(--radius-lg)] border overflow-hidden"
      style={{ borderColor: "var(--border-primary)", boxShadow: "var(--shadow-sm)" }}
    >
      {recordings.map((r) => (
        <div
          key={r.id}
          className="min-w-0 flex flex-col"
          style={{ flex: `${r.durationSec} 1 0` }}
        >
          <RecordingBlock item={r} playing={playingId === r.id} onTogglePlay={onTogglePlay} />
        </div>
      ))}
    </div>
  );
}

function AnchoredTile({
  file,
  Icon,
  tone,
  leftPercent,
}: {
  file: DemoAnchoredFile;
  Icon: LucideIcon;
  tone: "image" | "doc";
  /** 在「当日合并时间轴」上的 0–100% 位置 */
  leftPercent: number;
}): ReactElement {
  const stripe = zoneStripe(tone);

  return (
    <div
      className="absolute top-0 z-10 -translate-x-1/2"
      style={{
        left: `${leftPercent * 100}%`,
        width: "clamp(7rem, 22vw, 14rem)",
        maxWidth: "min(14rem, 92%)",
      }}
    >
      <div
        className="rounded-[var(--radius-md)] p-2.5 border-2 bg-[var(--surface-primary)]"
        style={{
          borderColor: stripe,
          boxShadow: "var(--shadow-md)",
        }}
      >
        <div className="flex items-start gap-2 min-w-0">
          <Icon size={15} className="shrink-0 mt-0.5" style={{ color: stripe }} />
          <div className="min-w-0 flex-1">
            <p
              className="text-[11px] font-semibold leading-snug line-clamp-3"
              style={{ color: "var(--text-primary)" }}
            >
              {file.fileName}
            </p>
            <p
              className="text-[10px] mt-1 px-2 py-0.5 rounded-full inline-block font-mono tabular-nums border"
              style={{
                color: "var(--text-secondary)",
                background: zoneSurface(tone),
                borderColor: stripe,
              }}
            >
              {file.timeTag}
            </p>
            <TagStrip tags={file.tags} accent={stripe} />
          </div>
        </div>
      </div>
    </div>
  );
}

function ModalityLane({
  title,
  tone,
  children,
}: {
  title: string;
  tone: ZoneTone;
  children: ReactNode;
}): ReactElement {
  const stripe = zoneStripe(tone);
  const surface = zoneSurface(tone);

  return (
    <div
      className="rounded-[var(--radius-lg)] border overflow-hidden min-w-0 w-full flex flex-col"
      style={{
        borderColor: "var(--border-primary)",
        background: surface,
        boxShadow: "var(--shadow-sm)",
      }}
    >
      <div
        className="flex items-center gap-2 px-3 py-2 border-b shrink-0"
        style={{
          borderColor: "var(--border-primary)",
          borderLeftWidth: 4,
          borderLeftStyle: "solid",
          borderLeftColor: stripe,
          background: "color-mix(in srgb, var(--surface-primary) 82%, transparent)",
        }}
      >
        <span className="text-[10px] font-bold uppercase tracking-[0.1em]" style={{ color: "var(--text-primary)" }}>
          {title}
        </span>
        <span className="text-[9px]" style={{ color: "var(--text-tertiary)" }}>
          与上方录音时间轴同一横坐标对齐
        </span>
      </div>
      {/* 不加左右内边距，保证 left% 与录音行同一宽度坐标系 */}
      <div className="min-w-0 min-h-[8.5rem] relative py-1">{children}</div>
    </div>
  );
}

/** 单日：一行完整横向时间轴 + 下方图片轨、文档轨按时间点落位 */
function DayTimelineBlock({
  day,
  dayIndex,
  playingId,
  onTogglePlay,
  maxRecordingMin: globalMaxMin,
}: {
  day: (typeof DEMO_TIMELINE_DAYS)[0];
  dayIndex: number;
  playingId: string | null;
  onTogglePlay: (id: string) => void;
  maxRecordingMin: number;
}): ReactElement {
  const recordings = day.recordings;
  const totalSec = recordings.reduce((s, r) => s + r.durationSec, 0) || 1;

  return (
    <section
      className="w-full flex flex-col border-b last:border-b-0 min-w-0"
      style={{
        borderColor: "var(--border-primary)",
        background: "var(--surface-primary)",
      }}
    >
      <header
        className="flex flex-wrap items-center justify-between gap-2 px-3 py-2.5 shrink-0 border-b"
        style={{
          borderColor: "var(--border-primary)",
          background: "var(--surface-secondary)",
        }}
      >
        <div className="flex items-center gap-2 min-w-0">
          <span
            className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full text-[10px] font-bold"
            style={{
              background: "var(--timeline-rail-line)",
              color: "#fff",
            }}
            aria-hidden
          >
            {dayIndex + 1}
          </span>
          <h3 className="text-[var(--text-sm)] font-bold tracking-tight truncate" style={{ color: "var(--text-primary)" }}>
            {day.dateLabel}
          </h3>
        </div>
        <span
          className="text-[10px] font-mono tabular-nums px-2 py-0.5 rounded-full border shrink-0"
          style={{
            color: "var(--text-secondary)",
            borderColor: "var(--border-primary)",
            background: "var(--surface-tertiary)",
          }}
        >
          {day.dateIso}
        </span>
      </header>

      <div className="p-3 space-y-3 min-w-0">
        <div className="space-y-2 min-w-0">
          <p className="text-[10px] font-bold uppercase tracking-[0.12em]" style={{ color: "var(--text-secondary)" }}>
            录音时间轴
          </p>
          <p className="text-[9px] leading-snug" style={{ color: "var(--text-tertiary)" }}>
            多段录音按时长比例排在同一行；总时长约 {Math.round((totalSec / 60) * 10) / 10} min。
          </p>
          <TimeAxisSpine recordings={recordings} />
          <TimeAxisRuler recordings={recordings} />
          <RecordingTimelineRow recordings={recordings} playingId={playingId} onTogglePlay={onTogglePlay} />
        </div>

        <ModalityLane title="图片" tone="image">
          <div
            className="absolute inset-x-0 inset-y-1 rounded-[var(--radius-md)] border border-dashed pointer-events-none"
            style={{
              borderColor: "color-mix(in srgb, var(--border-hover) 65%, transparent)",
              background: `linear-gradient(180deg, ${zoneSurface("image")} 0%, transparent 55%)`,
            }}
          />
          <div
            className="pointer-events-none absolute inset-x-0 top-1/2 h-px -translate-y-1/2 opacity-35"
            style={{ background: zoneStripe("image") }}
          />
          {day.images.map((f) => (
            <AnchoredTile
              key={f.id}
              file={f}
              Icon={Image}
              tone="image"
              leftPercent={globalAnchorPercent(f, recordings)}
            />
          ))}
        </ModalityLane>

        <ModalityLane title="文档" tone="doc">
          <div
            className="absolute inset-x-0 inset-y-1 rounded-[var(--radius-md)] border border-dashed pointer-events-none"
            style={{
              borderColor: "color-mix(in srgb, var(--border-hover) 65%, transparent)",
              background: `linear-gradient(180deg, ${zoneSurface("doc")} 0%, transparent 55%)`,
            }}
          />
          <div
            className="pointer-events-none absolute inset-x-0 top-1/2 h-px -translate-y-1/2 opacity-35"
            style={{ background: zoneStripe("doc") }}
          />
          {day.documents.map((f) => (
            <AnchoredTile
              key={f.id}
              file={f}
              Icon={FileText}
              tone="doc"
              leftPercent={globalAnchorPercent(f, recordings)}
            />
          ))}
        </ModalityLane>
      </div>

      <footer className="text-[10px] px-3 pb-3 pt-0 shrink-0" style={{ color: "var(--text-tertiary)" }}>
        比例尺示意 · 演示全局最长单段录音约 {globalMaxMin} min
      </footer>
    </section>
  );
}

export function TimelineFlowView(): ReactElement {
  const [playingId, setPlayingId] = useState<string | null>(null);

  const togglePlay = useCallback((id: string) => {
    setPlayingId((prev) => (prev === id ? null : id));
  }, []);

  const maxRecordingSec = useMemo(() => {
    let max = 0;
    for (const d of DEMO_TIMELINE_DAYS) {
      for (const r of d.recordings) {
        max = Math.max(max, r.durationSec);
      }
    }
    return max || 1;
  }, []);

  const globalMaxMin = Math.round((maxRecordingSec / 60) * 10) / 10;

  return (
    <div
      className="flex flex-col h-full min-h-0 flex-1 rounded-[var(--radius-xl)] border overflow-hidden min-w-0 bg-[var(--surface-primary)]"
      style={{ borderColor: "var(--border-primary)", boxShadow: "var(--shadow-float)" }}
    >
      <p
        className="text-[var(--text-xs)] leading-relaxed px-3 py-2.5 shrink-0 border-b"
        style={{
          borderColor: "var(--border-primary)",
          color: "var(--text-secondary)",
          background: "color-mix(in srgb, var(--brand-navy) 6%, var(--surface-primary))",
        }}
      >
        <span className="font-semibold" style={{ color: "var(--brand-navy)" }}>
          时间流
        </span>
        <span className="mx-2 opacity-40">|</span>
        每日一行：首行为合并横向时间轴（多段录音按时长比例铺开）；其下图片、文档轨与同一时间坐标对齐落位。
      </p>

      <div
        className="flex flex-1 min-h-0 min-w-0 overflow-y-auto overflow-x-hidden flex-row items-stretch"
        style={{ background: "var(--timeline-scroll-bg)" }}
      >
        <div
          className="shrink-0 w-11 flex flex-col items-center py-3 px-1 border-r"
          style={{
            borderColor: "var(--border-primary)",
            background: "var(--timeline-rail-bg)",
          }}
        >
          <div
            className="w-1 flex-1 rounded-full min-h-[120px]"
            style={{ background: "var(--timeline-rail-line)" }}
          />
        </div>

        <div className="flex-1 min-w-0 flex flex-col">
          {DEMO_TIMELINE_DAYS.map((day, dayIndex) => (
            <DayTimelineBlock
              key={day.id}
              day={day}
              dayIndex={dayIndex}
              playingId={playingId}
              onTogglePlay={togglePlay}
              maxRecordingMin={globalMaxMin}
            />
          ))}
        </div>
      </div>
    </div>
  );
}
