import { useCallback, useState } from "react";
import type { Keyframe, Asset } from "../../../types";

interface KeyframeThumbProps {
  keyframe: Keyframe;
  asset: Asset | undefined;
  isActive?: boolean;
  /** "随音现图"高亮状态 */
  isHighlighted?: boolean;
  onClick?: (keyframe: Keyframe) => void;
  onContextMenu?: (e: React.MouseEvent, keyframe: Keyframe) => void;
  onMouseDown?: (e: React.MouseEvent, keyframe: Keyframe) => void;
}

const DEFAULT_SIZE = 48;
const HOVER_SIZE = 120;
const HIGHLIGHT_SIZE = 80;

export function KeyframeThumb({
  keyframe,
  asset,
  isActive = false,
  isHighlighted = false,
  onClick,
  onContextMenu,
  onMouseDown,
}: KeyframeThumbProps) {
  const [isHovered, setIsHovered] = useState(false);

  const size = isHovered
    ? HOVER_SIZE
    : isHighlighted
      ? HIGHLIGHT_SIZE
      : DEFAULT_SIZE;

  const handleClick = useCallback(() => {
    onClick?.(keyframe);
  }, [keyframe, onClick]);

  const handleContextMenu = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      onContextMenu?.(e, keyframe);
    },
    [keyframe, onContextMenu]
  );

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      onMouseDown?.(e, keyframe);
    },
    [keyframe, onMouseDown]
  );

  const thumbnailSrc = asset?.filePath ?? "";
  const displayName = asset?.name ?? "未知素材";

  return (
    <div
      className="relative flex flex-col items-center cursor-pointer select-none"
      style={{ zIndex: isHovered ? 30 : isHighlighted ? 25 : isActive ? 20 : 10 }}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      onClick={handleClick}
      onContextMenu={handleContextMenu}
      onMouseDown={handleMouseDown}
    >
      {/* 缩略图 — 宝丽来风格：白色实体边框 + 清晰微投影 */}
      <div
        className="rounded-[var(--radius-md)] overflow-hidden flex-shrink-0"
        style={{
          width: size,
          height: size,
          transition: "all var(--duration-fast) var(--ease-out-expo)",
          boxShadow: "var(--shadow-sm)",
          border: isHighlighted
            ? "2px solid var(--border-active)"
            : isActive
              ? "2px solid #52525b"
              : "2px solid white",
          animation: isHighlighted ? "magic-pulse 1.5s ease-in-out infinite" : "none",
        }}
      >
        {thumbnailSrc ? (
          <img
            src={thumbnailSrc}
            alt={displayName}
            className="w-full h-full object-cover"
            draggable={false}
          />
        ) : (
          <div
            className="w-full h-full flex items-center justify-center"
            style={{
              backgroundColor: "var(--surface-secondary)",
              color: "var(--text-tertiary)",
              fontSize: isHovered ? 14 : 10,
            }}
          >
            {asset?.type === "scan_text" ? "T" : "?"}
          </div>
        )}
      </div>

      {/* hover 时显示名称 */}
      {isHovered && (
        <div
          className="absolute -bottom-6 whitespace-nowrap text-center px-[var(--space-1)] py-0.5 rounded-[var(--radius-sm)]"
          style={{
            backgroundColor: "var(--surface-elevated)",
            border: "1px solid var(--border-primary)",
            boxShadow: "var(--shadow-sm)",
            fontSize: 10,
            color: "var(--text-secondary)",
            maxWidth: 140,
            overflow: "hidden",
            textOverflow: "ellipsis",
          }}
        >
          {displayName}
        </div>
      )}
    </div>
  );
}
