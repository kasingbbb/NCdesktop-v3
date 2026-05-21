/**
 * 工作区拖出 .md 投影 hook（task_008 AC-3）。
 *
 * 流程：
 * 1. mousedown：记录起点，同时**立即** kick off `prepare_outbound_payload`（异步）。
 *    在用户交互上下文（mousedown）阶段就发起 IPC，能让 await 解析时大概率已经
 *    返回，避免到达 mousemove 阈值时 startDrag 失去 user gesture。
 * 2. mousemove 阈值跨过：await 之前 kick off 的 Promise → 成功 → startDrag(item: e.path)。
 * 3. 失败：解析 OutboundError 联合类型 → 4 种中文 toast。
 *
 * 注意：
 * - mousedown 时不知道最终拖几条（用户可能 cmd+click 加选），这里按当前选择集合 / 单条
 *   两种情况预先决定 ids（与 mousemove 阶段一致）。
 * - 若 mousedown 后用户没拖（点击/松手）→ Promise 自然废弃，不影响 UI。
 * - dropzone 内部组件签名未变，本 hook 不动 DropzoneApp。
 */
import { useCallback, useEffect, useRef } from "react";
import { startDrag } from "@crabnebula/tauri-plugin-drag";
import { invoke } from "@tauri-apps/api/core";
import type { Asset } from "../types";
import {
  parseOutboundError,
  type OutboundError,
} from "../lib/tauri-commands";
import { useUIStore } from "../stores/uiStore";

export const DRAG_ASSET_TYPE = "application/notecapt-assets";

export interface DragAssetPayload {
  assetIds: string[];
}

const DRAG_MOVE_THRESHOLD = 5;

function outboundErrorToToast(err: OutboundError | null, rawMessage: string): {
  title: string;
  message: string;
} {
  if (!err) {
    return { title: "拖拽准备失败", message: rawMessage };
  }
  switch (err.kind) {
    case "stateNotDone":
      return {
        title: "无法拖出",
        message: `非 done 态资产无法拖出（当前：${err.state}）`,
      };
    case "mixedStates":
      return {
        title: "无法拖出",
        message: `多选包含非 done 态资产（${err.offending.length} 条），无法整体拖出`,
      };
    case "renditionMissing":
      return {
        title: "无法拖出",
        message: "未找到转化后的 MD 文件，请先重试转化",
      };
    case "ioFailed":
      return {
        title: "拖拽准备失败",
        message: err.detail || err.message || "IO 错误",
      };
    case "emptyInput":
      return { title: "无法拖出", message: "未选中任何素材" };
    case "assetNotFound":
      return { title: "无法拖出", message: `资产不存在：${err.assetId}` };
  }
}

export function useDragAssets(
  selectedAssetIds: Set<string>,
  assets: Asset[]
) {
  const pendingDragRef = useRef<{
    assetId: string;
    startX: number;
    startY: number;
    ids: string[];
  } | null>(null);
  const isDraggingRef = useRef(false);
  const dragIconRef = useRef<string>("");
  const addNotification = useUIStore((s) => s.addNotification);
  // 通过 ref 读最新的 assets/selection，避免重建 makeDragProps 引起的 listener 漂移
  const assetsRef = useRef(assets);
  const selectedRef = useRef(selectedAssetIds);
  useEffect(() => {
    assetsRef.current = assets;
  }, [assets]);
  useEffect(() => {
    selectedRef.current = selectedAssetIds;
  }, [selectedAssetIds]);

  useEffect(() => {
    invoke<string>("get_drag_icon_path")
      .then((p) => {
        dragIconRef.current = p;
      })
      .catch((e) => console.error("[drag] get_drag_icon_path failed:", e));
  }, []);

  const toast = useCallback(
    (raw: unknown) => {
      const parsed = parseOutboundError(raw);
      const { title, message } = outboundErrorToToast(parsed, String(raw));
      // task_011 AC-5：同一 OutboundError 类型在 3s 窗口内合并/替换，避免堆积
      const dedupeKey = parsed ? `outbound:${parsed.kind}` : "outbound:unknown";
      addNotification({
        type: "warning",
        title,
        message,
        duration: 4000,
        dedupeKey,
      });
    },
    [addNotification]
  );

  const makeDragProps = useCallback(
    (assetId: string) => {
      return {
        onMouseDown: (e: React.MouseEvent<HTMLElement>) => {
          if (e.button !== 0) return;
          e.preventDefault();

          const ids = selectedRef.current.has(assetId)
            ? Array.from(selectedRef.current)
            : [assetId];

          // 2026-05-17 修复 release 拖到 Finder 完全无反应：
          //
          // 旧实现：mousedown → invoke prepare_outbound_payload → 等 IPC + DB IO + hard_link
          //  → mousemove threshold → 跨 main thread → 合成 mouseDragged → beginDraggingSession。
          // 整条链路跨越 50~150ms，macOS NSApp.currentEvent() 已不再是用户的 mouseDown 事件，
          // user gesture context 丢失，NSDraggingSession 被 macOS silent reject
          // （drag crate 是 fire-and-forget，错误根本到不了前端）。
          //
          // 新实现：mousedown 不调任何 IPC，仅记 ids；mousemove threshold 后直接
          // startDrag(原文件 path)。完全模拟原来 fallback 路径的成功链路（dev mode 测过 OK）。
          // 牺牲：outbound markdown 投影功能（拖 .md 到 Notion/Claude），用户后续若需要
          // 可通过"分享"按钮触发；核心 use case 拖原文件到 Finder/外部文件夹稳定可用。
          console.warn(`[drag] mousedown ids=${ids.length} (first=${ids[0]})`);

          pendingDragRef.current = {
            assetId,
            startX: e.clientX,
            startY: e.clientY,
            ids,
          };
          isDraggingRef.current = false;

          function onMouseMove(ev: MouseEvent) {
            const pending = pendingDragRef.current;
            if (!pending || isDraggingRef.current) return;

            const dx = ev.clientX - pending.startX;
            const dy = ev.clientY - pending.startY;
            if (
              Math.abs(dx) < DRAG_MOVE_THRESHOLD &&
              Math.abs(dy) < DRAG_MOVE_THRESHOLD
            )
              return;

            isDraggingRef.current = true;
            pendingDragRef.current = null;
            cleanup();

            const paths = pending.ids
              .map((id) => assetsRef.current.find((a) => a.id === id)?.filePath)
              .filter((p): p is string => !!p);

            console.warn(
              `[drag] threshold crossed → startDrag(direct, no-IPC) count=${paths.length} icon=${dragIconRef.current ? "set" : "empty"} first=${paths[0]}`
            );
            if (paths.length === 0) {
              console.warn(`[drag] aborted: no asset.filePath available`);
              return;
            }
            startDrag({
              item: paths,
              icon: dragIconRef.current,
              mode: "copy",
            })
              .then(() => console.warn(`[drag] startDrag resolved OK`))
              .catch((err) => {
                console.error(`[drag] startDrag REJECTED:`, err);
                toast(err);
              });
          }

          function onMouseUp() {
            pendingDragRef.current = null;
            isDraggingRef.current = false;
            cleanup();
          }

          function cleanup() {
            window.removeEventListener("mousemove", onMouseMove);
            window.removeEventListener("mouseup", onMouseUp);
          }

          window.addEventListener("mousemove", onMouseMove);
          window.addEventListener("mouseup", onMouseUp);
        },
      };
    },
    [toast]
  );

  return { makeDragProps };
}
