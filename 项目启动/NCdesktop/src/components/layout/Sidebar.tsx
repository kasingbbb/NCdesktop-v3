import { Clock, CalendarDays, Sun, Search, Lightbulb, FolderOpen } from "lucide-react";
import { SidebarItem, SidebarSection } from "./SidebarItem";
import { ProjectTree } from "../features/ProjectTree";
import { TagTree } from "../features/TagTree";
import { SidebarFooter } from "./SidebarFooter";
import { useUIStore } from "../../stores/uiStore";
import { useProjectStore } from "../../stores/projectStore";
import { useFeatureToggles } from "../../stores/settingsStore";

interface SidebarProps {
  width: number;
  onSettingsOpen?: () => void;
  onSearchOpen?: () => void;
}

export function Sidebar({ width, onSettingsOpen, onSearchOpen }: SidebarProps) {
  const { activeSidebarSection, setSidebarSection } = useUIStore();
  const setActiveProject = useProjectStore((s) => s.setActiveProject);
  const { showKnowledgeSystem, showStudentCenter } = useFeatureToggles();

  // 点击「项目」/「最近」时清掉 activeProjectId，
  // 否则即使切了 section，ContentArea 仍会渲染当前项目的 AssetListView。
  const gotoLibrary = (section: "projects" | "recent") => {
    setActiveProject(null);
    setSidebarSection(section);
  };

  return (
    <aside
      className="glass-sidebar flex flex-col h-full overflow-hidden"
      style={{ width: `${width}px` }}
    >
      {/* 品牌标识区 */}
      <div className="pt-[60px] px-[14px] pb-[10px] flex items-center gap-[8px]">
        <div
          className="w-[26px] h-[26px] rounded-[6px] flex items-center justify-center shrink-0 text-[13px] font-bold text-white"
          style={{ background: "rgba(255,255,255,0.08)" }}
        >
          N
        </div>
        <div>
          <div className="text-[13px] font-bold text-white leading-tight">NoteCapt</div>
          <div
            className="text-[9px] uppercase tracking-[0.08em]"
            style={{ color: "var(--sidebar-text-dim)" }}
          >
            Knowledge
          </div>
        </div>
      </div>

      <div className="h-px mx-0" style={{ background: "var(--sidebar-divider)" }} />

      {/* 导航列表 */}
      <nav className="flex-1 overflow-y-auto px-[8px] py-[4px]">
        <SidebarItem
          icon={<Search size={14} />}
          label="搜索"
          onClick={onSearchOpen}
        />
        <SidebarItem
          icon={<FolderOpen size={14} />}
          label="项目"
          active={activeSidebarSection === "projects"}
          onClick={() => gotoLibrary("projects")}
        />
        <SidebarItem
          icon={<Clock size={14} />}
          label="最近"
          active={activeSidebarSection === "recent"}
          onClick={() => gotoLibrary("recent")}
        />
        {showStudentCenter && (
          <SidebarItem
            icon={<CalendarDays size={14} />}
            label="日历"
            active={activeSidebarSection === "calendar"}
            onClick={() => setSidebarSection("calendar")}
          />
        )}

        {showKnowledgeSystem && (
          <>
            <div className="h-px my-[6px]" style={{ background: "var(--sidebar-divider)" }} />
            <SidebarSection title="知识系统">
              <SidebarItem
                icon={<Sun size={14} />}
                label="今日复习"
                badge={3}
                active={activeSidebarSection === "today"}
                onClick={() => setSidebarSection("today")}
              />
              <SidebarItem
                icon={<Lightbulb size={14} />}
                label="知识库"
                active={activeSidebarSection === "knowledge-hub"}
                onClick={() => setSidebarSection("knowledge-hub")}
              />
            </SidebarSection>
          </>
        )}

        <div className="h-px my-[6px]" style={{ background: "var(--sidebar-divider)" }} />
        <SidebarSection title="项目">
          <ProjectTree />
        </SidebarSection>

        <TagTree />
      </nav>

      {/* 底部状态栏 */}
      <SidebarFooter onSettingsOpen={onSettingsOpen} />
    </aside>
  );
}
