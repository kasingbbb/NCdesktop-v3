/**
 * AssetsStep — KnowledgeHubView 第 1 step（assets）
 *
 * 薄 wrapper：根据 activeProjectId 渲染 ProjectListView 或 AssetListView，
 * 与 ContentArea 的 isLibraryView 分支保持一致语义。本期不重写。
 */

import { useProjectStore } from "../../../../stores/projectStore";
import { ProjectListView } from "../../ProjectListView";
import { AssetListView } from "../../AssetListView";

export function AssetsStep() {
  const activeProjectId = useProjectStore((s) => s.activeProjectId);
  return activeProjectId ? <AssetListView /> : <ProjectListView />;
}
