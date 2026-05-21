
interface ScanTextViewerProps {
  text: string;
}

export function ScanTextViewer({ text }: ScanTextViewerProps) {
  return (
    <div className="w-full h-full flex flex-col rounded-[var(--radius-lg)] overflow-hidden" style={{ background: "var(--surface-primary)" }}>
      <div className="px-4 py-2 border-b shrink-0" style={{ borderColor: "var(--border-primary)" }}>
        <h3 className="text-[var(--text-sm)] text-[var(--text-secondary)] font-medium">Scanned Text Document</h3>
      </div>
      <div className="flex-1 overflow-y-auto p-6">
        <div className="max-w-3xl mx-auto">
          <p className="text-[var(--text-base)] leading-relaxed whitespace-pre-wrap font-serif" style={{ color: "var(--text-primary)" }}>
            {text}
          </p>
        </div>
      </div>
    </div>
  );
}
