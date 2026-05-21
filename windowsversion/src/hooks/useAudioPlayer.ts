import { useCallback, useEffect, useRef } from "react";
import { useTimelineStore } from "../stores";

interface UseAudioPlayerOptions {
  audioUrl: string | null;
}

export function useAudioPlayer({ audioUrl }: UseAudioPlayerOptions): {
  play: () => void;
  pause: () => void;
  seek: (time: number) => void;
  setPlaybackRate: (rate: number) => void;
  setVolume: (vol: number) => void;
} {
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const animFrameRef = useRef<number>(0);
  const store = useTimelineStore;

  useEffect(() => {
    if (!audioUrl) return;

    const audio = new Audio(audioUrl);
    audio.preload = "auto";
    audioRef.current = audio;

    const onTimeUpdate = (): void => {
      store.getState().seek(audio.currentTime);
    };

    const onEnded = (): void => {
      store.getState().pause();
    };

    audio.addEventListener("timeupdate", onTimeUpdate);
    audio.addEventListener("ended", onEnded);

    return () => {
      audio.removeEventListener("timeupdate", onTimeUpdate);
      audio.removeEventListener("ended", onEnded);
      cancelAnimationFrame(animFrameRef.current);
      audio.pause();
      audio.src = "";
    };
  }, [audioUrl]);

  useEffect(() => {
    const unsub = store.subscribe((state, prev) => {
      const audio = audioRef.current;
      if (!audio) return;

      if (state.playback.isPlaying !== prev.playback.isPlaying) {
        if (state.playback.isPlaying) {
          audio.play().catch(() => {});
        } else {
          audio.pause();
        }
      }

      if (state.playback.playbackSpeed !== prev.playback.playbackSpeed) {
        audio.playbackRate = state.playback.playbackSpeed;
      }

      if (state.playback.volume !== prev.playback.volume || state.playback.isMuted !== prev.playback.isMuted) {
        audio.volume = state.playback.isMuted ? 0 : state.playback.volume;
      }
    });

    return unsub;
  }, []);

  const play = useCallback(() => {
    store.getState().play();
  }, []);

  const pause = useCallback(() => {
    store.getState().pause();
  }, []);

  const seek = useCallback((time: number) => {
    const audio = audioRef.current;
    if (audio) {
      audio.currentTime = time;
    }
    store.getState().seek(time);
  }, []);

  const setPlaybackRate = useCallback((rate: number) => {
    store.getState().setPlaybackSpeed(rate);
  }, []);

  const setVolume = useCallback((vol: number) => {
    store.getState().setVolume(vol);
  }, []);

  return { play, pause, seek, setPlaybackRate, setVolume };
}
