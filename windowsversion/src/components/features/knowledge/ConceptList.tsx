/**
 * ConceptList — 知识关联左侧概念列表
 *
 * - 搜索过滤后实时渲染
 * - 激活态高亮
 * - 加载骨架屏
 * - 空状态提示
 *
 * v1.3 task_009 IN-04（fix 回填）：每条概念右侧加 "合并" disabled 占位按钮。
 *   实际合并 modal 业务逻辑推 v1.4；本期仅 UI 占位 + data-merge-id 便于 e2e。
 *
 * 注意：原外层是 `<button>`，无法嵌套 `<button>`。改为 `<div role="button" tabIndex={0}>`
 * 并显式处理键盘事件（Enter / Space），保持 a11y 一致。
 *
 * 约束（宪章 A1/A2）：named export，CSS 变量
 */

import { Pin } from "lucide-react";
import type { ConceptWithStats } from "../../../types/knowledge";

interface Props {
  concepts: ConceptWithStats[];
  selectedId: string | null;
  isLoading: boolean;
  onSelect: (id: string | null) => void;
}

export function ConceptList({ concepts, selectedId, isLoading, onSelect }: Props) {
  // 加载骨架
  if (isLoading && concepts.length === 0) {
    return (
      <div className="p-[var(--space-2)] space-y-[var(--space-1)] animate-pulse">
        {Array.from({ length: 8 }).map((_, i) => (
          <div
            key={i}
            className="h-8 rounded-[var(--radius-sm)]"
            style={{ background: "var(--surface-tertiary)", opacity: 1 - i * 0.1 }}
          />
        ))}
      </div>
    );
  }

  // 空状态
  if (concepts.length === 0) {
    return (
      <div className="flex items-center justify-center h-32 px-[var(--space-3)]">
        <p
          className="text-[var(--text-xs)] text-center"
          style={{ color: "var(--text-tertiary)" }}
        >
          无匹配概念
        </p>
      </div>
    );
  }

  return (
    <div className="py-[var(--space-1)]">
      {concepts.map((concept) => {
        const isActive = selectedId === concept.id;
        return (
          <div
            key={concept.id}
            role="button"
            tabIndex={0}
            onClick={() => onSelect(concept.id)}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                onSelect(concept.id);
              }
            }}
            className="w-full flex items-start gap-[var(--space-2)] px-[var(--space-3)] py-[var(--space-2)] text-left transition-colors cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-accent)]"
            style={{
              background: isActive
                ? "var(--sidebar-active-bg, var(--surface-tertiary))"
                : "transparent",
            }}
          >
            <span
              className="flex-shrink-0 mt-0.5"
              style={{ color: isActive ? "var(--brand-navy)" : "var(--text-tertiary)" }}
            >
              <Pin size={13} aria-hidden />
            </span>

            <div className="min-w-0 flex-1">
              <p
                className="text-[var(--text-md)] font-medium truncate leading-5"
                style={{
                  color: isActive ? "var(--text-primary)" : "var(--text-secondary)",
                }}
                title={concept.name}
              >
                {concept.name}
              </p>

              <p
                className="text-[10px] mt-0.5"
                style={{ color: "var(--text-tertiary)" }}
              >
                {concept.sourceProjectCount > 0
                  ? `${concept.sourceProjectCount} 个项目引用`
                  : ""}
                {concept.viewpointCount > 0
                  ? ` · ${concept.viewpointCount} 个观点`
                  : ""}
              </p>
            </div>

            {concept.userEdited && (
              <span
                className="flex-shrink-0 text-[9px] px-1 py-px rounded mt-0.5"
                style={{
                  background: "var(--surface-tertiary)",
                  color: "var(--text-tertiary)",
                  border: "1px solid var(--border-primary)",
                }}
              >
                已编辑
              </span>
            )}

            {/* v1.3 task_009 IN-04：合并按钮 disabled 占位（v1.4 接入合并 modal） */}
            <button
              type="button"
              disabled
              data-merge-id={concept.id}
              title="v1.4 合并 modal 待开"
              onClick={(e) => e.stopPropagation()}
              className="flex-shrink-0 text-[10px] px-[var(--space-2)] py-px rounded-[var(--radius-sm)] cursor-not-allowed disabled:opacity-50"
              style={{
                color: "var(--text-tertiary)",
                border: "1px solid var(--border-primary)",
                background: "var(--surface-primary)",
              }}
            >
              合并
            </button>
          </div>
        );
      })}
    </div>
  );
}
