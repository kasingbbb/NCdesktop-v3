import { useDropzoneStore } from "../../../stores/dropzoneStore";
import { Sparkles } from "lucide-react";

interface DropzoneIdleProps {
  isAttract?: boolean;
}

export function DropzoneIdle({ isAttract = false }: DropzoneIdleProps) {
  const toggleExpand = useDropzoneStore((s) => s.toggleExpand);

  return (
    <button
      onClick={toggleExpand}
      className="flex flex-col items-center justify-center cursor-pointer relative overflow-hidden group"
      style={{
        width: "clamp(64px, 18vw, 88px)",
        height: "clamp(64px, 18vw, 88px)",
        borderRadius: 28,
        boxShadow: isAttract
          ? "0 0 20px rgba(59,130,246,0.3)"
          : "0 2px 8px rgba(0,0,0,0.3)",
        border: isAttract
          ? "2px solid #3b82f6"
          : "1px solid #2d3a50",
        background: isAttract
          ? "linear-gradient(135deg, #1e3a5f 0%, #1a2233 100%)"
          : "#1e2940",
        transition: "all var(--duration-normal) var(--ease-spring)",
      }}
    >
      {/* 吸引状态动效脉冲 */}
      {isAttract && (
        <div
          className="absolute inset-0 rounded-[28px]"
          style={{
            border: "2px solid rgba(59,130,246,0.5)",
            animation: "magic-pulse 1.5s infinite var(--ease-out-expo)",
          }}
        />
      )}

      <div
        className="flex flex-col items-center justify-center gap-[2px] z-10 group-hover:scale-105"
        style={{ transition: "transform var(--duration-normal)" }}
      >
        <Sparkles
          size={isAttract ? 28 : 22}
          style={{
            color: isAttract ? "#93c5fd" : "#3b82f6",
            transition: "all var(--duration-normal)",
          }}
          strokeWidth={2}
        />
        {!isAttract && (
          <span
            className="text-[9px] font-bold tracking-widest uppercase"
            style={{ color: "rgba(255,255,255,0.5)" }}
          >
            Drop
          </span>
        )}
      </div>
    </button>
  );
}
