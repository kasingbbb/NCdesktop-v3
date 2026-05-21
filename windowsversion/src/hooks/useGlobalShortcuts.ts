import { useEffect } from "react";

interface UseGlobalShortcutsOptions {
  onSearchOpen: () => void;
  onNewProject?: () => void;
  onToggleInspector?: () => void;
}

export function useGlobalShortcuts({
  onSearchOpen,
  onNewProject,
  onToggleInspector,
}: UseGlobalShortcutsOptions): void {
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent): void {
      const mod = e.metaKey || e.ctrlKey;

      // ⌘K — 全局搜索
      if (mod && !e.shiftKey && e.key.toLowerCase() === "k") {
        e.preventDefault();
        onSearchOpen();
        return;
      }

      // ⇧⌘D — 切换悬浮窗
      if (mod && e.shiftKey && e.key.toLowerCase() === "d") {
        e.preventDefault();
        import("@tauri-apps/api/core").then(({ invoke }) => {
          invoke("toggle_dropzone_window").catch(console.error);
        });
        return;
      }

      // ⌘N — 新建项目
      if (mod && e.key === "n" && onNewProject) {
        e.preventDefault();
        onNewProject();
        return;
      }

      // ⌘I — Inspector 面板
      if (mod && e.key === "i" && onToggleInspector) {
        e.preventDefault();
        onToggleInspector();
        return;
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onSearchOpen, onNewProject, onToggleInspector]);
}
