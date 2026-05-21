/**
 * 用户自定义 Prompt 功能 — 前端 zustand store
 *
 * 真相来源：custom_prompt_v1 / task_001_architect / output.md § 6.3 + ADR-005
 * 与 task_005 落地的契约层（types/user-prompt.ts + lib/tauri-commands.ts）配套。
 *
 * 命名隔离（ADR-005 / R6）：
 *   - 名称固定 `useUserPromptStore`，与 PR-4 半成品 `stores/promptStore.ts` 中
 *     `usePromptStore`（kind = classify/naming/tagging）字面与语义独立、不复用。
 *   - 不修改 `promptStore.ts`。
 *
 * 设计要点：
 *   1. items / drafts / dirty 三表均以 PromptModule 为主键，初始空骨架（避免 UI 渲染时
 *      undefined 访问）。
 *   2. dirty 的口径（input.md AC-3）：`draft !== (item?.userText ?? item?.defaultText ?? "")`，
 *      即"与当前生效文本对比"，不是"与初次加载快照对比"。loadAll/save/reset 后 dirty 应自动归零。
 *   3. save 错误：不修改 drafts / dirty（让用户原地重试），仅写 error 并抛出。
 *   4. byteLen 与后端 Rust `text.len()`（UTF-8 字节）口径一致：`TextEncoder().encode(text).length`。
 *      这是 ADR-004 字节校验的前端镜像，UI 字节计数提示由 task_007 实现。
 */

import { create } from "zustand";
import * as cmd from "../lib/tauri-commands";
import {
  PROMPT_MODULES,
  type PromptInfo,
  type PromptModule,
} from "../types/user-prompt";

/** 4 个 module 全 null 骨架（用于 items 初值与重置）。 */
function emptyItems(): Record<PromptModule, PromptInfo | null> {
  return {
    tagging: null,
    para: null,
    concept: null,
    aggregation: null,
  };
}

/** 4 个 module 全 "" 骨架（用于 drafts 初值）。 */
function emptyDrafts(): Record<PromptModule, string> {
  return {
    tagging: "",
    para: "",
    concept: "",
    aggregation: "",
  };
}

/** 4 个 module 全 false 骨架（用于 dirty 初值）。 */
function emptyDirty(): Record<PromptModule, boolean> {
  return {
    tagging: false,
    para: false,
    concept: false,
    aggregation: false,
  };
}

/**
 * 计算 module 的"当前生效文本"：
 *   - 用户已自定义 ⇒ userText
 *   - 否则 ⇒ defaultText
 *   - item 为 null（未加载） ⇒ "" 兜底，避免比较 NaN
 *
 * 用于 setDraft 的 dirty 判定（AC-3）。
 */
function effectiveText(item: PromptInfo | null): string {
  if (!item) return "";
  return item.userText ?? item.defaultText ?? "";
}

interface UserPromptStore {
  /** 4 个 module 的最新服务端快照（List/Get 返回）。null = 尚未加载或加载失败。 */
  items: Record<PromptModule, PromptInfo | null>;
  /** 编辑中的草稿；初次加载后 = userText ?? defaultText（用户首次打开看到当前生效内容，AC-2）。 */
  drafts: Record<PromptModule, string>;
  /** 草稿与 server 当前生效文本是否不一致；setDraft / load / save / reset 时重算。 */
  dirty: Record<PromptModule, boolean>;
  /** 整体加载态（loadAll 进行中）。单条 save/reset 不影响此字段；如 UI 需要单条 saving 态，
   *  下游 task_007 在组件内自管 useState（input.md 技术约束："不在 store 中做 UI 状态"）。 */
  loading: boolean;
  /** 最近一次 IPC 错误：归属到具体 module（save/reset(module) 失败）或 null（全局：loadAll/reset(null) 失败）；
   *  message 原样透传后端中文错误消息。task_007_round2 由"单值字符串"升级为带归属对象，避免多个展开子项重复显示同一条错误。 */
  error: { module: PromptModule | null; message: string } | null;

  /** 加载全部 4 条；填充 items 与 drafts。 */
  loadAll: () => Promise<void>;
  /** 仅本地：更新单条草稿 + 重算 dirty；不发 IPC。 */
  setDraft: (module: PromptModule, text: string) => void;
  /** 保存单条；成功后 getUserPrompt 刷新 items[module] 并 dirty[module] = false。
   *  失败时不修改本地状态（让用户原地重试），只写 error 并抛出。 */
  save: (module: PromptModule) => Promise<void>;
  /** 恢复默认：null = 全部 4 条（之后 loadAll）；非 null = 单条（之后 getUserPrompt + drafts 同步 defaultText）。 */
  reset: (module: PromptModule | null) => Promise<void>;
  /** UTF-8 字节数（与后端 ADR-004 字节校验口径一致）。 */
  byteLen: (module: PromptModule) => number;
}

export const useUserPromptStore = create<UserPromptStore>((set, get) => ({
  items: emptyItems(),
  drafts: emptyDrafts(),
  dirty: emptyDirty(),
  loading: false,
  error: null,

  loadAll: async () => {
    set({ loading: true, error: null });
    try {
      const list = await cmd.listUserPrompts();

      // 重建 items / drafts / dirty 三表，未在返回中的 module 留 null/""/false。
      const items = emptyItems();
      const drafts = emptyDrafts();
      const dirty = emptyDirty();

      for (const info of list) {
        items[info.module] = info;
        // AC-2：首次打开看到"当前生效内容" = userText ?? defaultText
        drafts[info.module] = info.userText ?? info.defaultText;
        // 加载后必然 dirty=false（drafts 与 effectiveText 相等）
        dirty[info.module] = false;
      }

      set({ items, drafts, dirty, loading: false });
    } catch (e) {
      // 全局错误（loadAll）：module=null，UI 顶部展示
      set({ error: { module: null, message: String(e) }, loading: false });
    }
  },

  setDraft: (module, text) => {
    set((s) => {
      const reference = effectiveText(s.items[module]);
      return {
        drafts: { ...s.drafts, [module]: text },
        dirty: { ...s.dirty, [module]: text !== reference },
      };
    });
  },

  save: async (module) => {
    const draft = get().drafts[module];
    try {
      await cmd.saveUserPrompt(module, draft);
      // 刷新该 module 的 PromptInfo（含新的 userText / isCustom / updatedAt）
      const fresh = await cmd.getUserPrompt(module);
      set((s) => ({
        items: { ...s.items, [module]: fresh },
        // 保存成功后，草稿与 server 一致 → dirty 归零（drafts 维持用户当前输入）
        dirty: { ...s.dirty, [module]: false },
        error: null,
      }));
    } catch (e) {
      // 不动 drafts / dirty / items（让用户原地修改后重试）
      // 归属到失败的 module，UI 仅在该子项下方渲染（task_007_round2 去重）
      set({ error: { module, message: String(e) } });
      throw e;
    }
  },

  reset: async (module) => {
    try {
      await cmd.resetUserPrompt(module);
      if (module === null) {
        // 全部 4 条恢复默认：直接重载（loadAll 会同步刷新 items/drafts/dirty）
        await get().loadAll();
      } else {
        // 单条：拉新数据 + 把 drafts 同步为新的 defaultText（reset 后 userText = null）
        const fresh = await cmd.getUserPrompt(module);
        set((s) => ({
          items: { ...s.items, [module]: fresh },
          drafts: { ...s.drafts, [module]: fresh.defaultText },
          dirty: { ...s.dirty, [module]: false },
          error: null,
        }));
      }
    } catch (e) {
      // module=null（全部恢复）→ 全局错误；module 非 null → 单条错误归属该 module
      set({ error: { module, message: String(e) } });
      throw e;
    }
  },

  byteLen: (module) => {
    // UTF-8 字节数，与后端 Rust 的 `text.len()` 字节口径一致。
    return new TextEncoder().encode(get().drafts[module]).length;
  },
}));

// 模块初始化期断言：PROMPT_MODULES 与 items/drafts/dirty 骨架键一一对应。
// 若 task_005 在不通知本 store 的前提下扩展 PromptModule 联合，会在 dev 期 import 时立刻报。
// （生产 build 通常会 tree-shake 掉，但开发期早暴露错位胜过运行期定位。）
if (import.meta.env?.DEV) {
  const skeleton = Object.keys(emptyItems()).sort();
  const declared = [...PROMPT_MODULES].sort();
  if (skeleton.join(",") !== declared.join(",")) {
    // eslint-disable-next-line no-console
    console.warn(
      "[userPromptStore] PROMPT_MODULES 与 store 骨架不一致：",
      { skeleton, declared },
    );
  }
}
