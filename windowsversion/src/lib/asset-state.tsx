/**
 * 工作区四态徽章（task_008 AC-2 / AC-5）。
 *
 * - `assetStateLabel(state)`：状态 → 中文文案的唯一映射函数。
 * - `AssetStateBadge`：徽章组件，复用 lucide-react 已有图标；failed 态附带
 *   「重试」按钮，点击调用 `retryAssetConversion`（task_006 唯一入口）。
 *
 * 注意：UI 中所有显示的「state 文案」都必须通过 `assetStateLabel`，不允许
 * 在组件内出现 "已就绪" 等字面量（防止漂移）。
 */
import { useRef, useState } from "react";
import { CheckCircle2, AlertCircle, AlertTriangle, Loader2, WifiOff } from "lucide-react";
import type { AssetState } from "../types/workspaceAsset";
import { retryAssetConversion } from "./tauri-commands";
import {
  EXTRACTION_FAILURE_MESSAGES,
  isExtractionFailureLabel,
} from "./extraction-failure-codes";

/** task_011 AC-3：连点防抖窗口（毫秒）。同一资产 1 秒内重复点击直接忽略。 */
const RETRY_DEBOUNCE_MS = 1000;

const LABEL_MAP: Record<AssetState, string> = {
  done: "已就绪",
  converting: "转化中",
  failed: "失败",
  offline: "离线待转化",
};

export function assetStateLabel(state: AssetState): string {
  return LABEL_MAP[state];
}

/**
 * task_014 Fix-A4：判断当前 extractor_type 是否表示"占位 MD"（前缀 `placeholder_`）。
 *
 * 仅当 state="done" + isPlaceholder=true 时 UI 才显示"占位 MD"黄色徽章；
 * 真 MD（如 `markitdown`、`text_passthrough`）依旧"已就绪"绿色。
 */
export function isPlaceholderExtractor(extractorType?: string | null): boolean {
  if (!extractorType) return false;
  return extractorType.startsWith("placeholder_");
}

/** 徽章 + 重试按钮（仅 failed 态）。
 * - 父行需要把 `data-state={state}` 设在 row 上（PRD S3 测试断言）。
 * - 重试 UI 通过 onRetry / onError 回调上抛，组件自身保持纯 UI。
 */
export function AssetStateBadge({
  state,
  assetId,
  reason,
  extractorType,
  failureCode,
  onRetry,
  onError,
}: {
  state: AssetState;
  assetId: string;
  reason?: string | null;
  /** task_014 Fix-A4：用于区分占位 MD（placeholder_*）vs 真 MD */
  extractorType?: string | null;
  /**
   * task_014 AC-4：最近一行 `conversion_meta.failure_code`。
   * - `"legacy_unverified"` → 显示 ⚠️ 旧记录提示 + "重新转录"按钮
   * - `"E_*"` 8 错误码之一 → 与 state=failed 共同显示对应中文文案
   * - 其它 / null：不影响 badge
   */
  failureCode?: string | null;
  onRetry?: () => void;
  onError?: (msg: string) => void;
}) {
  // task_014 AC-4：legacy_unverified 是独立的"旧版未验证"态，
  // 优先级高于 state=done 的"已就绪"显示（即使 state 看似正常也要提示重新转录）。
  const isLegacyUnverified = failureCode === "legacy_unverified";

  // task_014 Fix-A4：state=done + placeholder_ 前缀 → 显示为"占位 MD"黄色徽章
  const isPlaceholder =
    !isLegacyUnverified &&
    state === "done" &&
    isPlaceholderExtractor(extractorType);

  // task_014 AC-4：state=failed 且携带 8 错误码 → 文案用错误码中文映射
  const failedLabel =
    state === "failed" && failureCode && isExtractionFailureLabel(failureCode)
      ? EXTRACTION_FAILURE_MESSAGES[failureCode]
      : null;

  const label = isLegacyUnverified
    ? "旧记录未校验"
    : isPlaceholder
      ? "占位 MD"
      : failedLabel ?? assetStateLabel(state);

  const renderIcon = () => {
    if (isLegacyUnverified) {
      return (
        <AlertTriangle
          size={13}
          style={{ color: "var(--color-warning, #f59e0b)" }}
          aria-hidden
        />
      );
    }
    if (isPlaceholder) {
      return (
        <AlertCircle
          size={13}
          style={{ color: "var(--color-warning, #f59e0b)" }}
          aria-hidden
        />
      );
    }
    switch (state) {
      case "done":
        return (
          <CheckCircle2
            size={13}
            style={{ color: "var(--color-success, #22c55e)" }}
            aria-hidden
          />
        );
      case "converting":
        return (
          <Loader2
            size={13}
            className="animate-spin"
            style={{ color: "var(--color-accent)" }}
            aria-hidden
          />
        );
      case "failed":
        return (
          <AlertCircle
            size={13}
            style={{ color: "var(--color-warning, #f59e0b)" }}
            aria-hidden
          />
        );
      case "offline":
        return (
          <WifiOff
            size={13}
            style={{ color: "var(--text-tertiary)" }}
            aria-hidden
          />
        );
    }
  };

  // task_011 AC-3：retrying 内部状态 + 1s 防抖。
  // - isRetrying：UI 暂停为"重试中…" + disabled。
  // - lastClickRef：1s 内重复点击直接忽略，避免连点重复触发 IPC。
  const [isRetrying, setIsRetrying] = useState(false);
  const lastClickRef = useRef<number>(0);

  const handleRetry = async (e: React.MouseEvent) => {
    e.stopPropagation();
    e.preventDefault();
    const now = Date.now();
    if (now - lastClickRef.current < RETRY_DEBOUNCE_MS) {
      return;
    }
    lastClickRef.current = now;
    if (isRetrying) return;
    setIsRetrying(true);
    try {
      await retryAssetConversion(assetId);
      onRetry?.();
    } catch (err) {
      onError?.(String(err));
    } finally {
      setIsRetrying(false);
    }
  };

  return (
    <span
      className="inline-flex items-center gap-1 text-[10px]"
      data-testid="asset-state-badge"
      data-state={state}
      data-placeholder={isPlaceholder ? "true" : "false"}
      data-failure-code={failureCode ?? ""}
      title={
        isLegacyUnverified
          ? EXTRACTION_FAILURE_MESSAGES.legacy_unverified
          : isPlaceholder
            ? "未配置该格式的提取器，仅写占位"
            : state === "failed" && (failedLabel || reason)
              ? `失败原因：${failedLabel ?? reason}`
              : label
      }
    >
      {renderIcon()}
      <span style={{ color: "var(--text-secondary)" }}>{label}</span>
      {(state === "failed" || isLegacyUnverified) ? (
        <button
          type="button"
          onClick={(e) => void handleRetry(e)}
          onMouseDown={(e) => {
            // 阻止 useDragAssets 的 mousedown 把它当成拖拽起点
            e.stopPropagation();
          }}
          disabled={isRetrying}
          className="ml-1 px-1.5 py-0.5 rounded-[var(--radius-sm)] border border-app"
          style={{
            fontSize: 10,
            color: "var(--color-accent)",
            background: "var(--surface-tertiary)",
            opacity: isRetrying ? 0.6 : 1,
            cursor: isRetrying ? "default" : "pointer",
          }}
          data-testid="asset-retry-button"
          data-retrying={isRetrying ? "true" : "false"}
          aria-label={
            isLegacyUnverified
              ? `重新转录 ${assetId}`
              : `重试转化 ${assetId}`
          }
        >
          {isRetrying
            ? "重试中…"
            : isLegacyUnverified
              ? "重新转录"
              : "重试"}
        </button>
      ) : null}
    </span>
  );
}
