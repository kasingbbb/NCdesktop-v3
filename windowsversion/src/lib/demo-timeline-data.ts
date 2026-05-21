/**
 * 时间流右栏演示数据：录音、图片、文档均带 Time Tag，并与录音时间轴对齐。
 * 后续可替换为项目内真实素材 + 锚点元数据。
 */

export interface DemoRecordingItem {
  id: string;
  /** 展示用文件名 */
  fileName: string;
  /** 录音开始（当日时钟） */
  clockStart: string;
  /** 时长标签，如 45:12 */
  durationLabel: string;
  durationSec: number;
  /** 横向占位权重（用于多段录音并排时的相对宽度） */
  widthWeight: number;
  /** 标签（含 PRD 中的时长、时间语义） */
  tags: string[];
}

export interface DemoAnchoredFile {
  id: string;
  fileName: string;
  kind: "image" | "document";
  /** 相对「所关联录音段」内部的 0–1 位置；UI 上按当日多段合并时间轴换算为全局横坐标 */
  anchorX: number;
  /** Time Tag：相对录音起点的时间戳文案 */
  timeTag: string;
  /** 关联的录音 id（锚定到哪一段） */
  linkedRecordingId: string;
  tags: string[];
}

export interface DemoTimelineDay {
  id: string;
  /** ISO 日期，用于左侧时间轴 */
  dateIso: string;
  /** 展示用，如 3月25日 周三 */
  dateLabel: string;
  recordings: DemoRecordingItem[];
  images: DemoAnchoredFile[];
  documents: DemoAnchoredFile[];
}

export const DEMO_TIMELINE_DAYS: DemoTimelineDay[] = [
  {
    id: "day-2026-03-25",
    dateIso: "2026-03-25",
    dateLabel: "3月25日 · 周三",
    recordings: [
      {
        id: "rec-a",
        fileName: "课堂讨论-上午.m4a",
        clockStart: "09:28",
        durationLabel: "42:18",
        durationSec: 2538,
        widthWeight: 1.4,
        tags: ["录音", "Time Tag 09:28", "时长 42:18"],
      },
      {
        id: "rec-b",
        fileName: "小组备忘-午后.m4a",
        clockStart: "14:05",
        durationLabel: "18:06",
        durationSec: 1086,
        widthWeight: 1,
        tags: ["录音", "Time Tag 14:05", "时长 18:06"],
      },
    ],
    images: [
      {
        id: "img-a1",
        fileName: "白板推导.jpg",
        kind: "image",
        anchorX: 0.12,
        timeTag: "相对起点 +00:08:20",
        linkedRecordingId: "rec-a",
        tags: ["图片", "Time Tag +00:08:20", "关联 rec-a"],
      },
      {
        id: "img-a2",
        fileName: "参考截图.png",
        kind: "image",
        anchorX: 0.55,
        timeTag: "相对起点 +00:35:02",
        linkedRecordingId: "rec-a",
        tags: ["图片", "Time Tag +00:35:02", "关联 rec-a"],
      },
      {
        id: "img-b1",
        fileName: "走廊抓拍.heic",
        kind: "image",
        anchorX: 0.72,
        timeTag: "相对起点 +00:03:10",
        linkedRecordingId: "rec-b",
        tags: ["图片", "Time Tag +00:03:10", "关联 rec-b"],
      },
    ],
    documents: [
      {
        id: "doc-a1",
        fileName: "会议摘要.md",
        kind: "document",
        anchorX: 0.38,
        timeTag: "相对起点 +00:22:44",
        linkedRecordingId: "rec-a",
        tags: ["文档", "Time Tag +00:22:44", "Markdown"],
      },
      {
        id: "doc-b1",
        fileName: "临时笔记.txt",
        kind: "document",
        anchorX: 0.88,
        timeTag: "相对起点 +00:12:00",
        linkedRecordingId: "rec-b",
        tags: ["文档", "Time Tag +00:12:00", "纯文本"],
      },
    ],
  },
  {
    id: "day-2026-03-26",
    dateIso: "2026-03-26",
    dateLabel: "3月26日 · 周四",
    recordings: [
      {
        id: "rec-c",
        fileName: "通勤灵感.m4a",
        clockStart: "08:12",
        durationLabel: "06:40",
        durationSec: 400,
        widthWeight: 0.85,
        tags: ["录音", "Time Tag 08:12", "时长 06:40"],
      },
    ],
    images: [
      {
        id: "img-c1",
        fileName: "街景.jpg",
        kind: "image",
        anchorX: 0.25,
        timeTag: "相对起点 +00:01:05",
        linkedRecordingId: "rec-c",
        tags: ["图片", "Time Tag +00:01:05"],
      },
    ],
    documents: [
      {
        id: "doc-c1",
        fileName: "语音转写占位.pdf",
        kind: "document",
        anchorX: 0.5,
        timeTag: "相对起点 +00:04:30",
        linkedRecordingId: "rec-c",
        tags: ["文档", "Time Tag +00:04:30", "PDF"],
      },
    ],
  },
];
