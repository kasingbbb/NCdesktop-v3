/** 双栏骨架屏 — 与 AssetListView 的实际布局完全对应 */
export function SkeletonAssetPanel() {
  return (
    <div
      className="flex flex-1 min-h-0 gap-0 overflow-hidden rounded-[var(--radius-xl)] border"
      style={{
        borderColor: "var(--border-primary)",
        background: "var(--surface-primary)",
        boxShadow: "var(--shadow-float)",
      }}
    >
      {/* 左栏：导入原件 */}
      <div
        className="flex flex-col border-r shrink-0"
        style={{ width: 360, borderColor: "var(--raw-pane-border)" }}
      >
        <div
          className="px-3 py-2 border-b shrink-0"
          style={{ background: "var(--raw-pane-bg)", borderColor: "var(--raw-pane-border)" }}
        >
          <div className="skeleton h-[13px] w-[56px] rounded mb-[6px]" />
          <div className="skeleton h-[10px] w-[96px] rounded" />
        </div>
        <div className="flex-1 overflow-hidden" style={{ background: "var(--raw-pane-bg)" }}>
          {Array.from({ length: 6 }).map((_, i) => (
            <SkeletonRawRow key={i} />
          ))}
        </div>
      </div>

      {/* 右栏：工作区 */}
      <div className="flex flex-col flex-1 min-w-0">
        <div
          className="px-3 py-2 border-b shrink-0"
          style={{ background: "var(--surface-tertiary)", borderColor: "var(--border-primary)" }}
        >
          <div className="skeleton h-[13px] w-[48px] rounded mb-[6px]" />
          <div className="skeleton h-[10px] w-[160px] rounded" />
        </div>
        <div className="flex-1 overflow-hidden" style={{ background: "var(--surface-primary)" }}>
          {Array.from({ length: 4 }).map((_, i) => (
            <SkeletonProcessedCard key={i} />
          ))}
        </div>
      </div>
    </div>
  );
}

function SkeletonRawRow() {
  return (
    <div
      className="flex items-center gap-2 px-3 border-b"
      style={{ height: 36, borderColor: "var(--raw-pane-border)" }}
    >
      <div className="skeleton w-[20px] h-[20px] rounded-[var(--radius-sm)] shrink-0" />
      <div className="skeleton h-[11px] flex-1 rounded" style={{ maxWidth: "68%" }} />
      <div className="skeleton h-[10px] w-[56px] rounded shrink-0" />
    </div>
  );
}

function SkeletonProcessedCard() {
  return (
    <div
      className="px-3 py-2.5 border-b flex flex-col gap-[6px]"
      style={{ borderColor: "var(--border-primary)" }}
    >
      <div className="flex items-center gap-2">
        <div className="skeleton w-[14px] h-[14px] rounded shrink-0" />
        <div className="skeleton h-[12px] rounded flex-1" style={{ maxWidth: "72%" }} />
        <div className="skeleton h-[18px] w-[32px] rounded shrink-0" />
      </div>
      <div className="flex items-center gap-1.5">
        <div className="skeleton h-[18px] w-[40px] rounded-full" />
        <div className="skeleton h-[18px] w-[32px] rounded-full" />
        <div className="skeleton h-[10px] w-[72px] rounded ml-auto" />
      </div>
    </div>
  );
}
