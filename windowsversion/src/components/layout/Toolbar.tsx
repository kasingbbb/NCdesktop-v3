import { Search, Plus, LayoutGrid, List, ChevronLeft, Loader2, PanelRight, Lightbulb } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { useLibraryStore } from "../../stores/libraryStore";
import { useUIStore } from "../../stores/uiStore";

interface ToolbarProps {
  onSearchOpen?: () => void;
}

export function Toolbar({ onSearchOpen }: ToolbarProps) {
  const {
    viewMode: projectViewMode,
    setViewMode: setProjectViewMode,
    createProject,
    setActiveProject,
    activeProjectId,
    getActiveProject,
  } = useProjectStore();
  const { activeLibraryId, ensureActiveLibrary } = useLibraryStore();
  const { inspectorOpen, toggleInspector, setRightPanelMode } = useUIStore();

  const activeProject = activeProjectId ? getActiveProject() : undefined;

  if (activeProjectId && activeProject) {
    return (
      <div
        className="h-[52px] flex items-center justify-between px-[var(--space-4)] border-b shrink-0 bg-[var(--surface-primary)]"
        style={{ borderColor: "var(--border-primary)", boxShadow: "var(--shadow-sm)" }}
      >
        <div className="flex items-center gap-[var(--space-3)] min-w-0">
          <button
            type="button"
            className="flex items-center gap-[4px] px-[var(--space-2)] py-1.5 rounded-[var(--radius-full)] transition-all text-[12px]"
            style={{ color: "var(--text-secondary)", background: "transparent" }}
            onClick={() => setActiveProject(null)}
            onMouseEnter={(e) => { (e.currentTarget as HTMLElement).style.background = "var(--surface-tertiary)"; }}
            onMouseLeave={(e) => { (e.currentTarget as HTMLElement).style.background = "transparent"; }}
          >
            <ChevronLeft size={14} />
            <span>项目</span>
          </button>
          <h2
            className="text-[15px] font-semibold truncate"
            style={{ color: "var(--text-primary)" }}
            title={activeProject.name}
          >
            {activeProject.name}
          </h2>
        </div>
        <div className="flex items-center gap-[6px] shrink-0">
          {/* 提取状态徽章 */}
          <div
            className="flex items-center gap-[4px] text-[11px] px-[8px] py-[3px] rounded-[var(--radius-md)]"
            style={{
              color: "var(--text-secondary)",
              background: "var(--surface-secondary)",
              border: "1px solid var(--border-primary)",
            }}
          >
            <Loader2 size={12} className="animate-spin" />
            提取中 2 个
          </div>
          {/* 知识关联按钮 */}
          <button
            type="button"
            className="flex items-center gap-[4px] h-[28px] px-[8px] text-[11px] font-medium rounded-[var(--radius-full)] transition-all"
            style={{
              color: "var(--text-primary)",
              background: "var(--surface-primary)",
              border: "1px solid var(--border-primary)",
              boxShadow: "var(--shadow-sm)",
            }}
            onClick={() => {
              setRightPanelMode("knowledge_association");
              if (!inspectorOpen) toggleInspector();
            }}
            onMouseEnter={(e) => {
              (e.currentTarget as HTMLElement).style.background = "var(--surface-secondary)";
              (e.currentTarget as HTMLElement).style.borderColor = "var(--border-hover)";
            }}
            onMouseLeave={(e) => {
              (e.currentTarget as HTMLElement).style.background = "var(--surface-primary)";
              (e.currentTarget as HTMLElement).style.borderColor = "var(--border-primary)";
            }}
          >
            <Lightbulb size={12} />
            知识关联
          </button>
          {/* Inspector 切换 */}
          <button
            type="button"
            className="w-[30px] h-[30px] flex items-center justify-center rounded-[var(--radius-md)] transition-colors"
            title="Inspector"
            onClick={toggleInspector}
            style={{ color: inspectorOpen ? "var(--text-primary)" : "var(--text-tertiary)" }}
            onMouseEnter={(e) => { (e.currentTarget as HTMLElement).style.background = "var(--surface-tertiary)"; }}
            onMouseLeave={(e) => { (e.currentTarget as HTMLElement).style.background = "transparent"; }}
          >
            <PanelRight size={14} />
          </button>
        </div>
      </div>
    );
  }

  return (
    <div
      className="h-[52px] flex items-center justify-between px-[var(--space-4)] border-b shrink-0 bg-[var(--surface-primary)]"
      style={{ borderColor: "var(--border-primary)", boxShadow: "var(--shadow-sm)" }}
    >
      <div className="flex items-center gap-[10px] flex-1 min-w-0">
        <h2 className="text-[15px] font-semibold whitespace-nowrap" style={{ color: "var(--text-primary)" }}>
          项目列表
        </h2>
        {/* 搜索栏 — 点击打开 ⌘K Command Palette */}
        <div
          className="flex items-center gap-[7px] px-[12px] h-[30px] min-w-[180px] max-w-[280px] flex-1 rounded-[var(--radius-full)] cursor-pointer transition-all"
          style={{
            background: "var(--surface-secondary)",
            border: "1px solid var(--border-primary)",
          }}
          onClick={onSearchOpen}
          onMouseEnter={(e) => { (e.currentTarget as HTMLElement).style.borderColor = "var(--border-hover)"; }}
          onMouseLeave={(e) => { (e.currentTarget as HTMLElement).style.borderColor = "var(--border-primary)"; }}
        >
          <Search size={12} style={{ color: "var(--text-tertiary)" }} />
          <span className="text-[12px] flex-1" style={{ color: "var(--text-tertiary)" }}>
            搜索项目…
          </span>
          <span
            className="text-[10px] px-[5px] py-[1px] rounded-[3px] shrink-0 whitespace-nowrap font-mono"
            style={{
              color: "var(--text-tertiary)",
              background: "var(--surface-tertiary)",
              border: "1px solid var(--border-primary)",
            }}
          >
            ⌘K
          </span>
        </div>
      </div>

      <div className="flex items-center gap-[6px] shrink-0">
        <button
          className="flex items-center gap-[5px] h-[30px] px-[12px] text-[12px] font-medium rounded-[var(--radius-full)] transition-all"
          style={{ background: "var(--brand-navy)", color: "#fff", border: "1px solid var(--brand-navy)" }}
          onMouseEnter={(e) => { (e.currentTarget as HTMLElement).style.background = "var(--brand-navy-light)"; }}
          onMouseLeave={(e) => { (e.currentTarget as HTMLElement).style.background = "var(--brand-navy)"; }}
          onClick={() => {
            void (async () => {
              const libId = activeLibraryId ?? (await ensureActiveLibrary());
              const now = new Date();
              const name = `新建项目 ${now.toLocaleString()}`;
              const project = await createProject(libId, name);
              setActiveProject(project.id);
            })();
          }}
        >
          <Plus size={13} />
          新建
        </button>
        <div
          className="flex rounded-[var(--radius-lg)] p-[2px] gap-[1px]"
          style={{ border: "1px solid var(--border-primary)", background: "var(--surface-tertiary)" }}
        >
          <button
            type="button"
            className="w-[26px] h-[26px] flex items-center justify-center rounded-[var(--radius-sm)] transition-colors"
            onClick={() => setProjectViewMode("grid")}
            style={{
              color: projectViewMode === "grid" ? "var(--text-primary)" : "var(--text-tertiary)",
              background: projectViewMode === "grid" ? "var(--surface-primary)" : "transparent",
              border: projectViewMode === "grid" ? "1px solid var(--border-primary)" : "1px solid transparent",
              boxShadow: projectViewMode === "grid" ? "var(--shadow-sm)" : "none",
            }}
          >
            <LayoutGrid size={13} />
          </button>
          <button
            type="button"
            className="w-[26px] h-[26px] flex items-center justify-center rounded-[var(--radius-sm)] transition-colors"
            onClick={() => setProjectViewMode("list")}
            style={{
              color: projectViewMode === "list" ? "var(--text-primary)" : "var(--text-tertiary)",
              background: projectViewMode === "list" ? "var(--surface-primary)" : "transparent",
              border: projectViewMode === "list" ? "1px solid var(--border-primary)" : "1px solid transparent",
              boxShadow: projectViewMode === "list" ? "var(--shadow-sm)" : "none",
            }}
          >
            <List size={13} />
          </button>
        </div>
        {/* Inspector 切换（无论是否在项目内都可用，避免关闭后无入口重开） */}
        <button
          type="button"
          className="w-[30px] h-[30px] flex items-center justify-center rounded-[var(--radius-md)] transition-colors"
          title="Inspector"
          onClick={toggleInspector}
          style={{ color: inspectorOpen ? "var(--text-primary)" : "var(--text-tertiary)" }}
          onMouseEnter={(e) => { (e.currentTarget as HTMLElement).style.background = "var(--surface-tertiary)"; }}
          onMouseLeave={(e) => { (e.currentTarget as HTMLElement).style.background = "transparent"; }}
        >
          <PanelRight size={14} />
        </button>
      </div>
    </div>
  );
}
