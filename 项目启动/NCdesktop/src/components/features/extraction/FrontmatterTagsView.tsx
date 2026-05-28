/**
 * task_017_frontmatter_renderer_dep — KC 标签展示组件
 *
 * 接收 frontmatter 解析后的 aiTags + ruleTags，渲染：
 *   "#AI #机器学习 #深度学习 [AI ⓘ]"
 *
 * 设计要点：
 * - 纯展示组件，无副作用（无 store / 无 IPC / 无 useEffect）。
 * - AI 标签优先显示在前，规则标签紧随其后；同名去重（AI 标签为准）。
 * - 有 AI 标签 → 渲染 [AI ⓘ] 角标 + tooltip 说明"由 KC LLM 生成"。
 * - 全空 → "（无标签）"。
 *
 * 不接入 Inspector/DocumentViewer（task_018/019 负责接入）。
 */
import { Info } from "lucide-react";

interface FrontmatterTagsViewProps {
  aiTags?: string[];
  ruleTags?: string[];
}

export function FrontmatterTagsView({ aiTags, ruleTags }: FrontmatterTagsViewProps) {
  const ai = aiTags ?? [];
  const rule = ruleTags ?? [];

  if (ai.length === 0 && rule.length === 0) {
    return (
      <div
        className="text-[var(--text-xs)]"
        style={{ color: "var(--text-tertiary)" }}
        data-testid="frontmatter-tags-empty"
      >
        （无标签）
      </div>
    );
  }

  // 去重：AI 标签优先，规则标签去掉与 AI 重叠部分
  const aiSet = new Set(ai);
  const dedupedRule = rule.filter((t) => !aiSet.has(t));

  return (
    // task_018 AC-5 (TD-2)：根元素 role="list" 让 SR 把整个标签条视为列表；
    // 每个 tag chip role="listitem" + aria-label 区分 "AI 标签" / "规则标签" 来源。
    <div
      className="flex flex-wrap items-center gap-[var(--space-1)]"
      data-testid="frontmatter-tags"
      role="list"
      aria-label="文档标签"
    >
      <span
        className="text-[var(--text-xs)] mr-[var(--space-1)]"
        style={{ color: "var(--text-secondary)" }}
        aria-hidden="true"
      >
        标签：
      </span>

      {ai.map((tag) => (
        <span
          key={`ai-${tag}`}
          className="tag-pill"
          data-testid="tag-ai"
          data-source="ai"
          role="listitem"
          aria-label={`AI 标签 ${tag}`}
        >
          #{tag}
        </span>
      ))}

      {dedupedRule.map((tag) => (
        <span
          key={`rule-${tag}`}
          className="tag-pill"
          data-testid="tag-rule"
          data-source="rule"
          style={{ opacity: 0.85 }}
          role="listitem"
          aria-label={`规则标签 ${tag}`}
        >
          #{tag}
        </span>
      ))}

      {ai.length > 0 ? (
        <span
          className="inline-flex items-center gap-1 text-[var(--text-xs)] ml-[var(--space-1)]"
          style={{ color: "var(--text-tertiary)" }}
          title="标签由 KC LLM 生成"
          data-testid="ai-marker"
        >
          AI
          <Info size={12} />
        </span>
      ) : null}
    </div>
  );
}
