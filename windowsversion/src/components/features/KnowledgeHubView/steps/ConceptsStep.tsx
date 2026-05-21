/**
 * ConceptsStep — KnowledgeHubView 第 2 step（concepts）
 *
 * 薄 wrapper：复用既有 KnowledgeAssociationView（其内部已组合 ConceptList +
 * 详情面板），符合 input.md AC-8「不内部重写」。
 */

import { KnowledgeAssociationView } from "../../knowledge/KnowledgeAssociationView";

export function ConceptsStep() {
  return <KnowledgeAssociationView />;
}
