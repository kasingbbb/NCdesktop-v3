/**
 * LibraryStep — KnowledgeHubView 第 3 step（library）
 *
 * 薄 wrapper：内联渲染既有 KnowledgeLibraryView（不删原文件，PRD §8）。
 */

import { KnowledgeLibraryView } from "../../knowledge/KnowledgeLibraryView";

export function LibraryStep() {
  return <KnowledgeLibraryView />;
}
