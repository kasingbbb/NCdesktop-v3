import type { Keyframe, Asset } from "../../../types";
import { KeyframeThumb } from "./KeyframeThumb";

interface KeyframeGroupProps {
  keyframes: Keyframe[];
  assets: Map<string, Asset>;
  activeKeyframeId: string | null;
  highlightedKeyframeId: string | null;
  onKeyframeClick: (keyframe: Keyframe) => void;
  onKeyframeContextMenu: (e: React.MouseEvent, keyframe: Keyframe) => void;
}

/** 同一时间段（±2s）的关键帧聚合显示 */
export function KeyframeGroup({
  keyframes,
  assets,
  activeKeyframeId,
  highlightedKeyframeId,
  onKeyframeClick,
  onKeyframeContextMenu,
}: KeyframeGroupProps) {
  return (
    <div className="flex items-end gap-[var(--space-1)]">
      {keyframes.map((kf) => (
        <KeyframeThumb
          key={kf.id}
          keyframe={kf}
          asset={assets.get(kf.assetId)}
          isActive={kf.id === activeKeyframeId}
          isHighlighted={kf.id === highlightedKeyframeId}
          onClick={onKeyframeClick}
          onContextMenu={onKeyframeContextMenu}
        />
      ))}
    </div>
  );
}
