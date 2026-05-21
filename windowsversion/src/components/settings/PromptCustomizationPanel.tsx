/**
 * 用户自定义 Prompt 功能 — 设置面板组件（task_007_dev_frontend_ui）
 *
 * 真相来源：
 *   - PRD § 3.1（UI 草图与文案）
 *   - Architect output.md § 4.5（UI 视觉分层）/ § 5.5（数据流）/ § 7（目录结构）/ ADR-005 / R4
 *
 * 命名隔离（ADR-005 / R6）：
 *   - 组件名 `PromptCustomizationPanel`，与 PR-4 半成品 `PromptEditor.tsx`（kind=classify/naming/tagging）
 *     字面与语义完全独立，不复用。
 *   - 数据源 `useUserPromptStore`（task_006 落地）。
 *
 * 设计要点：
 *   1. 4 个折叠子项按 `PROMPT_MODULES` 固定顺序渲染（tagging → para → concept → aggregation）。
 *   2. mount 时 `useEffect` 调一次 `loadAll()`；折叠状态由本组件 useState 自管，不入 store。
 *   3. textarea 与 store.drafts 双向绑定；dirty / byteLen / placeholder 缺失由 store 与本地派生。
 *   4. 保存按钮三态禁用：缺占位符 / 字节超限 / dirty=false。
 *   5. 字节计数颜色三段：<80% maxBytes 灰 / 80-100% 橙 / >100% 红。
 *   6. 错误显示：每子项操作前清空 store.error；失败时在该子项下方红色横条展示。
 *   7. "全部恢复默认" + 单条"恢复默认"均 `window.confirm` 二次确认（按 AC-1 / AC-2 文案）。
 */

import { useCallback, useEffect, useState } from "react";
import { ChevronDown, ChevronRight, Loader2, AlertTriangle } from "lucide-react";
import { useUserPromptStore } from "../../stores/userPromptStore";
import {
  PROMPT_MODULES,
  PROMPT_MODULE_TITLES,
  type PromptInfo,
  type PromptModule,
} from "../../types/user-prompt";

/** R4 方案 B：折叠头副标题（tagging / para 揭示"合并到同一次分类调用"线索）。 */
const PROMPT_MODULE_SUBTITLES: Partial<Record<PromptModule, string>> = {
  tagging: "与「PARA 分组」共用同一次分类调用，两者同时生效",
  para: "与「文件打标签」共用同一次分类调用，两者同时生效",
};

/** 占位符是否全部满足。 */
function checkPlaceholdersOk(text: string, required: string[]): boolean {
  if (required.length === 0) return true;
  return required.every((p) => text.includes(p));
}

/** 字节计数颜色：按比例分三段。 */
function byteColor(n: number, max: number): string {
  if (n > max) return "#ef4444"; // red-500：超限
  if (n >= max * 0.8) return "#f59e0b"; // amber-500：接近上限
  return "var(--text-tertiary)";
}

export function PromptCustomizationPanel() {
  // ────────────────────────────────────────────────────
  // store hooks（细粒度 selector，避免无关字段变更触发重渲）
  // ────────────────────────────────────────────────────
  const items = useUserPromptStore((s) => s.items);
  const drafts = useUserPromptStore((s) => s.drafts);
  const dirty = useUserPromptStore((s) => s.dirty);
  const error = useUserPromptStore((s) => s.error);
  const loadAll = useUserPromptStore((s) => s.loadAll);
  const setDraft = useUserPromptStore((s) => s.setDraft);
  const save = useUserPromptStore((s) => s.save);
  const reset = useUserPromptStore((s) => s.reset);
  const byteLen = useUserPromptStore((s) => s.byteLen);

  // ────────────────────────────────────────────────────
  // 折叠态：本地 useState 自管（不入 store；input.md 技术约束）
  // 初始全部折叠（AC-1）
  // ────────────────────────────────────────────────────
  const [expanded, setExpanded] = useState<Record<PromptModule, boolean>>({
    tagging: false,
    para: false,
    concept: false,
    aggregation: false,
  });

  // ────────────────────────────────────────────────────
  // 挂载时一次性加载全部 4 条（AC-1）
  // ────────────────────────────────────────────────────
  useEffect(() => {
    void loadAll();
    // 仅 mount 时执行；loadAll 引用稳定（zustand action）。
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const toggleExpanded = useCallback((module: PromptModule) => {
    setExpanded((s) => ({ ...s, [module]: !s[module] }));
  }, []);

  // ────────────────────────────────────────────────────
  // "全部恢复默认" 操作（AC-1）
  // ────────────────────────────────────────────────────
  const handleResetAll = useCallback(async () => {
    const ok = window.confirm(
      "将恢复全部 4 条 Prompt 为内置默认值，已有自定义会丢失。继续？",
    );
    if (!ok) return;
    // 操作前清空 error（AC-4）
    useUserPromptStore.setState({ error: null });
    try {
      await reset(null);
    } catch {
      /* error 已写入 store，UI 自动展示 */
    }
  }, [reset]);

  return (
    <div
      className="space-y-[var(--space-4)]"
      data-testid="prompt-customization-panel"
    >
      {/* 顶部标题 */}
      <h3
        className="text-[var(--text-base)] font-semibold"
        style={{ color: "var(--text-primary)" }}
      >
        Prompt 自定义
      </h3>

      {/* 顶部说明文案（PRD § 3.1） */}
      <div
        className="text-[var(--text-xs)] leading-relaxed"
        style={{ color: "var(--text-secondary)" }}
      >
        <p>以下为系统内置的 AI 处理策略。</p>
        <p>修改后将影响对应功能的输出结果。</p>
        <p>如不确定，请保持默认值。</p>
      </div>

      {/* 全局错误横条（AC-3：loadAll / reset(null) 失败时顶部展示一次，不再每个子项重复） */}
      {error && error.module === null && (
        <div
          data-testid="error-banner-global"
          className="px-[var(--space-2)] py-[var(--space-2)] rounded-[var(--radius-sm)] text-[var(--text-xs)]"
          style={{
            backgroundColor: "rgba(239, 68, 68, 0.08)",
            color: "#dc2626",
            border: "1px solid rgba(239, 68, 68, 0.2)",
          }}
        >
          {error.message}
        </div>
      )}

      {/* 4 个折叠子项 */}
      <div className="space-y-[var(--space-2)]">
        {PROMPT_MODULES.map((module) => (
          <PromptModuleSection
            key={module}
            module={module}
            title={PROMPT_MODULE_TITLES[module]}
            subtitle={PROMPT_MODULE_SUBTITLES[module]}
            item={items[module]}
            draft={drafts[module]}
            isDirty={dirty[module]}
            isExpanded={expanded[module]}
            error={error}
            onToggle={() => toggleExpanded(module)}
            onDraftChange={(text) => setDraft(module, text)}
            onSave={async () => {
              useUserPromptStore.setState({ error: null });
              try {
                await save(module);
              } catch {
                /* error 已写入 store */
              }
            }}
            onReset={async () => {
              const ok = window.confirm(
                `将恢复「${PROMPT_MODULE_TITLES[module]}」为内置默认值。继续？`,
              );
              if (!ok) return;
              useUserPromptStore.setState({ error: null });
              try {
                await reset(module);
              } catch {
                /* error 已写入 store */
              }
            }}
            byteLen={byteLen(module)}
          />
        ))}
      </div>

      {/* 底部「全部恢复默认」 */}
      <div className="flex justify-end pt-[var(--space-2)]">
        <button
          type="button"
          data-testid="reset-all-button"
          onClick={() => void handleResetAll()}
          className="px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-sm)] text-[var(--text-xs)] transition-colors"
          style={{
            backgroundColor: "transparent",
            color: "var(--text-secondary)",
            border: "1px solid var(--border-primary)",
          }}
        >
          全部恢复默认
        </button>
      </div>
    </div>
  );
}

// ──────────────────────────────────────────────────────────────
// 单个 module 折叠子项
// ──────────────────────────────────────────────────────────────

interface PromptModuleSectionProps {
  module: PromptModule;
  title: string;
  /** R4 方案 B：折叠头副标题（tagging/para 有，concept/aggregation 无） */
  subtitle?: string;
  item: PromptInfo | null;
  draft: string;
  isDirty: boolean;
  isExpanded: boolean;
  error: { module: PromptModule | null; message: string } | null;
  onToggle: () => void;
  onDraftChange: (text: string) => void;
  onSave: () => Promise<void>;
  onReset: () => Promise<void>;
  byteLen: number;
}

function PromptModuleSection({
  module,
  title,
  subtitle,
  item,
  draft,
  isDirty,
  isExpanded,
  error,
  onToggle,
  onDraftChange,
  onSave,
  onReset,
  byteLen,
}: PromptModuleSectionProps) {
  const required = item?.requiredPlaceholders ?? [];
  const maxBytes = item?.maxBytes ?? 16384;
  const placeholdersOk = checkPlaceholdersOk(draft, required);
  const overByteLimit = byteLen > maxBytes;
  const isCustom = item?.isCustom ?? false;

  // 保存中（task_007_round2 AC-1）：组件内自管，按 input.md "不在 store 中做 UI 状态"
  const [saving, setSaving] = useState(false);

  // 保存按钮禁用条件（AC-2 顺序）：① 占位符未满足 ② 字节超限 ③ dirty=false ④ saving 中
  const saveDisabled = !placeholdersOk || overByteLimit || !isDirty || saving;

  // 单条"恢复默认"按钮：仅在已自定义时可点
  const resetDisabled = !isCustom;

  // 保存按钮禁用原因（AC-2：title tooltip 解释）
  const saveDisabledReason = !isDirty
    ? "无未保存修改"
    : !placeholdersOk
    ? "占位符未满足"
    : overByteLimit
    ? "字节超出上限"
    : saving
    ? "保存中…"
    : undefined;

  // AC-3：仅当 store.error 归属该 module 时才在子项下方渲染（全局错误由顶部 banner 渲染）
  const moduleError = error && error.module === module ? error.message : null;

  return (
    <div
      data-testid={`prompt-section-${module}`}
      className="rounded-[var(--radius-md)]"
      style={{
        border: "1px solid var(--border-primary)",
        backgroundColor: "var(--surface-secondary)",
      }}
    >
      {/* 折叠头：点击展开 / 收起 */}
      <button
        type="button"
        onClick={onToggle}
        data-testid={`prompt-toggle-${module}`}
        className="w-full flex items-center gap-[var(--space-2)] px-[var(--space-3)] py-[var(--space-2)] text-left"
        aria-expanded={isExpanded}
      >
        {isExpanded ? (
          <ChevronDown size={12} style={{ color: "var(--text-tertiary)" }} />
        ) : (
          <ChevronRight size={12} style={{ color: "var(--text-tertiary)" }} />
        )}
        <div className="flex-1 min-w-0">
          <div
            className="text-[var(--text-sm)] font-medium"
            style={{ color: "var(--text-primary)" }}
          >
            {title}
          </div>
          {/* R4 方案 B：tagging/para 副标题，揭示后端共用调用 */}
          {subtitle && (
            <div
              data-testid={`prompt-subtitle-${module}`}
              className="text-[var(--text-xs)] mt-0.5"
              style={{ color: "var(--text-tertiary)" }}
            >
              {subtitle}
            </div>
          )}
        </div>
        {/* 标题行右侧的状态指示（折叠态也可见，方便用户一眼看到哪些已自定义） */}
        <span
          className="text-[var(--text-xs)] flex items-center gap-1 flex-shrink-0"
          data-testid={`prompt-status-${module}`}
          style={{
            color: isCustom ? "var(--color-accent)" : "var(--text-tertiary)",
          }}
        >
          <span aria-hidden>●</span>
          <span>{isCustom ? "已自定义" : "默认"}</span>
        </span>
      </button>

      {/* 折叠体 */}
      {isExpanded && (
        <div
          className="px-[var(--space-3)] pb-[var(--space-3)] space-y-[var(--space-2)] border-t"
          style={{ borderColor: "var(--border-primary)" }}
        >
          {/* 必含占位符提示（chip） */}
          {required.length > 0 && (
            <div className="pt-[var(--space-2)] flex items-center flex-wrap gap-[var(--space-2)]">
              <span
                className="text-[var(--text-xs)]"
                style={{ color: "var(--text-tertiary)" }}
              >
                必含占位符：
              </span>
              {required.map((p) => (
                <code
                  key={p}
                  className="px-[var(--space-2)] py-0.5 rounded-[var(--radius-sm)] text-[11px] font-mono"
                  style={{
                    backgroundColor: "var(--surface-tertiary)",
                    color: "var(--text-secondary)",
                    border: "1px solid var(--border-primary)",
                  }}
                >
                  {p}
                </code>
              ))}
            </div>
          )}

          {/* textarea */}
          <textarea
            data-testid={`prompt-textarea-${module}`}
            value={draft}
            onChange={(e) => onDraftChange(e.target.value)}
            rows={14}
            aria-label={`${title} 的 Prompt 编辑区`}
            className="w-full font-mono text-[12px] px-[var(--space-2)] py-[var(--space-2)] rounded-[var(--radius-sm)] resize-y"
            style={{
              backgroundColor: "var(--surface-elevated)",
              color: "var(--text-primary)",
              border: "1px solid var(--border-primary)",
              lineHeight: 1.5,
            }}
            spellCheck={false}
          />

          {/* 占位符警告（缺失时） */}
          {!placeholdersOk && (
            <div
              data-testid={`placeholder-warning-${module}`}
              className="text-[var(--text-xs)]"
              style={{ color: "#ef4444" }}
            >
              缺少必含占位符：{required.filter((p) => !draft.includes(p)).join("、")}
              （保存按钮已禁用）
            </div>
          )}

          {/* 字节超限警告（AC-6：独立一行 + AlertTriangle，比同行更醒目） */}
          {overByteLimit && (
            <div
              data-testid={`byte-overflow-warning-${module}`}
              className="flex items-center gap-1 text-[var(--text-xs)]"
              style={{ color: "#ef4444" }}
            >
              <AlertTriangle size={14} aria-hidden />
              <span>已超过 16 KB 上限</span>
            </div>
          )}

          {/* 字节计数 */}
          <div className="flex items-center justify-end">
            <span
              data-testid={`byte-counter-${module}`}
              className="text-[var(--text-xs)] font-mono tabular-nums"
              style={{ color: byteColor(byteLen, maxBytes) }}
            >
              {byteLen} / {maxBytes} 字节
            </span>
          </div>

          {/* 按钮区 */}
          <div className="flex items-center justify-end gap-[var(--space-2)] pt-[var(--space-1)]">
            <button
              type="button"
              data-testid={`reset-button-${module}`}
              disabled={resetDisabled}
              aria-disabled={resetDisabled}
              onClick={() => void onReset()}
              className="px-[var(--space-3)] py-[var(--space-1)] rounded-[var(--radius-sm)] text-[var(--text-xs)] transition-colors"
              style={{
                backgroundColor: "transparent",
                color: resetDisabled ? "var(--text-tertiary)" : "var(--text-secondary)",
                border: "1px solid var(--border-primary)",
                cursor: resetDisabled ? "not-allowed" : "pointer",
                opacity: resetDisabled ? 0.5 : 1,
              }}
            >
              恢复默认
            </button>
            <button
              type="button"
              data-testid={`save-button-${module}`}
              disabled={saveDisabled}
              aria-disabled={saveDisabled}
              title={saveDisabledReason}
              onClick={async () => {
                setSaving(true);
                try {
                  await onSave();
                } finally {
                  setSaving(false);
                }
              }}
              className="inline-flex items-center gap-1 px-[var(--space-3)] py-[var(--space-1)] rounded-[var(--radius-sm)] text-[var(--text-xs)] font-medium transition-colors"
              style={{
                backgroundColor: saveDisabled ? "var(--surface-tertiary)" : "var(--color-accent)",
                color: saveDisabled ? "var(--text-tertiary)" : "#ffffff",
                border: "1px solid transparent",
                cursor: saveDisabled ? "not-allowed" : "pointer",
                opacity: saveDisabled ? 0.6 : 1,
              }}
            >
              {saving && <Loader2 size={12} className="animate-spin" aria-hidden />}
              <span>{saving ? "保存中…" : "保存"}</span>
            </button>
          </div>

          {/* 错误横条（AC-3：仅在 store.error 归属本 module 时渲染；全局错误由顶部 banner 渲染） */}
          {moduleError && (
            <div
              data-testid={`error-banner-${module}`}
              className="px-[var(--space-2)] py-[var(--space-2)] rounded-[var(--radius-sm)] text-[var(--text-xs)]"
              style={{
                backgroundColor: "rgba(239, 68, 68, 0.08)",
                color: "#dc2626",
                border: "1px solid rgba(239, 68, 68, 0.2)",
              }}
            >
              {moduleError}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
