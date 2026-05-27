/**
 * task_016_settings_form — KcSettingsForm
 *
 * F11 Settings UI：让用户配置 KC（Knowledge Compiler）增强能力：
 * - 总开关 `kcEnabled`
 * - 双 Key（智谱 AI / OpenAI）输入：mask + keep/clear/set 三态
 * - 3 子开关（useAi / enableQa / enableLinks），仅在至少一个 Key 配置时启用
 * - KC 服务状态行（`get_kc_health` 拉取 + 订阅 `notecapt/kc-status-changed`）
 * - AI 能力状态行（**PM ESCALATE 2026-05-27 补丁**，读 KcHealthStatus.aiEnabled）
 * - 每个 Key 输入框旁的 [测试连通性] 按钮（**AC-7 PM 补丁**，调 `getKcHealth`）
 *
 * ## 关键约束
 *
 * - 表单状态用 React `useState`（局部状态足够，未引入 Zustand）。
 * - 视觉与交互模式参考 `LLMSettingsForm.tsx`（Key 输入 + mask + 保存按钮）。
 * - 初值读取走 `getAllSettings()` 通用 command（避免新增后端 `get_kc_settings`）。
 * - 健康检查轮询：仅在 document.visibilityState === "visible" 时每 5s 一次，
 *   unmount + visibilitychange + 卸载事件订阅均清理（reviewer 重点关注项）。
 * - Key 输入框 `type="password"` + `autoComplete="off"`。
 * - 不引入新 npm 依赖（用 lucide-react + Tailwind）。
 *
 * ## 与 `LLMSettingsForm` 的差异
 *
 * - LLMSettingsForm 单 Key，本组件双 Key；
 * - 本组件多了一层 "kc 服务状态"实时显示 + 测试连通性按钮；
 * - LLMSettingsForm 保存路径是 `saveLLMConfig`，本组件走 `setKcSettings`
 *   （后端 Key 变化时 spawn restart，前端通过订阅 `notecapt/kc-status-changed`
 *   感知 restart 过程，不阻塞 save Ok 回包）。
 */
import { useCallback, useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  AlertCircle,
  Check,
  Key as KeyIcon,
  Loader2,
  RotateCw,
  Zap,
} from "lucide-react";
import {
  getAllSettings,
  getKcHealth,
  restartKcProcess,
  setKcSettings,
  type KcHealthStatus,
} from "../../../lib/tauri-commands";

// =====================================================================
// 1. 常量 / 工具
// =====================================================================

/** KC settings DB 行 key（与 `src-tauri/src/kc/settings.rs` 7 个常量对齐）。 */
const KEY_KC_ENABLED = "kc.enabled";
const KEY_KC_USE_AI = "kc.use_ai";
const KEY_KC_ENABLE_QA = "kc.enable_qa";
const KEY_KC_ENABLE_LINKS = "kc.enable_links";
const KEY_KC_ZHIPU_API_KEY = "kc.zhipu_api_key";
const KEY_KC_OPENAI_API_KEY = "kc.openai_api_key";

/** 健康检查轮询间隔（ms）。仅在页面可见时启用。 */
const HEALTH_POLL_INTERVAL_MS = 5000;

/** 把 settings 表 string 值解析为 bool，缺失或非 "true" 视为 false。 */
function parseBool(raw: string | undefined, fallback: boolean): boolean {
  if (raw == null || raw === "") return fallback;
  return raw === "true";
}

/** Key 字符串 trim 后非空才视为"已配置"。 */
function isKeyConfigured(raw: string | undefined): boolean {
  return typeof raw === "string" && raw.trim().length > 0;
}

/** Key 中间打码：前 4 后 4 之间用 ****，长度不足则全打 *。 */
function maskKey(raw: string): string {
  const k = raw.trim();
  if (k.length === 0) return "";
  if (k.length <= 8) return "*".repeat(k.length);
  return `${k.slice(0, 4)}****${k.slice(-4)}`;
}

// =====================================================================
// 2. 子组件：紧凑型 toggle（与 SettingsPanel 现有 ToggleSwitch 视觉一致）
// =====================================================================

interface ToggleProps {
  checked: boolean;
  disabled?: boolean;
  onChange: (next: boolean) => void;
  ariaLabel: string;
}

function Toggle({ checked, disabled, onChange, ariaLabel }: ToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={ariaLabel}
      aria-disabled={disabled ? true : undefined}
      disabled={disabled}
      className="relative w-10 h-6 rounded-full transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
      style={{
        backgroundColor: checked ? "#111827" : "var(--surface-tertiary)",
        border: `1px solid ${checked ? "#111827" : "var(--border-primary)"}`,
      }}
      onClick={() => onChange(!checked)}
    >
      <div
        className="absolute top-0.5 w-4 h-4 rounded-full transition-transform bg-white"
        style={{
          transform: checked ? "translateX(18px)" : "translateX(2px)",
          boxShadow: "var(--shadow-sm)",
        }}
      />
    </button>
  );
}

// =====================================================================
// 3. 主组件
// =====================================================================

/** 单 Key 输入区的临时连通性测试结果（AC-7）。 */
type ConnectivityResult =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "ok"; text: string }
  | { kind: "err"; text: string };

export function KcSettingsForm() {
  // ---------- 表单字段（AC-4：纯 useState）----------
  const [kcEnabled, setKcEnabled] = useState(true);
  const [useAi, setUseAi] = useState(true);
  const [enableQa, setEnableQa] = useState(true);
  const [enableLinks, setEnableLinks] = useState(true);

  /** Key 是否已在 DB 中配置（用于显示 mask + 是否允许 keep）。 */
  const [zhipuConfigured, setZhipuConfigured] = useState(false);
  const [openaiConfigured, setOpenaiConfigured] = useState(false);
  /** Key mask 显示（仅用于 UI 提示，不会泄漏完整 Key）。 */
  const [zhipuMasked, setZhipuMasked] = useState("");
  const [openaiMasked, setOpenaiMasked] = useState("");

  /** 用户输入的新 Key 草稿（留空 = keep，clear 按钮另算）。 */
  const [zhipuDraft, setZhipuDraft] = useState("");
  const [openaiDraft, setOpenaiDraft] = useState("");
  /** 用户点击"清除"后，下次保存时该 Key 走 clear 语义。 */
  const [zhipuClearPending, setZhipuClearPending] = useState(false);
  const [openaiClearPending, setOpenaiClearPending] = useState(false);

  // ---------- 加载 / 保存 / 健康状态 ----------
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [restarting, setRestarting] = useState(false);
  const [message, setMessage] = useState<{ type: "ok" | "err"; text: string } | null>(null);
  const [health, setHealth] = useState<KcHealthStatus | null>(null);

  // ---------- AC-7 连通性测试 ----------
  const [zhipuConn, setZhipuConn] = useState<ConnectivityResult>({ kind: "idle" });
  const [openaiConn, setOpenaiConn] = useState<ConnectivityResult>({ kind: "idle" });

  /** 轮询定时器引用（visibility 切换时启停）。 */
  const pollTimerRef = useRef<number | null>(null);

  // ---------- 初值加载 ----------

  const loadInitial = useCallback(async (): Promise<void> => {
    try {
      const all = await getAllSettings();
      setKcEnabled(parseBool(all[KEY_KC_ENABLED], true));
      setUseAi(parseBool(all[KEY_KC_USE_AI], true));
      setEnableQa(parseBool(all[KEY_KC_ENABLE_QA], true));
      setEnableLinks(parseBool(all[KEY_KC_ENABLE_LINKS], true));

      const zhipu = all[KEY_KC_ZHIPU_API_KEY] ?? "";
      const openai = all[KEY_KC_OPENAI_API_KEY] ?? "";
      setZhipuConfigured(isKeyConfigured(zhipu));
      setOpenaiConfigured(isKeyConfigured(openai));
      setZhipuMasked(isKeyConfigured(zhipu) ? maskKey(zhipu) : "");
      setOpenaiMasked(isKeyConfigured(openai) ? maskKey(openai) : "");
    } catch {
      // 初值读取失败：保留默认值；不 toast（loading 阶段静默）。
    }
  }, []);

  const refreshHealth = useCallback(async (): Promise<void> => {
    try {
      const h = await getKcHealth();
      setHealth(h);
    } catch {
      // health_check 后端永不抛错；catch 仅为兜底，不更新 state。
    }
  }, []);

  // ---------- mount：拉初值 + 第一次 health + 订阅事件 + 启动 visibility 轮询 ----------

  useEffect(() => {
    let mounted = true;
    let unlisten: UnlistenFn | null = null;

    void (async () => {
      setLoading(true);
      await Promise.all([loadInitial(), refreshHealth()]);
      if (mounted) {
        setLoading(false);
      }
    })();

    // AC-2：订阅 kc-status-changed → 立即重新拉 health（含 aiEnabled）。
    void listen("notecapt/kc-status-changed", () => {
      void refreshHealth();
    }).then((fn) => {
      if (mounted) {
        unlisten = fn;
      } else {
        // 组件已卸载 / 但订阅刚返回：立即清理。
        fn();
      }
    });

    return () => {
      mounted = false;
      unlisten?.();
    };
  }, [loadInitial, refreshHealth]);

  // ---------- visibility-gated 轮询（reviewer 重点：unmount 清理 + visibility 暂停） ----------

  useEffect(() => {
    const startPolling = (): void => {
      if (pollTimerRef.current != null) return;
      pollTimerRef.current = window.setInterval(() => {
        void refreshHealth();
      }, HEALTH_POLL_INTERVAL_MS);
    };
    const stopPolling = (): void => {
      if (pollTimerRef.current != null) {
        window.clearInterval(pollTimerRef.current);
        pollTimerRef.current = null;
      }
    };

    const handleVisibility = (): void => {
      if (document.visibilityState === "visible") {
        startPolling();
        // 立即拉一次（visibility 切回时不等 5s）。
        void refreshHealth();
      } else {
        stopPolling();
      }
    };

    if (document.visibilityState === "visible") {
      startPolling();
    }
    document.addEventListener("visibilitychange", handleVisibility);

    return () => {
      stopPolling();
      document.removeEventListener("visibilitychange", handleVisibility);
    };
  }, [refreshHealth]);

  // ---------- 派生状态 ----------

  /** 任一 Key 配置（已在 DB 中 OR 当前草稿 set）。 */
  const anyKeyConfigured =
    (zhipuConfigured && !zhipuClearPending) ||
    (openaiConfigured && !openaiClearPending) ||
    zhipuDraft.trim().length > 0 ||
    openaiDraft.trim().length > 0;

  /** 子开关是否被禁用（kcEnabled OFF 或 无任何 Key）。 */
  const subTogglesDisabled = !kcEnabled || !anyKeyConfigured;

  /** KC 服务状态行文案。 */
  const statusText = (() => {
    if (!health) return "加载中…";
    switch (health.status) {
      case "ready":
        return "已就绪";
      case "starting":
        return "启动中…";
      case "stopped":
        return "未启动";
      case "unavailable":
        return `不可用：${health.reason ?? "未知原因"}`;
      default:
        return health.status;
    }
  })();

  const showRestartButton = health?.status === "unavailable" || health?.status === "stopped";

  /** AI 能力状态行文案（PM ESCALATE 补丁）。`null/undefined → "未知"`。 */
  const aiEnabledText = (() => {
    if (!health) return null;
    if (health.aiEnabled == null) return "未知";
    return health.aiEnabled ? "已启用" : "未启用";
  })();

  // ---------- 事件处理 ----------

  async function handleSave(): Promise<void> {
    setSaving(true);
    setMessage(null);
    try {
      const zhipuTrim = zhipuDraft.trim();
      const openaiTrim = openaiDraft.trim();
      const zhipuAction: "keep" | "clear" | "set" = zhipuClearPending
        ? "clear"
        : zhipuTrim.length > 0
          ? "set"
          : "keep";
      const openaiAction: "keep" | "clear" | "set" = openaiClearPending
        ? "clear"
        : openaiTrim.length > 0
          ? "set"
          : "keep";

      await setKcSettings({
        enabled: kcEnabled,
        useAi,
        enableQa,
        enableLinks,
        zhipuKeyAction: zhipuAction,
        zhipuKeyValue: zhipuAction === "set" ? zhipuTrim : undefined,
        openaiKeyAction: openaiAction,
        openaiKeyValue: openaiAction === "set" ? openaiTrim : undefined,
      });

      // 清掉 draft + pending clear；重新加载初值（拿到新的 mask）。
      setZhipuDraft("");
      setOpenaiDraft("");
      setZhipuClearPending(false);
      setOpenaiClearPending(false);
      await loadInitial();
      // 触发一次 health 刷新（restart 在 backend 异步跑，event 还会再推一次）。
      void refreshHealth();
      setMessage({ type: "ok", text: "已保存。Key 若有变化将在后台重启 KC（数秒）。" });
    } catch (e) {
      setMessage({ type: "err", text: `保存失败：${String(e)}` });
    } finally {
      setSaving(false);
    }
  }

  async function handleRestart(): Promise<void> {
    setRestarting(true);
    setMessage(null);
    try {
      await restartKcProcess();
      setMessage({ type: "ok", text: "已触发 KC 重启" });
      // 立即拉一次 health；event 会继续推 starting → ready/unavailable。
      void refreshHealth();
    } catch (e) {
      setMessage({ type: "err", text: `重启失败：${String(e)}` });
    } finally {
      setRestarting(false);
    }
  }

  /** AC-7：测试 Key 连通性（调 getKcHealth，根据 aiEnabled 判定）。 */
  async function testConnectivity(provider: "zhipu" | "openai"): Promise<void> {
    const setConn = provider === "zhipu" ? setZhipuConn : setOpenaiConn;
    setConn({ kind: "loading" });
    try {
      const h = await getKcHealth();
      if (h.status !== "ready") {
        setConn({
          kind: "err",
          text: `KC 服务不可用（${h.status}）`,
        });
        return;
      }
      if (h.aiEnabled === true) {
        setConn({ kind: "ok", text: "AI 已就绪（ai_enabled=true）" });
      } else if (h.aiEnabled === false) {
        setConn({
          kind: "err",
          text: "Key 配置但 AI 未启用（检查 KC 后端 ai_provider 配置）",
        });
      } else {
        // null/undefined：KC backend 未透传或字段缺失，可视为 KC 服务 ready 但能力未知。
        setConn({
          kind: "err",
          text: "KC 已就绪，但 ai_enabled 字段未知（请检查 KC 后端版本）",
        });
      }
    } catch (e) {
      setConn({ kind: "err", text: `测试失败：${String(e)}` });
    }
  }

  // ---------- 渲染 ----------

  if (loading) {
    return (
      <div className="p-[var(--space-4)] text-center">
        <span
          className="text-[var(--text-sm)]"
          style={{ color: "var(--text-tertiary)" }}
        >
          加载 KC 配置中…
        </span>
      </div>
    );
  }

  return (
    <div className="space-y-[var(--space-4)]" data-testid="kc-settings-form">
      <h3
        className="text-[var(--text-base)] font-semibold"
        style={{ color: "var(--text-primary)" }}
      >
        知识增强（KC）
      </h3>

      {/* 消息条 */}
      {message && (
        <div
          className="text-[var(--text-sm)] px-[var(--space-3)] py-[var(--space-2)] rounded-[var(--radius-md)]"
          style={{
            backgroundColor:
              message.type === "ok" ? "rgba(52, 199, 89, 0.1)" : "rgba(255, 59, 48, 0.1)",
            color: message.type === "ok" ? "#34C759" : "#FF3B30",
          }}
          data-testid="kc-message"
        >
          {message.text}
        </div>
      )}

      {/* 总开关 */}
      <div className="flex items-center justify-between">
        <div>
          <div
            className="text-[var(--text-sm)] font-medium"
            style={{ color: "var(--text-primary)" }}
          >
            启用知识增强
          </div>
          <div
            className="text-[var(--text-xs)]"
            style={{ color: "var(--text-tertiary)" }}
          >
            关闭后导入素材将直接走 MarkItDown 基础转换，不再调用 KC。
          </div>
        </div>
        <Toggle
          checked={kcEnabled}
          onChange={setKcEnabled}
          ariaLabel="启用知识增强"
        />
      </div>

      {/* ── AI 增强能力（双 Key） ───────────────── */}
      <div
        className="p-[var(--space-3)] rounded-[var(--radius-md)] space-y-[var(--space-3)]"
        style={{
          backgroundColor: "rgba(31, 69, 110, 0.06)",
          border: "1px solid var(--border-primary)",
        }}
      >
        <div
          className="text-[var(--text-sm)] font-medium"
          style={{ color: "var(--text-primary)" }}
        >
          AI 增强能力
        </div>
        <p
          className="text-[var(--text-xs)] leading-relaxed"
          style={{ color: "var(--text-secondary)" }}
        >
          KC 增强使用智谱 AI / OpenAI Key，与 NC 内部 LLM Key 独立。仅 KC 子进程使用。
        </p>

        {/* 智谱 Key */}
        <KeyInputBlock
          label="智谱 AI Key"
          configured={zhipuConfigured && !zhipuClearPending}
          masked={zhipuMasked}
          draft={zhipuDraft}
          onDraftChange={setZhipuDraft}
          clearPending={zhipuClearPending}
          onClearToggle={() => {
            setZhipuClearPending((v) => !v);
            setZhipuDraft("");
          }}
          connectivity={zhipuConn}
          onTest={() => void testConnectivity("zhipu")}
          testId="zhipu"
        />

        {/* OpenAI Key */}
        <KeyInputBlock
          label="OpenAI Key"
          configured={openaiConfigured && !openaiClearPending}
          masked={openaiMasked}
          draft={openaiDraft}
          onDraftChange={setOpenaiDraft}
          clearPending={openaiClearPending}
          onClearToggle={() => {
            setOpenaiClearPending((v) => !v);
            setOpenaiDraft("");
          }}
          connectivity={openaiConn}
          onTest={() => void testConnectivity("openai")}
          testId="openai"
        />
      </div>

      {/* ── 子开关 ─────────────────────────────── */}
      <div
        className="p-[var(--space-3)] rounded-[var(--radius-md)] space-y-[var(--space-3)]"
        style={{
          backgroundColor: "rgba(31, 69, 110, 0.06)",
          border: "1px solid var(--border-primary)",
        }}
      >
        <div className="flex items-center justify-between">
          <div
            className="text-[var(--text-sm)] font-medium"
            style={{ color: "var(--text-primary)" }}
          >
            功能开关
          </div>
          {!anyKeyConfigured && (
            <span
              className="text-[var(--text-xs)]"
              style={{ color: "var(--text-tertiary)" }}
              data-testid="no-key-hint"
            >
              至少配置一个 Key 后可用
            </span>
          )}
        </div>
        <SubToggleRow
          label="AI 增强（摘要 / 标签）"
          checked={useAi}
          disabled={subTogglesDisabled}
          onChange={setUseAi}
          ariaLabel="AI 增强（摘要 / 标签）"
        />
        <SubToggleRow
          label="问答抽取"
          checked={enableQa}
          disabled={subTogglesDisabled}
          onChange={setEnableQa}
          ariaLabel="问答抽取"
        />
        <SubToggleRow
          label="链接抽取"
          checked={enableLinks}
          disabled={subTogglesDisabled}
          onChange={setEnableLinks}
          ariaLabel="链接抽取"
        />
      </div>

      {/* ── KC 服务状态行 ─────────────────────────── */}
      <div
        className="p-[var(--space-3)] rounded-[var(--radius-md)] space-y-[var(--space-2)]"
        style={{
          backgroundColor: "var(--surface-primary)",
          border: "1px solid var(--border-primary)",
        }}
      >
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <span
              className="text-[var(--text-xs)]"
              style={{ color: "var(--text-secondary)" }}
            >
              KC 服务状态：
            </span>
            <span
              className="text-[var(--text-sm)] font-medium"
              style={{
                color:
                  health?.status === "ready"
                    ? "#34C759"
                    : health?.status === "unavailable"
                      ? "#FF3B30"
                      : "var(--text-primary)",
              }}
              data-testid="kc-service-status"
            >
              {statusText}
            </span>
          </div>
          {showRestartButton && (
            <button
              type="button"
              disabled={restarting}
              onClick={() => void handleRestart()}
              className="btn-glass px-[var(--space-3)] py-[var(--space-1)] rounded-[var(--radius-md)] text-[var(--text-xs)] flex items-center gap-1"
              data-testid="kc-restart-button"
            >
              {restarting ? (
                <Loader2 size={12} className="animate-spin" />
              ) : (
                <RotateCw size={12} />
              )}
              重启 KC 服务
            </button>
          )}
        </div>
        {/* AI 能力状态（PM ESCALATE 补丁） */}
        {aiEnabledText != null && (
          <div className="flex items-center gap-2">
            <span
              className="text-[var(--text-xs)]"
              style={{ color: "var(--text-secondary)" }}
            >
              AI 能力：
            </span>
            <span
              className="text-[var(--text-sm)] font-medium"
              style={{
                color:
                  health?.aiEnabled === true
                    ? "#34C759"
                    : health?.aiEnabled === false
                      ? "#FF3B30"
                      : "var(--text-tertiary)",
              }}
              data-testid="kc-ai-enabled-status"
            >
              {aiEnabledText}
            </span>
          </div>
        )}
      </div>

      {/* ── 保存按钮 ─────────────────────────────── */}
      <div className="flex justify-end gap-[var(--space-2)]">
        <button
          type="button"
          disabled={saving}
          onClick={() => void handleSave()}
          className="btn-glass px-[var(--space-4)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-sm)] font-medium flex items-center gap-2"
          style={{ backgroundColor: "rgba(31, 69, 110, 0.35)" }}
          data-testid="kc-save-button"
        >
          {saving ? <Loader2 size={14} className="animate-spin" /> : null}
          保存
        </button>
      </div>
    </div>
  );
}

// =====================================================================
// 4. KeyInputBlock 子组件
// =====================================================================

interface KeyInputBlockProps {
  label: string;
  configured: boolean;
  masked: string;
  draft: string;
  onDraftChange: (v: string) => void;
  clearPending: boolean;
  onClearToggle: () => void;
  connectivity: ConnectivityResult;
  onTest: () => void;
  testId: "zhipu" | "openai";
}

function KeyInputBlock({
  label,
  configured,
  masked,
  draft,
  onDraftChange,
  clearPending,
  onClearToggle,
  connectivity,
  onTest,
  testId,
}: KeyInputBlockProps) {
  return (
    <div className="space-y-[var(--space-1)]">
      <div className="flex items-center justify-between gap-2">
        <span
          className="text-[var(--text-xs)] font-medium flex items-center gap-1"
          style={{ color: "var(--text-secondary)" }}
        >
          <KeyIcon size={12} />
          {label}
        </span>
        {configured && (
          <span
            className="text-[10px]"
            style={{ color: "var(--text-tertiary)" }}
            data-testid={`${testId}-mask`}
          >
            当前 Key：{masked}
          </span>
        )}
      </div>
      <div className="flex items-center gap-[var(--space-2)]">
        <input
          type="password"
          autoComplete="off"
          value={draft}
          onChange={(e) => onDraftChange(e.target.value)}
          placeholder={
            configured
              ? "留空并保存 = 保留当前 Key"
              : "在此粘贴 Key（保存后存入本机数据库）"
          }
          className="flex-1 px-[var(--space-2)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-sm)] border outline-none focus:border-gray-500"
          style={{
            color: "var(--text-primary)",
            background: "var(--surface-primary)",
            borderColor: clearPending ? "#FF3B30" : "var(--border-primary)",
          }}
          data-testid={`${testId}-input`}
          aria-label={`${label} 输入`}
        />
        {/* AC-7：测试连通性按钮 */}
        <button
          type="button"
          disabled={connectivity.kind === "loading"}
          onClick={onTest}
          className="px-[var(--space-2)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-xs)] flex items-center gap-1"
          style={{
            color: "var(--text-secondary)",
            border: "1px solid var(--border-primary)",
            background: "transparent",
          }}
          data-testid={`${testId}-test-button`}
          title="测试连通性 / Test connectivity"
        >
          {connectivity.kind === "loading" ? (
            <Loader2 size={12} className="animate-spin" />
          ) : (
            <Zap size={12} />
          )}
          测试连通性
        </button>
        {configured && (
          <button
            type="button"
            onClick={onClearToggle}
            className="px-[var(--space-2)] py-[var(--space-2)] rounded-[var(--radius-md)] text-[var(--text-xs)]"
            style={{
              color: clearPending ? "#FF3B30" : "var(--text-tertiary)",
              border: `1px solid ${clearPending ? "#FF3B30" : "var(--border-primary)"}`,
              background: "transparent",
            }}
            data-testid={`${testId}-clear-button`}
            title={clearPending ? "撤销清除" : "保存时清除应用内 Key"}
          >
            {clearPending ? "撤销清除" : "清除"}
          </button>
        )}
      </div>
      {/* 连通性结果（AC-7） */}
      {connectivity.kind === "ok" && (
        <div
          className="text-[var(--text-xs)] flex items-center gap-1"
          style={{ color: "#34C759" }}
          data-testid={`${testId}-conn-result`}
          data-conn-kind="ok"
        >
          <Check size={12} /> {connectivity.text}
        </div>
      )}
      {connectivity.kind === "err" && (
        <div
          className="text-[var(--text-xs)] flex items-center gap-1"
          style={{ color: "#FF3B30" }}
          data-testid={`${testId}-conn-result`}
          data-conn-kind="err"
        >
          <AlertCircle size={12} /> {connectivity.text}
        </div>
      )}
    </div>
  );
}

// =====================================================================
// 5. SubToggleRow 子组件
// =====================================================================

interface SubToggleRowProps {
  label: string;
  checked: boolean;
  disabled: boolean;
  onChange: (v: boolean) => void;
  ariaLabel: string;
}

function SubToggleRow({ label, checked, disabled, onChange, ariaLabel }: SubToggleRowProps) {
  return (
    <div className="flex items-center justify-between">
      <span
        className="text-[var(--text-sm)]"
        style={{
          color: disabled ? "var(--text-tertiary)" : "var(--text-primary)",
        }}
      >
        {label}
      </span>
      <Toggle
        checked={checked}
        disabled={disabled}
        onChange={onChange}
        ariaLabel={ariaLabel}
      />
    </div>
  );
}
