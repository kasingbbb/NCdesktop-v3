/**
 * ExtensionPanel — 知识拓展面板
 *
 * 样式：surface-tertiary 背景 + 虚线边框 + 上下游两区块
 * Coming Soon 状态（v2.1 scope 不做点击深入）
 * 约束（宪章 A1/A2）：named export，CSS 变量
 */

import type { ConceptExtension } from "../../../types/knowledge";

interface Props {
  extensions: ConceptExtension[];
  isLoading: boolean;
  onGenerate: () => void;
}

export function ExtensionPanel({ extensions, isLoading, onGenerate }: Props) {
  const upstream = extensions.filter((e) => e.direction === "upstream");
  const downstream = extensions.filter((e) => e.direction === "downstream");
  const isEmpty = extensions.length === 0;

  return (
    <div
      className="rounded-[var(--radius-md)] p-[var(--space-3)]"
      style={{
        background: "var(--surface-tertiary)",
        border: "1.5px dashed var(--border-primary)",
      }}
    >
      {isEmpty && !isLoading ? (
        /* 未生成状态 */
        <div className="flex items-center justify-between">
          <span
            className="text-[var(--text-xs)]"
            style={{ color: "var(--text-tertiary)" }}
          >
            尚未生成知识拓展
          </span>
          <button
            type="button"
            onClick={onGenerate}
            className="text-[var(--text-xs)] px-[var(--space-2)] py-px rounded transition-colors"
            style={{
              color: "var(--brand-navy)",
              border: "1px solid var(--brand-navy)",
            }}
          >
            生成
          </button>
        </div>
      ) : isLoading ? (
        <div className="animate-pulse space-y-[var(--space-2)]">
          {[70, 50, 80, 55].map((w, i) => (
            <div
              key={i}
              className="h-3 rounded"
              style={{ width: `${w}%`, background: "var(--surface-secondary)" }}
            />
          ))}
        </div>
      ) : (
        <div className="space-y-[var(--space-3)]">
          {/* 前置知识 */}
          {upstream.length > 0 && (
            <div>
              <p
                className="text-[var(--text-xs)] font-semibold mb-[var(--space-2)]"
                style={{ color: "var(--text-secondary)" }}
              >
                ⬆ 前置知识 Prerequisites
              </p>
              <div className="space-y-[var(--space-1)]">
                {upstream.map((ext) => (
                  <ExtensionItem key={ext.id} ext={ext} />
                ))}
              </div>
            </div>
          )}

          {/* 应用方向 */}
          {downstream.length > 0 && (
            <div>
              <p
                className="text-[var(--text-xs)] font-semibold mb-[var(--space-2)]"
                style={{ color: "var(--text-secondary)" }}
              >
                ⬇ 应用方向 Applications
              </p>
              <div className="space-y-[var(--space-1)]">
                {downstream.map((ext) => (
                  <ExtensionItem key={ext.id} ext={ext} />
                ))}
              </div>
            </div>
          )}

          {/* Coming Soon 提示 */}
          <p
            className="text-[10px] text-right"
            style={{ color: "var(--text-tertiary)" }}
          >
            点击深入学习 — Coming Soon
          </p>
        </div>
      )}
    </div>
  );
}

function ExtensionItem({ ext }: { ext: ConceptExtension }) {
  return (
    <div className="flex items-start gap-[var(--space-2)]">
      <span style={{ color: "var(--text-tertiary)", flexShrink: 0 }}>·</span>
      <div className="min-w-0">
        <span
          className="text-[var(--text-xs)] font-medium"
          style={{ color: "var(--text-primary)" }}
        >
          {ext.name}
        </span>
        {ext.description && (
          <span
            className="text-[var(--text-xs)] ml-[var(--space-1)]"
            style={{ color: "var(--text-tertiary)" }}
          >
            — {ext.description}
          </span>
        )}
      </div>
    </div>
  );
}
