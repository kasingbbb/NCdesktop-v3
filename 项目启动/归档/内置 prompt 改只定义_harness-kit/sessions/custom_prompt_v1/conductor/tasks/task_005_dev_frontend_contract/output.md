# Task 交付 — task_005_dev_frontend_contract

## 实现摘要

在 NCdesktop 前端建立"用户自定义 Prompt"功能的**类型契约层**，作为后续 store / UI 的接口锚点：

1. **新建 `src/types/user-prompt.ts`** — 按 Architect output.md § 5.3 严格对齐：
   - `PromptModule` 字面量联合 `'tagging' | 'para' | 'concept' | 'aggregation'`
   - `PromptInfo` 9 个 camelCase 字段（与后端 serde `rename_all = "camelCase"` 严格对齐）
   - 运行时常量 `PROMPT_MODULES`（固定顺序）+ `PROMPT_MODULE_TITLES`（PRD § 3.2 中文标题）
2. **修改 `src/lib/tauri-commands.ts`** — 末尾追加 `// ── User Prompt ────` 分段，含 4 个 `*UserPrompt*` 前缀的 invoke 封装：`listUserPrompts / getUserPrompt / saveUserPrompt / resetUserPrompt`。`import type` 仅引入类型，不引入运行时依赖（AC-2）。
3. **修改 `src/types/index.ts`** — 追加 `export * from "./user-prompt";`，使桶导出可达（AC-3）。
4. **新增测试 `src/lib/__tests__/user-prompt.contract.test.ts`** — Vitest 14 用例，`vi.mock("@tauri-apps/api/core")` 拦截 invoke，覆盖 4 个 AC（不发起真实 IPC，符合 AC-5）。

**核心设计决策**：
- **命名隔离（ADR-005 / R6）**：前缀全部用 `userPrompt` / `user_prompt`，与 PR-4 半成品 `src/stores/promptStore.ts` 中已存在但 dangling 的 `PromptInfo / cmd.getPrompt / savePrompt` 字面隔离，避免后续 import 解析冲突。
- **`PromptInfo` 字段切分**：保留 `defaultText` 与 `userText: string | null` 两字段（而非合并的 `promptText`），与后端 `commands::user_prompt::PromptInfo` 字段完全 1:1，使前端可在不再次 IPC 的前提下区分"显示文本"和"是否自定义"。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `src/types/user-prompt.ts` | 新建 | `PromptModule` 联合类型 + `PromptInfo` 接口（9 camelCase 字段）+ `PROMPT_MODULES` 顺序常量 + `PROMPT_MODULE_TITLES` 中文标题 |
| `src/lib/tauri-commands.ts` | 修改（追加 ~40 行） | 末尾追加 `// ── User Prompt ────` 分段：`import type` + 4 个 invoke 封装函数 |
| `src/types/index.ts` | 修改（+2 行注释 +1 行 export） | `export * from "./user-prompt";` |
| `src/lib/__tests__/user-prompt.contract.test.ts` | 新建 | Vitest 14 用例（AC-1 常量 / AC-2 invoke 透传 / AC-3 桶导出 / camelCase 字段镜像 / String 错误透传） |

## 对 Architect 方案的遵守声明

- [x] **目录结构与 Architect 方案一致**：新建文件路径与 task_001_architect / output.md § 7 完全一致（`src/types/user-prompt.ts` + `src/lib/tauri-commands.ts` 末尾追加）。
- [x] **API 路径/命名与 Architect 方案一致**：4 个前端封装函数名（`listUserPrompts / getUserPrompt / saveUserPrompt / resetUserPrompt`）与 § 6.2 1:1；invoke 的 command 名（`list_user_prompts / get_user_prompt / save_user_prompt / reset_user_prompt`）与后端 § 6.1 1:1；参数包 `{ module }` / `{ module, text }` / `{ module: null | string }` 与 § 6.1 完全对齐。
- [x] **数据模型与 Architect 方案一致**：`PromptModule` 4 字面量 + `PromptInfo` 9 camelCase 字段（`module / displayTitle / defaultText / userText / isCustom / builtinVersion / updatedAt / requiredPlaceholders / maxBytes`），与 § 5.3 逐字段对齐；与已交付的 task_002 后端 `PromptInfo` 字段命名/类型完全一致（已在 task_002 output.md 第 38 行确认 camelCase + 4 module 白名单 + 16 KiB）。
- [x] **未引入计划外的新依赖**：仅 `import type` + 既有 `invoke`（已有 import）+ Vitest（既有 devDep）。`package.json` 未改。
- 偏离说明：**无**。

## 测试命令

```bash
cd "/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop"
pnpm tsc --noEmit
pnpm test src/lib/__tests__/user-prompt.contract.test.ts
```

## 测试结果

### `pnpm tsc --noEmit`（exit code 0，类型全绿）

```
> ncdesktop@0.0.0
> tsc --noEmit
（无输出，exit code 0）
```

### `pnpm test src/lib/__tests__/user-prompt.contract.test.ts`

```
> ncdesktop@0.0.0 test /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop
> vitest run src/lib/__tests__/user-prompt.contract.test.ts


 RUN  v4.1.1 /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop

 ✓  src/lib/__tests__/user-prompt.contract.test.ts (14 tests) 4ms
   ✓ PromptModule 字面量与常量（AC-1）
     ✓ PROMPT_MODULES 顺序固定 tagging → para → concept → aggregation，恒 4 条
     ✓ PROMPT_MODULE_TITLES 4 个 module 均有非空中文标题，文案严格按 PRD § 3.2
   ✓ types/index.ts re-export（AC-3）
     ✓ PROMPT_MODULES 通过 types 桶导入与直接导入引用一致
   ✓ tauri-commands 封装函数（AC-2）
     ✓ listUserPrompts 调用 'list_user_prompts'，不带参数
     ✓ getUserPrompt(tagging) 调用 'get_user_prompt' 并传 { module }
     ✓ getUserPrompt(para) 调用 'get_user_prompt' 并传 { module }
     ✓ getUserPrompt(concept) 调用 'get_user_prompt' 并传 { module }
     ✓ getUserPrompt(aggregation) 调用 'get_user_prompt' 并传 { module }
     ✓ saveUserPrompt 调用 'save_user_prompt' 并传 { module, text }
     ✓ resetUserPrompt(null) 调用 'reset_user_prompt' 并传 { module: null }（全部恢复默认）
     ✓ resetUserPrompt('tagging') 调用 'reset_user_prompt' 并传 { module: 'tagging' }（单条恢复）
     ✓ 后端以 string 形式 reject 时，封装函数透传 string 异常（与 Result<T, String> 对齐）
   ✓ PromptInfo 类型契约（AC-1 字段命名 camelCase）
     ✓ 9 个字段均为预期类型（编译时检查的运行时镜像）
     ✓ userText / updatedAt 允许 null（未自定义场景）

 Test Files  1 passed (1)
      Tests  14 passed (14)
   Start at  16:13:58
   Duration  658ms (transform 40ms, setup 71ms, import 37ms, tests 4ms, environment 470ms)
```

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|----------|----------|------|-----------|
| ✅ 正常路径 | `pnpm tsc --noEmit` 全仓库类型检查 | 已测 | PASS — exit code 0，无 error |
| ✅ 正常路径 | `PROMPT_MODULES` 顺序恒为 `tagging → para → concept → aggregation`（AC-1） | 已测 | PASS — `expect(PROMPT_MODULES).toEqual([...])` |
| ✅ 正常路径 | `PROMPT_MODULE_TITLES` 4 个标题严格按 PRD § 3.2（AC-1） | 已测 | PASS — `文件打标签 / PARA 分组 / 知识概念提取 / 知识聚合` |
| ✅ 正常路径 | `listUserPrompts` 透传 `invoke("list_user_prompts")` 无参（AC-2） | 已测 | PASS |
| ✅ 正常路径 | `getUserPrompt(m)` × 4 module 透传 `invoke("get_user_prompt", { module })`（AC-2） | 已测 | PASS — `it.each` 4 个 module 全过 |
| ✅ 正常路径 | `saveUserPrompt(m, text)` 透传 `invoke("save_user_prompt", { module, text })`（AC-2） | 已测 | PASS |
| ✅ 正常路径 | `resetUserPrompt(null)` / `resetUserPrompt('tagging')` 两形态透传（AC-2） | 已测 | PASS |
| ✅ 正常路径 | `types/index.ts` re-export 后从桶导入与直接导入引用一致（AC-3） | 已测 | PASS — `toBe` 引用相等 |
| ⚠️ 边界条件 | `PromptInfo.userText` / `updatedAt` 接受 `null`（未自定义场景） | 已测 | PASS — 显式构造 null 字面量编译通过 + 运行期 `toBeNull` |
| ⚠️ 边界条件 | 9 个字段全 camelCase（运行时 `Object.keys` 镜像 + 编译时类型约束） | 已测 | PASS |
| ❌ 异常路径 | 后端以 `String` 形式 reject 时，封装函数原样透传字符串异常（与 `Result<T, String>` 范式对齐） | 已测 | PASS — `rejects.toBe("自定义 Prompt 过长...")` |
| ❌ 异常路径 | 本 task 不发起真实 IPC（AC-5） | 已测 | PASS — 全程 `vi.mock("@tauri-apps/api/core")`，无任何真实 invoke 调用 |
| ⚠️ 边界条件 | 不破坏既有 prompt 相关代码的 dangling import（AC-4） | 已测 | PASS — `tsc --noEmit` 全绿；未触碰 `promptStore.ts` / `PromptEditor.tsx` / `commands/prompts.rs` |

## 已知局限

1. **桶导出冲突隐性风险**：`types/index.ts` 现已 `export * from "./user-prompt";` 与既有 PR-4 半成品 `stores/promptStore.ts` 内自定义的 `PromptInfo / PromptModule` 在符号层面命名相同（虽然 store 内的是 `cmd.PromptInfo`/`kind: classify/naming/tagging`，含义不同）。当前 `tsc` 全绿因为 store 内的类型仅作为局部 alias 使用、未通过 `types/index.ts` 桶导出；如未来 store 改造时无意 `import { PromptInfo } from "@/types"`，IDE 提示会落到本 task 新建的契约层 —— 这是 ADR-005 / R6 期望的行为，但需要 task_010_architecture_guard 时显式确认。
2. **既有 `__tests__/` 中 9 个测试文件 fail**（共 43 tests fail / 293 tests pass，于全套 vitest run 时出现）：fail 全部集中于 `Sidebar / SettingsPanel / TitleBar / Inspector / TagTree / useDragAssets / AppLayout / ContentArea / turnLearningOff` 等 UI 层，根因为 jsdom 缺 `window.matchMedia` polyfill + working-tree 未提交的 PR-A/B SettingsPanel / Sidebar 改动（见 `git status`），**与本 task 完全无关**，且按 R6 不在本 task 触碰范围内。本 task 自有的 `user-prompt.contract.test.ts` 14/14 全绿。
3. **`PromptInfo.requiredPlaceholders / maxBytes` 当前在前端是"被动接收"字段**：本 task 不实现校验逻辑，仅类型契约层暴露给 store/UI。前端实际占位符校验由 task_003（后端 Layer B）执行，前端 UI 校验由 task_007 完成（基于 `requiredPlaceholders` 数组实时高亮）—— 这是 Architect § 5.3 / ADR-003 的明确分工。

## 需要 Reviewer 特别关注的地方

1. **`src/types/user-prompt.ts:38-48` `PromptInfo` 字段顺序与命名**：必须与后端 `src-tauri/src/commands/user_prompt.rs::PromptInfo`（Serde `#[serde(rename_all = "camelCase")]`）严格一致。已与 task_002 output.md 第 38 行 + 第 8-9 行交叉确认对齐，但建议 Reviewer 直接打开后端 `PromptInfo` 结构体 diff 比对一遍。
2. **`src/lib/tauri-commands.ts:828-829` `saveUserPrompt` 入参包**：必须为 `{ module, text }` 两键的字面量对象（Tauri invoke 默认按字段名传递 Rust 入参 `module: String, text: String`）。如改成 `{ module, content }` 或 `{ module, prompt }` 会让后端反序列化失败但 TS 不报错（已被错误透传测试覆盖，但 Reviewer 仍建议人肉确认 key 名）。
3. **`src/lib/__tests__/user-prompt.contract.test.ts:32-35` 桶导出引用相等测试**：用 `.toBe()`（引用相等）而非 `.toEqual()`（结构相等）—— 这是为了把 "tree-shaking 不会重新构造数组" 的语义钉死。如未来切到 `babel-plugin-` 之类做 inline 优化导致 `PROMPT_MODULES_FROM_INDEX !== PROMPT_MODULES`，本测试会失败，从而保护 `types/index.ts` re-export 语义。
