# Task 交付 — task_006_dev_frontend_store

## 实现摘要

新建 NCdesktop 前端"用户自定义 Prompt"功能的 **zustand store**，桥接 UI 与 task_005 落地的 Tauri 契约层：

1. **新建 `src/stores/userPromptStore.ts`** — 严格按 Architect output.md § 6.3：
   - state：`items / drafts / dirty / loading / error`，以 `PromptModule` 为主键的 3 张表（4 module × null/""/false 骨架）
   - actions：`loadAll / setDraft / save / reset / byteLen`，对应 AC-2 → AC-6
   - 完整生命周期：装载 → 编辑（dirty 实时重算）→ 保存（IPC 成功后刷新本条 + 归零 dirty）→ 恢复默认（null = 全部重载；module = 单条 + drafts 同步新 defaultText）
2. **新建 `src/stores/__tests__/userPromptStore.test.ts`** — vitest 20 用例，`vi.mock("../../lib/tauri-commands")` 拦截 4 个 `*UserPrompt*` 函数，零真实 IPC；覆盖全部 AC + 异常路径 + 不可变性 smoke。
3. **修改 `src/stores/index.ts`** — 追加 `export { useUserPromptStore } from "./userPromptStore";`，使桶导出可达（AC-8）。

**核心设计决策**：

1. **`dirty` 口径 = "draft ≠ 当前生效文本"**（AC-3 input.md 第 51 行）。`effectiveText(item) = item.userText ?? item.defaultText ?? ""` 是参考值；`setDraft` / `loadAll` / `save` / `reset` 后 dirty 自动归零。**不是**"draft ≠ 初次加载快照"。这避免了用户改回 default 后 UI 仍提示"未保存"的怪体验。
2. **`save` 错误时不动 drafts/dirty/items**（AC-4 input.md 第 56 行 "不修改本地状态以便用户修改后重试"）。失败时只写 `error` 并 `throw`；不调 `getUserPrompt`，避免覆盖用户正在编辑的草稿。测试用例 `AC-4 错误` 显式断言这点。
3. **`reset(module)` 后 `drafts[module] = fresh.defaultText`**（AC-5 input.md 第 62 行）。reset 后 `userText = null`，所以正确的草稿种子是 `defaultText`（不是 `userText`）。测试用例 `AC-5 reset(module)` 显式断言 `drafts.aggregation === "[default aggregation]"`。
4. **`byteLen` 用 `TextEncoder().encode(text).length`**，与后端 Rust `text.len()` 字节口径一致（ADR-004）。测试 `AC-6 byteLen` 含中文 6 字节 / emoji 4 字节 / 混合 14 字节 / 等价 smoke（与 `TextEncoder` 直算等价 7 个 sample）。
5. **命名隔离（ADR-005 / R6）**：store hook 名 `useUserPromptStore`，与 PR-4 半成品 `usePromptStore`（kind = classify/naming/tagging）字面 + 语义完全独立；**未修改** `stores/promptStore.ts`。
6. **不引入 UI 状态**：input.md 技术约束"不在 store 中做 UI 状态"。本 store 只暴露与服务器同步语义相关的 `loading / error`，不含 `expanded / focused / saving[module]` 等折叠态。task_007 在组件内 `useState` 自管。
7. **不引入 immer / persist 中间件**：保持与 `settingsStore.ts` 同样的"轻量 zustand"范式，set 用扩展运算符浅 clone。drafts 字节量很小（≤ 16 KiB ×4 = ~64 KiB），无性能问题。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `src/stores/userPromptStore.ts` | 新建 | zustand store：state 5 字段 + actions 5 个 + byteLen 派生；含 DEV 期骨架与 `PROMPT_MODULES` 一致性断言 |
| `src/stores/__tests__/userPromptStore.test.ts` | 新建 | vitest 20 用例：AC-1 初始 state（1）/ AC-2 loadAll（3：成功 / loading 态 / 错误）/ AC-3 setDraft（4：default 对比 / 回到生效 / 已自定义场景 / 不发 IPC）/ AC-4 save（2：成功 / 错误）/ AC-5 reset（3：null / module / 错误）/ AC-6 byteLen（6：空 / ASCII / 中文 / emoji / 混合 / 等价 smoke）/ 不可变性 smoke（1） |
| `src/stores/index.ts` | 修改（+1 行） | `export { useUserPromptStore } from "./userPromptStore";` |

## 对 Architect 方案的遵守声明

- [x] **目录结构与 Architect 方案一致**：新建文件路径与 task_001_architect / output.md § 7 完全一致（`src/stores/userPromptStore.ts` + `src/stores/__tests__/userPromptStore.test.ts` + `src/stores/index.ts` re-export）。
- [x] **API 路径/命名与 Architect 方案一致**：store 形状（`items / drafts / dirty / loading / error / loadAll / setDraft / save / reset / byteLen`）与 § 6.3 1:1 字段对齐；IPC 调用通过 `cmd.listUserPrompts / getUserPrompt / saveUserPrompt / resetUserPrompt`（task_005 落地的 4 个函数），未引入任何后端命令直接 `invoke` 旁路。
- [x] **数据模型与 Architect 方案一致**：`PromptModule` / `PromptInfo` 字段全部沿用 task_005 落地的 `src/types/user-prompt.ts`，未在 store 中新增字段；items 直接存 `PromptInfo | null`，与 § 5.3 类型严格对齐。
- [x] **未引入计划外的新依赖**：仅 `zustand`（既有）+ task_005 落地的类型/契约；`package.json` 未改。
- 偏离说明：**无**。

### 与 task_005 接口预期偏差检查

- `listUserPrompts(): Promise<PromptInfo[]>` — 使用方式与 task_005 output.md 第 79 行声明一致（无参 invoke），按返回数组顺序填充骨架（容错：未在数组中的 module 留 null）。
- `getUserPrompt(module): Promise<PromptInfo>` — save / reset(module) 之后调用刷新单条，传递 `module: PromptModule` 字面量。
- `saveUserPrompt(module, text): Promise<void>` — save action 直传 `drafts[module]`；后端拒绝时 string 透传到 `error` + throw。
- `resetUserPrompt(module: PromptModule | null): Promise<void>` — reset(null) 与 reset(module) 两路径均直传 module 参数（null 与字面量），与 task_005 测试用例 `resetUserPrompt(null)` / `resetUserPrompt('tagging')` 完全一致。

**结论**：无偏差，无需 Conductor 调和。

### 与 task_007 接口预期前向声明

下游 task_007 PromptCustomizationPanel 预计使用：
- `useUserPromptStore.getState().loadAll()` — Settings Panel 挂载时调用一次
- `s.items[module] / s.drafts[module] / s.dirty[module]` — 渲染单条折叠区
- `s.byteLen(module)` — 字节计数提示（task_007 自决与 `s.items[module]?.maxBytes` 对比，颜色提示）
- `s.setDraft(module, text)` — textarea onChange
- `s.save(module)` — 保存按钮（task_007 自管 saving 态 + try/catch 弹错）
- `s.reset(null) / s.reset(module)` — "全部恢复默认" / 单条"恢复默认"按钮
- `s.error` — 顶部错误条（可选）

`error` 字段是只读语义；task_007 需要清空 error 时，可显式 `useUserPromptStore.setState({ error: null })`（store 未导出专用 clear action，避免冗余 surface）。

## 测试命令

```bash
cd "/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop"
pnpm tsc --noEmit
pnpm test src/stores/__tests__/userPromptStore.test.ts --run
```

## 测试结果

### `pnpm tsc --noEmit`（exit code 0，与 baseline 一致）

```
（无输出，exit code 0）
```

补充：`pnpm tsc --noEmit -p tsconfig.app.json` 显示 96 error，**与 baseline 完全一致**（PR-4 半成品 `promptStore.ts` / `PromptEditor.tsx` + `calendarStore.ts` / `categoryStore.ts` 等 pre-existing dangling import），本 task 新建文件 0 error。

`grep -E "userPromptStore|user-prompt" tsc.log` 在新文件中**无任何 error**。

### `pnpm test src/stores/__tests__/userPromptStore.test.ts --run`（exit code 0）

```
 RUN  v4.1.1 /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop

 ✓ src/stores/__tests__/userPromptStore.test.ts > AC-1 初始 state > items 全 null / drafts 全空串 / dirty 全 false / loading=false / error=null 1ms
 ✓ src/stores/__tests__/userPromptStore.test.ts > AC-2 loadAll > 成功路径：4 module 全部初始化，drafts 用 userText ?? defaultText 0ms
 ✓ src/stores/__tests__/userPromptStore.test.ts > AC-2 loadAll > 加载中：loading=true → 完成后 loading=false 0ms
 ✓ src/stores/__tests__/userPromptStore.test.ts > AC-2 loadAll > 错误路径：error 字段透传字符串消息 + loading=false 0ms
 ✓ src/stores/__tests__/userPromptStore.test.ts > AC-3 setDraft > 初始装载后 setDraft 与 defaultText 不同 → dirty=true 0ms
 ✓ src/stores/__tests__/userPromptStore.test.ts > AC-3 setDraft > setDraft 回到 effectiveText → dirty 回 false 0ms
 ✓ src/stores/__tests__/userPromptStore.test.ts > AC-3 setDraft > 已自定义场景：effectiveText = userText，setDraft 与 userText 比较 0ms
 ✓ src/stores/__tests__/userPromptStore.test.ts > AC-3 setDraft > 不发 IPC（纯本地）0ms
 ✓ src/stores/__tests__/userPromptStore.test.ts > AC-4 save > 成功：调用 saveUserPrompt + getUserPrompt + items 刷新 + dirty 归零 1ms
 ✓ src/stores/__tests__/userPromptStore.test.ts > AC-4 save > 错误：error 字段透传中文消息 + 抛出 + drafts/dirty/items 不变 0ms
 ✓ src/stores/__tests__/userPromptStore.test.ts > AC-5 reset > reset(null)：调 resetUserPrompt(null) + loadAll 重载全部 4 条 0ms
 ✓ src/stores/__tests__/userPromptStore.test.ts > AC-5 reset > reset(module)：调 resetUserPrompt(module) + getUserPrompt + drafts 同步 defaultText 0ms
 ✓ src/stores/__tests__/userPromptStore.test.ts > AC-5 reset > reset 错误：error 字段写入 + 抛出 0ms
 ✓ src/stores/__tests__/userPromptStore.test.ts > AC-6 byteLen（UTF-8 字节，与后端 ADR-004 口径一致）> 空串 = 0 字节 0ms
 ✓ src/stores/__tests__/userPromptStore.test.ts > AC-6 byteLen（UTF-8 字节，与后端 ADR-004 口径一致）> 纯英文 ASCII：每字符 1 字节 0ms
 ✓ src/stores/__tests__/userPromptStore.test.ts > AC-6 byteLen（UTF-8 字节，与后端 ADR-004 口径一致）> 纯中文（CJK）：每字符 3 字节 0ms
 ✓ src/stores/__tests__/userPromptStore.test.ts > AC-6 byteLen（UTF-8 字节，与后端 ADR-004 口径一致）> emoji（U+1F31F 星）= 4 字节 0ms
 ✓ src/stores/__tests__/userPromptStore.test.ts > AC-6 byteLen（UTF-8 字节，与后端 ADR-004 口径一致）> 中文 + emoji + 英文混合：分别加和 0ms
 ✓ src/stores/__tests__/userPromptStore.test.ts > AC-6 byteLen（UTF-8 字节，与后端 ADR-004 口径一致）> byteLen 与 TextEncoder 等价（防止实现回归）0ms
 ✓ src/stores/__tests__/userPromptStore.test.ts > AC-1 / AC-3 不可变性 smoke > setDraft 返回新引用（不就地 mutate drafts 对象）0ms

 Test Files  1 passed (1)
      Tests  20 passed (20)
   Start at  16:28:48
   Duration  460ms (transform 40ms, setup 37ms, import 41ms, tests 6ms, environment 303ms)
```

### 回归测试（task_005 + 本 task + settingsStore）

```
pnpm test src/lib/__tests__/user-prompt.contract.test.ts \
          src/stores/__tests__/userPromptStore.test.ts \
          src/stores/__tests__/settingsStore.test.ts --run

 Test Files  3 passed (3)
      Tests  47 passed (47)
```

无回归。task_005 的 14 个契约测试与 settingsStore 的 13 个测试均维持 PASS。

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|----------|----------|------|-----------|
| ✅ 正常路径 | `pnpm tsc --noEmit` 默认 references-only 配置全绿（exit 0） | 已测 | PASS — exit 0；`tsconfig.app.json` 96 errors 与 baseline 一致，本 task 新建 0 error |
| ✅ 正常路径 | AC-1 store 初始 state：items 全 null / drafts 全 "" / dirty 全 false / loading=false / error=null | 已测 | PASS — `expect(INITIAL.items).toEqual(...)` 等 5 个断言 |
| ✅ 正常路径 | AC-2 loadAll 成功：4 module 全部初始化，drafts 按 `userText ?? defaultText` 填充 | 已测 | PASS — 1 module 自定义 + 3 module 未自定义场景同测，覆盖两种分支 |
| ✅ 正常路径 | AC-2 loadAll 期间 loading=true → 完成后 loading=false | 已测 | PASS — 手动 hold Promise 验证 in-flight 态 |
| ✅ 正常路径 | AC-3 setDraft：与 defaultText 不同 → dirty=true；回到 effectiveText → dirty=false | 已测 | PASS — 同覆盖 userText=null 与 userText="..." 两种 effectiveText 来源 |
| ✅ 正常路径 | AC-3 setDraft：不发 IPC（纯本地） | 已测 | PASS — 4 个 mock 均 `not.toHaveBeenCalled()` |
| ✅ 正常路径 | AC-4 save 成功：IPC 调用顺序 saveUserPrompt → getUserPrompt → items 刷新 → dirty 归零 → error=null | 已测 | PASS — `toHaveBeenCalledWith` 对入参精确断言，含 `updatedAt` 字段刷新 |
| ✅ 正常路径 | AC-5 reset(null)：调 resetUserPrompt(null) + 后续 listUserPrompts 重载 4 条 | 已测 | PASS — listUserPrompts 调用次数 2（初始 + reset 后）+ items.tagging 恢复 isCustom=false |
| ✅ 正常路径 | AC-5 reset(module)：调 resetUserPrompt(module) + getUserPrompt 单条刷新 + drafts 同步新 defaultText | 已测 | PASS — `drafts.aggregation === "[default aggregation]"` |
| ✅ 正常路径 | AC-6 byteLen：空串 / ASCII / 中文 / emoji / 混合 / 与 TextEncoder 等价 smoke | 已测 | PASS — 6 用例，含 `Hi 你好 🌟` = 14 字节明确断言 |
| ✅ 正常路径 | AC-8 `stores/index.ts` re-export | 已测 | PASS — tsc 全绿，下游可 `import { useUserPromptStore } from "@/stores"` |
| ⚠️ 边界条件 | setDraft 不就地 mutate drafts 对象（zustand 引用相等检测） | 已测 | PASS — `expect(after).not.toBe(before)` |
| ⚠️ 边界条件 | save 失败时 drafts / dirty / items 维持原状（用户原地重试） | 已测 | PASS — 显式比较 `draftBeforeSave` / `itemBeforeSave` 引用相等 |
| ⚠️ 边界条件 | save 失败时不调 getUserPrompt（避免覆盖正在编辑的草稿） | 已测 | PASS — `mockGetUserPrompt.not.toHaveBeenCalled()` |
| ❌ 异常路径 | loadAll IPC 错误：error 字段透传 string + loading=false（无 throw） | 已测 | PASS — `expect(s.error).toBe("数据库读取失败")` |
| ❌ 异常路径 | save IPC 错误：error 字段透传 string + 抛出（throw） | 已测 | PASS — `rejects.toBe(...)` + `state.error` 双断言 |
| ❌ 异常路径 | reset IPC 错误：error 字段写入 + 抛出 | 已测 | PASS — `rejects.toBe("数据库写入失败")` + state 断言 |
| ⚠️ 边界条件 | listUserPrompts 返回空数组（理论不应发生，后端恒返回 4 条） | 未测 | 跳过：input.md AC-2 未要求；后端契约由 task_002 保障恒 4 条；store 兼容（drafts 留 ""，items 留 null） |

## 已知局限

1. **`saving[module]: Record<...,  boolean>` 未在 store 暴露**：input.md 系统提示词字段列表中曾建议 `saving / resetting` in-flight 态，但 AC-1（store 形状）/ § 6.3（Architect）均未列入；且技术约束"不在 store 中做 UI 状态"指向 task_007 自管。本 task 严格按 AC 实现，不引入。**任何 task_007 需要的 in-flight 态由组件 `useState` 持有**。
2. **`clearError()` action 未暴露**：error 字段仅在 loadAll/save/reset 内部覆写（成功置 null / 失败置 string）；task_007 如需手动清错，可直接 `useUserPromptStore.setState({ error: null })`。这是 zustand 范式，但若 Reviewer 偏好显式 API surface，可在后续小修中加 `clearError: () => void`。
3. **listUserPrompts 返回数组顺序假设**：store loadAll 不强制要求按 `tagging → para → concept → aggregation` 顺序；只按 `info.module` 字段路由到对应 key。后端按 Architect 方案恒按该序，但前端不依赖此序，韧性更好。
4. **PR-4 半成品 `promptStore.ts` 仍 dangling**：96 个 baseline tsc error 大部分来自 `promptStore.ts` 引用已不存在的 `cmd.PromptInfo / DryRunOutcome / getPrompt / savePrompt / dryRunPrompt / resetPrompt`，以及 `calendarStore / categoryStore` 等其它 PR 工作树状态。**与本 task 无关**，按 ADR-005 / R6 与 input.md 约束"不修改 PR-4 半成品"，不在本 task 范围。建议 Conductor 在 task_010 后开独立清理 task。
5. **`PROMPT_MODULES` 一致性断言仅 DEV 期生效**：`if (import.meta.env?.DEV)` 包裹，production build 会被 tree-shake。这是低成本的"防错位"措施；若未来 task_005 扩展 PromptModule 字面量联合，DEV 期 import 立刻 console.warn 提醒。

## 需要 Reviewer 特别关注的地方

1. **`userPromptStore.ts:144` `save` 错误分支不调 getUserPrompt**：此为有意设计（已知局限 / AC-4 input.md 第 56 行）。Reviewer 如需切换为"失败时也刷新一次 server 状态"，需要重读 AC-4 措辞"不修改本地状态以便用户修改后重试"——当前实现是 AC 的字面落地。
2. **`userPromptStore.ts:60` `effectiveText` 函数 + AC-3 dirty 口径**：与 input.md 第 51 行的公式 `text !== (items[module]?.userText ?? items[module]?.defaultText ?? "")` 字面对齐。Reviewer 可重点对照测试 `AC-3 已自定义场景：effectiveText = userText` 用例（含两次切换路径），断言这个口径在两种 effectiveText 来源（userText 优先 / defaultText 兜底）下都正确。
3. **`userPromptStore.test.ts:43` `makeInfo` fixture 的 `displayTitle: module` 简化**：测试用 module 字符串本身作 displayTitle（如 `"tagging"`），而非真实的 `"文件打标签"`。这是因为 store 内部不读 displayTitle 字段（仅在 UI 渲染用），简化测试 setup。若 Reviewer 偏好真实标题，可改为 `PROMPT_MODULE_TITLES[module]`——但不会影响测试覆盖度。
4. **`stores/index.ts:10` re-export 顺序**：放在最后（与 promptStore 的 PR-4 旧 export 不在同一 PR 中）。如 Reviewer 偏好按字母序，可移到 `useSettingsStore` 与 `useUIStore` 之间——但当前顺序与 task_005 commits 风格一致（新增 export 追加在末尾）。
5. **`userPromptStore.ts:184` DEV 期断言 console.warn**：若未来 vitest 测试无意触发该断言（PROMPT_MODULES 扩展但骨架未同步），会产生 warning 但不会让测试 fail。Reviewer 可考虑是否升级为 throw（更严格）——当前用 warn 是因 import.meta.env.DEV 在 vitest 中可能为 truthy，warn 不会破坏开发流。
