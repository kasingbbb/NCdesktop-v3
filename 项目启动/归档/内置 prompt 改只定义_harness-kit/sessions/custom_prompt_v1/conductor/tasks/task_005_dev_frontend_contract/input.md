# Task 输入 — task_005_dev_frontend_contract

## 目标

在 NCdesktop 前端建立"用户自定义 Prompt"功能的类型契约层：新建 `src/types/user-prompt.ts` 并在 `src/lib/tauri-commands.ts` 追加 4 个 invoke 封装函数，作为后续 store / UI 的接口锚点。

## 前置条件

- 依赖 task：无（可与 task_002 / task_003 并行，本 task 只依赖 Architect output.md § 5.3 与 § 6.2 中的契约定义）
- 必须先存在的文件/接口：
  - `src/lib/tauri-commands.ts`（既有，需要在末尾追加新段）
  - `src/types/index.ts`（既有，需要 re-export）

## 验收标准（Acceptance Criteria）

1. **AC-1（types/user-prompt.ts 全套类型）** — 新建 `src/types/user-prompt.ts`，按 Architect output.md § 5.3 严格字段对齐：

   ```typescript
   export type PromptModule = "tagging" | "para" | "concept" | "aggregation";

   export interface PromptInfo {
     module: PromptModule;
     displayTitle: string;
     defaultText: string;
     userText: string | null;
     isCustom: boolean;
     builtinVersion: string;
     updatedAt: string | null;
     requiredPlaceholders: string[];
     maxBytes: number;
   }

   export const PROMPT_MODULES: PromptModule[] = ["tagging", "para", "concept", "aggregation"];
   export const PROMPT_MODULE_TITLES: Record<PromptModule, string> = {
     tagging: "文件打标签",
     para: "PARA 分组",
     concept: "知识概念提取",
     aggregation: "知识聚合",
   };
   ```

   - 字段命名与后端 `PromptInfo` Serde `rename_all = "camelCase"` 后的形态严格一致
   - `PROMPT_MODULES` 常量必须按"tagging → para → concept → aggregation"顺序

2. **AC-2（tauri-commands.ts 4 个函数）** — 在 `src/lib/tauri-commands.ts` 末尾追加 `// ── User Prompt ────` 分段，包含：

   ```typescript
   import type { PromptInfo, PromptModule } from "../types/user-prompt";

   export async function listUserPrompts(): Promise<PromptInfo[]> {
     return invoke<PromptInfo[]>("list_user_prompts");
   }
   export async function getUserPrompt(module: PromptModule): Promise<PromptInfo> {
     return invoke<PromptInfo>("get_user_prompt", { module });
   }
   export async function saveUserPrompt(module: PromptModule, text: string): Promise<void> {
     return invoke<void>("save_user_prompt", { module, text });
   }
   export async function resetUserPrompt(module: PromptModule | null): Promise<void> {
     return invoke<void>("reset_user_prompt", { module });
   }
   ```

   - 函数命名严格 `listUserPrompts / getUserPrompt / saveUserPrompt / resetUserPrompt`（不允许写成 `getPrompt / savePrompt` —— 必须避免与 PR-4 `promptStore.ts` 中已有的 `getPrompt` 字面名冲突）
   - import 必须使用 `import type`（不引入运行时依赖）

3. **AC-3（types/index.ts re-export）** — 在 `src/types/index.ts` 中追加 `export * from "./user-prompt";`

4. **AC-4（不破坏既有契约）** — `pnpm tsc --noEmit` 通过（前端类型检查全绿）；既有 prompt 相关代码（`promptStore.ts` / `PromptEditor.tsx`）的引用 `import type { PromptInfo } from "../../lib/tauri-commands"` 此前已 dangling，本 task 不修复（标注为已知 stale ref，在 Architect 决议清理 task 中处理）

5. **AC-5（无运行时 invoke）** — 本 task 仅写契约层，不调用 invoke。前端应用启动后不会触发新增的 IPC（由 store/UI task 触发）

## 技术约束

- **代码规范**：
  - 类型文件不带任何业务逻辑，纯类型 / 常量
  - 字段命名严格 camelCase（与 Tauri serde 默认行为对齐）
- **Architect 方案约束**：
  - `displayTitle` 中文文案严格按 PRD § 3.2 第 1 列文本
  - 命名前缀 `userPrompt` / `user_prompt`（避免与 PR-4 `promptStore.ts` 的 `PromptInfo` 类型冲突，详见 ADR-005 / R6）

## 参考文件

**必读**：
- Architect output.md `§ 5.3`（TypeScript 类型）
- Architect output.md `§ 6.2`（前端契约层 4 函数签名）
- Architect output.md `§ ADR-005 / R6`（命名隔离的理由）

**代码参考**：
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/lib/tauri-commands.ts:1-50` — 现有 invoke 封装范式
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/types/index.ts` — re-export 范式
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/types/settings.ts` — types 文件结构范例

## 预估影响范围

- **新建文件**：
  - `src/types/user-prompt.ts`
- **修改文件**：
  - `src/lib/tauri-commands.ts`（末尾追加 ~25 行）
  - `src/types/index.ts`（追加 1 行 re-export）
- **预估变更**：~150 行（含基础测试，可选）
