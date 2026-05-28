import { Settings, Search } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";

interface TitleBarProps {
  onSettingsOpen?: () => void;
  onSearchOpen?: () => void;
}

export function TitleBar({ onSettingsOpen, onSearchOpen }: TitleBarProps) {
  const activeProject = useProjectStore((s) => s.getActiveProject());

  return (
    <header className="titlebar-drag-region glass-titlebar flex items-center h-[56px] px-[var(--space-4)] relative">
      {/* macOS 红绿灯留白：系统按钮由 Tauri Overlay titleBarStyle 渲染，这里只保留宽度占位 */}
      <div className="w-[80px] shrink-0" aria-hidden />

      {/* 面包屑（容器本身保留可拖拽；按钮/链接/输入已由 .titlebar-drag-region CSS 自动 no-drag） */}
      <div className="flex-1 flex items-center justify-center gap-[6px] text-[12px] tracking-[0.02em]">
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

      {/* 右侧工具区（按钮自身已 no-drag，容器留可拖以扩大拖拽面积） */}
      <div className="w-[80px] shrink-0 flex items-center justify-end gap-[4px] pr-[12px]">
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
