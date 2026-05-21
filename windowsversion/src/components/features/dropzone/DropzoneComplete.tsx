import { Check } from "lucide-react";

export function DropzoneComplete() {
  return (
    <div
      className="flex items-center justify-center relative overflow-hidden"
      style={{
        width: 68,
        height: 68,
        borderRadius: "var(--radius-2xl)",
        background: "#1e2940",
        boxShadow: "0 0 16px rgba(34,197,94,0.25)",
        border: "2px solid #22c55e",
        animation: "modal-in var(--duration-fast) var(--ease-out-expo)",
      }}
    >
      <Check size={32} strokeWidth={3} style={{ color: "#22c55e" }} className="animate-pulse" />
    </div>
  );
}
