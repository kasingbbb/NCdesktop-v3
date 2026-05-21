import { useCallback, useMemo, useRef, useState } from "react";
import { useTimelineStore, useAssetStore } from "../../../stores";
import { useUIStore } from "../../../stores/uiStore";
import { KeyframeGroup } from "./KeyframeGroup";
import { groupKeyframesByTime } from "./keyframe-grouping";
import { KeyframeConnector } from "./KeyframeConnector";
import { KeyframeContextMenu } from "./KeyframeContextMenu";
import { useKeyframeDrag } from "../../../hooks/useKeyframeDrag";
import { useKeyframeDrop } from "../../../hooks/useKeyframeDrop";
import type { Keyframe, Asset } from "../../../types";

interface KeyframeTrackProps {
  containerWidth: number;
  waveformTop: number;
  onKeyframeClick?: (keyframe: Keyframe) => void;
}

const TRACK_HEIGHT = 72;

export function KeyframeTrack({
  containerWidth,
  waveformTop,
  onKeyframeClick,
}: KeyframeTrackProps) {
  const trackRef = useRef<HTMLDivElement>(null);
  const keyframes = useTimelineStore((s) => s.keyframes);
  const viewport = useTimelineStore((s) => s.viewport);
  const timeline = useTimelineStore((s) => s.timeline);
  const deleteKeyframe = useTimelineStore((s) => s.deleteKeyframe);
  const assets = useAssetStore((s) => s.assets);

  const highlightedKeyframeId = useUIStore((s) => s.magicMoment.highlightedKeyframeId);
  const activeKeyframeIdFromMM = useUIStore((s) => s.magicMoment.activeKeyframeId);

  const [contextMenu, setContextMenu] = useState<{
    keyframe: Keyframe;
    position: { x: number; y: number };
  } | null>(null);
  const [localActiveKfId, setLocalActiveKfId] = useState<string | null>(null);

  const activeKeyframeId = activeKeyframeIdFromMM ?? localActiveKfId;

  const assetMap = useMemo(() => {
    const map = new Map<string, Asset>();
    for (const a of assets) {
      map.set(a.id, a);
    }
    return map;
  }, [assets]);

  const groups = useMemo(
    () => groupKeyframesByTime(keyframes),
    [keyframes]
  );

  const timeToX = useCallback(
    (time: number): number => {
      const { startTime, endTime } = viewport;
      const ratio = (time - startTime) / (endTime - startTime);
      return ratio * containerWidth;
    },
    [viewport, containerWidth]
  );

  useKeyframeDrag({
    containerRef: trackRef,
    onDragEnd: () => {},
  });

  const { handleDragOver, handleDrop } = useKeyframeDrop({
    containerRef: trackRef,
    timelineId: timeline?.id ?? null,
  });

  const handleKeyframeClick = useCallback(
    (kf: Keyframe) => {
      setLocalActiveKfId(kf.id);
      onKeyframeClick?.(kf);
    },
    [onKeyframeClick]
  );

  const handleContextMenu = useCallback(
    (e: React.MouseEvent, kf: Keyframe) => {
      setContextMenu({ keyframe: kf, position: { x: e.clientX, y: e.clientY } });
    },
    []
  );

  return (
    <div
      ref={trackRef}
      className="relative w-full"
      style={{ height: TRACK_HEIGHT }}
      onDragOver={handleDragOver}
      onDrop={handleDrop}
    >
      {groups.map((group, idx) => {
        const firstKf = group[0];
        const x = timeToX(firstKf.anchorTime);

        return (
          <div
            key={`group-${idx}`}
            className="absolute bottom-0 flex flex-col items-center"
            style={{
              left: x,
              transform: "translateX(-50%)",
            }}
          >
            <KeyframeGroup
              keyframes={group}
              assets={assetMap}
              activeKeyframeId={activeKeyframeId}
              highlightedKeyframeId={highlightedKeyframeId}
              onKeyframeClick={handleKeyframeClick}
              onKeyframeContextMenu={handleContextMenu}
            />

            <KeyframeConnector
              thumbCenterX={0}
              thumbBottomY={TRACK_HEIGHT}
              waveformY={waveformTop}
            />
          </div>
        );
      })}

      {contextMenu && (
        <KeyframeContextMenu
          keyframe={contextMenu.keyframe}
          position={contextMenu.position}
          onClose={() => setContextMenu(null)}
          onViewDetail={(kf) => onKeyframeClick?.(kf)}
          onEditNote={() => {}}
          onUnanchor={() => {}}
          onDelete={(kf) => deleteKeyframe(kf.id)}
        />
      )}
    </div>
  );
}
