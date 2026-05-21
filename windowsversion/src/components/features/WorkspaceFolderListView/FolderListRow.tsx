/**
 * 列表行（task_007 T5a）
 *
 * 单行渲染：图标 + 名称 / 项目数（右对齐）/ 修改时间（右对齐）
 * - ai_organized：图标右下贴一颗 8px Sparkles 角标
 * - 选中：背景 var(--border-active) + 文字反白
 * - hover：rgba(0,0,0,0.04)（浅）或通过 CSS hover 处理
 * - 行高 ~24px、无斑马纹、无分隔线、无阴影
 * - 本期 `draggable={false}`（拖拽留 T6）
 */
import { Folder, Sparkles } from "lucide-react";

export type RowKind = "root" | "ai_organized" | "__ROOT__" | "root_import";

export interface FolderListRowItem {
  relativePath: string;
  displayLabel: string;
  kind: RowKind;
  count: number;
  /** ISO 字符串；为空则不渲染修改时间 */
  modifiedAt: string | null;
}

interface FolderListRowProps {
  item: FolderListRowItem;
  selected: boolean;
  onSelect: () => void;
  onDoubleClick: () => void;
  onContextMenu: (e: React.MouseEvent) => void;
  /** [T5b] 行内编辑器节点；存在时替代名称文本 */
  nameEditor?: React.ReactNode;
  /** [T5b] 是否处于"已发起 IPC 等待"状态：禁用 hover、降低交互（不阻挡子元素） */
  pending?: boolean;
  /** [T5b] 是否为幽灵新建行（虚化 + 不可右键 / 双击） */
  ghost?: boolean;
}

/** ISO → "MM/DD HH:mm"；非法返回空串 */
export function formatModifiedAt(iso: string | null): string {
  if (!iso) return "";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  const hh = String(d.getHours()).padStart(2, "0");
  const mi = String(d.getMinutes()).padStart(2, "0");
  return `${mm}/${dd} ${hh}:${mi}`;
}

export function FolderListRow({
  item,
  selected,
  onSelect,
  onDoubleClick,
  onContextMenu,
  nameEditor,
  pending,
  ghost,
}: FolderListRowProps) {
  const isEditing = !!nameEditor;
  const bg = selected ? "var(--border-active)" : "transparent";
  const fg = selected ? "var(--text-on-accent, #fff)" : "var(--text-primary)";
  const subFg = selected ? "var(--text-on-accent, #fff)" : "var(--text-secondary)";

  return (
    <div
      role="row"
      data-testid={`folder-row-${item.relativePath}`}
      data-kind={item.kind}
      data-selected={selected ? "true" : "false"}
      data-editing={isEditing ? "true" : "false"}
      data-pending={pending ? "true" : "false"}
      data-ghost={ghost ? "true" : "false"}
      draggable={false}
      onClick={isEditing || ghost ? undefined : onSelect}
      onDoubleClick={isEditing || ghost ? undefined : onDoubleClick}
      onContextMenu={isEditing || ghost ? (e) => e.preventDefault() : onContextMenu}
      className="flex items-center gap-2 px-2 select-none"
      style={{
        height: 24,
        background: bg,
        color: fg,
        cursor: isEditing ? "text" : ghost ? "default" : "pointer",
        opacity: pending ? 0.7 : 1,
      }}
      onMouseEnter={(e) => {
        if (!selected) {
          (e.currentTarget as HTMLDivElement).style.background =
            "var(--row-hover-bg, rgba(0,0,0,0.04))";
        }
      }}
      onMouseLeave={(e) => {
        if (!selected) {
          (e.currentTarget as HTMLDivElement).style.background = "transparent";
        }
      }}
    >
      {/* 名称列：弹性宽 + 16px 文件夹图标（ai_organized 加 Sparkles 角标） */}
      <div className="flex-1 min-w-0 flex items-center gap-2">
        <span className="relative inline-flex shrink-0" style={{ width: 16, height: 16 }}>
          <Folder size={16} aria-hidden style={{ color: subFg }} />
          {item.kind === "ai_organized" ? (
            <Sparkles
              data-testid={`folder-row-${item.relativePath}-sparkle`}
              size={8}
              aria-label="AI 归类"
              style={{
                position: "absolute",
                right: -2,
                bottom: -2,
                color: "var(--accent-emphasis, #8b5cf6)",
              }}
            />
          ) : null}
        </span>
        {nameEditor ? (
          nameEditor
        ) : (
          <span className="text-[13px] truncate" title={item.displayLabel}>
            {item.displayLabel}
          </span>
        )}
      </div>

      {/* 项目数列：右对齐，固定宽 */}
      <div
        className="text-[12px] tabular-nums text-right shrink-0"
        style={{ width: 56, color: subFg }}
      >
        {item.count}
      </div>

      {/* 修改时间列：右对齐，固定宽 */}
      <div
        className="text-[12px] tabular-nums text-right shrink-0"
        style={{ width: 92, color: subFg }}
      >
        {formatModifiedAt(item.modifiedAt)}
      </div>
    </div>
  );
}
