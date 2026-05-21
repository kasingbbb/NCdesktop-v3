import { Download } from "lucide-react";

export function DropzoneAttract() {
  return (
    <div
      className="flex flex-col items-center justify-center relative overflow-hidden"
      style={{
        width: 68,
        height: 68,
        background: "linear-gradient(135deg, #1e3a5f 0%, #1a2233 100%)",
        border: "2px dashed #3b82f6",
        borderRadius: "var(--radius-md)",
        boxShadow: "0 0 20px rgba(59,130,246,0.3)",
        animation: "magic-pulse 1.5s infinite ease-in-out",
      }}
    >
      <Download size={26} strokeWidth={2.5} className="animate-bounce z-10" style={{ color: "#93c5fd" }} />
    </div>
  );
}
