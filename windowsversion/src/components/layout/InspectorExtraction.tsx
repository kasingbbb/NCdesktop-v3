import { useEffect, useCallback, useState } from "react";
import { FileText, Copy, RefreshCw, ChevronDown, ChevronRight } from "lucide-react";
import type { Asset } from "../../types";
import { useExtractionStore } from "../../stores/extractionStore";
import { ExtractionBadge } from "../features/extraction/ExtractionBadge";
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
              <pre
                className="text-[var(--text-xs)] leading-relaxed whitespace-pre-wrap break-words max-h-[240px] overflow-y-auto rounded-[var(--radius-sm)] p-2"
                style={{
                  color: "var(--text-primary)",
                  background: "var(--surface-primary)",
                }}
              >
                {content.structuredMd}
              </pre>
              {content.qualityLevel > 0 && (
                <p
                  className="text-[10px]"
                  style={{ color: "var(--text-tertiary)" }}
                >
                  质量：{qualityLabel(content.qualityLevel)}（{content.qualityLevel}） · 转换来源：{extractorLabel(content.extractorType)}
                </p>
              )}
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
