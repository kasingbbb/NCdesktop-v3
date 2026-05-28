/**
 * task_016_settings_form — KcSettingsForm 单元测试
 *
 * 覆盖 AC-5 全部 7 测试 + AC-7 PM ESCALATE 补丁 2 测试：
 *
 *  1. renders_with_default_settings
 *  2. toggle_kcEnabled_disables_sub_toggles
 *  3. restart_button_only_shown_when_unavailable
 *  4. key_input_masks_value
 *  5. kc_use_ai_disabled_when_no_key
 *  6. test_key_connectivity_button_calls_health_endpoint  ← PM 补丁
 *  7. ai_enabled_status_renders_from_health_dto           ← PM 补丁
 *
 * 全部 IPC 走 vi.mock — 不依赖真 Tauri runtime。
 */
import { render, screen, fireEvent, waitFor, act } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import type { KcHealthStatus } from "../../../../lib/tauri-commands";

// =====================================================================
// 0. mock Tauri commands + event listener
// =====================================================================

const mockGetKcHealth = vi.fn<() => Promise<KcHealthStatus>>();
const mockRestartKcProcess = vi.fn<() => Promise<void>>();
const mockSetKcSettings = vi.fn<
  (s: {
    enabled: boolean;
    useAi: boolean;
    enableQa: boolean;
    enableLinks: boolean;
    zhipuKeyAction: "keep" | "clear" | "set";
    zhipuKeyValue?: string;
    openaiKeyAction: "keep" | "clear" | "set";
    openaiKeyValue?: string;
  }) => Promise<void>
>();
const mockGetAllSettings = vi.fn<() => Promise<Record<string, string>>>();

vi.mock("../../../../lib/tauri-commands", () => ({
  getKcHealth: (...args: unknown[]) =>
    mockGetKcHealth(...(args as Parameters<typeof mockGetKcHealth>)),
  restartKcProcess: (...args: unknown[]) =>
    mockRestartKcProcess(...(args as Parameters<typeof mockRestartKcProcess>)),
  setKcSettings: (...args: unknown[]) =>
    mockSetKcSettings(...(args as Parameters<typeof mockSetKcSettings>)),
  getAllSettings: (...args: unknown[]) =>
    mockGetAllSettings(...(args as Parameters<typeof mockGetAllSettings>)),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));

// =====================================================================
// 1. 测试 fixtures
// =====================================================================

const HEALTH_READY: KcHealthStatus = {
  status: "ready",
  reason: null,
  port: 58234,
  uptimeSecs: 42,
  lastCheck: "2026-05-27T12:00:00Z",
  aiEnabled: true,
};

const HEALTH_READY_AI_OFF: KcHealthStatus = {
  status: "ready",
  reason: null,
  port: 58234,
  uptimeSecs: 42,
  lastCheck: "2026-05-27T12:00:00Z",
  aiEnabled: false,
};

const HEALTH_UNAVAILABLE: KcHealthStatus = {
  status: "unavailable",
  reason: "python not found",
  port: null,
  uptimeSecs: null,
  lastCheck: "2026-05-27T12:00:00Z",
  aiEnabled: null,
};

const HEALTH_STOPPED: KcHealthStatus = {
  status: "stopped",
  reason: null,
  port: null,
  uptimeSecs: null,
  lastCheck: "2026-05-27T12:00:00Z",
  aiEnabled: null,
};

const DEFAULT_SETTINGS_ROW: Record<string, string> = {
  "kc.enabled": "true",
  "kc.use_ai": "true",
  "kc.enable_qa": "true",
  "kc.enable_links": "true",
  "kc.zhipu_api_key": "zp-1234567890abcdef",
  "kc.openai_api_key": "",
};

beforeEach(() => {
  vi.clearAllMocks();
  mockGetAllSettings.mockResolvedValue({ ...DEFAULT_SETTINGS_ROW });
  mockGetKcHealth.mockResolvedValue({ ...HEALTH_READY });
  mockRestartKcProcess.mockResolvedValue();
  mockSetKcSettings.mockResolvedValue();
});

/** 等待 loading -> rendered 完成（form 出现）。 */
async function renderAndWait(): Promise<void> {
  const { KcSettingsForm } = await import("../KcSettingsForm");
  render(<KcSettingsForm />);
  await waitFor(() => {
    expect(screen.getByTestId("kc-settings-form")).toBeInTheDocument();
  });
}

// =====================================================================
// 2. 测试用例
// =====================================================================

describe("KcSettingsForm", () => {
  // ---------- AC-5.1 ----------
  it("renders_with_default_settings — 加载默认 KcSettings 渲染标题/总开关/双 Key/子开关/状态行", async () => {
    await renderAndWait();

    // 标题
    expect(screen.getByText("知识增强（KC）")).toBeInTheDocument();

    // 总开关
    expect(
      screen.getByRole("switch", { name: "启用知识增强" }),
    ).toBeInTheDocument();

    // 双 Key 输入框
    expect(screen.getByTestId("zhipu-input")).toBeInTheDocument();
    expect(screen.getByTestId("openai-input")).toBeInTheDocument();

    // 3 个子开关
    expect(
      screen.getByRole("switch", { name: "AI 增强（摘要 / 标签）" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("switch", { name: "问答抽取" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("switch", { name: "链接抽取" }),
    ).toBeInTheDocument();

    // KC 服务状态行
    expect(screen.getByTestId("kc-service-status")).toHaveTextContent("已就绪");

    // 保存按钮
    expect(screen.getByTestId("kc-save-button")).toBeInTheDocument();
  });

  // ---------- AC-5.2 ----------
  it("toggle_kcEnabled_disables_sub_toggles — 关闭总开关后 3 子开关 disabled", async () => {
    await renderAndWait();

    // 初始：3 子开关 enabled（kcEnabled=true + zhipu key configured）
    const useAi = screen.getByRole("switch", { name: "AI 增强（摘要 / 标签）" });
    expect(useAi).not.toBeDisabled();

    // 点击总开关 OFF
    const main = screen.getByRole("switch", { name: "启用知识增强" });
    fireEvent.click(main);

    // 子开关全部 disabled
    expect(useAi).toBeDisabled();
    expect(
      screen.getByRole("switch", { name: "问答抽取" }),
    ).toBeDisabled();
    expect(
      screen.getByRole("switch", { name: "链接抽取" }),
    ).toBeDisabled();
  });

  // ---------- AC-5.3 ----------
  it("restart_button_only_shown_when_unavailable — ready 态不显示重启按钮，unavailable 态显示", async () => {
    // 第一次：ready
    mockGetKcHealth.mockResolvedValueOnce({ ...HEALTH_READY });
    await renderAndWait();

    // ready 态：无重启按钮
    expect(screen.queryByTestId("kc-restart-button")).not.toBeInTheDocument();

    // 切换 mock 为 unavailable，模拟手动 refresh
    mockGetKcHealth.mockResolvedValue({ ...HEALTH_UNAVAILABLE });

    // 重置已渲染组件：通过 unmount/remount（模拟 status 变化）
    const { KcSettingsForm } = await import("../KcSettingsForm");
    const { unmount } = render(<KcSettingsForm />);
    // 等待 unavailable health 加载
    await waitFor(() => {
      const buttons = screen.queryAllByTestId("kc-restart-button");
      expect(buttons.length).toBeGreaterThanOrEqual(1);
    });

    // 验证：至少一个 restart-button 存在
    expect(screen.getAllByTestId("kc-restart-button").length).toBeGreaterThan(0);

    unmount();
  });

  // ---------- AC-5.4 ----------
  it("key_input_masks_value — Key 输入 type=password + 不显示原 Key 文本", async () => {
    await renderAndWait();

    // 输入是 password 类型
    const zhipuInput = screen.getByTestId("zhipu-input") as HTMLInputElement;
    expect(zhipuInput.type).toBe("password");

    // mask 显示前 4 后 4（"zp-1****cdef"）—— DB 原 Key 全文不应出现在 DOM
    const mask = screen.getByTestId("zhipu-mask");
    expect(mask).toHaveTextContent("zp-1****cdef");
    // 原 Key 全文不应出现
    expect(screen.queryByText("zp-1234567890abcdef")).not.toBeInTheDocument();
  });

  // ---------- AC-5.5 ----------
  it("kc_use_ai_disabled_when_no_key — 两个 Key 都未配置时子开关 disabled", async () => {
    // 覆盖：两个 Key 都为空串
    mockGetAllSettings.mockResolvedValue({
      ...DEFAULT_SETTINGS_ROW,
      "kc.zhipu_api_key": "",
      "kc.openai_api_key": "",
    });

    await renderAndWait();

    // useAi / enableQa / enableLinks 全部 disabled
    expect(
      screen.getByRole("switch", { name: "AI 增强（摘要 / 标签）" }),
    ).toBeDisabled();
    expect(
      screen.getByRole("switch", { name: "问答抽取" }),
    ).toBeDisabled();
    expect(
      screen.getByRole("switch", { name: "链接抽取" }),
    ).toBeDisabled();

    // 提示存在
    expect(screen.getByTestId("no-key-hint")).toHaveTextContent("至少配置一个 Key 后可用");
  });

  // ---------- AC-7 PM 补丁 ① ----------
  it("test_key_connectivity_button_calls_health_endpoint — 点击 [测试连通性] 调 getKcHealth 并显示结果", async () => {
    await renderAndWait();

    // 初次 mount 已调一次 getKcHealth；记录 call count baseline
    const baselineCalls = mockGetKcHealth.mock.calls.length;

    // 点击智谱测试按钮
    const zhipuTestBtn = screen.getByTestId("zhipu-test-button");
    await act(async () => {
      fireEvent.click(zhipuTestBtn);
    });

    // getKcHealth 被多调一次
    await waitFor(() => {
      expect(mockGetKcHealth.mock.calls.length).toBeGreaterThan(baselineCalls);
    });

    // ai_enabled=true → 显示 "AI 已就绪（ai_enabled=true）"
    await waitFor(() => {
      const result = screen.getByTestId("zhipu-conn-result");
      expect(result).toHaveTextContent(/AI 已就绪/);
      expect(result.getAttribute("data-conn-kind")).toBe("ok");
    });

    // 切到 aiEnabled=false 再点 openai
    mockGetKcHealth.mockResolvedValue({ ...HEALTH_READY_AI_OFF });
    const openaiTestBtn = screen.getByTestId("openai-test-button");
    await act(async () => {
      fireEvent.click(openaiTestBtn);
    });
    await waitFor(() => {
      const result = screen.getByTestId("openai-conn-result");
      expect(result).toHaveTextContent(/Key 配置但 AI 未启用/);
      expect(result.getAttribute("data-conn-kind")).toBe("err");
    });
  });

  // ---------- AC-7 PM 补丁 ② ----------
  it("ai_enabled_status_renders_from_health_dto — aiEnabled=true/false 状态行文案差异", async () => {
    // 起步：aiEnabled=true
    mockGetKcHealth.mockResolvedValueOnce({ ...HEALTH_READY });
    await renderAndWait();
    await waitFor(() => {
      expect(screen.getByTestId("kc-ai-enabled-status")).toHaveTextContent("已启用");
    });

    // 重新渲染（aiEnabled=false）
    mockGetKcHealth.mockResolvedValue({ ...HEALTH_READY_AI_OFF });

    const { KcSettingsForm } = await import("../KcSettingsForm");
    const { unmount } = render(<KcSettingsForm />);
    await waitFor(() => {
      // 多个实例都渲染了；取最后一个（最新挂载）
      const all = screen.getAllByTestId("kc-ai-enabled-status");
      expect(all[all.length - 1]).toHaveTextContent("未启用");
    });
    unmount();
  });

  // ---------- 额外：保存按钮走 keep 语义（draft 留空、key 已配置）----------
  it("save_button_uses_keep_when_draft_empty_and_key_configured — 保存按钮按 keep 语义调 setKcSettings", async () => {
    await renderAndWait();

    const saveBtn = screen.getByTestId("kc-save-button");
    await act(async () => {
      fireEvent.click(saveBtn);
    });

    await waitFor(() => {
      expect(mockSetKcSettings).toHaveBeenCalledTimes(1);
    });
    const payload = mockSetKcSettings.mock.calls[0][0];
    // zhipu 已配置 → draft 空 → keep；openai 未配置 → draft 空 → keep
    expect(payload.zhipuKeyAction).toBe("keep");
    expect(payload.openaiKeyAction).toBe("keep");
    expect(payload.enabled).toBe(true);
  });

  // ---------- 额外：stopped 也算"可重启"，应显示重启按钮 ----------
  it("restart_button_shown_when_stopped — stopped 态也显示重启按钮（与 unavailable 同处理）", async () => {
    mockGetKcHealth.mockResolvedValue({ ...HEALTH_STOPPED });
    await renderAndWait();
    await waitFor(() => {
      expect(screen.getByTestId("kc-restart-button")).toBeInTheDocument();
    });
  });
});
