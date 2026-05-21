import { useEffect, useRef } from "react";
import { useSettingsStore } from "../stores/settingsStore";
import { useCalendarStore } from "../stores/calendarStore";
import { useKnowledgeStore } from "../stores/knowledgeStore";
import { useLibraryStore } from "../stores/libraryStore";

/**
 * v2 Sidebar Redesign — 升级智能开启学习模式（ADR-003 / PRD F-P0-4 / AC-11）。
 *
 * 时序与不变量：
 *   1. 读 `learningAutoEnableEvaluated`，若已为 true → 直接退出（保证只评估一次，绝不
 *      覆盖用户后续主动关掉学习模式的决定 — PRD §7.S4）。
 *   2. 等 `libraryStore.ensureActiveLibrary()` resolve，再请求 calendarStore.fetchEvents +
 *      knowledgeStore.fetchConcepts；fetch 失败 fail-open（按"未检测到学习数据"处理）。
 *   3. 任一信号为真（events.length > 0 或 concepts.length > 0）→
 *      `updateSetting('showLearningFeatures', true)`。
 *   4. 无论结果 → `updateSetting('learningAutoEnableEvaluated', true)`。
 *
 * 不可妥协的安全底线：
 *   - **严禁**任何"未检测到 → 关闭"分支：本 hook 只能让 `showLearningFeatures` 由 false → true。
 *
 * 类型说明：
 *   `learningAutoEnableEvaluated` / `showLearningFeatures` 字段已由 task_003 在 `AppSettings`
 *   中正式定义，类型自然对齐，无需任何过渡断言。
 */
/**
 * @param enabled 默认 true。dropzone 窗口需传 false（AC-6：dropzone 路径不评估）。
 *                参数变化不会重新触发 evaluation；hook 内部用 ref 保证整次挂载只评估一次。
 */
export function useEvaluateLearningAutoEnableOnce(enabled: boolean = true): void {
  // 用 ref 兜底防止 React 18+ StrictMode 双调用 / 父组件重渲染再次触发评估。
  // settingsStore 的 `learningAutoEnableEvaluated` 是跨重启权威标记，但它落盘存在
  // 异步窗口，单挂载内额外加 ref 是更紧的护栏。
  const startedRef = useRef(false);

  useEffect(() => {
    if (!enabled) return;
    if (startedRef.current) return;
    startedRef.current = true;

    void runEvaluationOnce();
  }, [enabled]);
}

async function runEvaluationOnce(): Promise<void> {
  const settingsStore = useSettingsStore.getState();
  const settings = settingsStore.settings;

  // 一次性标记：true → 不再评估。fail-safe：未定义视为 false（首次启动）。
  if (settings.learningAutoEnableEvaluated === true) return;

  // 等 active library 就绪（calendar/knowledge fetch 都需要 libraryId）。
  let libraryId: string | null = null;
  try {
    libraryId = await useLibraryStore.getState().ensureActiveLibrary();
  } catch {
    // library 取不到 → 没法 fetch；按"未检测到"走，仍要写一次性标记避免每次启动重试。
  }

  let hasLearningSignal = false;

  if (libraryId) {
    // 两个 fetch 容错并行；任一失败不阻塞另一个；都失败按"未检测到"。
    const [eventsOk, conceptsOk] = await Promise.allSettled([
      useCalendarStore.getState().fetchEvents(libraryId),
      useKnowledgeStore.getState().fetchConcepts(libraryId),
    ]);

    if (eventsOk.status === "fulfilled") {
      const events = useCalendarStore.getState().events;
      if (events.length > 0) hasLearningSignal = true;
    }
    if (conceptsOk.status === "fulfilled") {
      const concepts = useKnowledgeStore.getState().concepts;
      if (concepts.length > 0) hasLearningSignal = true;
    }
  }

  // 写顺序：先 showLearningFeatures（如需），再写一次性标记。
  // 顺序不影响正确性（标记位幂等，主开关也幂等），但便于在调试时观察"先开后封"。
  try {
    if (hasLearningSignal && settings.showLearningFeatures !== true) {
      await settingsStore.updateSetting("showLearningFeatures", true);
    }
    await settingsStore.updateSetting("learningAutoEnableEvaluated", true);
  } catch {
    // settingsStore 写入失败（Tauri 不可达等）→ 不再重试；下次启动会再尝试一次。
    // 这里静默是因为 hook 是"机会式"路径，失败不应影响主 UI 渲染。
  }
}
