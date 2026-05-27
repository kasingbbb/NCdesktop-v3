import { type DragEvent, useEffect, useState } from "react";
import { GripHorizontal, MoveDiagonal2, Sparkles, X } from "lucide-react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { useDropzoneStore, type DropzoneStore } from "../../../stores/dropzoneStore";
import {
  useKcQueueStore,
  type KcEnrichedPayload,
  type KcQueuedPayload,
} from "../../../stores/kcQueueStore";
import * as cmd from "../../../lib/tauri-commands";
import { DropzoneIdle } from "./DropzoneIdle";
import { DropzoneAttract } from "./DropzoneAttract";
import { DropzoneProcessing } from "./DropzoneProcessing";
import { DropzoneComplete } from "./DropzoneComplete";
import { DropzoneExpanded } from "./DropzoneExpanded";
import { logger } from "../../../utils/logger";
import { formatDropzoneImportDetail } from "../../../lib/dropzone-import-detail";

function getDropzoneStore(): DropzoneStore {
  return useDropzoneStore.getState();
}

export function DropzoneApp() {
  const { phase: currentState, setPhase: setState, addItem, isExpanded } =
    useDropzoneStore();

  // v1.3 task_011 DZ-01：监听悬浮窗自身 focus/blur；失焦时半透明（主窗聚焦的等价信号）
  const [isFocused, setIsFocused] = useState(true);

  useEffect(() => {
    logger.info("DropzoneApp", "Phase changed", { phase: currentState });
  }, [currentState]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void getCurrentWindow()
      .onFocusChanged(({ payload: focused }) => setIsFocused(focused))
      .then((fn) => {
        unlisten = fn;
      })
      .catch((err) => {
        logger.warn("DropzoneApp", "onFocusChanged setup failed", { err: String(err) });
      });
    return () => {
      unlisten?.();
    };
  }, []);

  const simulateImport = (): void => {
    setState("processing");
    setTimeout(() => {
      addItem({
        id: Math.random().toString(),
        status: "done",
        fileName: "Mock File.png",
        fileType: "image/png",
        targetProjectId: null,
        addedAt: new Date().toISOString(),
      });
      setState("complete");
      setTimeout(() => setState("idle"), 2000);
    }, 1500);
  };

  useEffect(() => {
    let cancelled = false;
    let unlistenDrag: (() => void) | undefined;
    let unlistenAI: (() => void) | undefined;

    void getCurrentWebview()
      .onDragDropEvent((event) => {
        if (cancelled) return;

        const p = event.payload;

        if (p.type === "enter") {
          setState("attract");
          return;
        }
        if (p.type === "leave") {
          // 松手后常会先 drop 再 leave，避免把 processing/complete 打回 idle
          const phase = getDropzoneStore().phase;
          if (phase === "processing" || phase === "complete") {
            return;
          }
          setState("idle");
          return;
        }
        if (p.type === "over") {
          return;
        }
        if (p.type === "drop") {
          const paths = p.paths.filter((path) => path.length > 0);
          if (paths.length === 0) {
            addItem({
              id: `fail-${crypto.randomUUID()}`,
              status: "error",
              fileName: "未获取到文件路径，请从访达拖入文件",
              fileType: "error",
              targetProjectId: null,
              addedAt: new Date().toISOString(),
            });
            setState("idle");
            return;
          }

          void (async () => {
            const dz = getDropzoneStore();
            dz.setExpanded(true);
            dz.setProcessingUI("正在入库…", 0.45);
            setState("processing");
            try {
              const summary = await cmd.importDropPaths(paths);
              const projectHint =
                summary.importProjectName.trim().length > 0
                  ? ` · 主页左侧打开「${summary.importProjectName}」查看素材`
                  : "";

              for (const row of summary.created) {
                const detail =
                  formatDropzoneImportDetail(
                    row.aiClassified,
                    row.aiNote,
                    row.aiPending === true
                  ) + projectHint;
                addItem({
                  id: row.id,
                  status: "done",
                  fileName: row.name,
                  fileType: row.mimeType,
                  targetProjectId: row.projectId,
                  addedAt: row.importedAt,
                  detail,
                });
              }
              for (const msg of summary.failures) {
                addItem({
                  id: `fail-${crypto.randomUUID()}`,
                  status: "error",
                  fileName: msg,
                  fileType: "error",
                  targetProjectId: null,
                  addedAt: new Date().toISOString(),
                });
              }
              if (summary.created.length === 0 && summary.failures.length === 0) {
                addItem({
                  id: `fail-${crypto.randomUUID()}`,
                  status: "error",
                  fileName: "未能导入任何文件",
                  fileType: "error",
                  targetProjectId: null,
                  addedAt: new Date().toISOString(),
                });
              }
              setState("complete");
              setTimeout(() => setState("idle"), 4000);
            } catch (e) {
              addItem({
                id: `err-${crypto.randomUUID()}`,
                status: "error",
                fileName: String(e),
                fileType: "error",
                targetProjectId: null,
                addedAt: new Date().toISOString(),
              });
              setState("idle");
            } finally {
              getDropzoneStore().clearProcessingUI();
            }
          })();
        }
      })
      .then((fn) => {
        if (!cancelled) unlistenDrag = fn;
      })
      .catch((err) => {
        console.error("[dropzone] onDragDropEvent", err);
      });

    void listen<{ assetId: string }>("notecapt/dropzone-ai-finished", (event) => {
      if (cancelled) return;
      // 当 AI 分类完成后，尝试刷新该项的显示（显示已打标等）
      const item = getDropzoneStore().recentItems.find((i) => i.id === event.payload.assetId);
      if (item) {
        getDropzoneStore().updateItemDetail(item.id, "✨ AI 已完成自动分类与打标");
      }
    }).then((fn) => {
      if (!cancelled) unlistenAI = fn;
    });

    // task_025 AC-2：订阅 KC 队列起点 / 终点事件，drive kcQueueStore。
    let unlistenKcQueued: (() => void) | undefined;
    let unlistenKcEnriched: (() => void) | undefined;
    void listen<KcQueuedPayload>("notecapt/asset-kc-queued", (event) => {
      if (cancelled) return;
      useKcQueueStore.getState().enqueue(event.payload.assetId);
    }).then((fn) => {
      if (!cancelled) unlistenKcQueued = fn;
    });
    void listen<KcEnrichedPayload>("notecapt/asset-kc-enriched", (event) => {
      if (cancelled) return;
      useKcQueueStore.getState().dequeue(event.payload.assetId);
    }).then((fn) => {
      if (!cancelled) unlistenKcEnriched = fn;
    });

    return () => {
      cancelled = true;
      unlistenDrag?.();
      unlistenAI?.();
      // task_025：组件 unmount 必须清理事件订阅（Reviewer 重点关注）
      unlistenKcQueued?.();
      unlistenKcEnriched?.();
    };
  }, [setState, addItem]);

  // task_025 AC-3：KC 队列 toast 的"完成态"5s 窗口判定（用 50ms 间隔 tick state 触发 re-render）。
  // 用 effect 内 setInterval 而非定时器订阅，window 切换 / 实测 jsdom 下都安全。
  const [now, setNow] = useState(() => Date.now());
  const kcQueueLength = useKcQueueStore((s) => s.kcQueueLength);
  const kcCurrentAssetId = useKcQueueStore((s) => s.kcCurrentAssetId);
  const kcLastCompletedAt = useKcQueueStore((s) => s.lastCompletedAt);
  useEffect(() => {
    if (kcQueueLength === 0 && kcLastCompletedAt === null) {
      // 完全没在跑也没完成过 → 不需要 tick
      return;
    }
    const id = setInterval(() => setNow(Date.now()), 500);
    return () => clearInterval(id);
  }, [kcQueueLength, kcLastCompletedAt]);

  /** AC-3：派生 toast 形态 —— "running" / "done" / "hidden"。 */
  const kcToast = (() => {
    if (kcQueueLength > 0) {
      return {
        kind: "running" as const,
        text: `AI 增强中 ${kcQueueLength}…`,
        title: kcCurrentAssetId ? `当前: ${kcCurrentAssetId}` : "AI 增强进行中",
      };
    }
    // 队列空 + 完成时间戳在 5s 内 → done toast 显示，3s 后消失（5-3=2s 是延迟窗，简洁起见用单一 5s）
    if (kcLastCompletedAt !== null && now - kcLastCompletedAt < 5000) {
      return {
        kind: "done" as const,
        text: "AI 增强完成",
        title: "AI 增强已完成",
      };
    }
    return null;
  })();

  if (currentState === "hidden") return null;

  const handleDragEnter = (e: DragEvent): void => {
    e.preventDefault();
  };

  const handleDragLeave = (e: DragEvent): void => {
    e.preventDefault();
  };

  const handleDragOver = (e: DragEvent): void => {
    e.preventDefault();
  };

  const handleDrop = (e: DragEvent): void => {
    e.preventDefault();

    const files = Array.from(e.dataTransfer.files);
    if (files.length > 0) {
      simulateImport();
      return;
    }
    const text = e.dataTransfer.getData("text");
    if (text) {
      simulateImport();
      return;
    }
    // macOS WKWebView 从外部拖入时 files 通常为空，由 onDragDropEvent 的 paths 导入
  };

  const win = getCurrentWindow();

  return (
    <div
      data-testid="dropzone-root"
      data-focused={isFocused ? "true" : "false"}
      className={`w-screen h-screen flex flex-col select-none overflow-hidden relative p-2.5 box-border transition-opacity ${isFocused ? "" : "dropzone-blurred"}`}
      onDragEnter={handleDragEnter}
      onDragLeave={handleDragLeave}
      onDragOver={handleDragOver}
      onDrop={handleDrop}
      onContextMenu={(e) => e.preventDefault()}
      style={{ background: "transparent" }}
      title="拖入文件以快速导入"
    >
      <div
        className="flex flex-col flex-1 min-h-0 overflow-hidden rounded-xl border"
        style={{
          background: "#1a2233",
          borderColor: "#2d3a50",
          boxShadow:
            "0 10px 30px rgba(0, 0, 0, 0.35), 0 2px 6px rgba(0, 0, 0, 0.18)",
        }}
      >
        {/* 顶部拖动条 */}
        <div
          className="relative shrink-0 h-8 flex items-center justify-center gap-1.5 cursor-grab active:cursor-grabbing z-40 rounded-t-xl border-b"
          style={{ borderColor: "#2d3a50", background: "#1e2940" }}
          onMouseDown={(e) => {
            if (e.button !== 0) {
              return;
            }
            void win.startDragging();
          }}
        >
          <GripHorizontal size={14} style={{ color: "rgba(255,255,255,0.4)" }} />
          <span className="text-[10px] font-medium tracking-wide" style={{ color: "rgba(255,255,255,0.5)" }}>
            拖动移动
          </span>
          <button
            type="button"
            aria-label="关闭悬浮窗"
            className="absolute right-1.5 top-1/2 -translate-y-1/2 w-5 h-5 flex items-center justify-center rounded cursor-pointer transition-colors"
            style={{ color: "rgba(255,255,255,0.4)" }}
            onMouseDown={(e) => e.stopPropagation()}
            onMouseEnter={(e) => {
              (e.currentTarget as HTMLElement).style.background = "rgba(255,255,255,0.1)";
              (e.currentTarget as HTMLElement).style.color = "rgba(255,255,255,0.8)";
            }}
            onMouseLeave={(e) => {
              (e.currentTarget as HTMLElement).style.background = "transparent";
              (e.currentTarget as HTMLElement).style.color = "rgba(255,255,255,0.4)";
            }}
            onClick={(e) => {
              e.stopPropagation();
              cmd
                .closeDropzoneWindow()
                .then(() => {})
                .catch((err) => console.error("[dropzone] close failed", err));
            }}
          >
            <X size={12} />
          </button>
        </div>

        <div className="relative z-10 flex-1 min-h-0 flex flex-col items-center justify-center px-2 pb-2">
          <div className="pointer-events-auto w-full max-w-[min(100%,360px)] flex flex-col items-center justify-center flex-1 min-h-0 py-2">
            {currentState === "idle" && <DropzoneIdle />}
            {currentState === "attract" && <DropzoneAttract />}
            {currentState === "processing" && <DropzoneProcessing />}
            {currentState === "complete" && <DropzoneComplete />}
          </div>
        </div>

        {isExpanded && (
          <div className="absolute inset-0 top-8 z-30 flex items-stretch justify-center p-3 pointer-events-none">
            <div className="pointer-events-auto w-full max-w-[min(100%,380px)] min-h-0 flex flex-1">
              <DropzoneExpanded />
            </div>
          </div>
        )}

        {/* task_025 AC-3：KC 队列 toast（悬浮在 dropzone 上方，z-40，不阻塞拖拽） */}
        {kcToast && (
          <div
            data-testid="kc-queue-toast"
            data-kind={kcToast.kind}
            className="absolute top-10 left-1/2 -translate-x-1/2 z-40 flex items-center gap-1.5 px-2.5 py-1 rounded-full pointer-events-none"
            style={{
              background: kcToast.kind === "running" ? "rgba(59,130,246,0.15)" : "rgba(34,197,94,0.18)",
              border: `1px solid ${kcToast.kind === "running" ? "#3b82f6" : "#22c55e"}`,
              color: kcToast.kind === "running" ? "#93c5fd" : "#86efac",
              fontSize: 10,
              fontWeight: 500,
              boxShadow: "0 2px 8px rgba(0,0,0,0.25)",
              transition: "opacity var(--duration-normal)",
            }}
            title={kcToast.title}
          >
            <Sparkles size={10} strokeWidth={2.5} />
            <span>{kcToast.text}</span>
          </div>
        )}

        {/* 右下角：缩放 */}
        <button
          type="button"
          aria-label="拖曳缩放窗口"
          className="absolute bottom-2.5 right-2.5 z-50 w-8 h-8 flex items-end justify-end p-0.5 cursor-nwse-resize border-0 bg-transparent rounded-br-[22px]"
          style={{ color: "var(--text-tertiary)" }}
          onMouseDown={(e) => {
            e.preventDefault();
            e.stopPropagation();
            void win.startResizeDragging("SouthEast");
          }}
        >
          <MoveDiagonal2 size={14} className="opacity-70" />
        </button>
      </div>
    </div>
  );
}
