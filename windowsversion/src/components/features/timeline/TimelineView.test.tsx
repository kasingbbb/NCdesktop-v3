import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { TimelineView } from "./TimelineView";
import { logger } from "../../../utils/logger";
import type { Keyframe } from "../../../types";

vi.mock("../../../stores", () => ({
  useTimelineStore: vi.fn((selector) => {
    const state = {
      timeline: { id: "test", duration: 10 },
      seek: vi.fn()
    };
    return selector(state);
  })
}));

vi.mock("../../../hooks/useMagicMoment", () => ({
  useMagicMoment: () => ({
    seekToKeyframe: vi.fn(),
    previewAssetId: null
  })
}));

vi.mock("../../../hooks/useTimelineZoom", () => ({ useTimelineZoom: vi.fn() }));
vi.mock("../../../hooks/useTimelineDrag", () => ({ useTimelineDrag: vi.fn() }));
vi.mock("../../../hooks/useTimelineShortcuts", () => ({ useTimelineShortcuts: vi.fn() }));

// Mock components
vi.mock("./WaveformRenderer", () => ({ WaveformRenderer: () => <div data-testid="waveform" /> }));
vi.mock("./Playhead", () => ({ Playhead: () => <div data-testid="playhead" /> }));
vi.mock("./TimeRuler", () => ({ TimeRuler: () => <div data-testid="time-ruler" /> }));
vi.mock("./PlaybackControls", () => ({ PlaybackControls: () => <div data-testid="playback-controls" /> }));
vi.mock("./SelectionOverlay", () => ({ SelectionOverlay: () => <div data-testid="selection-overlay" /> }));
vi.mock("./KeyframeTrack", () => ({
  KeyframeTrack: ({ onKeyframeClick }: { onKeyframeClick?: (keyframe: Keyframe) => void }) => (
    <div
      data-testid="keyframe-track"
      onClick={() =>
        onKeyframeClick?.({
          id: "kf1",
          timelineId: "t1",
          assetId: "a1",
          anchorTime: 5,
          liveAudioClipId: null,
          source: "manual",
        })
      }
    />
  ),
}));

vi.spyOn(logger, "info");

describe("TimelineView Component", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders timeline components when timeline data exists", () => {
    render(<TimelineView waveformData={null} audioUrl={null} />);
    expect(screen.getByTestId("keyframe-track")).toBeInTheDocument();
    expect(screen.getByTestId("waveform")).toBeInTheDocument();
    expect(screen.getByTestId("time-ruler")).toBeInTheDocument();
    expect(screen.getByTestId("playback-controls")).toBeInTheDocument();
  });

  it("logs keyframe click event", () => {
    render(<TimelineView waveformData={null} audioUrl={null} />);
    const mockTrack = screen.getByTestId("keyframe-track");
    fireEvent.click(mockTrack);
    expect(logger.info).toHaveBeenCalledWith("TimelineView", "Seeking to keyframe", { id: "kf1" });
  });
});
