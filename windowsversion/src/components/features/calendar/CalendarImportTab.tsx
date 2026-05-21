/**
 * CalendarImportTab — 设置面板中的「课程日历」选项卡
 *
 * 布局：
 *   1. 拖入 .ics 文件区域（或文件选择器）
 *   2. iCal 订阅 URL 输入框
 *   3. 解析预览列表（可逐条勾选）+ 确认/取消按钮
 *   4. 已导入日历列表（刷新/删除）
 *
 * 约束（宪章 A1/A2）：named export，CSS 变量，无硬编码颜色
 */

import { useState } from "react";
import {
  Upload,
  Link,
  RefreshCw,
  Trash2,
  CheckSquare,
  Square,
  Loader2,
  CalendarDays,
  AlertCircle,
} from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { useCalendarStore } from "../../../stores/calendarStore";
import { useLibraryStore } from "../../../stores/libraryStore";
import type { ParsedEvent } from "../../../types/calendar";

// ─────────────────────────────────────────────────────────────────────────────

export function CalendarImportTab() {
  const libraryId = useLibraryStore((s) => s.activeLibraryId) ?? "";
  const {
    events,
    pendingImportResult,
    pendingSelectedIds,
    isLoading,
    error,
    importFromFile,
    importFromUrl,
    togglePendingSelect,
    selectAllPending,
    confirmImport,
    cancelImport,
    deleteSource,
    refreshSubscription,
  } = useCalendarStore();

  const [urlInput, setUrlInput] = useState("");
  const [confirming, setConfirming] = useState(false);
  const [successMsg, setSuccessMsg] = useState<string | null>(null);
  const [dragOver, setDragOver] = useState(false);

  // ── 检查库是否有效 ────────────────────────────────────────────────────────────

  const isLibraryValid = !!libraryId;

  // ── 文件选择 ────────────────────────────────────────────────────────────────

  const handleFileSelect = async () => {
    if (!isLibraryValid) {
      alert("请先选择一个知识库");
      return;
    }
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "iCalendar", extensions: ["ics"] }],
      });
      if (selected && typeof selected === "string") {
        await importFromFile(libraryId, selected);
      }
    } catch (error) {
      console.error("文件选择失败:", error);
    }
  };

  // ── 拖放处理 ────────────────────────────────────────────────────────────────

  const handleDragOver = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setDragOver(true);
  };

  const handleDragLeave = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setDragOver(false);
  };

  const handleDrop = async (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setDragOver(false);

    if (!isLibraryValid) {
      alert("请先选择一个知识库");
      return;
    }

    try {
      const files = Array.from(e.dataTransfer.files);
      const icsFiles = files.filter((f) => f.name.endsWith(".ics"));

      if (icsFiles.length === 0) {
        alert("请拖入 .ics 文件");
        return;
      }

      // 只处理第一个文件
      const file = icsFiles[0];
      // 获取文件的完整路径
      const filePath = (file as any).path || file.name;
      await importFromFile(libraryId, filePath);
    } catch (error) {
      console.error("拖放处理失败:", error);
    }
  };

  // ── URL 订阅 ─────────────────────────────────────────────────────────────────

  const handleUrlImport = async () => {
    if (!isLibraryValid) {
      alert("请先选择一个知识库");
      return;
    }
    if (!urlInput.trim()) return;
    try {
      await importFromUrl(libraryId, urlInput.trim());
      setUrlInput("");
    } catch (error) {
      console.error("URL 导入失败:", error);
    }
  };

  // ── 确认导入 ──────────────────────────────────────────────────────────────────

  const handleConfirm = async () => {
    setConfirming(true);
    const n = await confirmImport(libraryId);
    setConfirming(false);
    setSuccessMsg(`已导入 ${n} 个课程事件`);
    setTimeout(() => setSuccessMsg(null), 3000);
  };

  // ── 已导入的 URL 来源（去重） ───────────────────────────────────────────────

  const importedUrls = Array.from(
    new Set(events.filter((e) => e.calendarSource === "ics_url" && e.sourceUrl).map((e) => e.sourceUrl!))
  );
  const hasFileEvents = events.some((e) => e.calendarSource === "ics_file");

  // ─────────────────────────────────────────────────────────────────────────────
  // 渲染
  // ─────────────────────────────────────────────────────────────────────────────

  return (
    <div className="space-y-[var(--space-5)]">
      <h3
        className="text-[var(--text-base)] font-semibold"
        style={{ color: "var(--text-primary)" }}
      >
        课程日历
      </h3>

      {/* ── 库未选中提示 ── */}
      {!isLibraryValid && (
        <div
          className="px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-sm)] flex items-center gap-[var(--space-2)]"
          style={{
            background: "rgba(245,158,11,0.08)",
            border: "1px solid rgba(245,158,11,0.2)",
            color: "var(--text-primary)",
          }}
        >
          <AlertCircle size={16} style={{ color: "rgba(245,158,11,0.6)", flexShrink: 0 }} />
          请先在侧边栏选择一个知识库
        </div>
      )}

      {/* ── 错误提示 ── */}
      {error && (
        <div
          className="px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-sm)] flex items-center gap-[var(--space-2)]"
          style={{
            background: "rgba(239,68,68,0.08)",
            border: "1px solid rgba(239,68,68,0.2)",
            color: "var(--text-primary)",
          }}
        >
          <AlertCircle size={16} style={{ color: "rgba(239,68,68,0.6)", flexShrink: 0 }} />
          {error}
        </div>
      )}

      {/* ── 成功提示 ── */}
      {successMsg && (
        <div
          className="px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-sm)]"
          style={{
            background: "rgba(52,199,89,0.08)",
            border: "1px solid rgba(52,199,89,0.2)",
            color: "var(--text-primary)",
          }}
        >
          ✓ {successMsg}
        </div>
      )}

      {/* ── 如果正在预览，显示预览区域，否则显示导入区域 ── */}
      {pendingImportResult ? (
        <PendingPreview
          result={pendingImportResult.events}
          selectedIds={pendingSelectedIds}
          onToggle={togglePendingSelect}
          onSelectAll={selectAllPending}
          onConfirm={handleConfirm}
          onCancel={cancelImport}
          confirming={confirming}
        />
      ) : (
        <>
          {/* ── 拖入 .ics 区域 ── */}
          <div>
            <p
              className="text-[var(--text-sm)] mb-[var(--space-2)]"
              style={{ color: "var(--text-secondary)" }}
            >
              导入方式
            </p>
            <div
              className="relative flex flex-col items-center justify-center gap-[var(--space-2)] rounded-[var(--radius-md)] border-2 border-dashed py-[var(--space-6)] px-[var(--space-4)] cursor-pointer transition-colors"
              style={{
                borderColor: dragOver ? "var(--brand-navy)" : "var(--border-primary)",
                background: dragOver ? "rgba(30,53,109,0.05)" : "transparent",
              }}
              onClick={handleFileSelect}
              onDragOver={handleDragOver}
              onDragLeave={handleDragLeave}
              onDrop={handleDrop}
            >
              {isLoading ? (
                <Loader2
                  size={20}
                  className="animate-spin"
                  style={{ color: "var(--text-tertiary)" }}
                />
              ) : (
                <Upload size={20} style={{ color: "var(--text-tertiary)" }} />
              )}
              <span
                className="text-[var(--text-sm)] text-center"
                style={{ color: "var(--text-secondary)" }}
              >
                点击选择或拖入 .ics 文件
              </span>
            </div>
          </div>

          {/* ── 分隔 ── */}
          <div className="flex items-center gap-[var(--space-3)]">
            <div className="flex-1 h-px" style={{ background: "var(--border-primary)" }} />
            <span
              className="text-[var(--text-xs)]"
              style={{ color: "var(--text-tertiary)" }}
            >
              或
            </span>
            <div className="flex-1 h-px" style={{ background: "var(--border-primary)" }} />
          </div>

          {/* ── URL 订阅 ── */}
          <div className="space-y-[var(--space-2)]">
            <label
              className="text-[var(--text-sm)]"
              style={{ color: "var(--text-secondary)" }}
            >
              iCal 订阅链接
            </label>
            <div className="flex gap-[var(--space-2)]">
              <input
                type="url"
                placeholder="https://calendar.google.com/calendar/ical/..."
                value={urlInput}
                onChange={(e) => setUrlInput(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleUrlImport()}
                className="flex-1 input-glass px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-sm)]"
                style={{ color: "var(--text-primary)" }}
                disabled={!isLibraryValid || isLoading}
              />
              <button
                type="button"
                disabled={isLoading || !urlInput.trim() || !isLibraryValid}
                onClick={handleUrlImport}
                className="flex items-center gap-[var(--space-1)] px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-sm)] transition-colors"
                style={{
                  background: "var(--brand-navy)",
                  color: "#fff",
                  opacity:
                    isLoading || !urlInput.trim() || !isLibraryValid ? 0.4 : 1,
                }}
              >
                <Link size={14} />
                订阅
              </button>
            </div>
          </div>

          {/* ── 已导入的日历列表 ── */}
          {(hasFileEvents || importedUrls.length > 0) && (
            <div className="space-y-[var(--space-3)]">
              <div
                className="h-px"
                style={{ background: "var(--border-primary)" }}
              />
              <p
                className="text-[var(--text-sm)] font-medium"
                style={{ color: "var(--text-primary)" }}
              >
                已导入的日历
              </p>

              {/* 本地文件来源 */}
              {hasFileEvents && (
                <ImportedSourceRow
                  label={`本地 .ics 文件  ·  ${events.filter((e) => e.calendarSource === "ics_file").length} 个事件`}
                  onDelete={() => deleteSource(libraryId, "ics_file")}
                />
              )}

              {/* URL 订阅来源 */}
              {importedUrls.map((url) => {
                const count = events.filter((e) => e.sourceUrl === url).length;
                return (
                  <ImportedSourceRow
                    key={url}
                    label={`${truncateUrl(url)}  ·  ${count} 个事件`}
                    onRefresh={() => refreshSubscription(libraryId, url)}
                    onDelete={() => deleteSource(libraryId, "ics_url", url)}
                    isLoading={isLoading}
                  />
                );
              })}
            </div>
          )}
        </>
      )}
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// 子组件：预览列表
// ─────────────────────────────────────────────────────────────────────────────

function PendingPreview({
  result,
  selectedIds,
  onToggle,
  onSelectAll,
  onConfirm,
  onCancel,
  confirming,
}: {
  result: ParsedEvent[];
  selectedIds: Set<string>;
  onToggle: (id: string) => void;
  onSelectAll: (all: boolean) => void;
  onConfirm: () => void;
  onCancel: () => void;
  confirming: boolean;
}) {
  const allSelected = result.every((e) => selectedIds.has(e.tempId));
  const selectedCount = selectedIds.size;

  return (
    <div className="space-y-[var(--space-3)]">
      <div className="flex items-center justify-between">
        <p
          className="text-[var(--text-sm)] font-medium"
          style={{ color: "var(--text-primary)" }}
        >
          解析到 {result.length} 个课程事件
        </p>
        <button
          type="button"
          className="flex items-center gap-[var(--space-1)] text-[var(--text-xs)]"
          style={{ color: "var(--text-secondary)" }}
          onClick={() => onSelectAll(!allSelected)}
        >
          {allSelected ? <CheckSquare size={14} /> : <Square size={14} />}
          {allSelected ? "取消全选" : "全选"}
        </button>
      </div>

      {/* 事件列表 */}
      <div
        className="rounded-[var(--radius-md)] overflow-hidden border"
        style={{ borderColor: "var(--border-primary)", maxHeight: 280, overflowY: "auto" }}
      >
        {result.map((ev) => {
          const checked = selectedIds.has(ev.tempId);
          return (
            <button
              key={ev.tempId}
              type="button"
              onClick={() => onToggle(ev.tempId)}
              className="w-full flex items-start gap-[var(--space-3)] px-[var(--space-3)] py-[var(--space-2)] text-left transition-colors border-b last:border-b-0"
              style={{
                background: checked ? "var(--surface-secondary)" : "var(--surface-primary)",
                borderColor: "var(--border-primary)",
              }}
            >
              {checked ? (
                <CheckSquare size={14} className="mt-0.5 flex-shrink-0" style={{ color: "var(--brand-navy)" }} />
              ) : (
                <Square size={14} className="mt-0.5 flex-shrink-0" style={{ color: "var(--text-tertiary)" }} />
              )}
              <div className="min-w-0">
                <p
                  className="text-[var(--text-sm)] font-medium truncate"
                  style={{ color: "var(--text-primary)" }}
                >
                  {ev.title}
                </p>
                <p
                  className="text-[var(--text-xs)] truncate"
                  style={{ color: "var(--text-tertiary)" }}
                >
                  {formatEventTime(ev.startTime, ev.endTime)}
                  {ev.location ? `  ·  ${ev.location}` : ""}
                </p>
              </div>
            </button>
          );
        })}
      </div>

      {/* 操作按钮 */}
      <div className="flex items-center justify-between pt-[var(--space-1)]">
        <span
          className="text-[var(--text-xs)]"
          style={{ color: "var(--text-tertiary)" }}
        >
          已选 {selectedCount} / {result.length} 个
        </span>
        <div className="flex gap-[var(--space-2)]">
          <button
            type="button"
            onClick={onCancel}
            className="px-[var(--space-3)] py-[var(--space-1)] rounded-[var(--radius-md)] text-[var(--text-sm)] transition-colors"
            style={{
              background: "var(--surface-secondary)",
              color: "var(--text-secondary)",
              border: "1px solid var(--border-primary)",
            }}
          >
            取消
          </button>
          <button
            type="button"
            disabled={confirming || selectedCount === 0}
            onClick={onConfirm}
            className="flex items-center gap-[var(--space-1)] px-[var(--space-3)] py-[var(--space-1)] rounded-[var(--radius-md)] text-[var(--text-sm)] transition-colors"
            style={{
              background: "var(--brand-navy)",
              color: "#fff",
              opacity: selectedCount === 0 ? 0.4 : 1,
            }}
          >
            {confirming && <Loader2 size={12} className="animate-spin" />}
            确认导入
          </button>
        </div>
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// 子组件：已导入来源行
// ─────────────────────────────────────────────────────────────────────────────

function ImportedSourceRow({
  label,
  onRefresh,
  onDelete,
  isLoading,
}: {
  label: string;
  onRefresh?: () => void;
  onDelete: () => void;
  isLoading?: boolean;
}) {
  return (
    <div
      className="flex items-center justify-between px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-md)]"
      style={{
        background: "var(--surface-secondary)",
        border: "1px solid var(--border-primary)",
      }}
    >
      <div className="flex items-center gap-[var(--space-2)] min-w-0">
        <CalendarDays size={14} style={{ color: "var(--text-tertiary)", flexShrink: 0 }} />
        <span
          className="text-[var(--text-sm)] truncate"
          style={{ color: "var(--text-primary)" }}
        >
          {label}
        </span>
      </div>
      <div className="flex gap-[var(--space-1)] flex-shrink-0 ml-[var(--space-2)]">
        {onRefresh && (
          <button
            type="button"
            onClick={onRefresh}
            disabled={isLoading}
            className="p-1 rounded-[var(--radius-sm)] transition-colors"
            title="刷新"
            style={{ color: "var(--text-secondary)" }}
          >
            <RefreshCw size={13} className={isLoading ? "animate-spin" : ""} />
          </button>
        )}
        <button
          type="button"
          onClick={onDelete}
          className="p-1 rounded-[var(--radius-sm)] transition-colors"
          title="删除"
          style={{ color: "var(--text-tertiary)" }}
        >
          <Trash2 size={13} />
        </button>
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// 工具函数
// ─────────────────────────────────────────────────────────────────────────────

function formatEventTime(start: string, end: string): string {
  const s = new Date(start);
  const e = new Date(end);
  const fmt = (d: Date) =>
    d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  const dateStr = s.toLocaleDateString([], { month: "short", day: "numeric", weekday: "short" });
  return `${dateStr}  ${fmt(s)}–${fmt(e)}`;
}

function truncateUrl(url: string, max = 40): string {
  return url.length > max ? url.slice(0, max) + "…" : url;
}
