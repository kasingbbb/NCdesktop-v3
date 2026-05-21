import { lazy, Suspense, useCallback, useMemo, useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { AppLayout } from "./components/layout/AppLayout";
import { useGlobalShortcuts } from "./hooks/useGlobalShortcuts";
import {
  useHydrateActiveProjectFromSettings,
  useFetchAssetsWhenProjectActive,
} from "./hooks/useProjectWorkspaceSync";
import { useUIStore } from "./stores/uiStore";
import { DropzoneApp } from "./components/features/dropzone/DropzoneApp";
import { useLibraryStore } from "./stores/libraryStore";
import { useProjectStore } from "./stores/projectStore";
import { useAssetStore } from "./stores/assetStore";
import { useSettingsStore } from "./stores/settingsStore";
import { logger } from "./utils/logger";

interface ImportDropFinishedPayload {
  projectId: string;
  importProjectName: string;
}

const SearchPanel = lazy(() =>
  import("./components/features/SearchPanel").then((m) => ({ default: m.SearchPanel }))
);
const SettingsPanel = lazy(() =>
  import("./components/features/SettingsPanel").then((m) => ({ default: m.SettingsPanel }))
);

export default function App() {
  const isDropzone = useMemo(() => window.location.pathname === "/dropzone", []);

  const [searchOpen, setSearchOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const toggleInspector = useUIStore((s) => s.toggleInspector);
  const activeLibraryId = useLibraryStore((s) => s.activeLibraryId);
  const ensureActiveLibrary = useLibraryStore((s) => s.ensureActiveLibrary);
  const createProject = useProjectStore((s) => s.createProject);
  const setActiveProject = useProjectStore((s) => s.setActiveProject);

  const handleSearchOpen = useCallback(() => setSearchOpen(true), []);
  const handleNewProject = useCallback(() => {
    void (async () => {
      const libId = activeLibraryId ?? (await ensureActiveLibrary());
      const now = new Date();
      const name = `新建项目 ${now.toLocaleString()}`;
      const project = await createProject(libId, name);
      setActiveProject(project.id);
    })();
  }, [activeLibraryId, ensureActiveLibrary, createProject, setActiveProject]);

  useGlobalShortcuts({
    onSearchOpen: handleSearchOpen,
    onToggleInspector: toggleInspector,
    onNewProject: handleNewProject,
  });

  useEffect(() => {
    logger.info("App", "Application mounted", { isDropzone });
  }, [isDropzone]);

  // 启动时加载持久化设置并应用主题
  useEffect(() => {
    if (isDropzone) return;
    void useSettingsStore
      .getState()
      .loadSettings()
      .then(() => {
        const settings = useSettingsStore.getState().settings;
        const theme = settings.theme;
        if (theme === "dark") {
          document.documentElement.setAttribute("data-theme", "dark");
        } else if (theme === "light") {
          document.documentElement.removeAttribute("data-theme");
        } else {
          const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
          if (prefersDark) {
            document.documentElement.setAttribute("data-theme", "dark");
          } else {
            document.documentElement.removeAttribute("data-theme");
          }
        }

        const section = useUIStore.getState().activeSidebarSection;
        const calendarLocked = !settings.showStudentCenter && section === "calendar";
        const knowledgeLocked =
          !settings.showLearningFeatures && (section === "today" || section === "knowledge-hub");
        if (calendarLocked || knowledgeLocked) {
          useUIStore.getState().setSidebarSection("recent");
        }
      });
  }, [isDropzone]);

  useHydrateActiveProjectFromSettings();
  useFetchAssetsWhenProjectActive();

  useEffect(() => {
    if (isDropzone) return;

    const handleRefresh = (projectId: string) => {
      const tagId = useUIStore.getState().assetTagFilterId;
      if (tagId) {
        void useAssetStore.getState().fetchAssetsByTag(projectId, tagId);
      } else {
        void useAssetStore.getState().fetchAssets(projectId);
      }
      void (async () => {
        const lib = useLibraryStore.getState();
        const libId = lib.activeLibraryId ?? (await lib.ensureActiveLibrary());
        await useProjectStore.getState().fetchProjects(libId);
      })();
    };

    let unlistenImport: (() => void) | undefined;
    let unlistenAI: (() => void) | undefined;
    let unlistenConverted: (() => void) | undefined;
    let cancelled = false;

    void listen<ImportDropFinishedPayload>("notecapt/import-drop-finished", (event) => {
      const { projectId } = event.payload;
      useProjectStore.getState().setActiveProject(projectId);
      handleRefresh(projectId);
    }).then((fn) => {
      if (!cancelled) unlistenImport = fn;
    });

    void listen<{ assetId: string; projectId: string }>("notecapt/dropzone-ai-finished", (event) => {
      const { projectId } = event.payload;
      handleRefresh(projectId);
    }).then((fn) => {
      if (!cancelled) unlistenAI = fn;
    });

    // 文件转换 v1.1：后端物化衍生 .md 后发出此事件，触发当前项目列表刷新，
    // 用户无需手动按钮即可看到「转换自 xxx」的新 Asset。
    void listen<{ sourceAssetId: string; derivedAssetId: string; projectId: string }>(
      "notecapt/asset-converted",
      (event) => {
        handleRefresh(event.payload.projectId);
      },
    ).then((fn) => {
      if (!cancelled) unlistenConverted = fn;
    });

    return () => {
      cancelled = true;
      unlistenImport?.();
      unlistenAI?.();
      unlistenConverted?.();
    };
  }, [isDropzone]);

  if (isDropzone) {
    return <DropzoneApp />;
  }

  return (
    <>
      <AppLayout
        onSettingsOpen={() => setSettingsOpen(true)}
        onSearchOpen={handleSearchOpen}
      />

      <Suspense fallback={null}>
        {searchOpen && (
          <SearchPanel
            isOpen={searchOpen}
            onClose={() => setSearchOpen(false)}
          />
        )}
        {settingsOpen && (
          <SettingsPanel onClose={() => setSettingsOpen(false)} />
        )}
      </Suspense>
    </>
  );
}
