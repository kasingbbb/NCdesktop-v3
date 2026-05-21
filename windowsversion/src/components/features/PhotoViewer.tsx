import { useState } from "react";
import { ZoomIn, ZoomOut, Maximize } from "lucide-react";

interface PhotoViewerProps {
  url: string;
}

export function PhotoViewer({ url }: PhotoViewerProps) {
  const [scale, setScale] = useState(1);

  return (
    <div className="relative w-full h-full flex items-center justify-center overflow-hidden rounded-[var(--radius-lg)]" style={{ background: "var(--surface-primary)" }}>
      <div className="absolute top-4 right-4 flex gap-2 z-10">
        <button 
          className="p-2 rounded-full transition-colors" style={{ background: "var(--surface-tertiary)", color: "var(--text-primary)" }}
          onClick={() => setScale(s => Math.min(s + 0.25, 3))}
        >
          <ZoomIn size={16} />
        </button>
        <button 
          className="p-2 rounded-full transition-colors" style={{ background: "var(--surface-tertiary)", color: "var(--text-primary)" }}
          onClick={() => setScale(s => Math.max(s - 0.25, 0.5))}
        >
          <ZoomOut size={16} />
        </button>
        <button 
          className="p-2 rounded-full transition-colors" style={{ background: "var(--surface-tertiary)", color: "var(--text-primary)" }}
          onClick={() => setScale(1)}
        >
          <Maximize size={16} />
        </button>
      </div>

      <div
        className="origin-center"
        style={{ transform: `scale(${scale})`, transition: "transform var(--duration-fast) var(--ease-out-quart)" }}
      >
        <img 
          src={url} 
          alt="Asset Preview" 
          className="max-w-full max-h-full object-contain shadow-2xl"
          draggable={false}
        />
      </div>
    </div>
  );
}
