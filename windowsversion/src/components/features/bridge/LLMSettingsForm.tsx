import { useCallback, useEffect, useState } from "react";
import { Key, Globe, Cpu, AlertCircle, Check, Loader2 } from "lucide-react";
import {
  getLLMConfig,
  saveLLMConfig,
  llmProbe,
  type LLMConfig,
  type ClassifyResult,
} from "../../../lib/tauri-commands";

export function LLMSettingsForm() {
  const [config, setConfig] = useState<LLMConfig | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<{ type: "ok" | "err"; text: string } | null>(null);

  const [baseUrl, setBaseUrl] = useState("");
  const [model, setModel] = useState("");
  /** 仅用于写入新 Key；留空并保存表示保留原 Key */
  const [apiKeyDraft, setApiKeyDraft] = useState("");
  const [probeLoading, setProbeLoading] = useState(false);
  const [probeResult, setProbeResult] = useState<ClassifyResult | null>(null);

  const loadConfig = useCallback(async (): Promise<void> => {
    try {
      const cfg = await getLLMConfig();
      setConfig(cfg);
      setBaseUrl(cfg.base_url);
      setModel(cfg.model);
      setApiKeyDraft("");
    } catch {
      setConfig(null);
    }
  }, []);

  useEffect(() => {
    void (async () => {
      setLoading(true);
      await loadConfig();
      setLoading(false);
    })();
  }, [loadConfig]);

  async function handleSave(): Promise<void> {
    setSaving(true);
    setMessage(null);
    try {
      const trimmedKey = apiKeyDraft.trim();
      const apiKeyAction: "keep" | "set" = trimmedKey.length > 0 ? "set" : "keep";
      await saveLLMConfig({
        baseUrl: baseUrl.trim(),
        model: model.trim(),
        apiKeyAction,
        apiKeyValue: trimmedKey.length > 0 ? trimmedKey : undefined,
      });
      setApiKeyDraft("");
      await loadConfig();
      setMessage({ type: "ok", text: "已保存" });
    } catch (e) {
      setMessage({ type: "err", text: String(e) });
    } finally {
      setSaving(false);
    }
  }

  async function handleProbe(): Promise<void> {
    setProbeLoading(true);
    setProbeResult(null);
    setMessage(null);
    try {
      const r = await llmProbe();
      setProbeResult(r);
      setMessage({
        type: "ok",
        text: `分类 API 正常。category=${r.category}，tags=${r.tags.join("、") || "（无）"}`,
      });
    } catch (e) {
      setMessage({ type: "err", text: String(e) });
    } finally {
      setProbeLoading(false);
    }
  }

  async function handleClearKey(): Promise<void> {
    setSaving(true);
    setMessage(null);
    try {
      await saveLLMConfig({
        baseUrl: baseUrl.trim(),
        model: model.trim(),
        apiKeyAction: "clear",
      });
      setApiKeyDraft("");
      await loadConfig();
      setMessage({ type: "ok", text: "已清除应用内保存的 API Key" });
    } catch (e) {
      setMessage({ type: "err", text: String(e) });
    } finally {
      setSaving(false);
    }
  }

  if (loading) {
    return (
      <div className="p-[var(--space-4)] text-center">
        <span className="text-[var(--text-sm)]" style={{ color: "var(--text-tertiary)" }}>
          加载配置中...
        </span>
      </div>
    );
  }

  return (
    <div className="space-y-[var(--space-4)]">
      <h3
        className="text-[var(--text-base)] font-semibold"
        style={{ color: "var(--text-primary)" }}
      >
        AI / LLM 设置
      </h3>

      {message && (
        <div
          className="text-[var(--text-sm)] px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-md)]"
          style={{
            backgroundColor:
              message.type === "ok" ? "rgba(52, 199, 89, 0.1)" : "rgba(255, 59, 48, 0.1)",
            color: message.type === "ok" ? "#34C759" : "#FF3B30",
          }}
        >
          {message.text}
        </div>
      )}

      {/* 配置状态 */}
      <div
        className="flex items-center gap-[var(--space-2)] p-[var(--space-3)] rounded-[var(--radius-md)]"
        style={{
          backgroundColor: config?.is_configured
            ? "rgba(52, 199, 89, 0.08)"
            : "rgba(255, 59, 48, 0.08)",
          border: `1px solid ${config?.is_configured ? "rgba(52, 199, 89, 0.3)" : "rgba(255, 59, 48, 0.3)"}`,
        }}
      >
        {config?.is_configured ? (
          <Check size={16} style={{ color: "#34C759" }} />
        ) : (
          <AlertCircle size={16} style={{ color: "#FF3B30" }} />
        )}
        <span className="text-[var(--text-sm)]" style={{ color: "var(--text-primary)" }}>
          {config?.is_configured ? "API Key 已配置（应用内或环境变量）" : "API Key 未配置"}
        </span>
      </div>

      {/* 可编辑表单 */}
      <div className="space-y-[var(--space-3)] pointer-events-auto">
        <label className="block space-y-[var(--space-1)]">
          <span className="text-[var(--text-xs)] font-medium flex items-center gap-1" style={{ color: "var(--text-secondary)" }}>
            <Key size={12} /> API Key
          </span>
          <input
            type="password"
            autoComplete="off"
            value={apiKeyDraft}
            onChange={(e) => setApiKeyDraft(e.target.value)}
            placeholder={config?.is_configured ? "留空并保存 = 保留当前 Key" : "在此粘贴方舟 / OpenAI 兼容 Key"}
            className="w-full px-[var(--space-2)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-sm)] border outline-none focus:border-gray-500"
            style={{ color: "var(--text-primary)", background: "var(--surface-primary)", borderColor: "var(--border-primary)" }}
          />
        </label>

        <label className="block space-y-[var(--space-1)]">
          <span className="text-[var(--text-xs)] font-medium flex items-center gap-1" style={{ color: "var(--text-secondary)" }}>
            <Globe size={12} /> Base URL
          </span>
          <input
            type="url"
            value={baseUrl}
            onChange={(e) => setBaseUrl(e.target.value)}
            className="w-full px-[var(--space-2)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-sm)] border outline-none focus:border-gray-500"
            style={{ color: "var(--text-primary)", background: "var(--surface-primary)", borderColor: "var(--border-primary)" }}
          />
        </label>

        <label className="block space-y-[var(--space-1)]">
          <span className="text-[var(--text-xs)] font-medium flex items-center gap-1" style={{ color: "var(--text-secondary)" }}>
            <Cpu size={12} /> Model
          </span>
          <input
            type="text"
            value={model}
            onChange={(e) => setModel(e.target.value)}
            className="w-full px-[var(--space-2)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-sm)] border outline-none focus:border-gray-500"
            style={{ color: "var(--text-primary)", background: "var(--surface-primary)", borderColor: "var(--border-primary)" }}
          />
        </label>

        <div className="flex flex-wrap gap-[var(--space-2)] pt-[var(--space-1)]">
          <button
            type="button"
            disabled={saving}
            onClick={() => void handleSave()}
            className="btn-glass px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-sm)] font-medium flex items-center gap-2"
            style={{ backgroundColor: "rgba(31, 69, 110, 0.35)" }}
          >
            {saving ? <Loader2 size={14} className="animate-spin" /> : null}
            保存
          </button>
          <button
            type="button"
            disabled={saving || !config?.is_configured}
            onClick={() => void handleClearKey()}
            className="px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-xs)]"
            style={{ color: "var(--text-tertiary)", border: "1px solid rgba(255,255,255,0.15)" }}
            title="仅清除应用内保存的 Key，不影响系统环境变量"
          >
            清除应用内 Key
          </button>
        </div>
      </div>

      {config?.is_configured && (
        <div className="flex items-center gap-[var(--space-2)] text-[var(--text-xs)]" style={{ color: "var(--text-tertiary)" }}>
          <Key size={12} />
          <span>当前 Key 前缀：{config.api_key_masked}</span>
        </div>
      )}

      <div
        className="p-[var(--space-3)] rounded-[var(--radius-md)] space-y-[var(--space-2)]"
        style={{
          backgroundColor: "rgba(31, 69, 110, 0.12)",
          border: "1px solid rgba(31, 69, 110, 0.25)",
        }}
      >
        <p className="text-[var(--text-xs)] font-medium" style={{ color: "var(--text-primary)" }}>
          自动分类当前能分析什么？
        </p>
        <ul className="text-[var(--text-xs)] leading-relaxed list-disc pl-4 space-y-1" style={{ color: "var(--text-secondary)" }}>
          <li>
            请求里会带上：<strong>文件名、MIME、资产类型</strong>；对 <code className="text-[10px]">text/*</code>{" "}
            与 Markdown 还会读取<strong>最多约 32KB 文本片段</strong>。
          </li>
          <li>
            <strong>不会</strong>把图片、PDF、音视频等以二进制或多模态形式发给当前接口；这类文件主要依赖文件名与类型做推断。
          </li>
          <li>
            分类成功后会写入标签，并在磁盘上将文件整理到{" "}
            <code className="text-[10px]">assets/&lt;项目ID&gt;/organized/&lt;类别&gt;/</code>，并按模型返回的{" "}
            <code className="text-[10px]">suggestedFileName</code> 重命名（保留素材 ID 前缀防冲突）。
          </li>
        </ul>
        <button
          type="button"
          disabled={probeLoading || !config?.is_configured}
          onClick={() => void handleProbe()}
          className="btn-glass px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-xs)] font-medium flex items-center gap-2"
          style={{ backgroundColor: "rgba(31, 69, 110, 0.35)" }}
        >
          {probeLoading ? <Loader2 size={14} className="animate-spin" /> : null}
          测试分类 API（返回 JSON 含 tags）
        </button>
        {probeResult ? (
          <pre
            className="text-[10px] p-2 rounded overflow-x-auto max-h-32"
            style={{ background: "rgba(0,0,0,0.25)", color: "var(--text-tertiary)" }}
          >
            {JSON.stringify(probeResult, null, 2)}
          </pre>
        ) : null}
      </div>

      <div
        className="p-[var(--space-3)] rounded-[var(--radius-md)]"
        style={{
          backgroundColor: "rgba(255, 192, 0, 0.06)",
          border: "1px solid rgba(255, 192, 0, 0.15)",
        }}
      >
        <p className="text-[var(--text-xs)] leading-relaxed" style={{ color: "var(--text-secondary)" }}>
          API Key 保存在本机数据库（<code className="text-[10px]">notecapt.db</code>
          的 settings 表），不会上传到云端。若同时配置了环境变量，应用内保存的 Key 优先用于请求。
        </p>
      </div>
    </div>
  );
}
