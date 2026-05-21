import { useState, useEffect, useCallback } from "react";
import { TitleBar } from "./TitleBar";
import { Sidebar } from "./Sidebar";
import { ContentArea } from "./ContentArea";
import { Inspector } from "./Inspector";
import { ResizeHandle } from "./ResizeHandle";
import { useResizable } from "../../hooks/useResizable";
import { logger } from "../../utils/logger";
import { useUIStore } from "../../stores/uiStore";

type LayoutMode = "three-column" | "two-column" | "single-column";

interface AppLayoutProps {
  onSettingsOpen?: () => void;
  onSearchOpen?: () => void;
}

export function AppLayout({
  onSettingsOpen,
  onSearchOpen,
}: AppLayoutProps) {
  const inspectorOpen = useUIStore((s) => s.inspectorOpen);
  const setInspectorOpen = useUIStore((s) => s.setInspectorOpen);
  const [layoutMode, setLayoutMode] = useState<LayoutMode>(() => {
    const w = window.innerWidth;
    if (w >= 1200) return "three-column";
    if (w >= 700) return "two-column";
    return "single-column";
  });

  const sidebar = useResizable({
    initialWidth: 220,
    minWidth: 160,
    maxWidth: 300,
  });

  /** 第三栏（Inspector / 时间流）左缘拖拽：向左拖加宽，向右拖变窄 */
  const inspectorPanel = useResizable({
    initialWidth: 320,
    minWidth: 260,
    maxWidth: 960,
    direction: "left",
  });

  const handleResize = useCallback(() => {
    const w = window.innerWidth;
    if (w >= 1200) {
      setLayoutMode((prev) => {
        if (prev !== "three-column") logger.info("AppLayout", "Layout changed", { mode: "three-column" });
        return "three-column";
      });
    } else if (w >= 700) {
      setLayoutMode((prev) => {
        if (prev !== "two-column") logger.info("AppLayout", "Layout changed", { mode: "two-column" });
        return "two-column";
      });
      setInspectorOpen(false);
    } else {
      setLayoutMode((prev) => {
        if (prev !== "single-column") logger.info("AppLayout", "Layout changed", { mode: "single-column" });
        return "single-column";
      });
      setInspectorOpen(false);
    }
  }, [setInspectorOpen]);

  useEffect(() => {
    handleResize();
  }, [handleResize]);

  useEffect(() => {
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, [handleResize]);

  const showSidebar = layoutMode !== "single-column";
  const showInspector = layoutMode === "three-column" && inspectorOpen;

  return (
    <div
      className="flex flex-col h-screen w-screen overflow-hidden"
      style={{ background: "var(--surface-canvas)" }}
    >
      <TitleBar onSettingsOpen={onSettingsOpen} onSearchOpen={onSearchOpen} />

      <div className="flex flex-1 overflow-hidden">
        {showSidebar && (
          <>
            <Sidebar
              width={sidebar.width}
              onSettingsOpen={onSettingsOpen}
              onSearchOpen={onSearchOpen}
            />
            <ResizeHandle
              onMouseDown={sidebar.handleMouseDown}
              isResizing={sidebar.isResizing}
            />
          </>
        )}

        <ContentArea onSearchOpen={onSearchOpen} />

        {showInspector && (
          <>
            <ResizeHandle
              onMouseDown={inspectorPanel.handleMouseDown}
              isResizing={inspectorPanel.isResizing}
            />
            <Inspector width={inspectorPanel.width} />
          </>
        )}
      </div>
    </div>
  );
}
