import { useAssetStore } from "../../stores/assetStore";
import { PhotoViewer } from "./PhotoViewer";
import { ScanTextViewer } from "./ScanTextViewer";

export function AssetPreview() {
  const { selectedAssetId, assets } = useAssetStore();
  const activeAsset = assets.find(a => a.id === selectedAssetId);

  if (!activeAsset) {
    return (
      <section className="flex-1 rounded-[var(--radius-lg)] flex items-center justify-center" style={{ background: "var(--surface-secondary)" }}>
        <div className="text-center">
          <p className="text-[var(--text-xl)] font-semibold mb-[var(--space-2)]" style={{ color: "var(--text-secondary)" }}>
            Asset Preview Panel
          </p>
          <p className="text-[var(--text-sm)]" style={{ color: "var(--text-tertiary)" }}>
            Select an asset to preview
          </p>
        </div>
      </section>
    );
  }

  // Render view based on asset type
  return (
    <section className="flex-1 rounded-[var(--radius-lg)] overflow-hidden flex flex-col">
      {activeAsset.type === 'image' && activeAsset.filePath ? (
        <PhotoViewer url={`file://${activeAsset.filePath}`} />
      ) : activeAsset.type === 'scan_text' && activeAsset.aiAnalysis?.ocrText ? (
        <ScanTextViewer text={activeAsset.aiAnalysis.ocrText} />
      ) : (
        <div className="flex-1 flex items-center justify-center" style={{ background: "var(--surface-secondary)" }}>
          <p className="text-[var(--text-secondary)]">Preview not available for this format type.</p>
        </div>
      )}
    </section>
  );
}
