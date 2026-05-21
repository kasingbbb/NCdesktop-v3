/**
 * 删除二次确认 Modal（task_008 T5b · F3）
 *
 * 行为（AC-4）：
 *  - N === 0：文案「删除文件夹『xxx』？」
 *  - N > 0：文案「该文件夹包含 N 个素材，一同移到废纸篓？」
 *  - dirty 重弹：调用方在 catch `E_FOLDER_DIRTY` 时用 `details.now` 重弹，
 *    文案前缀为「内容已变化（原 N，现 details.now），请重新确认？」并把
 *    `expectedCount` 替换为 `details.now`。
 */
interface DeleteConfirmModalProps {
  /** 目标文件夹显示名（不是 relativePath） */
  displayName: string;
  /** 用户当前看到的素材数 N（来自前端聚合） */
  expectedCount: number;
  /** 若 > 0 则表示这是一次 dirty 重弹；显示「内容已变化（原 N，现 X）」前缀 */
  dirtyPrev?: number | null;
  onCancel: () => void;
  onConfirm: () => void;
  /** [T5b] 提交中视觉（按钮 disabled） */
  busy?: boolean;
}

export function DeleteConfirmModal({
  displayName,
  expectedCount,
  dirtyPrev,
  onCancel,
  onConfirm,
  busy,
}: DeleteConfirmModalProps) {
  const isDirty = typeof dirtyPrev === "number";
  const body = isDirty
    ? `内容已变化（原 ${dirtyPrev}，现 ${expectedCount}），请重新确认？`
    : expectedCount > 0
      ? `该文件夹包含 ${expectedCount} 个素材，一同移到废纸篓？`
      : `删除文件夹「${displayName}」？`;

  return (
    <div
      data-testid="folder-delete-modal"
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
      onClick={onCancel}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          minWidth: 360,
          maxWidth: 480,
          background: "var(--surface-primary)",
          border: "1px solid var(--border-primary)",
          borderRadius: 8,
          boxShadow: "0 8px 32px rgba(0,0,0,0.2)",
          padding: 20,
          color: "var(--text-primary)",
        }}
      >
        <div style={{ fontSize: 14, marginBottom: 16, lineHeight: 1.5 }}>{body}</div>
        <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
          <button
            type="button"
            data-testid="folder-delete-cancel"
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
            type="button"
            data-testid="folder-delete-confirm"
            disabled={busy}
            onClick={onConfirm}
            className="text-[13px] px-3 py-1 rounded-[var(--radius-sm)]"
            style={{
              background: "var(--accent-danger, #ef4444)",
              color: "#fff",
              border: "none",
              opacity: busy ? 0.6 : 1,
            }}
          >
            移到废纸篓
          </button>
        </div>
      </div>
    </div>
  );
}
