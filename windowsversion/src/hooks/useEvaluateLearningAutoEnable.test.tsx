/**
 * task_009 / E 段 / AC-11 — useEvaluateLearningAutoEnableOnce 升级智能 ON 兼容矩阵。
 *
 * 覆盖：
 *   - 有 calendar.events → showLearningFeatures 被写 true + learningAutoEnableEvaluated=true
 *   - 有 knowledge.concepts → 同上
 *   - 两者皆空 → showLearningFeatures 保持 false + learningAutoEnableEvaluated=true
 *   - 已 evaluated → 第二次启动直接 skip（不再 fetch / 不再写 show）
 *   - fail-open：library 失败 / fetch 失败也写 evaluated=true，不卡
 *   - enabled=false（dropzone 路径）→ 完全 no-op
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook } from "@testing-library/react";

vi.mock("../lib/tauri-commands", () => {
  const kv: Record<string, string> = {};
  return {
    setSetting: vi.fn(async (key: string, value: string) => {
      kv[key] = value;
    }),
    getAllSettings: vi.fn(async () => ({ ...kv })),
    __kv: kv,
  };
});

import { useEvaluateLearningAutoEnableOnce } from "./useEvaluateLearningAutoEnable";
import { useSettingsStore } from "../stores/settingsStore";
import { useCalendarStore } from "../stores/calendarStore";
import { useKnowledgeStore } from "../stores/knowledgeStore";
import { useLibraryStore } from "../stores/libraryStore";

const INITIAL_SETTINGS = useSettingsStore.getState().settings;
const INITIAL_CAL = useCalendarStore.getState();
const INITIAL_KN = useKnowledgeStore.getState();

async function flushMicrotasks() {
  // hook 内部链路：useEffect → runEvaluationOnce 内多个 await。
  // 至少需要排空若干轮 microtask + 一次 task。
  for (let i = 0; i < 10; i++) {
    await Promise.resolve();
  }
}

beforeEach(() => {
  useSettingsStore.setState({
    settings: { ...INITIAL_SETTINGS, learningAutoEnableEvaluated: false, showLearningFeatures: false },
    isLoading: false,
  });
  useCalendarStore.setState({ ...INITIAL_CAL, events: [] });
  useKnowledgeStore.setState({ ...INITIAL_KN, concepts: [] });
  vi.clearAllMocks();
});

describe("useEvaluateLearningAutoEnableOnce — 升级智能 ON 矩阵 (AC-11)", () => {
  it("E-1: calendar.events.length > 0 → showLearningFeatures=true + evaluated=true", async () => {
    vi.spyOn(useLibraryStore.getState(), "ensureActiveLibrary").mockResolvedValue("lib-1");
    vi.spyOn(useCalendarStore.getState(), "fetchEvents").mockImplementation(async () => {
      // 模拟 fetch 成功后 events 被填充
      useCalendarStore.setState({
        ...useCalendarStore.getState(),
        // @ts-expect-error 测试 fixture：插入最小 event 形状
        events: [{ id: "e1" }],
      });
    });
    vi.spyOn(useKnowledgeStore.getState(), "fetchConcepts").mockResolvedValue();

    renderHook(() => useEvaluateLearningAutoEnableOnce(true));
    await flushMicrotasks();

    const s = useSettingsStore.getState().settings;
    expect(s.showLearningFeatures).toBe(true);
    expect(s.learningAutoEnableEvaluated).toBe(true);
  });

  it("E-2: knowledge.concepts.length > 0 → showLearningFeatures=true + evaluated=true", async () => {
    vi.spyOn(useLibraryStore.getState(), "ensureActiveLibrary").mockResolvedValue("lib-1");
    vi.spyOn(useCalendarStore.getState(), "fetchEvents").mockResolvedValue();
    vi.spyOn(useKnowledgeStore.getState(), "fetchConcepts").mockImplementation(async () => {
      useKnowledgeStore.setState({
        ...useKnowledgeStore.getState(),
        // @ts-expect-error 测试 fixture
        concepts: [{ id: "c1" }],
      });
    });

    renderHook(() => useEvaluateLearningAutoEnableOnce(true));
    await flushMicrotasks();

    const s = useSettingsStore.getState().settings;
    expect(s.showLearningFeatures).toBe(true);
    expect(s.learningAutoEnableEvaluated).toBe(true);
  });

  it("E-3: 两者都为空 → showLearningFeatures 保持 false，evaluated 写 true（不再重试）", async () => {
    vi.spyOn(useLibraryStore.getState(), "ensureActiveLibrary").mockResolvedValue("lib-1");
    vi.spyOn(useCalendarStore.getState(), "fetchEvents").mockResolvedValue();
    vi.spyOn(useKnowledgeStore.getState(), "fetchConcepts").mockResolvedValue();

    renderHook(() => useEvaluateLearningAutoEnableOnce(true));
    await flushMicrotasks();

    const s = useSettingsStore.getState().settings;
    expect(s.showLearningFeatures).toBe(false);
    expect(s.learningAutoEnableEvaluated).toBe(true);
  });

  it("E-4: 已 evaluated → 第二次启动直接 skip，不写 show，不调 fetch", async () => {
    useSettingsStore.setState({
      settings: { ...INITIAL_SETTINGS, learningAutoEnableEvaluated: true, showLearningFeatures: false },
      isLoading: false,
    });
    const fetchEvSpy = vi.spyOn(useCalendarStore.getState(), "fetchEvents").mockResolvedValue();
    const fetchKnSpy = vi.spyOn(useKnowledgeStore.getState(), "fetchConcepts").mockResolvedValue();
    const ensureSpy = vi.spyOn(useLibraryStore.getState(), "ensureActiveLibrary").mockResolvedValue("lib-1");

    renderHook(() => useEvaluateLearningAutoEnableOnce(true));
    await flushMicrotasks();

    // 已 evaluated：早退，不应该走到 ensureActiveLibrary / fetch。
    expect(ensureSpy).not.toHaveBeenCalled();
    expect(fetchEvSpy).not.toHaveBeenCalled();
    expect(fetchKnSpy).not.toHaveBeenCalled();
    // showLearningFeatures 保持 false（绝不覆盖用户后续主动关掉的决定）。
    expect(useSettingsStore.getState().settings.showLearningFeatures).toBe(false);
  });

  it("E-5: ensureActiveLibrary 抛错 → fail-open，evaluated=true，show=false", async () => {
    vi.spyOn(useLibraryStore.getState(), "ensureActiveLibrary").mockRejectedValue(new Error("no library"));

    renderHook(() => useEvaluateLearningAutoEnableOnce(true));
    await flushMicrotasks();

    const s = useSettingsStore.getState().settings;
    expect(s.learningAutoEnableEvaluated).toBe(true);
    expect(s.showLearningFeatures).toBe(false);
  });

  it("E-6: fetch 失败也写 evaluated=true，不卡（Promise.allSettled fail-open）", async () => {
    vi.spyOn(useLibraryStore.getState(), "ensureActiveLibrary").mockResolvedValue("lib-1");
    vi.spyOn(useCalendarStore.getState(), "fetchEvents").mockRejectedValue(new Error("fetch ev failed"));
    vi.spyOn(useKnowledgeStore.getState(), "fetchConcepts").mockRejectedValue(new Error("fetch kn failed"));

    renderHook(() => useEvaluateLearningAutoEnableOnce(true));
    await flushMicrotasks();

    const s = useSettingsStore.getState().settings;
    expect(s.learningAutoEnableEvaluated).toBe(true);
    expect(s.showLearningFeatures).toBe(false);
  });

  it("E-7: enabled=false（dropzone 路径）→ 完全 no-op，不写 evaluated", async () => {
    const ensureSpy = vi.spyOn(useLibraryStore.getState(), "ensureActiveLibrary").mockResolvedValue("lib-1");

    renderHook(() => useEvaluateLearningAutoEnableOnce(false));
    await flushMicrotasks();

    expect(ensureSpy).not.toHaveBeenCalled();
    // evaluated 保持 false（未触发）
    expect(useSettingsStore.getState().settings.learningAutoEnableEvaluated).toBe(false);
  });
});
