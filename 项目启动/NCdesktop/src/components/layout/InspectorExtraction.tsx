import { useEffect, useCallback, useMemo, useState } from "react";
import { FileText, Copy, RefreshCw, Sparkles, ChevronDown, ChevronRight } from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import type { Asset } from "../../types";
import { useExtractionStore } from "../../stores/extractionStore";
import * as cmd from "../../lib/tauri-commands";
import { ExtractionBadge } from "../features/extraction/ExtractionBadge";
import { FrontmatterSummaryView } from "../features/extraction/FrontmatterSummaryView";
import { FrontmatterTagsView } from "../features/extraction/FrontmatterTagsView";
import { parseFrontmatter } from "../../utils/parseFrontmatter";
import { mapKcEnrichedToLabel } from "../../utils/kcEnrichedLabel";
import type { ExtractionStatus } from "../../types/extraction";

interface InspectorExtractionProps {
  asset: Asset;
}

function qualityLabel(level: number): string {
  if (level >= 4) return "优秀";
  if (level >= 3) return "良好";
  if (level >= 2) return "可用";
  if (level >= 1) return "较弱";
  return "空";
}

function extractorLabel(name: string): string {
  const map: Record<string, string> = {
    markitdown: "MarkItDown",
    materialized_markdown: "物化 Markdown",
    source_markdown: "源 Markdown",
    pdf_text: "内置 PDF 文本提取",
    pdf_scan_ocr: "扫描 PDF OCR",
    docx: "内置 DOCX 提取",
    pptx: "内置 PPTX 提取",
    text: "文本提取",
    vision_ocr: "图片 OCR",
    audio_asr: "音频转写",
    builtin: "内置提取器",
  };
  return map[name] ?? name;
}

function errorClassLabel(cls: string | null | undefined): string {
  const map: Record<string, string> = {
    file_not_found: "找不到文件",
    permission_denied: "权限不足",
    unsupported_format: "格式不支持",
    markitdown_not_installed: "未安装 MarkItDown",
    python_unavailable: "Python 不可用",
    empty_output: "转换输出为空",
    timeout: "转换超时",
    conversion_error: "转换出错",
  };
  return cls && map[cls] ? map[cls] : "提取失败";
}

function formatConversionMs(ms: number | null | undefined): string {
  if (ms === null || ms === undefined) return "—";
  if (ms > 1000) return `${(ms / 1000).toFixed(1)}s`;
  return `${ms} ms`;
}

export function InspectorExtraction({ asset }: InspectorExtractionProps) {
  const {
    contentCache,
    statusCache,
    conversionMetaCache,
    fetchExtractedContent,
    fetchConversionMeta,
    retryExtraction,
  } = useExtractionStore();

  const [expanded, setExpanded] = useState(true);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    void fetchExtractedContent(asset.id);
    // 失败态/重试场景下 fetchExtractedContent 内部可能因无内容跳过 meta 拉取，
    // 这里兜底再触发一次（store 内部去重无副作用，失败仅 warn）。
    void fetchConversionMeta(asset.id);
  }, [asset.id, fetchExtractedContent, fetchConversionMeta]);

  const content = contentCache[asset.id];
  const status = (statusCache[asset.id] ?? content?.status ?? "pending") as ExtractionStatus;
  // 最新一行转换元数据（后端按 converted_at DESC 返回）
  const latestMeta = conversionMetaCache[asset.id]?.[0];

  // task_018 AC-1：每次 asset 切换都重新 parseFrontmatter。
  // 用 useMemo 把解析结果缓存在 structuredMd 字面值上 —— 同一 asset 多次 re-render 不会重复解析。
  const parsed = useMemo(() => {
    const md = content?.structuredMd;
    if (typeof md !== "string" || md.length === 0) {
      return { frontmatter: null, body: "", parseError: undefined as string | undefined };
    }
    return parseFrontmatter(md);
  }, [content?.structuredMd]);
  // 仅当 frontmatter 解析成功且 parseError 不存在时启用 markdown 视图；否则回退到 <pre>
  const useFrontmatterView = parsed.frontmatter !== null && !parsed.parseError;

  const handleCopy = useCallback(async () => {
    const text = content?.rawText ?? content?.structuredMd;
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      /* clipboard API 不可用 */
    }
  }, [content]);

  const handleRetry = useCallback(() => {
    void retryExtraction(asset.id);
  }, [asset.id, retryExtraction]);

  // task_026 AC-3：Inspector "重新增强"按钮 —— 调 retriggerExtraction(asset.id, true)
  // 强制清 kc_enriched 让 task_012 enrichment 重跑。
  // 显隐条件由 `showReEnrichButton` 计算（见下）。
  const [reEnriching, setReEnriching] = useState(false);
  const handleReEnrich = useCallback(async () => {
    if (reEnriching) return;
    setReEnriching(true);
    try {
      await cmd.retriggerExtraction(asset.id, true);
      // 拉一次最新状态（kc_enriched 已被清，UI 应隐藏按钮直到重 enrich 完成回填）
      void fetchExtractedContent(asset.id);
    } catch (err) {
      console.error("重新增强失败:", err);
    } finally {
      setReEnriching(false);
    }
  }, [asset.id, reEnriching, fetchExtractedContent]);

  // AC-3 显隐：
  // - asset_type='md' 不显示（不能误触发对 markdown 原件的 KC）
  // - extracted_content.kcEnriched 必须非 null 且非 "false"
  //   （null = 未做过 KC，"false" = enrich 失败 —— 都不允许"重新增强"
  //   只允许"true"/"partial" 强制重跑）。
  const showReEnrichButton =
    asset.assetType !== "md" &&
    !!content?.kcEnriched &&
    content.kcEnriched !== "false";

  return (
    <div className="mb-[var(--space-4)]">
      <button
        type="button"
        className="w-full flex items-center gap-1 mb-[var(--space-2)]"
        onClick={() => setExpanded((v) => !v)}
      >
        {expanded ? (
          <ChevronDown size={12} style={{ color: "var(--text-tertiary)" }} />
        ) : (
          <ChevronRight size={12} style={{ color: "var(--text-tertiary)" }} />
        )}
        <h3
          className="text-[var(--text-sm)] uppercase tracking-[0.08em] flex items-center gap-1.5"
          style={{ color: "var(--text-tertiary)" }}
        >
          <FileText size={14} className="text-gray-500" />
          提取内容
        </h3>
        <span className="ml-auto">
          <ExtractionBadge status={status} size="md" />
        </span>
      </button>

      {expanded && (
        <div
          className="rounded-[var(--radius-md)] p-[var(--space-3)] border"
          style={{
            background: "var(--surface-secondary)",
            borderColor: "var(--border-primary)",
          }}
        >
          {status === "extracting" && (
            <p
              className="text-[var(--text-xs)] flex items-center gap-1.5"
              style={{ color: "var(--text-secondary)" }}
            >
              正在提取内容…
            </p>
          )}

          {status === "pending" && (
            <p
              className="text-[var(--text-xs)]"
              style={{ color: "var(--text-tertiary)" }}
            >
              尚未提取，可在工具栏触发提取。
            </p>
          )}

          {status === "unsupported" && (
            <p
              className="text-[var(--text-xs)]"
              style={{ color: "var(--text-tertiary)" }}
            >
              此素材类型暂不支持内容提取。
            </p>
          )}

          {status === "failed" && (
            <div className="space-y-2">
              <p className="text-[var(--text-xs)]" style={{ color: "#FF3B30" }}>
                {errorClassLabel(latestMeta?.errorClass)}
              </p>
              <button
                type="button"
                className="inline-flex items-center gap-1 text-[var(--text-xs)] px-2 py-1 rounded-[var(--radius-sm)] border border-app transition-colors hover:bg-[var(--surface-tertiary)]"
                style={{ color: "var(--text-secondary)" }}
                onClick={handleRetry}
              >
                <RefreshCw size={11} />
                重试
              </button>
            </div>
          )}

          {status === "extracted" && content?.structuredMd && (
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <span
                  className="text-[var(--text-xs)] uppercase tracking-[0.05em]"
                  style={{ color: "var(--text-tertiary)" }}
                >
                  Markdown 预览
                </span>
                <div className="inline-flex items-center gap-1">
                  {showReEnrichButton && (
                    <button
                      type="button"
                      data-testid="re-enrich-button"
                      aria-label="重新增强"
                      title="重新增强 / Re-enhance with KC"
                      className="inline-flex items-center gap-1 text-[var(--text-xs)] px-1.5 py-0.5 rounded-[var(--radius-sm)] transition-colors hover:bg-[var(--surface-tertiary)] disabled:opacity-50"
                      style={{ color: "var(--text-secondary)" }}
                      onClick={handleReEnrich}
                      disabled={reEnriching}
                    >
                      <Sparkles size={11} />
                      {reEnriching ? "重新增强中…" : "重新增强"}
                    </button>
                  )}
                  <button
                    type="button"
                    className="inline-flex items-center gap-1 text-[var(--text-xs)] px-1.5 py-0.5 rounded-[var(--radius-sm)] transition-colors hover:bg-[var(--surface-tertiary)]"
                    style={{ color: "var(--text-secondary)" }}
                    onClick={handleCopy}
                  >
                    <Copy size={11} />
                    {copied ? "已复制" : "复制"}
                  </button>
                </div>
              </div>
              {useFrontmatterView && parsed.frontmatter ? (
                <div className="space-y-2" data-testid="frontmatter-view">
                  <FrontmatterSummaryView
                    summary={parsed.frontmatter.aiSummary}
                    isAi={true}
                  />
                  <FrontmatterTagsView
                    aiTags={parsed.frontmatter.aiTags}
                    ruleTags={parsed.frontmatter.ruleTags}
                  />
                  <div
                    className="markdown-body text-[var(--text-xs)] leading-relaxed break-words max-h-[240px] overflow-y-auto rounded-[var(--radius-sm)] p-2"
                    style={{
                      color: "var(--text-primary)",
                      background: "var(--surface-primary)",
                    }}
                    data-testid="markdown-body"
                  >
                    {/*
                      task_018 AC-1：用 react-markdown + remark-gfm 渲染正文。
                      安全：react-markdown v9 默认禁用 raw HTML（不传 rehype-raw），
                      用户写的 <script> 等 tag 不会被执行，等价于 allowDangerousHtml: false。
                    */}
                    <ReactMarkdown remarkPlugins={[remarkGfm]}>
                      {parsed.body}
                    </ReactMarkdown>
                  </div>
                </div>
              ) : (
                <pre
                  className="text-[var(--text-xs)] leading-relaxed whitespace-pre-wrap break-words max-h-[240px] overflow-y-auto rounded-[var(--radius-sm)] p-2"
                  style={{
                    color: "var(--text-primary)",
                    background: "var(--surface-primary)",
                  }}
                  data-testid="pre-fallback"
                >
                  {content.structuredMd}
                </pre>
              )}
              {content.qualityLevel > 0 && (
                <p
                  className="text-[10px]"
                  style={{ color: "var(--text-tertiary)" }}
                >
                  质量：{qualityLabel(content.qualityLevel)}（{content.qualityLevel}） · 转换来源：{extractorLabel(content.extractorType)}
                </p>
              )}
              {(() => {
                // task_018 AC-2 / task_019 TD-4：kc_enriched 字面映射 → 用户文案。
                // null → 不渲染该行（历史数据）。translation 由 shared helper 提供，
                // 保证 InspectorExtraction 与 DocumentViewer 文案一致。
                const kcMapped = mapKcEnrichedToLabel(content.kcEnriched);
                if (kcMapped === null) return null;
                return (
                  <p
                    className="text-[10px]"
                    style={{ color: "var(--text-tertiary)" }}
                    data-testid="kc-enriched-label"
                  >
                    {kcMapped.label}
                  </p>
                );
              })()}
              {latestMeta && (
                <div className="space-y-1">
                  <p
                    className="text-[10px]"
                    style={{ color: "var(--text-tertiary)" }}
                  >
                    转换信息：{extractorLabel(latestMeta.converterName)} {latestMeta.converterVersion}
                    {" · "}
                    {formatConversionMs(latestMeta.conversionMs)}
                  </p>
                  {latestMeta.fallbackUsed && (
                    <p
                      className="text-[10px]"
                      style={{
                        // 优先使用全局 token，未定义时回退到 #FF9500（待后续提取到 globals.css --color-warning）
                        color: "var(--color-warning, #FF9500)",
                      }}
                    >
                      已自动回退到 {extractorLabel(latestMeta.converterName)}
                    </p>
                  )}
                </div>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
