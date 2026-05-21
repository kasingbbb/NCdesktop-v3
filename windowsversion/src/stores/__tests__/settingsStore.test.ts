/**
 * v2 Sidebar Redesign — settingsStore 单元测试（task_003 / F-P0-1 / ADR-002）。
 *
 * 覆盖：
 *   - AppSettings 4 字段默认值（AC-1）
 *   - loadSettings JSON.parse 还原 boolean（AC-2）
 *   - updateSetting 写入后 store state 同步（AC-2）
 *   - useEffectiveLearningSettings 派生四行真值表（AC-3）
 *   - 真值不丢：主开关 OFF 不会清零依赖字段内存值（AC-4，PRD 不可妥协底线 1）
 *   - 主开关 OFF 时写依赖字段，effective 仍为 false 但真值保留（值不丢的核心）
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook } from "@testing-library/react";

// 把 tauri-commands mock 掉 — settingsStore 不应触达真实 Tauri IPC（测试无 host）。
vi.mock("../../lib/tauri-commands", () => {
  const kv: Record<string, string> = {};
  return {
    setSetting: vi.fn(async (key: string, value: string) => {
      kv[key] = value;
    }),
    getAllSettings: vi.fn(async () => ({ ...kv })),
    __kv: kv,
  };
});

import { useSettingsStore, useEffectiveLearningSettings } from "../settingsStore";
import * as cmd from "../../lib/tauri-commands";

// 通过测试 mock 暴露的 kv 句柄（用于直接注入 Tauri KV 模拟数据）。
const kv = (cmd as unknown as { __kv: Record<string, string> }).__kv;

const INITIAL = useSettingsStore.getState().settings;

beforeEach(() => {
  // 清 mock KV + 重置 store 到默认值。
  for (const k of Object.keys(kv)) delete kv[k];
  useSettingsStore.setState({ settings: { ...INITIAL }, isLoading: false });
  vi.clearAllMocks();
});

describe("AppSettings 4 个学习功能字段（AC-1 默认值）", () => {
  it("DEFAULT_SETTINGS.showLearningFeatures === false", () => {
    expect(useSettingsStore.getState().settings.showLearningFeatures).toBe(false);
  });

  it("DEFAULT_SETTINGS.bindSchoolCalendar === false", () => {
    expect(useSettingsStore.getState().settings.bindSchoolCalendar).toBe(false);
  });

  it("DEFAULT_SETTINGS.enableDailyReviewReminder === false", () => {
    expect(useSettingsStore.getState().settings.enableDailyReviewReminder).toBe(false);
  });

  it("DEFAULT_SETTINGS.learningAutoEnableEvaluated === false", () => {
    expect(useSettingsStore.getState().settings.learningAutoEnableEvaluated).toBe(false);
  });
});

describe("持久化路径：loadSettings JSON.parse 还原 boolean（AC-2）", () => {
  it("loadSettings 把 KV 中 'true' / 'false' 字符串还原为 boolean", async () => {
    kv["showLearningFeatures"] = "true";
    kv["bindSchoolCalendar"] = "true";
    kv["enableDailyReviewReminder"] = "false";
    kv["learningAutoEnableEvaluated"] = "true";

    await useSettingsStore.getState().loadSettings();

    const s = useSettingsStore.getState().settings;
    expect(s.showLearningFeatures).toBe(true);
    expect(s.bindSchoolCalendar).toBe(true);
    expect(s.enableDailyReviewReminder).toBe(false);
    expect(s.learningAutoEnableEvaluated).toBe(true);
  });

  it("updateSetting 写入后 store state 同步（boolean → JSON.stringify 走 setSetting）", async () => {
    await useSettingsStore.getState().updateSetting("showLearningFeatures", true);
    expect(cmd.setSetting).toHaveBeenCalledWith("showLearningFeatures", "true");
    expect(useSettingsStore.getState().settings.showLearningFeatures).toBe(true);

    await useSettingsStore.getState().updateSetting("learningAutoEnableEvaluated", true);
    expect(useSettingsStore.getState().settings.learningAutoEnableEvaluated).toBe(true);
  });
});

describe("useEffectiveLearningSettings 派生（AC-3 真值表）", () => {
  it("show=true, bind=true, daily=true → effective 全 true", () => {
    useSettingsStore.setState({
      settings: {
        ...INITIAL,
        showLearningFeatures: true,
        bindSchoolCalendar: true,
        enableDailyReviewReminder: true,
      },
    });
    const { result } = renderHook(() => useEffectiveLearningSettings());
    expect(result.current).toEqual({
      showLearningFeatures: true,
      bindSchoolCalendar: true,
      enableDailyReviewReminder: true,
    });
  });

  it("show=false, bind=true, daily=true → effective 全 false（依赖字段读时强制 OFF）", () => {
    useSettingsStore.setState({
      settings: {
        ...INITIAL,
        showLearningFeatures: false,
        bindSchoolCalendar: true,
        enableDailyReviewReminder: true,
      },
    });
    const { result } = renderHook(() => useEffectiveLearningSettings());
    expect(result.current).toEqual({
      showLearningFeatures: false,
      bindSchoolCalendar: false,
      enableDailyReviewReminder: false,
    });
  });

  it("show=false, bind=false, daily=false → effective 全 false", () => {
    useSettingsStore.setState({
      settings: {
        ...INITIAL,
        showLearningFeatures: false,
        bindSchoolCalendar: false,
        enableDailyReviewReminder: false,
      },
    });
    const { result } = renderHook(() => useEffectiveLearningSettings());
    expect(result.current).toEqual({
      showLearningFeatures: false,
      bindSchoolCalendar: false,
      enableDailyReviewReminder: false,
    });
  });

  it("show=true, bind=false, daily=true → 透传真值", () => {
    useSettingsStore.setState({
      settings: {
        ...INITIAL,
        showLearningFeatures: true,
        bindSchoolCalendar: false,
        enableDailyReviewReminder: true,
      },
    });
    const { result } = renderHook(() => useEffectiveLearningSettings());
    expect(result.current).toEqual({
      showLearningFeatures: true,
      bindSchoolCalendar: false,
      enableDailyReviewReminder: true,
    });
  });
});

describe("真值不丢（AC-4，PRD 不可妥协底线 1）", () => {
  it("show=true,bind=true → updateSetting('showLearningFeatures', false) → bindSchoolCalendar 内存真值仍 = true", async () => {
    const store = useSettingsStore.getState();
    await store.updateSetting("showLearningFeatures", true);
    await store.updateSetting("bindSchoolCalendar", true);
    await store.updateSetting("enableDailyReviewReminder", true);

    // 关主开关
    await useSettingsStore.getState().updateSetting("showLearningFeatures", false);

    const s = useSettingsStore.getState().settings;
    // 关键不变量：依赖字段真值不被联动写为 false
    expect(s.showLearningFeatures).toBe(false);
    expect(s.bindSchoolCalendar).toBe(true);
    expect(s.enableDailyReviewReminder).toBe(true);

    // 而 effective 视图仍正确强制 OFF
    const { result } = renderHook(() => useEffectiveLearningSettings());
    expect(result.current).toEqual({
      showLearningFeatures: false,
      bindSchoolCalendar: false,
      enableDailyReviewReminder: false,
    });
  });

  it("OFF→ON→OFF 来回切换，依赖字段真值始终保持", async () => {
    const store = useSettingsStore.getState();
    await store.updateSetting("showLearningFeatures", true);
    await store.updateSetting("bindSchoolCalendar", true);

    await useSettingsStore.getState().updateSetting("showLearningFeatures", false);
    expect(useSettingsStore.getState().settings.bindSchoolCalendar).toBe(true);

    await useSettingsStore.getState().updateSetting("showLearningFeatures", true);
    expect(useSettingsStore.getState().settings.bindSchoolCalendar).toBe(true);

    await useSettingsStore.getState().updateSetting("showLearningFeatures", false);
    expect(useSettingsStore.getState().settings.bindSchoolCalendar).toBe(true);
  });

  it("主开关 OFF 时仍允许写依赖字段；真值保留，effective 仍 false（写入端不阻止）", async () => {
    // 验证 ADR-002：(b) 读取端派生方案 — 写入端绝不拦截依赖字段
    const store = useSettingsStore.getState();
    await store.updateSetting("showLearningFeatures", false);
    await store.updateSetting("bindSchoolCalendar", true);

    expect(useSettingsStore.getState().settings.bindSchoolCalendar).toBe(true);

    const { result } = renderHook(() => useEffectiveLearningSettings());
    expect(result.current.bindSchoolCalendar).toBe(false);
  });
});
