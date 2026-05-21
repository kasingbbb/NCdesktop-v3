/**
 * SkillsStep — KnowledgeHubView 第 4 step（skills）
 *
 * 薄 wrapper：内联渲染既有 SkillsView（不删原文件，PRD §8）。
 * 当 libraryId === null 时显示空态文字（AC-9）。
 */

import { SkillsView } from "../../skills/SkillsView";

interface Props {
  libraryId: string | null;
}

export function SkillsStep({ libraryId }: Props) {
  if (!libraryId) {
    return (
      <div className="p-4 text-[var(--text-sm)]" style={{ color: "var(--text-tertiary)" }}>
        请先选择一个知识库
      </div>
    );
  }
  return <SkillsView libraryId={libraryId} />;
}
