import { Clock, HardDrive, Info } from "lucide-react";
import type { Asset } from "../../types";

interface InspectorDetailsProps {
  asset: Asset;
}

/**
 * InspectorDetails — v1.3 严格按 Color & Type Guide v1 §3.2
 *
 * 字阶：
 *   - 段标题 DETAILS：10px / 600 uppercase tracking-[.12em] (text-tertiary)
 *   - 文件名（page-sub）：17px / 600 (text-primary)
 *   - 字段 label：11.5px / 500 (text-tertiary)
 *   - 字段 value：13px / 400 (text-primary)
 */
export function InspectorDetails({ asset }: InspectorDetailsProps) {
  return (
    <div className="mb-[var(--space-4)]">
      <h3
        className="text-[10px] font-semibold uppercase mb-[var(--space-2)]"
        style={{ color: "var(--text-tertiary)", letterSpacing: "0.12em" }}
      >
        Details
      </h3>

      <div
        className="rounded-[var(--radius-md)] p-[var(--space-3)]"
        style={{ background: "var(--surface-secondary)" }}
      >
        <h4
          className="text-[var(--text-lg)] font-semibold mb-[var(--space-3)] truncate"
          style={{ color: "var(--text-primary)" }}
          title={asset.name || "Untitled Asset"}
        >
          {asset.name || "Untitled Asset"}
        </h4>

        <div className="space-y-[var(--space-2)]">
          <div className="flex items-center justify-between gap-[var(--space-3)]">
            <span
              className="flex items-center gap-[var(--space-1)] text-[11.5px] font-medium shrink-0"
              style={{ color: "var(--text-tertiary)" }}
            >
              <Clock size={12} /> Captured
            </span>
            <span
              className="text-[var(--text-md)] truncate tabular-nums"
              style={{ color: "var(--text-primary)" }}
            >
              {new Date(asset.capturedAt).toLocaleString()}
            </span>
          </div>
          <div className="flex items-center justify-between gap-[var(--space-3)]">
            <span
              className="flex items-center gap-[var(--space-1)] text-[11.5px] font-medium shrink-0"
              style={{ color: "var(--text-tertiary)" }}
            >
              <HardDrive size={12} /> Source
            </span>
            <span
              className="text-[var(--text-md)] truncate min-w-0 font-mono"
              style={{ color: "var(--text-primary)" }}
              title={asset.filePath ?? undefined}
            >
              {asset.filePath?.split("/").pop() || "Unknown"}
            </span>
          </div>
          <div className="flex items-center justify-between gap-[var(--space-3)]">
            <span
              className="flex items-center gap-[var(--space-1)] text-[11.5px] font-medium shrink-0"
              style={{ color: "var(--text-tertiary)" }}
            >
              <Info size={12} /> Type
            </span>
            <span
              className="text-[var(--text-md)] uppercase tabular-nums"
              style={{ color: "var(--text-primary)" }}
            >
              {asset.type}
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}
