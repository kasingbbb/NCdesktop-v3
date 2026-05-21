/**
 * InferenceToast — 低置信度信号推断 Toast（Step 3）
 *
 * 当 infer_asset_context 返回 confidence < 0.65 时，
 * Tauri 事件 `notecapt/inference-low-confidence` 触发此组件显示。
 *
 * 用户可：
 *   - 选择归属到候选知识单元之一
 *   - 决定"创建新知识单元"
 *   - 忽略（暂不处理）
 *
 * 宪章 K3：只展示最多 2 个候选，不让用户陷入选择困难。
 */

import { useEffect, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { X, Zap } from "lucide-react";
import type { InferenceLowConfidencePayload, CandidateKu } from "../../../lib/tauri-commands";
import "./InferenceToast.css";

interface ActiveToast {
  payload: InferenceLowConfidencePayload;
  id: string; // for keying / dismissal
}

interface Props {
  onLinkToKu?: (assetId: string, kuId: string) => void;
  onCreateNew?: (assetId: string) => void;
}

export function InferenceToastHost({ onLinkToKu, onCreateNew }: Props) {
  const [toasts, setToasts] = useState<ActiveToast[]>([]);

  useEffect(() => {
    let unlisten: UnlistenFn | null = null;

    listen<InferenceLowConfidencePayload>(
      "notecapt/inference-low-confidence",
      (event) => {
        setToasts((prev) => {
          // 相同素材的旧 toast 先移除
          const filtered = prev.filter(t => t.payload.assetId !== event.payload.assetId);
          return [
            ...filtered,
            { payload: event.payload, id: `${event.payload.assetId}-${Date.now()}` },
          ];
        });
      }
    ).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, []);

  const dismiss = (id: string) => {
    setToasts((prev) => prev.filter(t => t.id !== id));
  };

  if (toasts.length === 0) return null;

  return (
    <div className="inf-toast-stack">
      {toasts.map((toast) => (
        <InferenceToastItem
          key={toast.id}
          toast={toast}
          onDismiss={() => dismiss(toast.id)}
          onLinkToKu={(kuId) => {
            onLinkToKu?.(toast.payload.assetId, kuId);
            dismiss(toast.id);
          }}
          onCreateNew={() => {
            onCreateNew?.(toast.payload.assetId);
            dismiss(toast.id);
          }}
        />
      ))}
    </div>
  );
}

// ─── 单条 Toast ───────────────────────────────────────────────────────────────

interface ItemProps {
  toast: ActiveToast;
  onDismiss: () => void;
  onLinkToKu: (kuId: string) => void;
  onCreateNew: () => void;
}

function InferenceToastItem({ toast, onDismiss, onLinkToKu, onCreateNew }: ItemProps) {
  const { assetName, candidates, suggestedAction } = toast.payload;

  return (
    <div className="inf-toast">
      <div className="inf-toast-header">
        <Zap size={13} className="inf-toast-icon" />
        <span className="inf-toast-title">
          「{truncate(assetName, 20)}」属于哪个知识单元？
        </span>
        <button className="inf-toast-close" onClick={onDismiss} aria-label="忽略">
          <X size={13} />
        </button>
      </div>

      <p className="inf-toast-sub">AI 不确定，请你确认一下</p>

      <div className="inf-toast-actions">
        {/* 候选 KU 按钮 */}
        {candidates.map((ku: CandidateKu) => (
          <button
            key={ku.id}
            className="inf-toast-candidate"
            onClick={() => onLinkToKu(ku.id)}
            title={ku.coreInsight}
          >
            <span className="inf-cand-title">{truncate(ku.title, 28)}</span>
            <span className="inf-cand-sim">{Math.round(ku.similarity * 100)}% 相似</span>
          </button>
        ))}

        {/* 创建新 KU */}
        {(suggestedAction === "create_new" || suggestedAction === "choose") && (
          <button className="inf-toast-new" onClick={onCreateNew}>
            + 这是全新知识
          </button>
        )}
      </div>
    </div>
  );
}

// ─── 工具 ─────────────────────────────────────────────────────────────────────

function truncate(s: string, max: number): string {
  return s.length > max ? s.slice(0, max) + "…" : s;
}
