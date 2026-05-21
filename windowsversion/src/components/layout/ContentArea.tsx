import { Component, lazy, Suspense, type ErrorInfo, type ReactNode } from "react";
import { useUIStore } from "../../stores/uiStore";
import { useProjectStore } from "../../stores/projectStore";
import { useLibraryStore } from "../../stores/libraryStore";
import { Toolbar } from "./Toolbar";
import { ProjectListView } from "../features/ProjectListView";
import { AssetListView } from "../features/AssetListView";
import { AssetPreview } from "../features/AssetPreview";
import { CalendarWeekView } from "../features/calendar/CalendarWeekView";

// 懒加载：这些子树里有跨模块的破损 import（KnowledgeGraphView、SkillsView 等），
// 直接 eager import 会让 Vite 在初始 bundle 时炸掉整个 App（白屏）。
// 用 lazy + Suspense 隔离失败半径，任何一个 chunk 加载失败也只影响对应视图。
const TodayView = lazy(() =>
  import("../features/today/TodayView").then((m) => ({ default: m.TodayView }))
);
const KnowledgeHubView = lazy(() =>
  import("../features/KnowledgeHubView").then((m) => ({ default: m.KnowledgeHubView }))
);

function ViewFallback() {
  return (
    <div className="flex items-center justify-center h-full">
      <p className="text-[var(--text-sm)]" style={{ color: "var(--text-tertiary)" }}>加载中…</p>
    </div>
  );
}

class ViewErrorBoundary extends Component<
  { children: ReactNode; viewName: string },
  { error: Error | null }
> {
  state = { error: null as Error | null };
  static getDerivedStateFromError(error: Error) {
    return { error };
  }
  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error(`[${this.props.viewName}] render error:`, error, info);
  }
  render() {
    if (this.state.error) {
      return (
        <div className="flex flex-col items-center justify-center h-full gap-2 p-6">
          <p className="text-[var(--text-base)] font-semibold" style={{ color: "var(--text-primary)" }}>
            {this.props.viewName} 视图加载失败
          </p>
          <pre
            className="text-[11px] max-w-2xl overflow-auto p-3 rounded border whitespace-pre-wrap"
            style={{
              color: "var(--text-secondary)",
              background: "var(--surface-secondary)",
              borderColor: "var(--border-primary)",
            }}
          >
            {String(this.state.error?.message ?? this.state.error)}
          </pre>
          <button
            type="button"
            className="px-3 py-1 text-[12px] rounded"
            style={{ background: "var(--brand-navy)", color: "#fff" }}
            onClick={() => this.setState({ error: null })}
          >
            重试
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}

interface ContentAreaProps {
  onSearchOpen?: () => void;
}

export function ContentArea({ onSearchOpen }: ContentAreaProps) {
  const { activeSidebarSection, inspectorOpen, rightPanelMode } = useUIStore();
  const activeProjectId = useProjectStore((s) => s.activeProjectId);
  const activeLibraryId = useLibraryStore((s) => s.activeLibraryId);

  if (rightPanelMode !== "course_preview" && activeSidebarSection === "calendar") {
    return (
      <main
        className={`flex-1 flex flex-col h-full min-w-0 overflow-hidden bg-[var(--surface-canvas)] p-3 ${inspectorOpen ? "border-r border-app" : ""}`}
      >
        <CalendarWeekView />
      </main>
    );
  }

  if (activeSidebarSection === "today") {
    return (
      <main
        className={`flex-1 flex flex-col h-full min-w-0 overflow-hidden bg-[var(--surface-canvas)] p-3 ${inspectorOpen ? "border-r border-app" : ""}`}
      >
        <ViewErrorBoundary viewName="今日">
          <Suspense fallback={<ViewFallback />}>
            <TodayView libraryId={activeLibraryId ?? ""} />
          </Suspense>
        </ViewErrorBoundary>
      </main>
    );
  }

  if (activeSidebarSection === "knowledge-hub") {
    return (
      <main
        className={`flex-1 flex flex-col h-full min-w-0 overflow-hidden bg-[var(--surface-canvas)] p-3 ${inspectorOpen ? "border-r border-app" : ""}`}
      >
        <ViewErrorBoundary viewName="知识库">
          <Suspense fallback={<ViewFallback />}>
            <KnowledgeHubView libraryId={activeLibraryId} />
          </Suspense>
        </ViewErrorBoundary>
      </main>
    );
  }

  const isLibraryView = ["projects", "recent", "search", "starred"].includes(activeSidebarSection);

  if (isLibraryView) {
    return (
      <main
        className={`flex-1 flex flex-col h-full min-w-0 overflow-hidden bg-[var(--surface-canvas)] p-3 ${inspectorOpen ? "border-r border-app" : ""}`}
      >
        <div
          className="flex flex-col flex-1 min-h-0 rounded-[var(--radius-xl)] border overflow-hidden bg-[var(--surface-primary)] min-w-0"
          style={{
            borderColor: "var(--border-primary)",
            boxShadow: "var(--shadow-float)",
          }}
        >
          <Toolbar onSearchOpen={onSearchOpen} />
          {activeProjectId ? <AssetListView /> : <ProjectListView />}
        </div>
      </main>
    );
  }

  return (
    <main className="flex-1 flex flex-col h-full min-w-0 overflow-hidden p-[var(--space-4)] bg-[var(--surface-canvas)]">
      <AssetPreview />
      <div
        className="h-[180px] shrink-0 border-t"
        style={{
          borderColor: "var(--border-primary)",
          background: "var(--surface-secondary)",
        }}
      >
        <div className="flex items-center justify-center h-full">
          <p className="text-[var(--text-sm)]" style={{ color: "var(--text-tertiary)" }}>
            Recording Axis — 时间轴将在此渲染
          </p>
        </div>
      </div>
    </main>
  );
}
