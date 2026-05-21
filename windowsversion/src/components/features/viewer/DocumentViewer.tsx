import { useCallback, useEffect, useState } from "react";
import {
  X,
  ChevronLeft,
  ChevronRight,
  ZoomIn,
  ZoomOut,
  Maximize,
  FileText,
  Image,
  Music,
  File,
} from "lucide-react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useAssetStore } from "../../../stores/assetStore";
import { getFileContent } from "../../../lib/tauri-commands";
import type { Asset } from "../../../types";

interface DocumentViewerProps {
  assetId: string;
  onClose: () => void;
}

function assetKind(a: Asset): string {
  const r = a as Asset & { assetType?: string };
  return r.assetType ?? r.type ?? "other";
}

function isImageType(kind: string): boolean {
  return kind === "image" || kind === "photo";
}

function isTextType(kind: string): boolean {
  return kind === "markdown" || kind === "scan_text";
}

function isPdfType(kind: string): boolean {
  return kind === "pdf";
}

function isAudioType(kind: string): boolean {
  return kind === "audio_clip";
}

export function DocumentViewer({ assetId, onClose }: DocumentViewerProps) {
  const assets = useAssetStore((s) => s.assets);
  const [currentId, setCurrentId] = useState(assetId);

  const currentIndex = assets.findIndex((a) => a.id === currentId);
  const asset = assets[currentIndex];

  const goPrev = useCallback(() => {
    if (currentIndex > 0) {
      setCurrentId(assets[currentIndex - 1].id);
    }
  }, [assets, currentIndex]);

  const goNext = useCallback(() => {
    if (currentIndex < assets.length - 1) {
      setCurrentId(assets[currentIndex + 1].id);
    }
  }, [assets, currentIndex]);

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        onClose();
      } else if (e.key === "ArrowLeft" || (e.metaKey && e.key === "[")) {
        goPrev();
      } else if (e.key === "ArrowRight" || (e.metaKey && e.key === "]")) {
        goNext();
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose, goPrev, goNext]);

  if (!asset) {
    onClose();
    return null;
  }

  const kind = assetKind(asset);

  return (
    <div
      className="fixed inset-0 z-50 flex flex-col"
      style={{ background: "var(--surface-canvas)" }}
    >
      {/* Header */}
      <div
        className="h-12 flex items-center justify-between px-4 border-b shrink-0"
        style={{
          borderColor: "var(--border-primary)",
          background: "var(--surface-primary)",
        }}
      >
        <div className="flex items-center gap-3 min-w-0">
          <button
            type="button"
            onClick={onClose}
            className="p-1.5 rounded-[var(--radius-sm)] transition-colors"
            style={{ color: "var(--text-secondary)" }}
            title="Close (Esc)"
          >
            <X size={18} />
          </button>
          <span
            className="text-[var(--text-sm)] font-medium truncate"
            style={{ color: "var(--text-primary)" }}
          >
            {asset.name}
          </span>
          <span
            className="text-[var(--text-xs)]"
            style={{ color: "var(--text-tertiary)" }}
          >
            {currentIndex + 1} / {assets.length}
          </span>
        </div>

        <div className="flex items-center gap-1">
          <button
            type="button"
            onClick={goPrev}
            disabled={currentIndex <= 0}
            className="p-1.5 rounded-[var(--radius-sm)] transition-colors disabled:opacity-30"
            style={{ color: "var(--text-secondary)" }}
            title="Previous"
          >
            <ChevronLeft size={18} />
          </button>
          <button
            type="button"
            onClick={goNext}
            disabled={currentIndex >= assets.length - 1}
            className="p-1.5 rounded-[var(--radius-sm)] transition-colors disabled:opacity-30"
            style={{ color: "var(--text-secondary)" }}
            title="Next"
          >
            <ChevronRight size={18} />
          </button>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 min-h-0 overflow-hidden">
        {isImageType(kind) ? (
          <ImageContent asset={asset} />
        ) : isPdfType(kind) ? (
          <PdfContent asset={asset} />
        ) : isTextType(kind) ? (
          <TextContent asset={asset} />
        ) : isAudioType(kind) ? (
          <AudioContent asset={asset} />
        ) : (
          <FallbackContent asset={asset} />
        )}
      </div>
    </div>
  );
}

function ImageContent({ asset }: { asset: Asset }) {
  const [scale, setScale] = useState(1);
  const url = convertFileSrc(asset.filePath);

  return (
    <div className="relative w-full h-full flex items-center justify-center overflow-auto">
      <div className="absolute top-4 right-4 flex gap-2 z-10">
        <button
          className="p-2 rounded-full transition-colors"
          style={{
            background: "var(--surface-elevated)",
            color: "var(--text-primary)",
            boxShadow: "var(--shadow-float)",
          }}
          onClick={() => setScale((s) => Math.min(s + 0.25, 5))}
        >
          <ZoomIn size={16} />
        </button>
        <button
          className="p-2 rounded-full transition-colors"
          style={{
            background: "var(--surface-elevated)",
            color: "var(--text-primary)",
            boxShadow: "var(--shadow-float)",
          }}
          onClick={() => setScale((s) => Math.max(s - 0.25, 0.25))}
        >
          <ZoomOut size={16} />
        </button>
        <button
          className="p-2 rounded-full transition-colors"
          style={{
            background: "var(--surface-elevated)",
            color: "var(--text-primary)",
            boxShadow: "var(--shadow-float)",
          }}
          onClick={() => setScale(1)}
        >
          <Maximize size={16} />
        </button>
      </div>
      <img
        src={url}
        alt={asset.name}
        className="max-w-full max-h-full object-contain"
        style={{
          transform: `scale(${scale})`,
          transition: "transform var(--duration-fast) var(--ease-out)",
        }}
        draggable={false}
        onWheel={(e) => {
          e.preventDefault();
          const delta = e.deltaY > 0 ? -0.1 : 0.1;
          setScale((s) => Math.min(Math.max(s + delta, 0.25), 5));
        }}
      />
    </div>
  );
}

function PdfContent({ asset }: { asset: Asset }) {
  const url = convertFileSrc(asset.filePath);
  return (
    <div className="w-full h-full">
      <iframe
        src={url}
        title={asset.name}
        className="w-full h-full border-none"
        style={{ background: "var(--surface-primary)" }}
      />
    </div>
  );
}

function TextContent({ asset }: { asset: Asset }) {
  const [content, setContent] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);
    getFileContent(asset.filePath)
      .then((text) => setContent(text))
      .catch(() => setContent(null))
      .finally(() => setLoading(false));
  }, [asset.filePath]);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <p style={{ color: "var(--text-tertiary)" }}>Loading...</p>
      </div>
    );
  }

  if (content === null) {
    return (
      <div className="flex items-center justify-center h-full">
        <p style={{ color: "var(--text-tertiary)" }}>Unable to load file content.</p>
      </div>
    );
  }

  return (
    <div className="w-full h-full overflow-y-auto p-8">
      <div className="max-w-3xl mx-auto">
        <pre
          className="text-[var(--text-base)] leading-relaxed whitespace-pre-wrap font-mono"
          style={{ color: "var(--text-primary)" }}
        >
          {content}
        </pre>
      </div>
    </div>
  );
}

function AudioContent({ asset }: { asset: Asset }) {
  const url = convertFileSrc(asset.filePath);
  return (
    <div className="flex flex-col items-center justify-center h-full gap-6">
      <div
        className="w-24 h-24 rounded-full flex items-center justify-center"
        style={{ background: "var(--surface-tertiary)" }}
      >
        <Music size={40} style={{ color: "var(--text-secondary)" }} />
      </div>
      <p
        className="text-[var(--text-lg)] font-medium"
        style={{ color: "var(--text-primary)" }}
      >
        {asset.name}
      </p>
      <audio controls src={url} className="w-full max-w-lg" />
    </div>
  );
}

function FallbackContent({ asset }: { asset: Asset }) {
  const kind = assetKind(asset);
  const icon =
    kind === "image" || kind === "photo" ? (
      <Image size={40} />
    ) : kind === "markdown" || kind === "scan_text" ? (
      <FileText size={40} />
    ) : (
      <File size={40} />
    );

  return (
    <div className="flex flex-col items-center justify-center h-full gap-4">
      <div
        className="w-24 h-24 rounded-full flex items-center justify-center"
        style={{ background: "var(--surface-tertiary)", color: "var(--text-secondary)" }}
      >
        {icon}
      </div>
      <p
        className="text-[var(--text-base)]"
        style={{ color: "var(--text-secondary)" }}
      >
        Preview not available for this file type.
      </p>
      <p
        className="text-[var(--text-sm)]"
        style={{ color: "var(--text-tertiary)" }}
      >
        {asset.filePath}
      </p>
    </div>
  );
}
