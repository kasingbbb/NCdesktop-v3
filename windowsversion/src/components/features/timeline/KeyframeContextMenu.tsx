import { Eye, Edit3, Unlink, Trash2 } from "lucide-react";
import type { Keyframe } from "../../../types";

interface KeyframeContextMenuProps {
  keyframe: Keyframe;
  position: { x: number; y: number };
  onClose: () => void;
  onViewDetail: (keyframe: Keyframe) => void;
  onEditNote: (keyframe: Keyframe) => void;
  onUnanchor: (keyframe: Keyframe) => void;
  onDelete: (keyframe: Keyframe) => void;
}

const MENU_ITEMS = [
  { key: "detail", icon: Eye, label: "查看详情", action: "onViewDetail" },
  { key: "note", icon: Edit3, label: "编辑备注", action: "onEditNote" },
  { key: "unanchor", icon: Unlink, label: "取消锚定", action: "onUnanchor" },
  { key: "delete", icon: Trash2, label: "删除", action: "onDelete", danger: true },
] as const;

export function KeyframeContextMenu({
  keyframe,
  position,
  onClose,
  onViewDetail,
  onEditNote,
  onUnanchor,
  onDelete,
}: KeyframeContextMenuProps) {
  const handlers: Record<string, (kf: Keyframe) => void> = {
    onViewDetail,
    onEditNote,
    onUnanchor,
    onDelete,
  };

  return (
    <>
      {/* 背景遮罩 */}
      <div className="fixed inset-0 z-40" onClick={onClose} />

      {/* 菜单 */}
      <div
        className="glass-popover fixed z-50 py-[var(--space-1)] min-w-[160px]"
        style={{
          left: position.x,
          top: position.y,
          animation: "modal-in var(--duration-fast) var(--ease-out-expo)",
        }}
      >
        {MENU_ITEMS.map((item) => {
          const Icon = item.icon;
          return (
            <button
              key={item.key}
              className="w-full flex items-center gap-[var(--space-2)] px-[var(--space-3)] py-[var(--space-2)] text-[var(--text-xs)] hover:bg-[var(--surface-tertiary)] transition-colors"
              style={{
                color: "danger" in item && item.danger
                  ? "var(--color-error)"
                  : "var(--text-primary)",
              }}
              onClick={() => {
                handlers[item.action](keyframe);
                onClose();
              }}
            >
              <Icon size={14} />
              <span>{item.label}</span>
            </button>
          );
        })}
      </div>
    </>
  );
}
