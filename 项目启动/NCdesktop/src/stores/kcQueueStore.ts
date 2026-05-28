/**
 * task_025：KC 队列状态 store（前端 toast 数据源，单一职责）。
 *
 * ## 设计依据
 *
 * - **task_025 input.md AC-1**：追加状态 `kcQueueLength` + `kcCurrentAssetId`，订阅
 *   `notecapt/asset-kc-queued` / `notecapt/asset-kc-enriched` 维护队列长度。
 * - **AC-2 / AC-3**：DropzoneApp 据 store 派生 toast：
 *   - `kcQueueLength > 0` → "AI 增强中 N..." toast；
 *   - `kcQueueLength === 0` 且最近 5s 内有 enriched 事件 → "AI 增强完成" 短 toast，3s 后消失。
 * - **AC-4**：纯 Zustand + Tailwind，不引入 react-toastify 等新库。
 * - **AC-5（边界）**：事件去重 + ID 集合维护（同一 asset 不重复入队 / 不重复出队），
 *   防 emit 路径上的 retry / 闪烁。
 *
 * ## 不变量
 *
 * 1. `kcQueueLength === pendingAssetIds.size` —— length 必为非负、与 set 一致；
 * 2. 同一 `assetId` 重复入队幂等（set 天然去重）；
 * 3. 出队时若不在 set 内（如先收到 enriched 后才收到 queued、或纯 fallback 路径只发 enriched），
 *    走"防御性吞掉"语义（length 不减为负，lastCompletedAt 仍更新）；
 * 4. `lastCompletedAt` 仅在出队成功时更新（即收到 enriched 且 set 中存在该 id），
 *    避免"未在队列的 enriched"刷新完成时间戳（影响 toast "完成" 闪烁判定）。
 *
 * ## 与 dropzoneStore 的边界
 *
 * `dropzoneStore` 负责"拖拽 → 入库"阶段的 UI 状态（idle / attract / processing / complete）。
 * KC enrichment 在入库**之后**异步发生，时间窗与拖拽 phase 解耦——例如用户拖了 3 个文件后
 * dropzone 已 complete → idle，但 KC 增强可能还在跑 1-2 分钟。所以将 KC 队列状态拆到独立
 * store，DropzoneApp 各自订阅、组合展示。
 */
import { create } from "zustand";

/** AC-2 事件订阅消费的最小 payload 形状（后端 `build_kc_queued_payload` 严格对齐）。 */
export interface KcQueuedPayload {
  assetId: string;
}

/** AC-2 / task_011 `emit_kc_enriched` payload schema（仅本 store 关心 assetId）。 */
export interface KcEnrichedPayload {
  assetId: string;
  kcEnriched: "true" | "partial" | "false" | string;
  failureCode: string | null;
}

export interface KcQueueStore {
  /** AC-1：当前未完成 KC enrichment 的 asset 数量（=== `pendingAssetIds.size`）。 */
  kcQueueLength: number;
  /** AC-1：当前"代表性"在跑 asset id（取最近入队的，仅供 toast 文案显示 / 调试）。 */
  kcCurrentAssetId: string | null;
  /** 内部集合：用于去重 + 出队时识别"是否真的出队"。 */
  pendingAssetIds: Set<string>;
  /** AC-3：最近一次"非空 → 空"切换的时间戳（ms epoch）。null = 从未完成过。 */
  lastCompletedAt: number | null;

  /** AC-2：收到 `notecapt/asset-kc-queued` → 入队（幂等去重）。 */
  enqueue: (assetId: string) => void;
  /** AC-2：收到 `notecapt/asset-kc-enriched` → 出队 + 维护 lastCompletedAt。 */
  dequeue: (assetId: string) => void;
  /** 复位（测试 / dropzone 关闭场景调用）。 */
  reset: () => void;
}

export const useKcQueueStore = create<KcQueueStore>((set, get) => ({
  kcQueueLength: 0,
  kcCurrentAssetId: null,
  pendingAssetIds: new Set<string>(),
  lastCompletedAt: null,

  enqueue: (assetId) => {
    if (assetId.length === 0) {
      // 防御性：空 id 不入队（后端不可能发，但 IPC 反序列化失败时可能空字符串）
      return;
    }
    const current = get().pendingAssetIds;
    if (current.has(assetId)) {
      // 不变量 2：同 id 重复入队幂等
      return;
    }
    // 新 Set 实例触发 React re-render（不可变更新）
    const next = new Set(current);
    next.add(assetId);
    set({
      pendingAssetIds: next,
      kcQueueLength: next.size,
      // 最近入队的优先（toast 显示"当前在处理的 asset"语义）
      kcCurrentAssetId: assetId,
    });
  },

  dequeue: (assetId) => {
    if (assetId.length === 0) return;
    const current = get().pendingAssetIds;
    if (!current.has(assetId)) {
      // 不变量 3：未入队的 enriched 防御性吞掉，length 不变为负
      return;
    }
    const next = new Set(current);
    next.delete(assetId);
    const becameEmpty = next.size === 0;
    // 当 current 是最近 dequeue 的 id 时清空（不再回退到任意残留 id，避免 toast 闪跳）
    const newCurrent = becameEmpty
      ? null
      : get().kcCurrentAssetId === assetId
        ? // 在剩余 set 里挑任一作为"current"，保证 toast 文案不出现"null"
          (next.values().next().value ?? null)
        : get().kcCurrentAssetId;
    set({
      pendingAssetIds: next,
      kcQueueLength: next.size,
      kcCurrentAssetId: newCurrent,
      // 不变量 4：仅在真出队时更新 lastCompletedAt
      lastCompletedAt: becameEmpty ? Date.now() : get().lastCompletedAt,
    });
  },

  reset: () =>
    set({
      kcQueueLength: 0,
      kcCurrentAssetId: null,
      pendingAssetIds: new Set<string>(),
      lastCompletedAt: null,
    }),
}));
