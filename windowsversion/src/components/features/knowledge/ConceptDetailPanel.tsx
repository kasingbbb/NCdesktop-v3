/**
 * ConceptDetailPanel — 知识关联右侧概念详情面板
 *
 * 区块：
 *   1. 概念名 + 别名标签
 *   2. 定义区（可编辑，保存后标记 userEdited）
 *   3. 相关观点 Viewpoints（ViewpointCard）
 *   4. 观点案例 Cases（CaseCard）
 *   5. 知识拓展 Extension（ExtensionPanel）
 *
 * 约束（宪章 A1/A2）：named export，CSS 变量
 */

import { useState, useRef } from "react";
import { Edit2, Check, X, Loader2, RefreshCw } from "lucide-react";
import { ViewpointCard } from "./ViewpointCard";
import { CaseCard } from "./CaseCard";
import { ExtensionPanel } from "./ExtensionPanel";
import { DeepUnderstandButton } from "../../KnowledgeUnderstanding/DeepUnderstandButton";
import { FirstVisitTooltip } from "../../KnowledgeUnderstanding/FirstVisitTooltip";
import type { ConceptDetail } from "../../../types/knowledge";

interface Props {
  detail: ConceptDetail;
  isLoading: boolean;
  onUpdateDefinition: (def: string) => void;
  onSynthesizeViewpoints: () => void;
  onGenerateExtensions: () => void;
  onEnterUnderstanding?: (conceptId: string) => void;
}

export function ConceptDetailPanel({
  detail,
  isLoading,
  onUpdateDefinition,
  onSynthesizeViewpoints,
  onGenerateExtensions,
  onEnterUnderstanding,
}: Props) {
  const { concept, viewpoints, cases, extensions } = detail;

  // ── 定义编辑状态 ─────────────────────────────────────────────────────────
  const [editingDef, setEditingDef] = useState(false);
  const [defDraft, setDefDraft] = useState(concept.definition ?? "");
  const defRef = useRef<HTMLTextAreaElement>(null);

  const handleStartEdit = () => {
    setDefDraft(concept.definition ?? "");
    setEditingDef(true);
    setTimeout(() => defRef.current?.focus(), 0);
  };

  const handleSaveDef = () => {
    onUpdateDefinition(defDraft.trim());
    setEditingDef(false);
  };

  const handleCancelEdit = () => {
    setDefDraft(concept.definition ?? "");
    setEditingDef(false);
  };

  // ─────────────────────────────────────────────────────────────────────────
  // 渲染
  // ─────────────────────────────────────────────────────────────────────────

  return (
    <div className="p-[var(--space-5)] space-y-[var(--space-5)]">

      {/* ── 概念名 + 别名 ── */}
      <div>
        <h2
          className="text-[var(--text-lg)] font-bold leading-tight"
          style={{ color: "var(--text-primary)" }}
        >
          {concept.name}
        </h2>
        {concept.aliases.length > 0 && (
          <div className="flex flex-wrap gap-[var(--space-1)] mt-[var(--space-2)]">
            {concept.aliases.map((alias) => (
              <span
                key={alias}
                className="text-[10px] px-[var(--space-2)] py-px rounded-full"
                style={{
                  background: "var(--surface-tertiary)",
                  color: "var(--text-tertiary)",
                  border: "1px solid var(--border-primary)",
                }}
              >
                {alias}
              </span>
            ))}
          </div>
        )}
      </div>

      {/* ── 分隔线 ── */}
      <div className="h-px" style={{ background: "var(--border-primary)" }} />

      {/* ── 定义区 ── */}
      <section>
        <div className="flex items-center justify-between mb-[var(--space-2)]">
          <SectionTitle>定义 Definition</SectionTitle>
          <div className="flex items-center gap-[var(--space-2)]">
            {!editingDef && (
              <button
                type="button"
                onClick={handleStartEdit}
                className="flex items-center gap-1 text-[var(--text-xs)] transition-colors"
                style={{ color: "var(--text-tertiary)" }}
              >
                <Edit2 size={11} />
                编辑
              </button>
            )}
            {onEnterUnderstanding && (
              <div className="relative">
                <DeepUnderstandButton
                  conceptId={concept.id}
                  onEnterUnderstanding={onEnterUnderstanding}
                />
                <FirstVisitTooltip />
              </div>
            )}
          </div>
        </div>

        {editingDef ? (
          <div className="space-y-[var(--space-2)]">
            <textarea
              ref={defRef}
              value={defDraft}
              onChange={(e) => setDefDraft(e.target.value)}
              rows={3}
              className="w-full px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-sm)] resize-none outline-none"
              style={{
                background: "var(--surface-secondary)",
                border: "1px solid var(--border-active)",
                color: "var(--text-primary)",
              }}
            />
            <div className="flex gap-[var(--space-2)]">
              <button
                type="button"
                onClick={handleSaveDef}
                className="flex items-center gap-1 px-[var(--space-3)] py-[var(--space-1)] rounded-[var(--radius-sm)] text-[var(--text-xs)]"
                style={{ background: "var(--brand-navy)", color: "#fff" }}
              >
                <Check size={11} />
                保存
              </button>
              <button
                type="button"
                onClick={handleCancelEdit}
                className="flex items-center gap-1 px-[var(--space-3)] py-[var(--space-1)] rounded-[var(--radius-sm)] text-[var(--text-xs)]"
                style={{
                  background: "var(--surface-secondary)",
                  border: "1px solid var(--border-primary)",
                  color: "var(--text-secondary)",
                }}
              >
                <X size={11} />
                取消
              </button>
            </div>
          </div>
        ) : (
          <div
            className="px-[var(--space-3)] py-[var(--space-3)] rounded-[var(--radius-md)] text-[var(--text-sm)] leading-relaxed"
            style={{
              background: "var(--surface-secondary)",
              border: "1px solid var(--border-primary)",
              color: "var(--text-secondary)",
            }}
          >
            {concept.definition ?? (
              <span style={{ color: "var(--text-tertiary)", fontStyle: "italic" }}>
                暂无定义，点击「编辑」添加
              </span>
            )}
            {concept.userEdited && (
              <span
                className="ml-[var(--space-2)] text-[10px] px-1 py-px rounded"
                style={{
                  background: "var(--surface-tertiary)",
                  color: "var(--text-tertiary)",
                }}
              >
                已编辑
              </span>
            )}
          </div>
        )}

        {/* 空状态引导文字（AC-4） */}
        {onEnterUnderstanding && (
          <p
            className="mt-[var(--space-2)] text-[var(--text-xs)]"
            style={{ color: "var(--text-tertiary)" }}
          >
            想深入理解这个概念？
          </p>
        )}
      </section>

      {/* ── 相关观点 ── */}
      <section>
        <div className="flex items-center justify-between mb-[var(--space-3)]">
          <SectionTitle>
            相关观点 Viewpoints
            {viewpoints.length > 0 && (
              <Count>{viewpoints.length}</Count>
            )}
          </SectionTitle>
          <button
            type="button"
            disabled={isLoading}
            onClick={onSynthesizeViewpoints}
            className="flex items-center gap-1 text-[var(--text-xs)] transition-colors"
            style={{ color: "var(--text-tertiary)" }}
          >
            {isLoading ? (
              <Loader2 size={11} className="animate-spin" />
            ) : (
              <RefreshCw size={11} />
            )}
            {viewpoints.length === 0 ? "生成观点" : "重新生成"}
          </button>
        </div>

        {viewpoints.length === 0 && !isLoading ? (
          <EmptyHint>点击「生成观点」让 AI 分析不同课程视角</EmptyHint>
        ) : isLoading && viewpoints.length === 0 ? (
          <LoadingHint />
        ) : (
          <div className="space-y-[var(--space-2)]">
            {viewpoints.map((vp) => (
              <ViewpointCard key={vp.id} viewpoint={vp} />
            ))}
          </div>
        )}
      </section>

      {/* ── 观点案例 ── */}
      {cases.length > 0 && (
        <section>
          <div className="mb-[var(--space-3)]">
            <SectionTitle>
              观点案例 Cases
              <Count>{cases.length}</Count>
            </SectionTitle>
          </div>
          <div className="space-y-[var(--space-2)]">
            {cases.map((c) => (
              <CaseCard key={c.id} conceptCase={c} />
            ))}
          </div>
        </section>
      )}

      {/* ── 知识拓展 ── */}
      <section>
        <div className="mb-[var(--space-3)]">
          <SectionTitle>知识拓展 Extension</SectionTitle>
        </div>
        <ExtensionPanel
          extensions={extensions}
          isLoading={isLoading}
          onGenerate={onGenerateExtensions}
        />
      </section>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// 小工具组件
// ─────────────────────────────────────────────────────────────────────────────

function SectionTitle({ children }: { children: React.ReactNode }) {
  return (
    <h3
      className="text-[var(--text-sm)] font-semibold flex items-center gap-[var(--space-2)]"
      style={{ color: "var(--text-primary)" }}
    >
      {children}
    </h3>
  );
}

function Count({ children }: { children: React.ReactNode }) {
  return (
    <span
      className="text-[10px] px-[var(--space-2)] py-px rounded-full"
      style={{
        background: "var(--surface-tertiary)",
        color: "var(--text-tertiary)",
        fontWeight: 400,
      }}
    >
      {children}
    </span>
  );
}

function EmptyHint({ children }: { children: React.ReactNode }) {
  return (
    <p
      className="text-[var(--text-xs)] italic"
      style={{ color: "var(--text-tertiary)" }}
    >
      {children}
    </p>
  );
}

function LoadingHint() {
  return (
    <div className="space-y-[var(--space-2)] animate-pulse">
      {[80, 60, 75].map((w, i) => (
        <div
          key={i}
          className="h-3 rounded"
          style={{ width: `${w}%`, background: "var(--surface-tertiary)" }}
        />
      ))}
    </div>
  );
}
