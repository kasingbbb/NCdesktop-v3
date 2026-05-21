/**
 * useHubHashRoute — KnowledgeHubView 的 hash route hook（ADR-004）
 *
 * 职责：
 *   - 解析 `#/knowledge-hub/:step` → 当前 step
 *   - `pushState` 切 step；`popstate` 监听器同步回组件 state（前进/后退可用，PRD AC-13）
 *   - 旧 hash `#/skills` `#/knowledge` 等在初始化时 `replaceState` 重定向（PRD AC-12）
 *   - 不引入新路由库：只用 window.location.hash + history API
 *
 * 注意：
 *   - 重定向使用 `replaceState`，不污染 history（避免后退又被重定向 → 死循环）
 *   - cleanup 必须移除 popstate 监听
 */

import { useCallback, useEffect, useState } from "react";
import { DEFAULT_HUB_STEP, HUB_STEPS, isHubStep, type HubStep } from "./types";

const HUB_HASH_PREFIX = "#/knowledge-hub";

/**
 * 旧 hash → 目标 hash 的迁移矩阵。返回 null 表示不需要迁移。
 *
 * 同时返回 `applySidebarSection` 标记 —— 调用方在迁移时调用 `setSidebarSection('knowledge-hub')`，
 * 保证 store 的 activeSidebarSection 与 URL 一致。
 */
export interface LegacyHashMigration {
  nextHash: string;
  applySidebarSection: boolean;
  warnReason?: string;
}

export function migrateLegacyHash(currentHash: string): LegacyHashMigration | null {
  if (currentHash === "#/skills") {
    return { nextHash: `${HUB_HASH_PREFIX}/skills`, applySidebarSection: true };
  }
  if (currentHash === "#/knowledge") {
    return { nextHash: `${HUB_HASH_PREFIX}/library`, applySidebarSection: true };
  }
  if (currentHash === HUB_HASH_PREFIX || currentHash === `${HUB_HASH_PREFIX}/`) {
    return {
      nextHash: `${HUB_HASH_PREFIX}/${DEFAULT_HUB_STEP}`,
      applySidebarSection: false,
    };
  }
  if (currentHash.startsWith(`${HUB_HASH_PREFIX}/`)) {
    const rawStep = currentHash.slice(`${HUB_HASH_PREFIX}/`.length).split(/[?#]/)[0];
    if (!isHubStep(rawStep)) {
      return {
        nextHash: `${HUB_HASH_PREFIX}/${DEFAULT_HUB_STEP}`,
        applySidebarSection: false,
        warnReason: `unknown hub step '${rawStep}' → '${DEFAULT_HUB_STEP}'`,
      };
    }
    return null;
  }
  // 非 hub 相关 hash：不动（用户可能在其他视图）
  return null;
}

/**
 * 从 hash 解析当前 step。无法识别一律降级到默认 step（AC-3）。
 */
export function parseHubStep(currentHash: string): HubStep {
  if (!currentHash.startsWith(`${HUB_HASH_PREFIX}/`)) {
    return DEFAULT_HUB_STEP;
  }
  const rawStep = currentHash.slice(`${HUB_HASH_PREFIX}/`.length).split(/[?#]/)[0];
  return isHubStep(rawStep) ? rawStep : DEFAULT_HUB_STEP;
}

function devWarn(msg: string): void {
  if (import.meta.env.DEV) {
    console.warn(`[useHubHashRoute] ${msg}`);
  }
}

export interface UseHubHashRouteOptions {
  /** 用于在迁移旧 hash 时同步 store；可选，便于测试 */
  onLegacyMigrated?: () => void;
}

export interface UseHubHashRouteResult {
  step: HubStep;
  setStep: (next: HubStep) => void;
  steps: readonly HubStep[];
}

export function useHubHashRoute(options: UseHubHashRouteOptions = {}): UseHubHashRouteResult {
  const { onLegacyMigrated } = options;

  const [step, setStepState] = useState<HubStep>(() => {
    if (typeof window === "undefined") return DEFAULT_HUB_STEP;
    return parseHubStep(window.location.hash);
  });

  // 初始化：旧 hash 重定向（replaceState；不进 history）
  useEffect(() => {
    if (typeof window === "undefined") return;
    const migration = migrateLegacyHash(window.location.hash);
    if (migration) {
      if (migration.warnReason) devWarn(migration.warnReason);
      window.history.replaceState(null, "", migration.nextHash);
      if (migration.applySidebarSection) onLegacyMigrated?.();
      setStepState(parseHubStep(migration.nextHash));
    }
    // 仅在挂载时跑一次；onLegacyMigrated 故意不进依赖避免重入
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // popstate / hashchange 双向同步（AC-4）
  useEffect(() => {
    if (typeof window === "undefined") return;
    const sync = () => {
      const nextStep = parseHubStep(window.location.hash);
      setStepState((prev) => (prev === nextStep ? prev : nextStep));
    };
    window.addEventListener("popstate", sync);
    window.addEventListener("hashchange", sync);
    return () => {
      window.removeEventListener("popstate", sync);
      window.removeEventListener("hashchange", sync);
    };
  }, []);

  const setStep = useCallback((next: HubStep) => {
    if (!isHubStep(next)) {
      devWarn(`setStep: invalid step '${String(next)}' ignored`);
      return;
    }
    const target = `${HUB_HASH_PREFIX}/${next}`;
    if (typeof window !== "undefined" && window.location.hash !== target) {
      window.history.pushState(null, "", target);
    }
    setStepState(next);
  }, []);

  return { step, setStep, steps: HUB_STEPS };
}
