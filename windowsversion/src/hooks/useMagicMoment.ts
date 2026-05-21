import { useCallback, useEffect, useRef } from "react";
import { useTimelineStore } from "../stores/timelineStore";
import { useSettingsStore } from "../stores/settingsStore";
import { useUIStore } from "../stores/uiStore";
import type { Keyframe } from "../types";

interface UseMagicMomentReturn {
  /** 由图寻音：点击关键帧 → Pre-roll 回退 → 自动播放 */
  seekToKeyframe: (keyframe: Keyframe) => void;
  /** 当前被"随音现图"高亮的关键帧 ID */
  highlightedKeyframeId: string | null;
  /** 当前预览的素材 ID（内容区联动） */
  previewAssetId: string | null;
  /** 是否正在播放 Magic Moment 动画 */
  isAnimating: boolean;
}

const HIGHLIGHT_DURATION_MS = 2000;

export function useMagicMoment(): UseMagicMomentReturn {
  const keyframes = useTimelineStore((s) => s.keyframes);
  const currentTime = useTimelineStore((s) => s.playback.currentTime);
  const isPlaying = useTimelineStore((s) => s.playback.isPlaying);
  const seek = useTimelineStore((s) => s.seek);
  const play = useTimelineStore((s) => s.play);
  const preRollSeconds = useSettingsStore((s) => s.settings.preRollSeconds);
  const magicMoment = useUIStore((s) => s.magicMoment);
  const setMagicMoment = useUIStore((s) => s.setMagicMoment);

  const lastTriggeredRef = useRef<string | null>(null);
  const highlightTimerRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  // ━━━ 由图寻音（Image → Audio）━━━
  const seekToKeyframe = useCallback(
    (keyframe: Keyframe) => {
      const targetTime = Math.max(0, keyframe.anchorTime - preRollSeconds);

      setMagicMoment({
        activeKeyframeId: keyframe.id,
        previewAssetId: keyframe.assetId,
        isAnimating: true,
      });

      seek(targetTime);

      requestAnimationFrame(() => {
        play();
      });

      if (highlightTimerRef.current) {
        clearTimeout(highlightTimerRef.current);
      }
      highlightTimerRef.current = setTimeout(() => {
        setMagicMoment({ isAnimating: false });
      }, HIGHLIGHT_DURATION_MS);
    },
    [preRollSeconds, seek, play, setMagicMoment]
  );

  // ━━━ 随音现图（Audio → Image）━━━
  useEffect(() => {
    if (!isPlaying || keyframes.length === 0) return;

    const TRIGGER_THRESHOLD = 0.15;

    for (const kf of keyframes) {
      const delta = Math.abs(currentTime - kf.anchorTime);
      if (delta < TRIGGER_THRESHOLD && lastTriggeredRef.current !== kf.id) {
        lastTriggeredRef.current = kf.id;

        setMagicMoment({
          highlightedKeyframeId: kf.id,
          previewAssetId: kf.assetId,
          isAnimating: true,
        });

        if (highlightTimerRef.current) {
          clearTimeout(highlightTimerRef.current);
        }
        highlightTimerRef.current = setTimeout(() => {
          setMagicMoment({
            highlightedKeyframeId: null,
            isAnimating: false,
          });
        }, HIGHLIGHT_DURATION_MS);

        break;
      }
    }
  }, [currentTime, isPlaying, keyframes, setMagicMoment]);

  // 停止播放时清除高亮
  useEffect(() => {
    if (!isPlaying) {
      lastTriggeredRef.current = null;
    }
  }, [isPlaying]);

  // 卸载时清除定时器
  useEffect(() => {
    return () => {
      if (highlightTimerRef.current) {
        clearTimeout(highlightTimerRef.current);
      }
    };
  }, []);

  return {
    seekToKeyframe,
    highlightedKeyframeId: magicMoment.highlightedKeyframeId,
    previewAssetId: magicMoment.previewAssetId,
    isAnimating: magicMoment.isAnimating,
  };
}
