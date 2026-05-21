// Windows 版：标题栏右侧给系统按钮留 120px 占位（原 macOS 版是左侧 80px 红绿灯留白）
import { Settings, Search } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";

interface TitleBarProps {
  onSettingsOpen?: () => void;
  onSearchOpen?: () => void;
}

export function TitleBar({ onSettingsOpen, onSearchOpen }: TitleBarProps) {
  const activeProject = useProjectStore((s) => s.getActiveProject());

  return (
    <header className="titlebar-drag-region glass-titlebar flex items-center h-[48px] px-[var(--space-4)] relative">
      {/* Windows 版：标题栏右侧给系统按钮（最小化/最大化/关闭）留 120px 占位 */}
      <div className="ml-auto w-[120px] shrink-0" aria-hidden />

      {/* 面包屑 */}
      <div className="flex-1 flex items-center justify-center gap-[6px] text-[12px] tracking-[0.02em]" data-no-drag>
        {activeProject ? (
          <>
            <span style={{ color: "rgba(255,255,255,0.4)" }}>项目列表</span>
            <span style={{ color: "rgba(255,255,255,0.25)", fontSize: 11 }}>›</span>
            <span
              className="font-medium max-w-[260px] truncate"
              style={{ color: "rgba(255,255,255,0.8)" }}
              title={activeProject.name}
            >
              {activeProject.name}
            </span>
          </>
        ) : (
          <span
            className="font-medium"
            style={{ color: "rgba(255,255,255,0.5)" }}
          >
            NoteCapt
          </span>
        )}
      </div>

      {/* 右侧工具区 */}
      <div className="w-[80px] shrink-0 flex items-center justify-end gap-[4px] pr-[12px]" data-no-drag>
        {onSettingsOpen && (
          <button
            type="button"
            onClick={onSettingsOpen}
            className="w-[26px] h-[26px] flex items-center justify-center rounded-[var(--radius-sm)] transition-all"
            style={{ color: "rgba(255,255,255,0.4)" }}
            onMouseEnter={(e) => {
              (e.currentTarget as HTMLElement).style.background = "rgba(255,255,255,0.08)";
              (e.currentTarget as HTMLElement).style.color = "rgba(255,255,255,0.7)";
            }}
            onMouseLeave={(e) => {
              (e.currentTarget as HTMLElement).style.background = "transparent";
              (e.currentTarget as HTMLElement).style.color = "rgba(255,255,255,0.4)";
            }}
            title="设置"
            aria-label="打开设置"
          >
            <Settings size={13} />
          </button>
        )}
      </div>
    </header>
  );
}
