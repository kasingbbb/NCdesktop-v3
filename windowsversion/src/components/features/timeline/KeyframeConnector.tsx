interface KeyframeConnectorProps {
  thumbCenterX: number;
  waveformY: number;
  thumbBottomY: number;
}

export function KeyframeConnector({
  thumbCenterX,
  waveformY,
  thumbBottomY,
}: KeyframeConnectorProps) {
  return (
    <svg
      className="absolute pointer-events-none"
      style={{
        left: thumbCenterX - 1,
        top: thumbBottomY,
        width: 2,
        height: Math.max(0, waveformY - thumbBottomY),
        zIndex: 5,
      }}
    >
      <line
        x1={1}
        y1={0}
        x2={1}
        y2={Math.max(0, waveformY - thumbBottomY)}
        stroke="var(--border-primary)"
        strokeWidth={1}
        strokeDasharray="3 3"
      />
    </svg>
  );
}
