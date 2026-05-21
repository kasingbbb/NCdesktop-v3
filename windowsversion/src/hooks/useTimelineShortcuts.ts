import { useEffect } from "react";
import { useTimelineStore } from "../stores";

const SKIP_SECONDS = 5;

export function useTimelineShortcuts(): void {
  useEffect(() => {
    const handler = (e: KeyboardEvent): void => {
      const target = e.target as HTMLElement;
      if (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable) {
        return;
      }

      const state = useTimelineStore.getState();

      switch (e.code) {
        case "Space": {
          e.preventDefault();
          if (state.playback.isPlaying) {
            state.pause();
          } else {
            state.play();
          }
          break;
        }
        case "ArrowLeft": {
          e.preventDefault();
          const newTime = Math.max(0, state.playback.currentTime - SKIP_SECONDS);
          state.seek(newTime);
          break;
        }
        case "ArrowRight": {
          e.preventDefault();
          const maxTime = state.playback.duration;
          const newTime = Math.min(maxTime, state.playback.currentTime + SKIP_SECONDS);
          state.seek(newTime);
          break;
        }
        case "Home": {
          e.preventDefault();
          state.seek(0);
          break;
        }
        case "End": {
          e.preventDefault();
          state.seek(state.playback.duration);
          break;
        }
        case "Equal":
        case "NumpadAdd": {
          if (e.metaKey || e.ctrlKey) {
            e.preventDefault();
            state.zoomIn();
          }
          break;
        }
        case "Minus":
        case "NumpadSubtract": {
          if (e.metaKey || e.ctrlKey) {
            e.preventDefault();
            state.zoomOut();
          }
          break;
        }
        case "Digit0":
        case "Numpad0": {
          if (e.metaKey || e.ctrlKey) {
            e.preventDefault();
            state.resetZoom();
          }
          break;
        }
      }
    };

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);
}
