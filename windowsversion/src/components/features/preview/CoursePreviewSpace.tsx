/**
 * CoursePreviewSpace — AI 课程预习空间
 *
 * 布局（宪章 B5：ContentArea 的新视图模式，非模态）：
 *   顶部  课程信息栏（← Back / 课程名 / 时间教授教室）
 *   中段  AI 预习指南（Markdown 渲染 / 骨架屏 / 重新生成）
 *   下段  用户预习笔记区（可编辑，debounce 1s 自动保存）
 *   底部  「重新生成」「保存为素材」按钮
 *
 * 约束（宪章 A1/A2）：named export，CSS 变量，无硬编码颜色
 */

import { useEffect, useRef, useState, useCallback } from "react";
import {
  ArrowLeft,
  RefreshCw,
  BookmarkPlus,
  Loader2,
  Clock,
  MapPin,
  User,
  BookOpen,
} from "lucide-react";
import { useCalendarStore } from "../../../stores/calendarStore";
import { useUIStore } from "../../../stores/uiStore";
import * as cmd from "../../../lib/tauri-commands";
import type { CourseEvent } from "../../../types/calendar";
import type { CoursePreview } from "../../../types/calendar";

// ─── 简易 Markdown 渲染（无外部依赖）────────────────────────────────────────

function MarkdownBlock({ content }: { content: string }) {
  // 把 Markdown 转为基本 HTML
  const html = markdownToHtml(content);
  return (
    <div
      className="prose-preview text-[var(--text-sm)] leading-relaxed"
      style={{ color: "var(--text-primary)" }}
      // eslint-disable-next-line react/no-danger
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}

function markdownToHtml(md: string): string {
  return md
    // 标题
    .replace(/^### (.+)$/gm, '<h3 class="md-h3">$1</h3>')
    .replace(/^## (.+)$/gm, '<h2 class="md-h2">$1</h2>')
    .replace(/^# (.+)$/gm, '<h1 class="md-h1">$1</h1>')
    // 粗体 / 斜体
    .replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
    .replace(/\*(.+?)\*/g, "<em>$1</em>")
    // 行内代码
    .replace(/`(.+?)`/g, '<code class="md-code">$1</code>')
    // 无序列表
    .replace(/^[-*] (.+)$/gm, '<li class="md-li">$1</li>')
    // 有序列表
    .replace(/^\d+\. (.+)$/gm, '<li class="md-li md-li-num">$1</li>')
    // 段落 / 换行
    .replace(/\n{2,}/g, '</p><p class="md-p">')
    .replace(/\n/g, "<br/>")
    // 包裹开头
    .replace(/^/, '<p class="md-p">')
    .replace(/$/, "</p>");
}

// ─── 骨架屏 ──────────────────────────────────────────────────────────────────

function PreviewSkeleton() {
  return (
    <div className="space-y-[var(--space-3)] animate-pulse">
      {[80, 60, 90, 50, 70].map((w, i) => (
        <div
          key={i}
          className="h-3 rounded-full"
          style={{
            width: `${w}%`,
            background: "var(--surface-tertiary)",
          }}
        />
      ))}
      <div className="h-px" style={{ background: "var(--border-primary)" }} />
      {[65, 85, 55].map((w, i) => (
        <div
          key={i}
          className="h-3 rounded-full"
          style={{
            width: `${w}%`,
            background: "var(--surface-tertiary)",
          }}
        />
      ))}
    </div>
  );
}

// ─── 主组件 ──────────────────────────────────────────────────────────────────

interface Props {
  courseEventId: string;
}

export function CoursePreviewSpace({ courseEventId }: Props) {
  const { events } = useCalendarStore();
  const calendarStore = useCalendarStore();
  const {
    setRightPanelMode,
    setActiveCourseEventId,
    coursePreviewReturnTo,
    setCoursePreviewReturnTo,
    setSidebarSection,
  } = useUIStore();

  // 从 store 中找到当前课程事件
  const event: CourseEvent | undefined = events.find((e) => e.id === courseEventId);

  const [preview, setPreview] = useState<CoursePreview | null>(null);
  const [isGenerating, setIsGenerating] = useState(false);
  const [genError, setGenError] = useState<string | null>(null);
  const [notes, setNotes] = useState("");
  const [isSavingNotes, setIsSavingNotes] = useState(false);

  // debounce 保存笔记
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // ── 初始化：检查缓存或生成 ────────────────────────────────────────────────

  useEffect(() => {
    if (!courseEventId) return;
    let cancelled = false;

    const init = async () => {
      setGenError(null);
      // 先查缓存
      try {
        const cached = await cmd.getCoursePreview(courseEventId);
        if (cancelled) return;
        if (cached) {
          setPreview(cached);
          setNotes(cached.userNotes ?? "");
          return;
        }
      } catch {
        // 查缓存失败不阻塞，直接走生成
      }
      // 没有缓存 → 自动生成
      await generate(false, cancelled);
    };

    void init();
    return () => {
      cancelled = true;
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [courseEventId]);

  // ── 生成预习内容 ─────────────────────────────────────────────────────────

  const generate = useCallback(
    async (force: boolean, _cancelled = false) => {
      setIsGenerating(true);
      setGenError(null);
      try {
        const result = await cmd.generateCoursePreview(courseEventId, force);
        if (!_cancelled) {
          setPreview(result);
          setNotes(result.userNotes ?? "");
        }
      } catch (e) {
        if (!_cancelled) setGenError(String(e));
      } finally {
        if (!_cancelled) setIsGenerating(false);
      }
    },
    [courseEventId]
  );

  // ── 笔记 debounce 保存 ────────────────────────────────────────────────────

  const handleNotesChange = (value: string) => {
    setNotes(value);
    if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    saveTimerRef.current = setTimeout(async () => {
      setIsSavingNotes(true);
      try {
        await cmd.savePreviewNotes(courseEventId, value);
      } finally {
        setIsSavingNotes(false);
      }
    }, 1000);
  };

  // ── Back ──────────────────────────────────────────────────────────────────

  const handleBack = () => {
    if (coursePreviewReturnTo) {
      setSidebarSection(coursePreviewReturnTo.section);
      if (coursePreviewReturnTo.weekStart) {
        calendarStore.setActiveWeekStart(coursePreviewReturnTo.weekStart);
      }
      setCoursePreviewReturnTo(null);
    }
    setActiveCourseEventId(null);
    setRightPanelMode("inspector");
  };

  const backLabel = coursePreviewReturnTo?.section === "calendar" ? "← 课程日历" : "← 返回";

  // ── 渲染 ─────────────────────────────────────────────────────────────────

  return (
    <div className="flex flex-col h-full overflow-hidden bg-[var(--surface-primary)]">

      {/* ── 顶部：课程信息栏 ── */}
      <div
        className="flex-shrink-0 border-b px-[var(--space-4)] py-[var(--space-3)]"
        style={{ borderColor: "var(--border-primary)" }}
      >
        <div className="flex items-center gap-[var(--space-3)] mb-[var(--space-2)]">
          <button
            type="button"
            onClick={handleBack}
            className="flex items-center gap-[var(--space-1)] text-[var(--text-sm)] transition-colors"
            style={{ color: "var(--text-secondary)" }}
          >
            <ArrowLeft size={14} />
            {backLabel}
          </button>
          <h2
            className="text-[var(--text-base)] font-semibold truncate"
            style={{ color: "var(--text-primary)" }}
          >
            {event?.title ?? "课程预习"}
          </h2>
          {event?.courseCode && (
            <span
              className="text-[var(--text-xs)] px-[var(--space-2)] py-0.5 rounded-full flex-shrink-0"
              style={{
                background: "var(--surface-tertiary)",
                color: "var(--text-secondary)",
              }}
            >
              {event.courseCode}
            </span>
          )}
        </div>

        {event && (
          <div className="flex flex-wrap items-center gap-x-[var(--space-4)] gap-y-1">
            <MetaChip icon={<Clock size={12} />} text={formatTimeRange(event.startTime, event.endTime)} />
            {event.instructor && <MetaChip icon={<User size={12} />} text={event.instructor} />}
            {event.location && <MetaChip icon={<MapPin size={12} />} text={event.location} />}
          </div>
        )}
      </div>

      {/* ── 中段 + 下段：可滚动内容区 ── */}
      <div className="flex-1 overflow-y-auto px-[var(--space-5)] py-[var(--space-4)] space-y-[var(--space-5)]">

        {/* ── AI 预习指南 ── */}
        <section>
          <div className="flex items-center justify-between mb-[var(--space-3)]">
            <div className="flex items-center gap-[var(--space-2)]">
              <BookOpen size={15} style={{ color: "var(--brand-navy)" }} />
              <span
                className="text-[var(--text-sm)] font-semibold"
                style={{ color: "var(--text-primary)" }}
              >
                预习指南
              </span>
              {preview && (
                <span
                  className="text-[10px] px-[var(--space-2)] py-px rounded-full"
                  style={{
                    background: "var(--surface-tertiary)",
                    color: "var(--text-tertiary)",
                  }}
                >
                  AI Generated
                </span>
              )}
            </div>
            <button
              type="button"
              disabled={isGenerating}
              onClick={() => void generate(true)}
              className="flex items-center gap-[var(--space-1)] text-[var(--text-xs)] px-[var(--space-2)] py-1 rounded-[var(--radius-sm)] transition-colors"
              style={{
                color: "var(--text-secondary)",
                background: "var(--surface-secondary)",
                border: "1px solid var(--border-primary)",
              }}
            >
              <RefreshCw size={11} className={isGenerating ? "animate-spin" : ""} />
              重新生成
            </button>
          </div>

          <div
            className="rounded-[var(--radius-md)] p-[var(--space-4)] min-h-[200px]"
            style={{
              background: "var(--surface-secondary)",
              border: "1px solid var(--border-primary)",
            }}
          >
            {genError ? (
              <div className="space-y-[var(--space-2)]">
                <p className="text-[var(--text-sm)]" style={{ color: "var(--color-danger)" }}>
                  生成失败：{genError}
                </p>
                <button
                  type="button"
                  onClick={() => void generate(true)}
                  className="text-[var(--text-xs)] underline"
                  style={{ color: "var(--text-secondary)" }}
                >
                  重试
                </button>
              </div>
            ) : isGenerating && !preview ? (
              <PreviewSkeleton />
            ) : preview ? (
              <MarkdownBlock content={preview.content} />
            ) : (
              <div className="flex items-center justify-center h-40">
                <Loader2 size={20} className="animate-spin" style={{ color: "var(--text-tertiary)" }} />
              </div>
            )}
          </div>
        </section>

        {/* ── 用户预习笔记 ── */}
        <section>
          <div className="flex items-center justify-between mb-[var(--space-2)]">
            <span
              className="text-[var(--text-sm)] font-semibold"
              style={{ color: "var(--text-primary)" }}
            >
              我的预习笔记
            </span>
            {isSavingNotes && (
              <span className="text-[10px]" style={{ color: "var(--text-tertiary)" }}>
                保存中…
              </span>
            )}
          </div>
          <textarea
            value={notes}
            onChange={(e) => handleNotesChange(e.target.value)}
            placeholder="在这里记录你的预习想法、问题或摘要…（自动保存）"
            className="w-full rounded-[var(--radius-md)] px-[var(--space-3)] py-[var(--space-3)] text-[var(--text-sm)] resize-none outline-none transition-colors"
            style={{
              minHeight: 120,
              background: "var(--surface-secondary)",
              border: "1px solid var(--border-primary)",
              color: "var(--text-primary)",
            }}
            onFocus={(e) =>
              (e.currentTarget.style.borderColor = "var(--border-active)")
            }
            onBlur={(e) =>
              (e.currentTarget.style.borderColor = "var(--border-primary)")
            }
          />
        </section>
      </div>

      {/* ── 底部操作栏 ── */}
      <div
        className="flex-shrink-0 flex items-center justify-end gap-[var(--space-2)] px-[var(--space-4)] py-[var(--space-3)] border-t"
        style={{ borderColor: "var(--border-primary)" }}
      >
        {preview?.generatedAt && (
          <span
            className="text-[10px] mr-auto"
            style={{ color: "var(--text-tertiary)" }}
          >
            生成于 {new Date(preview.generatedAt).toLocaleString()}
            {preview.model ? `  ·  ${preview.model}` : ""}
          </span>
        )}
        <button
          type="button"
          disabled={isGenerating || !preview}
          onClick={() => void generate(true)}
          className="flex items-center gap-[var(--space-1)] px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-sm)] transition-colors"
          style={{
            background: "var(--surface-secondary)",
            border: "1px solid var(--border-primary)",
            color: "var(--text-secondary)",
            opacity: !preview ? 0.4 : 1,
          }}
        >
          <RefreshCw size={13} className={isGenerating ? "animate-spin" : ""} />
          重新生成
        </button>
        <button
          type="button"
          disabled={!preview}
          className="flex items-center gap-[var(--space-1)] px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-sm)] transition-colors"
          style={{
            background: "var(--brand-navy)",
            color: "#fff",
            opacity: !preview ? 0.4 : 1,
          }}
          title="将预习内容保存为项目素材（功能将在后续版本完善）"
        >
          <BookmarkPlus size={13} />
          保存为素材
        </button>
      </div>
    </div>
  );
}

// ─── 小工具 ──────────────────────────────────────────────────────────────────

function MetaChip({ icon, text }: { icon: React.ReactNode; text: string }) {
  return (
    <div
      className="flex items-center gap-[var(--space-1)] text-[var(--text-xs)]"
      style={{ color: "var(--text-tertiary)" }}
    >
      {icon}
      <span>{text}</span>
    </div>
  );
}

function formatTimeRange(start: string, end: string): string {
  const s = new Date(start);
  const e = new Date(end);
  const dateStr = s.toLocaleDateString([], {
    weekday: "short",
    month: "short",
    day: "numeric",
  });
  const startT = s.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  const endT = e.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  return `${dateStr}  ${startT}–${endT}`;
}
