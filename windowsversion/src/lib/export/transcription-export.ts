import type { TranscriptionSegment } from "../../types";

type ExportFormat = "txt" | "srt" | "markdown";

export function exportTranscription(
  segments: TranscriptionSegment[],
  format: ExportFormat,
  title?: string
): string {
  switch (format) {
    case "txt":
      return toPlainText(segments, title);
    case "srt":
      return toSRT(segments);
    case "markdown":
      return toMarkdown(segments, title);
  }
}

function toPlainText(segments: TranscriptionSegment[], title?: string): string {
  const lines: string[] = [];
  if (title) lines.push(title, "");

  for (const seg of segments) {
    const ts = formatTime(seg.startTime);
    const speaker = seg.speaker ? `[${seg.speaker}] ` : "";
    lines.push(`[${ts}] ${speaker}${seg.text}`);
  }

  return lines.join("\n");
}

function toSRT(segments: TranscriptionSegment[]): string {
  return segments
    .map((seg, i) => {
      const start = formatSRTTime(seg.startTime);
      const end = formatSRTTime(seg.endTime);
      return `${i + 1}\n${start} --> ${end}\n${seg.text}\n`;
    })
    .join("\n");
}

function toMarkdown(segments: TranscriptionSegment[], title?: string): string {
  const lines: string[] = [];
  if (title) lines.push(`# ${title}`, "");

  let currentSpeaker = "";
  for (const seg of segments) {
    if (seg.speaker && seg.speaker !== currentSpeaker) {
      currentSpeaker = seg.speaker;
      lines.push("", `### ${currentSpeaker}`, "");
    }

    const ts = formatTime(seg.startTime);
    lines.push(`> **${ts}** ${seg.text}`);
  }

  return lines.join("\n");
}

function formatTime(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}

function formatSRTTime(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);
  const ms = Math.round((seconds % 1) * 1000);
  return `${pad2(h)}:${pad2(m)}:${pad2(s)},${String(ms).padStart(3, "0")}`;
}

function pad2(n: number): string {
  return String(n).padStart(2, "0");
}
