/**
 * RelationNetworkSection — 概念关系网络卡片列表
 *
 * 显示与当前概念共现的关联概念，以及 v2.1 的 upstream/downstream 关系。
 * 点击关联概念可导航到该概念详情页。
 */

import { useEffect, useState } from "react";
import { Link2, ArrowUpRight, ArrowDownRight, AlertCircle } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { useKnowledgeUnderstandingStore } from "../../stores/knowledgeUnderstandingStore";
import type { ConceptRelationResult } from "../../types/knowledge-understanding.types";

interface RelationNetworkSectionProps {
  conceptId: string;
  onNavigateToConcept: (conceptId: string) => void;
}

export function RelationNetworkSection({
  conceptId,
  onNavigateToConcept,
}: RelationNetworkSectionProps) {
  const relations = useKnowledgeUnderstandingStore((s) => s.relations);
  const setRelations = useKnowledgeUnderstandingStore((s) => s.setRelations);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const loadRelations = async () => {
      try {
        const data = await invoke<ConceptRelationResult[]>(
          "knowledge_get_relations",
          { conceptId }
        );
        setRelations(data);
        setError(null);
      } catch (e) {
        setError(String(e));
      }
    };

    void loadRelations();
  }, [conceptId]);

  return (
    <section className="space-y-[var(--space-3)]">
      <h3
        className="text-[var(--text-sm)] font-semibold"
        style={{ color: "var(--text-primary)" }}
      >
        在你的知识库里，这个概念连接了：
      </h3>

      {error && (
        <div
          className="flex items-center gap-[var(--space-2)] px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-xs)]"
          style={{
            background: "rgba(239,68,68,0.06)",
            border: "1px solid rgba(239,68,68,0.15)",
            color: "var(--text-secondary)",
          }}
        >
          <AlertCircle size={12} />
          加载失败，请刷新
        </div>
      )}

      {!error && relations.length === 0 && (
        <p
          className="text-[var(--text-xs)]"
          style={{ color: "var(--text-tertiary)" }}
        >
          暂时还没发现相关概念。随着你导入更多文档，关联会逐渐丰富。
        </p>
      )}

      {!error && relations.length > 0 && (
        <div className="space-y-[var(--space-2)]">
          {relations.map((relation) => (
            <RelationCard
              key={relation.id}
              relation={relation}
              onNavigate={onNavigateToConcept}
            />
          ))}
        </div>
      )}
    </section>
  );
}

function RelationCard({
  relation,
  onNavigate,
}: {
  relation: ConceptRelationResult;
  onNavigate: (conceptId: string) => void;
}) {
  const borderColor =
    relation.relationType === "upstream"
      ? "rgba(59, 130, 246, 0.3)"
      : relation.relationType === "downstream"
        ? "rgba(34, 197, 94, 0.3)"
        : "var(--border-primary)";

  const tagBg =
    relation.relationType === "upstream"
      ? "rgba(59, 130, 246, 0.1)"
      : relation.relationType === "downstream"
        ? "rgba(34, 197, 94, 0.1)"
        : undefined;

  const tagColor =
    relation.relationType === "upstream"
      ? "rgb(59, 130, 246)"
      : relation.relationType === "downstream"
        ? "rgb(34, 197, 94)"
        : undefined;

  return (
    <div
      className="px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-md)] transition-colors cursor-pointer"
      style={{
        background: "var(--surface-secondary)",
        border: `1px solid ${borderColor}`,
      }}
      onClick={() => onNavigate(relation.otherConceptId)}
    >
      <div className="flex items-center gap-[var(--space-2)]">
        <RelationIcon type={relation.relationType} />
        <span
          className="text-[var(--text-sm)] font-medium"
          style={{ color: "var(--brand-navy)" }}
        >
          {relation.otherConceptName}
        </span>
        {relation.relationType !== "co_occurrence" && tagBg && tagColor && (
          <span
            className="text-[10px] px-[var(--space-2)] py-px rounded-full"
            style={{ background: tagBg, color: tagColor }}
          >
            {relation.relationType === "upstream" ? "前置知识" : "应用方向"}
          </span>
        )}
      </div>
      <p
        className="text-[var(--text-xs)] mt-[var(--space-1)]"
        style={{ color: "var(--text-tertiary)" }}
      >
        {formatRelationDescription(relation)}
      </p>
    </div>
  );
}

function RelationIcon({ type }: { type: string }) {
  if (type === "upstream") return <ArrowUpRight size={12} style={{ color: "rgb(59, 130, 246)" }} />;
  if (type === "downstream") return <ArrowDownRight size={12} style={{ color: "rgb(34, 197, 94)" }} />;
  return <Link2 size={12} style={{ color: "var(--text-tertiary)" }} />;
}

function formatRelationDescription(relation: ConceptRelationResult): string {
  if (relation.relationType === "upstream") return "前置知识";
  if (relation.relationType === "downstream") return "应用方向";

  const assetIds = relation.sourceAssetIds;
  if (assetIds.length === 0) {
    return `一起出现在 ${relation.coOccurrenceCount} 个文档中`;
  }
  if (assetIds.length <= 2) {
    return `一起出现在 ${assetIds.join("、")}`;
  }
  return `一起出现在 ${assetIds.slice(0, 2).join("、")} 等 ${assetIds.length} 个文档`;
}
