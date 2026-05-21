import { create } from "zustand";
import type {
  Timeline,
  AudioTrack,
  Keyframe,
  Marker,
  PlaybackState,
  TimelineViewport,
} from "../types";
import * as cmd from "../lib/tauri-commands";

interface TimelineStore {
  timeline: Timeline | null;
  audioTracks: AudioTrack[];
  keyframes: Keyframe[];
  markers: Marker[];
  playback: PlaybackState;
  viewport: TimelineViewport;
  isLoading: boolean;
  error: string | null;

  loadTimeline: (projectId: string) => Promise<void>;
  createTimeline: (params: {
    projectId: string;
    startTime: string;
    endTime: string;
    duration: number;
  }) => Promise<Timeline>;
  createAudioTrack: (params: {
    timelineId: string;
    filePath: string;
    fileName: string;
    format: string;
    duration: number;
    sampleRate: number;
    channels: number;
  }) => Promise<AudioTrack>;
  createKeyframe: (params: {
    timelineId: string;
    assetId: string;
    anchorTime: number;
    source: string;
  }) => Promise<Keyframe>;
  deleteKeyframe: (id: string) => Promise<void>;
  createMarker: (params: {
    timelineId: string;
    time: number;
    label: string;
    color: string;
    markerType: string;
  }) => Promise<Marker>;
  deleteMarker: (id: string) => Promise<void>;

  // 播放控制
  play: () => void;
  pause: () => void;
  seek: (time: number) => void;
  setPlaybackSpeed: (speed: number) => void;
  setVolume: (volume: number) => void;
  toggleMute: () => void;

  // 视口控制
  setViewport: (viewport: Partial<TimelineViewport>) => void;
  zoomIn: () => void;
  zoomOut: () => void;
  resetZoom: () => void;
}

export const useTimelineStore = create<TimelineStore>((set, get) => ({
  timeline: null,
  audioTracks: [],
  keyframes: [],
  markers: [],
  playback: {
    isPlaying: false,
    currentTime: 0,
    duration: 0,
    playbackSpeed: 1,
    volume: 1,
    isMuted: false,
  },
  viewport: {
    startTime: 0,
    endTime: 0,
    zoomLevel: 1,
  },
  isLoading: false,
  error: null,

  loadTimeline: async (projectId) => {
    set({ isLoading: true, error: null });
    try {
      const timeline = await cmd.getTimeline(projectId);
      if (timeline) {
        const [audioTracks, keyframes, markers] = await Promise.all([
          cmd.getAudioTracks(timeline.id),
          cmd.getKeyframes(timeline.id),
          cmd.getMarkers(timeline.id),
        ]);
        set({
          timeline,
          audioTracks,
          keyframes,
          markers,
          viewport: { startTime: 0, endTime: timeline.duration, zoomLevel: 1 },
          playback: { ...get().playback, duration: timeline.duration, currentTime: 0 },
          isLoading: false,
        });
      } else {
        set({
          timeline: null,
          audioTracks: [],
          keyframes: [],
          markers: [],
          isLoading: false,
        });
      }
    } catch (e) {
      set({ error: String(e), isLoading: false });
    }
  },

  createTimeline: async (params) => {
    const timeline = await cmd.createTimeline(params);
    set({ timeline });
    return timeline;
  },

  createAudioTrack: async (params) => {
    const track = await cmd.createAudioTrack(params);
    set((s) => ({ audioTracks: [...s.audioTracks, track] }));
    return track;
  },

  createKeyframe: async (params) => {
    const kf = await cmd.createKeyframe(params);
    set((s) => ({ keyframes: [...s.keyframes, kf].sort((a, b) => a.anchorTime - b.anchorTime) }));
    return kf;
  },

  deleteKeyframe: async (id) => {
    await cmd.deleteKeyframe(id);
    set((s) => ({ keyframes: s.keyframes.filter((k) => k.id !== id) }));
  },

  createMarker: async (params) => {
    const m = await cmd.createMarker(params);
    set((s) => ({ markers: [...s.markers, m].sort((a, b) => a.time - b.time) }));
    return m;
  },

  deleteMarker: async (id) => {
    await cmd.deleteMarker(id);
    set((s) => ({ markers: s.markers.filter((m) => m.id !== id) }));
  },

  play: () => set((s) => ({ playback: { ...s.playback, isPlaying: true } })),
  pause: () => set((s) => ({ playback: { ...s.playback, isPlaying: false } })),
  seek: (time) => set((s) => ({ playback: { ...s.playback, currentTime: time } })),
  setPlaybackSpeed: (speed) => set((s) => ({ playback: { ...s.playback, playbackSpeed: speed } })),
  setVolume: (volume) => set((s) => ({ playback: { ...s.playback, volume, isMuted: false } })),
  toggleMute: () => set((s) => ({ playback: { ...s.playback, isMuted: !s.playback.isMuted } })),

  setViewport: (vp) => set((s) => ({ viewport: { ...s.viewport, ...vp } })),
  zoomIn: () => set((s) => {
    const newZoom = Math.min(s.viewport.zoomLevel * 1.5, 32);
    return { viewport: { ...s.viewport, zoomLevel: newZoom } };
  }),
  zoomOut: () => set((s) => {
    const newZoom = Math.max(s.viewport.zoomLevel / 1.5, 0.25);
    return { viewport: { ...s.viewport, zoomLevel: newZoom } };
  }),
  resetZoom: () => set((s) => ({
    viewport: { startTime: 0, endTime: s.timeline?.duration ?? 0, zoomLevel: 1 },
  })),
}));
