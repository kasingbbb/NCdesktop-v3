import { useEffect, useRef } from "react";
import { getSetting } from "../lib/tauri-commands";
import { useProjectStore } from "../stores/projectStore";
import { useAssetStore } from "../stores/assetStore";
import { useUIStore } from "../stores/uiStore";

/** 启动时从数据库恢复上次选中的项目 */
export function useHydrateActiveProjectFromSettings(): void {
  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const id = await getSetting("ui.active_project_id");
        if (cancelled || !id?.trim()) {
          return;
        }
        useProjectStore.getState().setActiveProject(id.trim());
      } catch {
        /* 忽略：无设置或非 Tauri 环境 */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);
}

/** 当前项目变化时拉取素材列表（此前 UI 从未调用 fetchAssets，导致「已入库」却看不到） */
export function useFetchAssetsWhenProjectActive(): void {
  const activeProjectId = useProjectStore((s) => s.activeProjectId);
  const assetTagFilterId = useUIStore((s) => s.assetTagFilterId);
  const fetchAssets = useAssetStore((s) => s.fetchAssets);
  const fetchAssetsByTag = useAssetStore((s) => s.fetchAssetsByTag);
  const setAssetTagFilterId = useUIStore((s) => s.setAssetTagFilterId);
  const setWorkspaceFolderRelativePath = useUIStore(
    (s) => s.setWorkspaceFolderRelativePath
  );
  const prevProjectId = useRef<string | null>(null);

  useEffect(() => {
    if (!activeProjectId) {
      prevProjectId.current = null;
      useAssetStore.setState({
        assets: [],
        assetTagNamesById: {},
        selectedAssetId: null,
        error: null,
      });
      return;
    }

    const prev = prevProjectId.current;
    if (prev !== null && prev !== activeProjectId) {
      setAssetTagFilterId(null);
      setWorkspaceFolderRelativePath(null);
    }
    prevProjectId.current = activeProjectId;

    const tagId = useUIStore.getState().assetTagFilterId;
    if (tagId) {
      void fetchAssetsByTag(activeProjectId, tagId);
    } else {
      void fetchAssets(activeProjectId);
    }
  }, [
    activeProjectId,
    assetTagFilterId,
    fetchAssets,
    fetchAssetsByTag,
    setAssetTagFilterId,
    setWorkspaceFolderRelativePath,
  ]);
}
