import type { SelectionRect } from "../../../hooks/useRubberBandSelect";

interface SelectionOverlayProps {
  rect: SelectionRect | null;
}

export function SelectionOverlay({ rect }: SelectionOverlayProps) {
  if (!rect || (rect.width < 4 && rect.height < 4)) return null;

  return (
    <div
      className="pointer-events-none absolute z-20"
      style={{
        left: rect.x,
        top: rect.y,
        width: rect.width,
        height: rect.height,
        background: "rgba(31,69,110,0.08)",
        border: "1.5px solid var(--brand-navy)",
        borderRadius: "var(--radius-sm)",
      }}
    />
  );
}
