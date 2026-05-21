/**
 * 右键菜单（task_007 T5a）
 *
 * 3 kind 形态（PRD §3 P0）：
 *  - root：           「重命名」/「移到废纸篓」/—/「在文件资源管理器中显示」
 *  - ai_organized：   「重命名」「移到废纸篓」灰显 + tooltip「AI 归类目录受保护」，仅「在文件资源管理器中显示」可点
 *  - __ROOT__：       仅「在文件资源管理器中显示」（重命名 / 删除**不显示**，非灰显）
 *  - blank（空白处）：「新建文件夹」
 *
 * ⚠️ 红线（ADR-007）：菜单项 disabled 仅视觉；调用方 handler 入口仍须做
 *    `if (selection.kind !== 'root') return;` 拦截。
 */
import { useEffect, useRef } from "react";
import type { RowKind } from "./FolderListRow";

export type ContextMenuTarget =
  | { kind: "row"; rowKind: RowKind; relativePath: string }
  | { kind: "blank" };

interface FolderContextMenuProps {
  x: number;
  y: number;
  target: ContextMenuTarget;
  onClose: () => void;
  onRename: () => void;
  onDelete: () => void;
  onReveal: () => void;
  onCreate: () => void;
}

export function FolderContextMenu({
  x,
  y,
  target,
  onClose,
  onRename,
  onDelete,
  onReveal,
  onCreate,
}: FolderContextMenuProps) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handleMouseDown(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    }
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("mousedown", handleMouseDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("mousedown", handleMouseDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [onClose]);

  const menuStyle: React.CSSProperties = {
    position: "fixed",
    left: x,
    top: y,
    minWidth: 180,
    background: "var(--surface-primary)",
    border: "1px solid var(--border-primary)",
    borderRadius: "var(--radius-md, 6px)",
    boxShadow: "var(--shadow-md, 0 4px 12px rgba(0,0,0,0.15))",
    padding: 4,
    zIndex: 9999,
  };

  // 空白菜单
  if (target.kind === "blank") {
    return (
      <div ref={ref} role="menu" data-testid="folder-ctx-blank" style={menuStyle}>
        <MenuItem
          testId="ctx-create"
          label="新建文件夹"
          onClick={() => {
            onCreate();
            onClose();
          }}
        />
      </div>
    );
  }

  const { rowKind } = target;

  // __ROOT__：仅在文件资源管理器中显示（不渲染重命名/删除条目）
  if (rowKind === "__ROOT__") {
    return (
      <div ref={ref} role="menu" data-testid="folder-ctx-root-sentinel" style={menuStyle}>
        <MenuItem
          testId="ctx-reveal"
          label="在文件资源管理器中显示"
          onClick={() => {
            onReveal();
            onClose();
          }}
        />
      </div>
    );
  }

  // root / ai_organized / root_import 共用结构，但权限不同
  const writable = rowKind === "root";
  const protectedTip =
    rowKind === "ai_organized"
      ? "AI 归类目录受保护"
      : rowKind === "root_import"
        ? "导入副本受保护"
        : "";

  return (
    <div
      ref={ref}
      role="menu"
      data-testid={`folder-ctx-${rowKind}`}
      style={menuStyle}
    >
      <MenuItem
        testId="ctx-rename"
        label="重命名"
        disabled={!writable}
        title={writable ? undefined : protectedTip}
        onClick={() => {
          if (!writable) return; // 双保险：disabled 视觉 + 入口判定
          onRename();
          onClose();
        }}
      />
      <MenuItem
        testId="ctx-delete"
        label="移到废纸篓"
        disabled={!writable}
        title={writable ? undefined : protectedTip}
        onClick={() => {
          if (!writable) return;
          onDelete();
          onClose();
        }}
      />
      <MenuDivider />
      <MenuItem
        testId="ctx-reveal"
        label="在文件资源管理器中显示"
        onClick={() => {
          onReveal();
          onClose();
        }}
      />
    </div>
  );
}

function MenuItem({
  label,
  onClick,
  disabled,
  title,
  testId,
}: {
  label: string;
  onClick: () => void;
  disabled?: boolean;
  title?: string;
  testId?: string;
}) {
  return (
    <button
      type="button"
      role="menuitem"
      data-testid={testId}
      data-disabled={disabled ? "true" : "false"}
      aria-disabled={disabled ? "true" : "false"}
      title={title}
      onClick={onClick}
      className="w-full text-left text-[13px] px-2 py-1 rounded-[var(--radius-sm)] hover:opacity-90"
      style={{
        color: disabled ? "var(--text-tertiary)" : "var(--text-primary)",
        opacity: disabled ? 0.45 : 1,
        cursor: disabled ? "not-allowed" : "pointer",
        background: "transparent",
        border: "none",
      }}
    >
      {label}
    </button>
  );
}

function MenuDivider() {
  return (
    <div
      role="separator"
      style={{
        height: 1,
        margin: "4px 6px",
        background: "var(--border-primary)",
      }}
    />
  );
}
