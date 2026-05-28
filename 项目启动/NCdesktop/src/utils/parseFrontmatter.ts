/**
 * task_017_frontmatter_renderer_dep — 衍生件 .md 头部 frontmatter 解析器
 *
 * 设计要点：
 * 1) 安全：用 js-yaml 的 JSON_SCHEMA，只允许 null/bool/int/float/string/seq/map，
 *    禁止 !!js/* / !!python/* 等可执行 tag，杜绝 RCE 攻击面（即便 YAML 来自落地的
 *    .md，仍要按"不可信输入"对待，因为 KC 子进程或第三方编辑器都能写入）。
 * 2) 容错：YAML 解析失败时返回 { frontmatter: null, body: 原文 }，不 throw —
 *    failure 时 Inspector/DocumentViewer 仍可降级为原始 markdown 渲染。
 * 3) 字段映射：YAML 用 snake_case（NC + KC v6 schema），TS 接口用 camelCase。
 *    显式 whitelist 映射，避免 prototype pollution + 漏类型字段。
 *
 * 参考：
 * - Architect output.md §"Frontmatter Schema（衍生件 .md 头部）"
 * - KC v6 schema → intel/knowledge_compiler.md §4.4
 */
import yaml from "js-yaml";

/** 解析后的 frontmatter（NC schema 主键 + KC v6 扩展字段） */
export interface ParsedFrontmatter {
  // NC schema 主键
  sourceAssetId?: string;
  derivativeVersion?: number;
  extractorType?: string;
  qualityLevel?: number;
  extractedAt?: string;
  // KC 扩展字段
  kcDocId?: string;
  kcVersion?: string;
  kcGeneratedAt?: string;
  kcTagsSource?: "ai+rule" | "rule_only";
  kcEnriched?: "true" | "partial" | "false";
  aiTags?: string[];
  ruleTags?: string[];
  aiSummary?: string;
  aiQaPairsCount?: number;
  paragraphCount?: number;
}

export interface ParseResult {
  /** null = 没有 frontmatter 或 parse 失败 */
  frontmatter: ParsedFrontmatter | null;
  /** 去掉 frontmatter 后的正文（解析失败时返回 markdown 原文） */
  body: string;
  /** 解析失败时携带错误信息（用于日志/降级 UI 提示） */
  parseError?: string;
}

/**
 * 匹配文件头部 `---\n[yaml]\n---\n[body]` 块。
 *
 * - 首字符必须是 `---\n`（不容忍前导空白，避免误吞普通 markdown）
 * - YAML 内容可为空（`---\n---` 是合法的空 frontmatter）
 * - YAML 末尾换行 + 闭合 `---` 后换行均为可选
 *
 * 注：闭合 `---` 必须出现在行首（前面是换行或字符串起点），这样普通 markdown
 * 正文里的 `---` 水平线不会被吞——因为我们已经先消费了开头的 `---\n`，
 * 后续 `[\s\S]*?` 的非贪婪 + 显式 `\r?\n?---` 锚点保证只匹配到最近的行首 `---`。
 */
const FRONTMATTER_RE = /^---\r?\n([\s\S]*?)\r?\n?---\r?\n?([\s\S]*)$/;

/** snake_case → camelCase 字段白名单 + 类型守卫 */
function mapToCamelCase(raw: Record<string, unknown>): ParsedFrontmatter {
  const out: ParsedFrontmatter = {};

  // 字符串字段
  if (typeof raw.source_asset_id === "string") out.sourceAssetId = raw.source_asset_id;
  if (typeof raw.extractor_type === "string") out.extractorType = raw.extractor_type;
  if (typeof raw.extracted_at === "string") out.extractedAt = raw.extracted_at;
  if (typeof raw.kc_doc_id === "string") out.kcDocId = raw.kc_doc_id;
  if (typeof raw.kc_version === "string") out.kcVersion = raw.kc_version;
  if (typeof raw.kc_generated_at === "string") out.kcGeneratedAt = raw.kc_generated_at;
  if (typeof raw.ai_summary === "string") out.aiSummary = raw.ai_summary;

  // 数字字段
  if (typeof raw.derivative_version === "number") out.derivativeVersion = raw.derivative_version;
  if (typeof raw.quality_level === "number") out.qualityLevel = raw.quality_level;
  if (typeof raw.ai_qa_pairs_count === "number") out.aiQaPairsCount = raw.ai_qa_pairs_count;
  if (typeof raw.paragraph_count === "number") out.paragraphCount = raw.paragraph_count;

  // 枚举字段（kcTagsSource）
  if (raw.kc_tags_source === "ai+rule" || raw.kc_tags_source === "rule_only") {
    out.kcTagsSource = raw.kc_tags_source;
  }

  // 枚举字段（kcEnriched）—— Architect schema 是显式字符串
  if (raw.kc_enriched === "true" || raw.kc_enriched === "partial" || raw.kc_enriched === "false") {
    out.kcEnriched = raw.kc_enriched;
  }

  // 字符串数组字段
  if (Array.isArray(raw.ai_tags) && raw.ai_tags.every((t): t is string => typeof t === "string")) {
    out.aiTags = raw.ai_tags;
  }
  if (Array.isArray(raw.rule_tags) && raw.rule_tags.every((t): t is string => typeof t === "string")) {
    out.ruleTags = raw.rule_tags;
  }

  return out;
}

/**
 * 解析 markdown 头部的 YAML frontmatter。
 *
 * - 无 frontmatter（不以 `---` 开头）→ { frontmatter: null, body: markdown }
 * - YAML 非法/含危险 tag → { frontmatter: null, body: markdown, parseError }
 * - 成功 → { frontmatter: camelCase 对象, body: 去掉 frontmatter 后的正文 }
 */
export function parseFrontmatter(markdown: string): ParseResult {
  const match = FRONTMATTER_RE.exec(markdown);
  if (!match) {
    return { frontmatter: null, body: markdown };
  }

  const yamlStr = match[1];
  const body = match[2] ?? "";

  let loaded: unknown;
  try {
    // JSON_SCHEMA 只允许 null/bool/int/float/string/seq/map，
    // 阻断 !!js/function、!!python/object/apply 等任意代码执行向量。
    loaded = yaml.load(yamlStr, { schema: yaml.JSON_SCHEMA });
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    return { frontmatter: null, body: markdown, parseError: msg };
  }

  // 空 frontmatter（`---\n---`）→ yaml.load 返回 null/undefined
  if (loaded === null || loaded === undefined) {
    return { frontmatter: null, body };
  }

  // 非 plain object（如 frontmatter 写了一个数组）→ 视为无法解析
  if (typeof loaded !== "object" || Array.isArray(loaded)) {
    return {
      frontmatter: null,
      body: markdown,
      parseError: "frontmatter must be a YAML mapping (key/value pairs)",
    };
  }

  return {
    frontmatter: mapToCamelCase(loaded as Record<string, unknown>),
    body,
  };
}
