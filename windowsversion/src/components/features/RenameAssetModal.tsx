/**
 * task_011 AC-6：重命名 Modal（替代原生 window.prompt / window.alert）。
 *
 * - 输入框 + 字节计数（UTF-8）≤ 200 字节，超出红色提示并禁用确认按钮。
 * - 顶部一行 sanitize 规则简介（与后端 ADR-007 双写保持一致：去除路径分隔符 / 控制字符 / 首尾空白）。
 * - 失败统一通过 toast 上抛（onError 回调），不再 window.alert。
 *
 * 父组件（AssetListView）负责：
 * - 打开 Modal（按 Enter / F2 或右键菜单触发）。
 * - 提交时 await `renameAsset(assetId, newName)`；成功后 fetchAssets 刷新。
 */
import { useEffect, useRef, useState } from "react";

const MAX_NAME_BYTES = 200;
const PATH_SEP_REGEX = /[\\/]/;
const CTRL_CHARS_REGEX = /[\x00-\x1f\x7f]/;

function utf8ByteLength(s: string): number {
  // TextEncoder 在 jsdom / 浏览器 / Node 18+ 通用
  return new TextEncoder().encode(s).length;
}

function validateName(raw: string): { ok: boolean; error: string | null } {
  const trimmed = raw.trim();
  if (trimmed.length === 0) return { ok: false, error: "名称不能为空" };
  if (PATH_SEP_REGEX.test(trimmed)) return { ok: false, error: "名称不能包含 / 或 \\ 路径分隔符" };
  if (CTRL_CHARS_REGEX.test(trimmed)) return { ok: false, error: "名称不能包含控制字符" };
  if (utf8ByteLength(trimmed) > MAX_NAME_BYTES) return { ok: false, error: `名称超过 ${MAX_NAME_BYTES} 字节上限` };
  return { ok: true, error: null };
}

interface RenameAssetModalProps {
  /** 当前展示名（初始填入输入框） */
  initialName: string;
  /** 提交按钮 disabled 控制（busy=true 时显示提交中…） */
  busy?: boolean;
  onCancel: () => void;
  /** 用户提交，返回 trim 后的新名（已通过本地校验） */
  onSubmit: (newName: string) => void;
}

export function RenameAssetModal({
  initialName,
  busy,
  onCancel,
  onSubmit,
}: RenameAssetModalProps) {
  const [value, setValue] = useState(initialName);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);

  // ESC 关闭
  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        onCancel();
      }
    }
    document.addEventListener("keydown", handleKey);
    return () => document.removeEventListener("keydown", handleKey);
  }, [onCancel]);

  const byteLen = utf8ByteLength(value);
  const validation = validateName(value);
  const sameAsInitial = value.trim() === initialName.trim();
  const canSubmit = validation.ok && !sameAsInitial && !busy;

  function handleSubmit(e?: React.FormEvent) {
    e?.preventDefault();
    if (!canSubmit) return;
    onSubmit(value.trim());
  }

  const overLimit = byteLen > MAX_NAME_BYTES;

  return (
    <div
      data-testid="rename-asset-modal"
      role="dialog"
      aria-modal="true"
      aria-label="重命名资产"
      style={{
        position: "fixed",
        inset: 0,
        background: "rgba(0,0,0,0.35)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 9998,
      }}
      onClick={onCancel}
    >
      <form
        onSubmit={handleSubmit}
        onClick={(e) => e.stopPropagation()}
        style={{
          minWidth: 380,
          maxWidth: 520,
          background: "var(--surface-primary)",
          border: "1px solid var(--border-primary)",
          borderRadius: 8,
          boxShadow: "0 8px 32px rgba(0,0,0,0.2)",
          padding: 20,
          color: "var(--text-primary)",
        }}
      >
        <div style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>重命名</div>
        <div
          style={{
            fontSize: 11,
            color: "var(--text-tertiary)",
            marginBottom: 10,
            lineHeight: 1.4,
          }}
        >
          名称将同步到 markdown 衍生件；不能包含 / \ 或控制字符，上限 200 字节（UTF-8）。
        </div>
        <input
          ref={inputRef}
          data-testid="rename-asset-input"
          type="text"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          disabled={busy}
          style={{
            width: "100%",
            padding: "8px 10px",
            fontSize: 13,
            border: `1px solid ${overLimit ? "#ef4444" : "var(--border-primary)"}`,
            borderRadius: 6,
            background: "var(--surface-primary)",
            color: "var(--text-primary)",
            outline: "none",
          }}
        />
        <div
          style={{
            marginTop: 6,
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            fontSize: 11,
          }}
        >
          <span
            data-testid="rename-asset-validation"
            style={{ color: validation.ok ? "var(--text-tertiary)" : "#ef4444" }}
          >
            {validation.ok ? " " : validation.error}
          </span>
          <span
            data-testid="rename-asset-byte-count"
            style={{
              color: overLimit ? "#ef4444" : "var(--text-tertiary)",
              fontVariantNumeric: "tabular-nums",
            }}
          >
            {byteLen} / {MAX_NAME_BYTES} 字节
          </span>
        </div>
        <div
          style={{
            marginTop: 16,
            display: "flex",
            justifyContent: "flex-end",
            gap: 8,
          }}
        >
          <button
            type="button"
            data-testid="rename-asset-cancel"
            disabled={busy}
            onClick={onCancel}
            className="text-[13px] px-3 py-1 rounded-[var(--radius-sm)]"
            style={{
              border: "1px solid var(--border-primary)",
              background: "var(--surface-primary)",
              color: "var(--text-primary)",
            }}
          >
            取消
          </button>
          <button
            type="submit"
            data-testid="rename-asset-confirm"
            disabled={!canSubmit}
            className="text-[13px] px-3 py-1 rounded-[var(--radius-sm)]"
            style={{
              background: "var(--color-accent, #2563eb)",
              color: "#fff",
              border: "none",
              opacity: canSubmit ? 1 : 0.5,
              cursor: canSubmit ? "pointer" : "not-allowed",
            }}
          >
            {busy ? "提交中…" : "确认"}
          </button>
        </div>
      </form>
    </div>
  );
}

export const __test__ = { validateName, utf8ByteLength, MAX_NAME_BYTES };
