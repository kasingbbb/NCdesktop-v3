import { useCallback, useEffect, useMemo, useState } from "react";
import {
  X,
  ChevronLeft,
  ChevronRight,
  ZoomIn,
  ZoomOut,
  Maximize,
  FileText,
  Image,
  Music,
  File,
} from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useAssetStore } from "../../../stores/assetStore";
import { getFileContent } from "../../../lib/tauri-commands";
import { parseFrontmatter } from "../../../utils/parseFrontmatter";
import { mapKcEnrichedToLabel, type KcEnrichedTone } from "../../../utils/kcEnrichedLabel";
import type { Asset } from "../../../types";

interface DocumentViewerProps {
  assetId: string;
  onClose: () => void;
}

function assetKind(a: Asset): string {
  const r = a as Asset & { assetType?: string };
  return r.assetType ?? r.type ?? "other";
}

function isImageType(kind: string): boolean {
  return kind === "image" || kind === "photo";
}

function isTextType(kind: string): boolean {
  return kind === "markdown" || kind === "scan_text";
}

function isPdfType(kind: string): boolean {
  return kind === "pdf";
}

function isAudioType(kind: string): boolean {
  return kind === "audio_clip";
}

export function DocumentViewer({ assetId, onClose }: DocumentViewerProps) {
  const assets = useAssetStore((s) => s.assets);
  const [currentId, setCurrentId] = useState(assetId);

  const currentIndex = assets.findIndex((a) => a.id === currentId);
  const asset = assets[currentIndex];

  const goPrev = useCallback(() => {
    if (currentIndex > 0) {
      setCurrentId(assets[currentIndex - 1].id);
    }
  }, [assets, currentIndex]);

  const goNext = useCallback(() => {
    if (currentIndex < assets.length - 1) {
      setCurrentId(assets[currentIndex + 1].id);
    }
  }, [assets, currentIndex]);

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        onClose();
      } else if (e.key === "ArrowLeft" || (e.metaKey && e.key === "[")) {
        goPrev();
      } else if (e.key === "ArrowRight" || (e.metaKey && e.key === "]")) {
        goNext();
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose, goPrev, goNext]);

  if (!asset) {
    onClose();
    return null;
  }

  const kind = assetKind(asset);

  return (
    <div
      className="fixed inset-0 z-50 flex flex-col"
      style={{ background: "var(--surface-canvas)" }}
    >
      {/* Header */}
      <div
        className="h-12 flex items-center justify-between px-4 border-b shrink-0"
        style={{
          borderColor: "var(--border-primary)",
          background: "var(--surface-primary)",
        }}
      >
        <div className="flex items-center gap-3 min-w-0">
          <button
            type="button"
            onClick={onClose}
            className="p-1.5 rounded-[var(--radius-sm)] transition-colors"
            style={{ color: "var(--text-secondary)" }}
            title="Close (Esc)"
          >
            <X size={18} />
          </button>
          <span
            className="text-[var(--text-sm)] font-medium truncate"
            style={{ color: "var(--text-primary)" }}
          >
            {asset.name}
          </span>
          <span
            className="text-[var(--text-xs)]"
            style={{ color: "var(--text-tertiary)" }}
          >
            {currentIndex + 1} / {assets.length}
          </span>
        </div>

        <div className="flex items-center gap-1">
          <button
            type="button"
            onClick={goPrev}
            disabled={currentIndex <= 0}
            className="p-1.5 rounded-[var(--radius-sm)] transition-colors disabled:opacity-30"
            style={{ color: "var(--text-secondary)" }}
            title="Previous"
          >
            <ChevronLeft size={18} />
          </button>
          <button
            type="button"
            onClick={goNext}
            disabled={currentIndex >= assets.length - 1}
            className="p-1.5 rounded-[var(--radius-sm)] transition-colors disabled:opacity-30"
            style={{ color: "var(--text-secondary)" }}
            title="Next"
          >
            <ChevronRight size={18} />
          </button>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 min-h-0 overflow-hidden">
        {isImageType(kind) ? (
          <ImageContent asset={asset} />
        ) : isPdfType(kind) ? (
          <PdfContent asset={asset} />
        ) : isTextType(kind) ? (
          <TextContent asset={asset} />
        ) : isAudioType(kind) ? (
          <AudioContent asset={asset} />
        ) : (
          <FallbackContent asset={asset} />
        )}
      </div>
    </div>
  );
}

function ImageContent({ asset }: { asset: Asset }) {
  const [scale, setScale] = useState(1);
  const url = convertFileSrc(asset.filePath);

  return (
    <div className="relative w-full h-full flex items-center justify-center overflow-auto">
      <div className="absolute top-4 right-4 flex gap-2 z-10">
        <button
          className="p-2 rounded-full transition-colors"
          style={{
            background: "var(--surface-elevated)",
            color: "var(--text-primary)",
            boxShadow: "var(--shadow-float)",
          }}
          onClick={() => setScale((s) => Math.min(s + 0.25, 5))}
        >
          <ZoomIn size={16} />
        </button>
        <button
          className="p-2 rounded-full transition-colors"
          style={{
            background: "var(--surface-elevated)",
            color: "var(--text-primary)",
            boxShadow: "var(--shadow-float)",
          }}
          onClick={() => setScale((s) => Math.max(s - 0.25, 0.25))}
        >
          <ZoomOut size={16} />
        </button>
        <button
          className="p-2 rounded-full transition-colors"
          style={{
            background: "var(--surface-elevated)",
            color: "var(--text-primary)",
            boxShadow: "var(--shadow-float)",
          }}
          onClick={() => setScale(1)}
        >
          <Maximize size={16} />
        </button>
      </div>
      <img
        src={url}
        alt={asset.name}
        className="max-w-full max-h-full object-contain"
        style={{
          transform: `scale(${scale})`,
          transition: "transform var(--duration-fast) var(--ease-out)",
        }}
        draggable={false}
        onWheel={(e) => {
          e.preventDefault();
          const delta = e.deltaY > 0 ? -0.1 : 0.1;
          setScale((s) => Math.min(Math.max(s + delta, 0.25), 5));
        }}
      />
    </div>
  );
}

function PdfContent({ asset }: { asset: Asset }) {
  const url = convertFileSrc(asset.filePath);
  return (
    <div className="w-full h-full">
      <iframe
        src={url}
        title={asset.name}
        className="w-full h-full border-none"
        style={{ background: "var(--surface-primary)" }}
      />
    </div>
  );
}

/**
 * task_019_doc_viewer_render — KC v6 增强 MD 渲染
 *
 * AC-1：parseFrontmatter → 顶部 frontmatter 卡片 + react-markdown 主体
 * AC-2：Tailwind 表格 / typography / 锚点 / 代码块 / 64ch 宽
 * AC-3：ImageContent / PdfContent / AudioContent / FallbackContent 保持不变
 * AC-5 (TD-4)：kc_enriched 字面映射 helper 由 mapKcEnrichedToLabel 共享
 * 安全：react-markdown@9 默认不传 rehype-raw，等价于 allowDangerousHtml: false
 */
function TextContent({ asset }: { asset: Asset }) {
  const [content, setContent] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);
    getFileContent(asset.filePath)
      .then((text: string) => setContent(text))
      .catch(() => setContent(null))
      .finally(() => setLoading(false));
  }, [asset.filePath]);

  // 解析 frontmatter；YAML 失败/无 frontmatter 时 parsed.frontmatter = null。
  // useMemo 把结果 memo 在 content 字面值上，避免重渲染重复 parse。
  const parsed = useMemo(() => {
    if (content === null || content.length === 0) {
      return { frontmatter: null, body: "", parseError: undefined as string | undefined };
    }
    return parseFrontmatter(content);
  }, [content]);
  const useMarkdownView = parsed.frontmatter !== null && !parsed.parseError;

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <p style={{ color: "var(--text-tertiary)" }}>Loading...</p>
      </div>
    );
  }

  if (content === null) {
    return (
      <div className="flex items-center justify-center h-full">
        <p style={{ color: "var(--text-tertiary)" }}>Unable to load file content.</p>
      </div>
    );
  }

  // 失败回退：YAML 非法 / 无 frontmatter（历史 MD）→ 原 <pre> 模式。
  if (!useMarkdownView || !parsed.frontmatter) {
    return (
      <div className="w-full h-full overflow-y-auto p-8">
        <div className="max-w-3xl mx-auto">
          <pre
            data-testid="doc-viewer-pre-fallback"
            className="text-[var(--text-base)] leading-relaxed whitespace-pre-wrap font-mono"
            style={{ color: "var(--text-primary)" }}
          >
            {content}
          </pre>
        </div>
      </div>
    );
  }

  const fm = parsed.frontmatter;

  return (
    <div className="w-full h-full overflow-y-auto p-8">
      <div className="mx-auto" style={{ maxWidth: "64ch" }}>
        {/* 顶部 frontmatter 卡片（仅 KC 增强字段） */}
        <FrontmatterCard
          aiSummary={fm.aiSummary}
          aiTags={fm.aiTags}
          ruleTags={fm.ruleTags}
          kcEnriched={fm.kcEnriched ?? null}
        />

        {/* react-markdown 主体（含 remark-gfm 表格 / 锚点 / strikethrough）
            Tailwind typography 样式 + 表格边框 + 代码块背景。 */}
        <div
          data-testid="doc-viewer-markdown-body"
          className="markdown-body prose prose-sm max-w-none break-words
                     [&_h1]:text-[var(--text-2xl)] [&_h1]:font-semibold [&_h1]:mt-6 [&_h1]:mb-3
                     [&_h2]:text-[var(--text-xl)] [&_h2]:font-semibold [&_h2]:mt-5 [&_h2]:mb-2
                     [&_h3]:text-[var(--text-lg)] [&_h3]:font-semibold [&_h3]:mt-4 [&_h3]:mb-2
                     [&_h4]:text-[var(--text-base)] [&_h4]:font-semibold [&_h4]:mt-3 [&_h4]:mb-1
                     [&_p]:leading-relaxed [&_p]:my-3
                     [&_a]:text-[var(--accent-primary,#0a84ff)] [&_a]:underline
                     [&_ul]:list-disc [&_ul]:ml-6 [&_ul]:my-2
                     [&_ol]:list-decimal [&_ol]:ml-6 [&_ol]:my-2
                     [&_li]:my-1
                     [&_code]:font-mono [&_code]:text-[0.92em] [&_code]:px-1 [&_code]:py-0.5
                     [&_code]:rounded [&_code]:bg-[var(--surface-tertiary)]
                     [&_pre]:bg-[var(--surface-tertiary)] [&_pre]:rounded-[var(--radius-sm)]
                     [&_pre]:p-3 [&_pre]:my-3 [&_pre]:overflow-x-auto [&_pre]:font-mono [&_pre]:text-[0.92em]
                     [&_pre>code]:bg-transparent [&_pre>code]:p-0
                     [&_table]:my-4 [&_table]:w-full [&_table]:border-collapse
                     [&_table]:border [&_table]:border-[var(--border-primary)]
                     [&_th]:border [&_th]:border-[var(--border-primary)] [&_th]:px-3 [&_th]:py-2
                     [&_th]:bg-[var(--surface-secondary)] [&_th]:font-semibold [&_th]:text-left
                     [&_td]:border [&_td]:border-[var(--border-primary)] [&_td]:px-3 [&_td]:py-2
                     [&_tbody>tr:nth-child(odd)]:bg-[var(--surface-secondary)]
                     [&_blockquote]:border-l-4 [&_blockquote]:border-[var(--border-primary)]
                     [&_blockquote]:pl-4 [&_blockquote]:my-3 [&_blockquote]:text-[var(--text-secondary)]
                     [&_hr]:my-6 [&_hr]:border-[var(--border-primary)]"
          style={{ color: "var(--text-primary)" }}
        >
          <ReactMarkdown remarkPlugins={[remarkGfm]}>
            {parsed.body}
          </ReactMarkdown>
        </div>
      </div>
    </div>
  );
}

/**
 * task_019 AC-1 / AC-5：frontmatter 卡片
 * - 摘要（ai_summary）
 * - 标签（ai_tags + rule_tags 合并展示）
 * - kc_enriched 状态行（含 dot；null 整行隐藏）
 *
 * 视觉规范：与 NC 现有 typography 一致，固定背景 surface-secondary。
 */
function FrontmatterCard(props: {
  aiSummary: string | undefined;
  aiTags: string[] | undefined;
  ruleTags: string[] | undefined;
  kcEnriched: string | null;
}) {
  const { aiSummary, aiTags, ruleTags, kcEnriched } = props;
  const kc = mapKcEnrichedToLabel(kcEnriched);
  const allTags = [...(aiTags ?? []), ...(ruleTags ?? [])];

  // 卡片完全空（无任何 KC 字段）→ 不渲染，避免空白板块
  const hasAnything = !!aiSummary || allTags.length > 0 || kc !== null;
  if (!hasAnything) return null;

  return (
    <div
      data-testid="doc-viewer-frontmatter-card"
      className="mb-6 rounded-[var(--radius-md)] border p-4 space-y-3"
      style={{
        background: "var(--surface-secondary)",
        borderColor: "var(--border-primary)",
      }}
    >
      {aiSummary && (
        <div>
          <p
            className="text-[var(--text-xs)] uppercase tracking-[0.05em] mb-1"
            style={{ color: "var(--text-tertiary)" }}
          >
            AI 摘要
          </p>
          <p
            className="text-[var(--text-sm)] leading-relaxed"
            style={{ color: "var(--text-primary)" }}
          >
            {aiSummary}
          </p>
        </div>
      )}

      {allTags.length > 0 && (
        <div>
          <p
            className="text-[var(--text-xs)] uppercase tracking-[0.05em] mb-1"
            style={{ color: "var(--text-tertiary)" }}
          >
            标签
          </p>
          <div className="flex flex-wrap gap-1.5">
            {allTags.map((t) => (
              <span
                key={t}
                className="text-[var(--text-xs)] px-2 py-0.5 rounded-[var(--radius-sm)]"
                style={{
                  background: "var(--surface-tertiary)",
                  color: "var(--text-secondary)",
                }}
              >
                {t}
              </span>
            ))}
          </div>
        </div>
      )}

      {kc !== null && (
        <div
          className="flex items-center gap-2"
          data-testid="doc-viewer-kc-enriched-row"
        >
          <KcEnrichedDot tone={kc.tone} />
          <span
            className="text-[var(--text-xs)]"
            style={{ color: "var(--text-secondary)" }}
            data-testid="doc-viewer-kc-enriched-label"
          >
            {kc.label}
          </span>
        </div>
      )}
    </div>
  );
}

/** kc_enriched 状态的小圆点。颜色映射：success→green / partial→amber / inactive→grey */
function KcEnrichedDot({ tone }: { tone: KcEnrichedTone }) {
  const color =
    tone === "success"
      ? "var(--color-success, #34c759)"
      : tone === "partial"
        ? "var(--color-warning, #ff9500)"
        : "var(--text-tertiary, #8e8e93)";
  return (
    <span
      aria-hidden="true"
      data-testid={`doc-viewer-kc-dot-${tone}`}
      className="inline-block w-2 h-2 rounded-full"
      style={{ background: color }}
    />
  );
}

function AudioContent({ asset }: { asset: Asset }) {
  const url = convertFileSrc(asset.filePath);
  return (
    <div className="flex flex-col items-center justify-center h-full gap-6">
      <div
        className="w-24 h-24 rounded-full flex items-center justify-center"
        style={{ background: "var(--surface-tertiary)" }}
      >
        <Music size={40} style={{ color: "var(--text-secondary)" }} />
      </div>
      <p
        className="text-[var(--text-lg)] font-medium"
        style={{ color: "var(--text-primary)" }}
      >
        {asset.name}
      </p>
      <audio controls src={url} className="w-full max-w-lg" />
    </div>
  );
}

function FallbackContent({ asset }: { asset: Asset }) {
  const kind = assetKind(asset);
  const icon =
    kind === "image" || kind === "photo" ? (
      <Image size={40} />
    ) : kind === "markdown" || kind === "scan_text" ? (
      <FileText size={40} />
    ) : (
      <File size={40} />
    );

  return (
    <div className="flex flex-col items-center justify-center h-full gap-4">
      <div
        className="w-24 h-24 rounded-full flex items-center justify-center"
        style={{ background: "var(--surface-tertiary)", color: "var(--text-secondary)" }}
      >
        {icon}
      </div>
      <p
        className="text-[var(--text-base)]"
        style={{ color: "var(--text-secondary)" }}
      >
        Preview not available for this file type.
      </p>
      <p
        className="text-[var(--text-sm)]"
        style={{ color: "var(--text-tertiary)" }}
      >
        {asset.filePath}
      </p>
    </div>
  );
}
