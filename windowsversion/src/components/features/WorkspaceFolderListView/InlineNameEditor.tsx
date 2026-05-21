/**
 * 行内文件夹名编辑器（task_008 T5b）
 *
 * 用法：在 FolderListRow 中条件渲染 —— mode 为 'creating' / 'renaming' 时
 * 替换原文字 label 为本组件的受控输入框。
 *
 * 红线：
 *  - Enter 提交、Esc 取消、blur 提交（与 Finder 行为对齐；AC-2/AC-3）。
 *  - 提交失败保留编辑态 + 红框（由父组件控制 `error` prop）。
 *  - 校验失败时即时红框（`invalid` prop 由父组件根据 `validateFolderNameSync` 推导）。
 *  - 切走二次确认 modal 由父组件挂载；编辑器自身不处理。
 */
import { useEffect, useRef } from "react";

interface InlineNameEditorProps {
  value: string;
  onChange: (next: string) => void;
  onCommit: () => void;
  onCancel: () => void;
  /** 校验失败 / IPC 失败时显示红框；title 显示 tooltip */
  invalid?: boolean;
  /** 与 invalid 任意一项为真 → 红框（保留编辑态） */
  error?: string | null;
  /** 调试/测试用 testid 前缀 */
  testId?: string;
  /** mount 时是否自动 select all（默认 true） */
  selectAllOnMount?: boolean;
}

export function InlineNameEditor({
  value,
  onChange,
  onCommit,
  onCancel,
  invalid,
  error,
  testId = "inline-name-editor",
  selectAllOnMount = true,
}: InlineNameEditorProps) {
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const el = inputRef.current;
    if (!el) return;
    el.focus();
    if (selectAllOnMount) el.select();
    // 仅在 mount 时 focus/select
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const showError = invalid || !!error;

  return (
    <input
      ref={inputRef}
      type="text"
      data-testid={testId}
      data-error={showError ? "true" : "false"}
      value={value}
      title={error ?? undefined}
      onChange={(e) => onChange(e.target.value)}
      onClick={(e) => e.stopPropagation()}
      onDoubleClick={(e) => e.stopPropagation()}
      onMouseDown={(e) => e.stopPropagation()}
      onKeyDown={(e) => {
        // 内部捕获 Enter / Esc，避免冒泡到外层容器的 keydown
        if (e.key === "Enter") {
          e.preventDefault();
          e.stopPropagation();
          onCommit();
          return;
        }
        if (e.key === "Escape") {
          e.preventDefault();
          e.stopPropagation();
          onCancel();
          return;
        }
        // 其他键阻止冒泡，避免触发外层 ArrowDown / ⌘⌫ / ⌘⇧N
        e.stopPropagation();
      }}
      onBlur={() => {
        onCommit();
      }}
      className="text-[13px] px-1 outline-none"
      style={{
        flex: 1,
        minWidth: 0,
        border: showError
          ? "1px solid var(--accent-danger, #ef4444)"
          : "1px solid var(--border-active, #3b82f6)",
        borderRadius: 3,
        background: "var(--surface-primary, #fff)",
        color: "var(--text-primary)",
        height: 20,
      }}
    />
  );
}
