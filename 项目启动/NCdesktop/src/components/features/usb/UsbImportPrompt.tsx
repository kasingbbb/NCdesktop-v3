/**
 * U盘新图片导入确认弹窗。
 *
 * 由 `usbImportStore.pending` 驱动：后端检测到 Notecapt 卡上的新图片后，
 * 这里弹出「发现 N 张新图片，导入并转 MD？」。确认走现成 import_drop_paths 管线。
 * 产品决策：先弹确认再导入（非静默）。
 */
import { Usb, Sparkles, X } from "lucide-react";
import { useUsbImportStore } from "../../../stores/usbImportStore";

export function UsbImportPrompt() {
  const pending = useUsbImportStore((s) => s.pending);
  const isImporting = useUsbImportStore((s) => s.isImporting);
  const error = useUsbImportStore((s) => s.error);
  const dismiss = useUsbImportStore((s) => s.dismiss);
  const confirmImport = useUsbImportStore((s) => s.confirmImport);

  if (!pending) return null;

  const count = pending.newFiles.length;
  const previewNames = pending.newFiles.slice(0, 5).map((f) => f.name);
  const remaining = count - previewNames.length;

  return (
    <div
      data-testid="usb-import-modal"
      role="dialog"
      aria-modal="true"
      style={{
        position: "fixed",
        inset: 0,
        background: "rgba(0,0,0,0.35)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 9998,
      }}
      onClick={isImporting ? undefined : dismiss}
    >
      <div
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
        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 12 }}>
          <Usb size={18} style={{ color: "var(--accent-primary, #5b8def)" }} />
          <div style={{ fontSize: 15, fontWeight: 600 }}>
            在「{pending.deviceName}」中发现 {count} 张新图片
          </div>
        </div>

        <div style={{ fontSize: 13, lineHeight: 1.6, marginBottom: 12, color: "var(--text-secondary, inherit)" }}>
          是否导入并转换为 Markdown 材料加入工作区？
        </div>

        <ul
          style={{
            margin: "0 0 14px",
            padding: "8px 12px",
            listStyle: "none",
            background: "var(--surface-secondary, rgba(0,0,0,0.03))",
            borderRadius: 6,
            fontSize: 12,
            lineHeight: 1.7,
            maxHeight: 140,
            overflowY: "auto",
          }}
        >
          {previewNames.map((name) => (
            <li key={name} style={{ whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
              {name}
            </li>
          ))}
          {remaining > 0 && (
            <li style={{ color: "var(--text-secondary, #888)" }}>…等 {remaining} 张</li>
          )}
        </ul>

        {error && (
          <div
            data-testid="usb-import-error"
            style={{
              fontSize: 12,
              color: "var(--danger, #d4504a)",
              marginBottom: 12,
              whiteSpace: "pre-wrap",
            }}
          >
            导入失败：{error}
          </div>
        )}

        <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
          <button
            type="button"
            data-testid="usb-import-dismiss"
            disabled={isImporting}
            onClick={dismiss}
            className="text-[13px] px-3 py-1 rounded-[var(--radius-sm)]"
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: 4,
              border: "1px solid var(--border-primary)",
              background: "var(--surface-primary)",
              color: "var(--text-primary)",
              opacity: isImporting ? 0.5 : 1,
              cursor: isImporting ? "default" : "pointer",
            }}
          >
            <X size={13} /> 忽略
          </button>
          <button
            type="button"
            data-testid="usb-import-confirm"
            disabled={isImporting}
            onClick={() => void confirmImport()}
            className="text-[13px] px-3 py-1 rounded-[var(--radius-sm)]"
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: 4,
              border: "1px solid var(--accent-primary, #5b8def)",
              background: "var(--accent-primary, #5b8def)",
              color: "#fff",
              cursor: isImporting ? "default" : "pointer",
            }}
          >
            <Sparkles size={13} />
            {isImporting ? "导入中…" : `导入 ${count} 张`}
          </button>
        </div>
      </div>
    </div>
  );
}
