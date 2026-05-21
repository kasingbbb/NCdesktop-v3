interface ResizeHandleProps {
  onMouseDown: (e: React.MouseEvent) => void;
  isResizing: boolean;
}

export function ResizeHandle({ onMouseDown, isResizing }: ResizeHandleProps) {
  return (
    <div
      className="w-[4px] shrink-0 cursor-col-resize relative group"
      onMouseDown={onMouseDown}
      role="separator"
      aria-orientation="vertical"
      tabIndex={0}
    >
      <div
        className="absolute inset-y-0 left-1/2 -translate-x-1/2 w-px transition-colors"
        style={{
          background: isResizing
            ? "var(--text-secondary)"
            : "var(--border-primary)",
        }}
      />
      <div
        className="absolute inset-y-0 -left-[4px] -right-[4px] opacity-0 group-hover:opacity-100 transition-opacity"
        style={{ background: "rgba(31, 69, 110, 0.06)" }}
      />
    </div>
  );
}
