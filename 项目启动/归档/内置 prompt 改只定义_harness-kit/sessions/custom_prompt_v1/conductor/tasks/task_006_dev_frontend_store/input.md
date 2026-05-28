# Task 输入 — task_006_dev_frontend_store

## 目标

新建 `src/stores/userPromptStore.ts`（zustand store），负责 4 个 module 的加载、草稿管理、保存、恢复默认、字节计数派生，对应 UI 与 Tauri 后端的桥接。

## 前置条件

- 依赖 task：`task_005_dev_frontend_contract` 必须 DONE（提供 types 与 invoke 封装）
- 必须先存在的文件/接口：
  - `src/types/user-prompt.ts`
  - `src/lib/tauri-commands.ts` 中的 `listUserPrompts / getUserPrompt / saveUserPrompt / resetUserPrompt`

## 验收标准（Acceptance Criteria）

1. **AC-1（store 形状）** — 新建 `src/stores/userPromptStore.ts`，按 Architect output.md § 6.3：

   ```typescript
   import { create } from "zustand";
   import type { PromptInfo, PromptModule } from "../types/user-prompt";
   import { PROMPT_MODULES } from "../types/user-prompt";
   import * as cmd from "../lib/tauri-commands";

   interface UserPromptStore {
     items: Record<PromptModule, PromptInfo | null>;
     drafts: Record<PromptModule, string>;
     dirty: Record<PromptModule, boolean>;
     loading: boolean;
     error: string | null;

     loadAll: () => Promise<void>;
     setDraft: (module: PromptModule, text: string) => void;
     save: (module: PromptModule) => Promise<void>;
     reset: (module: PromptModule | null) => Promise<void>;
     byteLen: (module: PromptModule) => number;
   }

   export const useUserPromptStore = create<UserPromptStore>(...);
   ```

   - 初始 `items` 为 `{ tagging: null, para: null, concept: null, aggregation: null }`
   - 初始 `drafts` 全部空字符串；初始 `dirty` 全部 false

2. **AC-2（loadAll 实现）** — `loadAll()` 调用 `cmd.listUserPrompts()`，把返回的 4 条 `PromptInfo[]` 装载到 `items` 与 `drafts`：
   - 对每条 item：`drafts[module] = userText ?? defaultText`（让用户首次打开看到当前生效的内容）
   - `dirty[module] = false`
   - 错误：set `error: String(e)` + `loading: false`

3. **AC-3（setDraft）** — `setDraft(module, text)`：
   - 更新 `drafts[module] = text`
   - 重算 `dirty[module]`：`text !== (items[module]?.userText ?? items[module]?.defaultText ?? "")`
   - 不调用任何 invoke（纯本地状态）

4. **AC-4（save）** — `save(module)`：
   - 读 `drafts[module]`，调 `cmd.saveUserPrompt(module, draft)`；成功后调 `cmd.getUserPrompt(module)` 刷新本 module 的 `items[module]` 与 `dirty[module] = false`
   - 错误：set `error` + 抛出（让 UI 弹错），不修改本地状态以便用户修改后重试

5. **AC-5（reset）** — `reset(module | null)`：
   - 调 `cmd.resetUserPrompt(module)`；成功后：
     - `module === null` → 调 `loadAll()` 重新装载全部
     - `module !== null` → 调 `cmd.getUserPrompt(module)` 刷新该条 + 把 `drafts[module]` 同步为新的 `defaultText`，`dirty[module] = false`
   - 错误处理同 AC-4

6. **AC-6（byteLen 派生）** — `byteLen(module)`：返回 `new TextEncoder().encode(get().drafts[module]).length`（UTF-8 字节数，与后端 Rust 的 `text.len()` 字节口径一致）

7. **AC-7（vitest 单测）** — 在 `src/stores/__tests__/userPromptStore.test.ts` 新建测试：
   - mock `tauri-commands.ts` 的 4 个函数
   - 覆盖：① loadAll 成功路径 → items 与 drafts 写入 ② setDraft → dirty 切换 ③ save 调用后 dirty 归零 ④ reset(null) 调用了 listUserPrompts 与重载 ⑤ reset(module) 调用了 getUserPrompt 与单条刷新 ⑥ byteLen 中文 / emoji / 英文混合
   - 测试运行：`pnpm test --filter userPromptStore`

8. **AC-8（store re-export）** — 在 `src/stores/index.ts` 追加 `export * from "./userPromptStore";`

## 技术约束

- **代码规范**：
  - 使用 zustand `create<T>((set, get) => ({...}))` 范式（参考 `settingsStore.ts`）
  - 不在 store 中做 UI 状态（如 `expanded: Record<module, boolean>` 折叠态归 UI 组件内部 `useState`）
  - 错误处理：用 `String(e)` 一致地把异常转字符串塞 `error`，不吞 stack
- **Architect 方案约束**：
  - **不修改** 既有 `src/stores/promptStore.ts`（PR-4 半成品，详见 ADR-005 / R6）
  - 不允许使用 `useEffect` 在 store 中触发副作用（store 是纯状态容器）

## 参考文件

**必读**：
- Architect output.md `§ 6.3`（store 形状）
- task_005 input.md（理解类型与 invoke 契约）

**代码参考（必读）**：
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/stores/settingsStore.ts` — zustand 范式 + loadAll + 错误处理
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/stores/__tests__/settingsStore.test.ts`（如存在）— 单测范式
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/stores/promptStore.ts` — 不复用，但可参考其 load/save/reset 三段式

## 预估影响范围

- **新建文件**：
  - `src/stores/userPromptStore.ts`
  - `src/stores/__tests__/userPromptStore.test.ts`
- **修改文件**：
  - `src/stores/index.ts`（追加 1 行 re-export）
- **预估变更**：~250 行（含测试 ~120 行）
