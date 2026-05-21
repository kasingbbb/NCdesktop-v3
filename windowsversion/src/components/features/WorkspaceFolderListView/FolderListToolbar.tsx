/**
 * 工作区文件夹列表工具栏（task_007 T5a）
 *
 * 36px 高，3 按钮：
 *  - 「+ 新建文件夹」：永激活
 *  - 「重命名」：仅当当前选中行 kind === 'root' 时激活
 *  - 「移到废纸篓」：仅当当前选中行 kind === 'root' 时激活
 *
 * ⚠️ 红线（ADR-007）：disabled 只是视觉；handler 入口的权限判定不依赖此属性，
 *    由 `WorkspaceFolderListView` 内的 handler 首行 `if (selection.kind !== 'root') return;` 拦截。
 */
interface FolderListToolbarProps {
  /** 当前选中行的 kind；`null` = 没选中 */
  selectedKind: "root" | "ai_organized" | "__ROOT__" | "root_import" | null;
  onCreate: () => void;
  onRename: () => void;
  onDelete: () => void;
}

export function FolderListToolbar({
  selectedKind,
  onCreate,
  onRename,
  onDelete,
}: FolderListToolbarProps) {
  const writeEnabled = selectedKind === "root";

  return (
    <div
      data-testid="folder-list-toolbar"
      className="shrink-0 flex items-center gap-2 px-2"
      style={{
        height: 36,
        borderBottom: "1px solid var(--border-primary)",
        background: "var(--surface-secondary)",
      }}
    >
      <button
        type="button"
        data-testid="toolbar-create"
        className="text-[12px] px-2 py-1 rounded-[var(--radius-sm)] hover:opacity-80"
        style={{
          color: "var(--text-primary)",
          background: "var(--surface-primary)",
          border: "1px solid var(--border-primary)",
        }}
        onClick={onCreate}
      >
        + 新建文件夹
      </button>
      <button
        type="button"
        data-testid="toolbar-rename"
        className="text-[12px] px-2 py-1 rounded-[var(--radius-sm)] hover:opacity-80 disabled:opacity-40 disabled:cursor-not-allowed"
        style={{
          color: "var(--text-secondary)",
        }}
        disabled={!writeEnabled}
        onClick={onRename}
      >
        重命名
      </button>
      <button
        type="button"
        data-testid="toolbar-delete"
        className="text-[12px] px-2 py-1 rounded-[var(--radius-sm)] hover:opacity-80 disabled:opacity-40 disabled:cursor-not-allowed"
        style={{
          color: "var(--text-secondary)",
        }}
        disabled={!writeEnabled}
        onClick={onDelete}
      >
        移到废纸篓
      </button>
    </div>
  );
}
