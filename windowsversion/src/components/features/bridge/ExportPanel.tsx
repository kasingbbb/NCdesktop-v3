import { useCallback, useEffect, useState } from "react";
import { Download, FileText, Sparkles, Loader2, WifiOff } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { TargetSelector, type ExportTarget } from "./TargetSelector";

interface ExportOptions {
  project_id: string;
  include_transcription: boolean;
  include_ocr: boolean;
  include_ai_summary: boolean;
  include_tags: boolean;
  include_notes: boolean;
  include_timeline: boolean;
}

interface ExportResult {
  markdown: string;
  word_count: number;
  section_count: number;
}

interface ExportPanelProps {
  projectId: string;
  onClose?: () => void;
}

export function ExportPanel({ projectId, onClose }: ExportPanelProps) {
  const [target, setTarget] = useState<ExportTarget>("clipboard");
  const [options, setOptions] = useState<ExportOptions>({
    project_id: projectId,
    include_transcription: true,
    include_ocr: true,
    include_ai_summary: true,
    include_tags: true,
    include_notes: true,
    include_timeline: true,
  });
  const [result, setResult] = useState<ExportResult | null>(null);
  const [isExporting, setIsExporting] = useState(false);
  const [isEnhancing, setIsEnhancing] = useState(false);
  const [llmAvailable, setLlmAvailable] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    checkLLM();
    handlePreview();
  }, []);

  async function checkLLM(): Promise<void> {
    try {
      const config = await invoke<{ is_configured: boolean }>("get_llm_config");
      setLlmAvailable(config.is_configured);
    } catch {
      setLlmAvailable(false);
    }
  }

  async function handlePreview(): Promise<void> {
    setIsExporting(true);
    setError(null);
    try {
      const res = await invoke<ExportResult>("export_project_markdown", { options });
      setResult(res);
    } catch (e) {
      setError(String(e));
    } finally {
      setIsExporting(false);
    }
  }

  async function handleEnhance(): Promise<void> {
    if (!result || !llmAvailable) return;
    setIsEnhancing(true);
    setError(null);
    try {
      const enhanced = await invoke<string>("llm_enhance_export", {
        markdown: result.markdown,
      });
      setResult({ ...result, markdown: enhanced });
    } catch (e) {
      setError(String(e));
    } finally {
      setIsEnhancing(false);
    }
  }

  const handleExport = useCallback(async () => {
    if (!result) return;
    setError(null);

    try {
      switch (target) {
        case "clipboard":
          await navigator.clipboard.writeText(result.markdown);
          break;
        case "chatgpt":
          await navigator.clipboard.writeText(result.markdown);
          window.open("https://chat.openai.com", "_blank");
          break;
        case "claude":
          await navigator.clipboard.writeText(result.markdown);
          window.open("https://claude.ai", "_blank");
          break;
        case "notebooklm":
          await navigator.clipboard.writeText(result.markdown);
          window.open("https://notebooklm.google.com", "_blank");
          break;
      }
      onClose?.();
    } catch (e) {
      setError(String(e));
    }
  }, [result, target, onClose]);

  const toggleOption = (key: keyof ExportOptions): void => {
    if (key === "project_id") return;
    const newOpts = { ...options, [key]: !options[key] };
    setOptions(newOpts);
    setResult(null);
  };

  const OPTION_LABELS: Array<{ key: keyof ExportOptions; label: string }> = [
    { key: "include_timeline", label: "时间轴信息" },
    { key: "include_transcription", label: "音频转录" },
    { key: "include_ocr", label: "OCR 文本" },
    { key: "include_ai_summary", label: "AI 摘要" },
    { key: "include_tags", label: "标签" },
    { key: "include_notes", label: "笔记" },
  ];

  return (
    <div className="flex flex-col h-full max-h-[80vh] w-[560px]">
      {/* 头部 */}
      <div className="flex items-center justify-between px-[var(--space-4)] py-[var(--space-3)] border-b" style={{ borderColor: "var(--border-primary)" }}>
        <div className="flex items-center gap-[var(--space-2)]">
          <Download size={18} className="text-gray-600" />
          <h2 className="text-[var(--text-base)] font-semibold" style={{ color: "var(--text-primary)" }}>
            导出到 LLM
          </h2>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-[var(--space-4)] space-y-[var(--space-4)]">
        {/* 导出目标 */}
        <div>
          <h3 className="text-[var(--text-sm)] font-medium mb-[var(--space-2)]" style={{ color: "var(--text-secondary)" }}>
            导出目标
          </h3>
          <TargetSelector selected={target} onSelect={setTarget} />
        </div>

        {/* 内容勾选 */}
        <div>
          <h3 className="text-[var(--text-sm)] font-medium mb-[var(--space-2)]" style={{ color: "var(--text-secondary)" }}>
            导出内容
          </h3>
          <div className="grid grid-cols-2 gap-[var(--space-2)]">
            {OPTION_LABELS.map(({ key, label }) => (
              <label
                key={key}
                className="flex items-center gap-[var(--space-2)] p-[var(--space-2)] rounded-[var(--radius-sm)] cursor-pointer"
                style={{ backgroundColor: "var(--surface-secondary)" }}
              >
                <input
                  type="checkbox"
                  checked={options[key] as boolean}
                  onChange={() => toggleOption(key)}
                  className="accent-gray-700"
                />
                <span className="text-[var(--text-sm)]" style={{ color: "var(--text-primary)" }}>
                  {label}
                </span>
              </label>
            ))}
          </div>
        </div>

        {/* 预览 */}
        <div>
          <div className="flex items-center justify-between mb-[var(--space-2)]">
            <h3 className="text-[var(--text-sm)] font-medium" style={{ color: "var(--text-secondary)" }}>
              Markdown 预览
            </h3>
            {result && (
              <span className="text-[10px]" style={{ color: "var(--text-tertiary)" }}>
                {result.word_count} 字 · {result.section_count} 个章节
              </span>
            )}
          </div>

          <div
            className="rounded-[var(--radius-md)] p-[var(--space-3)] max-h-[240px] overflow-y-auto"
            style={{ backgroundColor: "var(--surface-tertiary)", fontFamily: "monospace" }}
          >
            {isExporting ? (
              <div className="flex items-center gap-[var(--space-2)] justify-center py-[var(--space-4)]">
                <Loader2 size={16} className="animate-spin text-gray-500" />
                <span className="text-[var(--text-sm)]" style={{ color: "var(--text-tertiary)" }}>生成预览中...</span>
              </div>
            ) : result ? (
              <pre className="text-[var(--text-xs)] whitespace-pre-wrap" style={{ color: "var(--text-secondary)" }}>
                {result.markdown.slice(0, 2000)}
                {result.markdown.length > 2000 && "\n\n... (内容过长，已截断预览)"}
              </pre>
            ) : (
              <button
                className="btn-glass w-full py-[var(--space-2)]"
                onClick={handlePreview}
              >
                <FileText size={14} />
                <span className="text-[var(--text-sm)]">生成预览</span>
              </button>
            )}
          </div>
        </div>

        {/* 错误提示 */}
        {error && (
          <div
            className="flex items-center gap-[var(--space-2)] p-[var(--space-3)] rounded-[var(--radius-md)]"
            style={{ backgroundColor: "rgba(255, 59, 48, 0.08)", border: "1px solid rgba(255, 59, 48, 0.3)" }}
          >
            <WifiOff size={14} style={{ color: "#FF3B30" }} />
            <span className="text-[var(--text-xs)]" style={{ color: "#FF3B30" }}>{error}</span>
          </div>
        )}
      </div>

      {/* 底部操作 */}
      <div className="flex items-center justify-between px-[var(--space-4)] py-[var(--space-3)] border-t" style={{ borderColor: "var(--border-primary)" }}>
        <button
          className="btn-glass px-[var(--space-3)] py-[var(--space-2)] flex items-center gap-[var(--space-1)]"
          onClick={handleEnhance}
          disabled={!llmAvailable || !result || isEnhancing}
          style={{ opacity: llmAvailable ? 1 : 0.4 }}
        >
          {isEnhancing ? (
            <Loader2 size={14} className="animate-spin" />
          ) : (
            <Sparkles size={14} className="text-gray-600" />
          )}
          <span className="text-[var(--text-sm)]">AI 润色</span>
        </button>

        <div className="flex items-center gap-[var(--space-2)]">
          <button className="btn-glass px-[var(--space-3)] py-[var(--space-2)]" onClick={onClose}>
            <span className="text-[var(--text-sm)]">取消</span>
          </button>
          <button
            className="px-[var(--space-4)] py-[var(--space-2)] rounded-[var(--radius-md)] flex items-center gap-[var(--space-1)]"
            style={{
              backgroundColor: "var(--brand-navy)",
              color: "#ffffff",
              opacity: result ? 1 : 0.5,
            }}
            onClick={handleExport}
            disabled={!result}
          >
            <Download size={14} />
            <span className="text-[var(--text-sm)] font-medium">导出</span>
          </button>
        </div>
      </div>
    </div>
  );
}
