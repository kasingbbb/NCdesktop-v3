/**
 * task_025_dev：kcQueueStore 单元测试。
 *
 * 覆盖（与 store 文件 ## 不变量 严格对齐）：
 *   - AC-1：初始 state（length=0 / currentAssetId=null / pendingAssetIds 空 / lastCompletedAt=null）
 *   - AC-2 入队：enqueue 累加 + 同 id 幂等
 *   - AC-2 出队：dequeue 减计 + 未入队的 dequeue 防御性吞掉
 *   - AC-3 lastCompletedAt：非空 → 空切换时更新；非清零时不动
 *   - reset：还原所有字段
 *   - 不可变更新：pendingAssetIds 是新 Set 实例（防 zustand selector 选择不到变更）
 */
import { beforeEach, describe, expect, it } from "vitest";
import { useKcQueueStore } from "../kcQueueStore";

const INITIAL = useKcQueueStore.getState();

beforeEach(() => {
  // 每个 case 重新构造干净的 state（reset 比 setState 更贴近用户路径）
  useKcQueueStore.getState().reset();
});

describe("kcQueueStore AC-1：初始 state", () => {
  it("length=0 / current=null / pendingAssetIds.size=0 / lastCompletedAt=null", () => {
    const s = useKcQueueStore.getState();
    expect(s.kcQueueLength).toBe(0);
    expect(s.kcCurrentAssetId).toBeNull();
    expect(s.pendingAssetIds.size).toBe(0);
    expect(s.lastCompletedAt).toBeNull();
  });

  it("INITIAL 与 reset 后的 state 字段对齐（防 selector 漂移）", () => {
    const after = useKcQueueStore.getState();
    expect(after.kcQueueLength).toBe(INITIAL.kcQueueLength);
    expect(after.kcCurrentAssetId).toBe(INITIAL.kcCurrentAssetId);
    expect(after.lastCompletedAt).toBe(INITIAL.lastCompletedAt);
  });
});

describe("kcQueueStore AC-2：enqueue 入队", () => {
  it("单个入队后 length=1 且 currentAssetId=入队 id", () => {
    useKcQueueStore.getState().enqueue("asset-a");
    const s = useKcQueueStore.getState();
    expect(s.kcQueueLength).toBe(1);
    expect(s.kcCurrentAssetId).toBe("asset-a");
    expect(s.pendingAssetIds.has("asset-a")).toBe(true);
  });

  it("多次入队累加，currentAssetId 跟随最近一次", () => {
    const s = useKcQueueStore.getState();
    s.enqueue("a-1");
    s.enqueue("a-2");
    s.enqueue("a-3");
    const after = useKcQueueStore.getState();
    expect(after.kcQueueLength).toBe(3);
    expect(after.kcCurrentAssetId).toBe("a-3");
    expect(after.pendingAssetIds.size).toBe(3);
  });

  it("同 id 重复入队幂等（不变量 2）", () => {
    const s = useKcQueueStore.getState();
    s.enqueue("dup");
    s.enqueue("dup");
    s.enqueue("dup");
    expect(useKcQueueStore.getState().kcQueueLength).toBe(1);
    expect(useKcQueueStore.getState().pendingAssetIds.size).toBe(1);
  });

  it("空 assetId 防御性忽略（IPC 反序列化失败兜底）", () => {
    useKcQueueStore.getState().enqueue("");
    const s = useKcQueueStore.getState();
    expect(s.kcQueueLength).toBe(0);
    expect(s.kcCurrentAssetId).toBeNull();
  });

  it("不可变更新：每次 enqueue 产出新 Set 实例（zustand 引用比较生效）", () => {
    const beforeSet = useKcQueueStore.getState().pendingAssetIds;
    useKcQueueStore.getState().enqueue("x");
    const afterSet = useKcQueueStore.getState().pendingAssetIds;
    expect(afterSet).not.toBe(beforeSet);
  });
});

describe("kcQueueStore AC-2：dequeue 出队", () => {
  it("出队已入队的 id → length -1", () => {
    const s = useKcQueueStore.getState();
    s.enqueue("a-1");
    s.enqueue("a-2");
    s.dequeue("a-1");
    const after = useKcQueueStore.getState();
    expect(after.kcQueueLength).toBe(1);
    expect(after.pendingAssetIds.has("a-1")).toBe(false);
    expect(after.pendingAssetIds.has("a-2")).toBe(true);
  });

  it("出队未入队的 id 防御性吞掉，length 不变为负（不变量 3）", () => {
    useKcQueueStore.getState().dequeue("never-enqueued");
    const s = useKcQueueStore.getState();
    expect(s.kcQueueLength).toBe(0);
    expect(s.kcCurrentAssetId).toBeNull();
  });

  it("空 assetId 防御性忽略", () => {
    useKcQueueStore.getState().enqueue("real");
    useKcQueueStore.getState().dequeue("");
    expect(useKcQueueStore.getState().kcQueueLength).toBe(1);
  });

  it("当 currentAssetId 被出队时切换到剩余 set 的任一 id（避免 toast null 闪烁）", () => {
    const s = useKcQueueStore.getState();
    s.enqueue("a-1");
    s.enqueue("a-2");
    // current = a-2（最近入队）
    expect(useKcQueueStore.getState().kcCurrentAssetId).toBe("a-2");
    s.dequeue("a-2");
    // current 应该切到 a-1 而不是 null（队列非空时不应显示 null）
    expect(useKcQueueStore.getState().kcCurrentAssetId).toBe("a-1");
    expect(useKcQueueStore.getState().kcQueueLength).toBe(1);
  });

  it("出队非 current 的 id → current 不变", () => {
    const s = useKcQueueStore.getState();
    s.enqueue("a-1");
    s.enqueue("a-2");
    s.dequeue("a-1");
    expect(useKcQueueStore.getState().kcCurrentAssetId).toBe("a-2");
  });
});

describe("kcQueueStore AC-3：lastCompletedAt 状态机", () => {
  it("非空 → 空切换时更新 lastCompletedAt（不变量 4）", () => {
    const s = useKcQueueStore.getState();
    expect(useKcQueueStore.getState().lastCompletedAt).toBeNull();
    s.enqueue("a-1");
    expect(useKcQueueStore.getState().lastCompletedAt).toBeNull();
    const before = Date.now();
    s.dequeue("a-1");
    const ts = useKcQueueStore.getState().lastCompletedAt;
    expect(ts).not.toBeNull();
    expect(ts!).toBeGreaterThanOrEqual(before);
  });

  it("队列从 2 → 1（仍非空）不更新 lastCompletedAt", () => {
    const s = useKcQueueStore.getState();
    s.enqueue("a-1");
    s.enqueue("a-2");
    s.dequeue("a-1");
    expect(useKcQueueStore.getState().lastCompletedAt).toBeNull();
  });

  it("已为 0 的 dequeue 不更新 lastCompletedAt（防御性吞掉路径）", () => {
    useKcQueueStore.getState().dequeue("phantom");
    expect(useKcQueueStore.getState().lastCompletedAt).toBeNull();
  });
});

describe("kcQueueStore reset", () => {
  it("reset 清零所有字段", () => {
    const s = useKcQueueStore.getState();
    s.enqueue("a-1");
    s.enqueue("a-2");
    s.dequeue("a-1"); // 设了 lastCompletedAt 之前还得让队列清空一次
    s.dequeue("a-2");
    expect(useKcQueueStore.getState().lastCompletedAt).not.toBeNull();
    s.reset();
    const after = useKcQueueStore.getState();
    expect(after.kcQueueLength).toBe(0);
    expect(after.kcCurrentAssetId).toBeNull();
    expect(after.pendingAssetIds.size).toBe(0);
    expect(after.lastCompletedAt).toBeNull();
  });
});
