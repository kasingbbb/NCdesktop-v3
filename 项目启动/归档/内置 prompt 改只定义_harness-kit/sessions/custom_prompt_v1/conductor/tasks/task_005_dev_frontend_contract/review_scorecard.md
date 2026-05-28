# Review Scorecard — task_005_dev_frontend_contract

## 审查思考过程

### 1. Task 意图
在 NCdesktop 前端建立"用户自定义 Prompt"功能的**类型契约层**：新建 `src/types/user-prompt.ts`（`PromptModule` 联合 + `PromptInfo` 9 字段 camelCase + `PROMPT_MODULES` 顺序常量 + `PROMPT_MODULE_TITLES` 中文标题）；在 `src/lib/tauri-commands.ts` 末尾追加 4 个 invoke 封装（`listUserPrompts / getUserPrompt / saveUserPrompt / resetUserPrompt`）；`src/types/index.ts` re-export。本 task 不调用 invoke，仅暴露契约。

### 2. AC 检查结果

| AC | 描述 | 结果 |
|----|------|------|
| **AC-1** | `types/user-prompt.ts` 含 `PromptModule` 联合 / `PromptInfo` 9 字段 camelCase / `PROMPT_MODULES` 顺序常量 / `PROMPT_MODULE_TITLES` 中文标题 | ✅ 字段命名与后端 `commands::user_prompt::PromptInfo`（Serde camelCase）逐项核对（`src-tauri/src/commands/user_prompt.rs:34-44`）一致：`module / displayTitle / defaultText / userText / isCustom / builtinVersion / updatedAt / requiredPlaceholders / maxBytes`；顺序常量 `[tagging, para, concept, aggregation]` 严格 |
| **AC-2** | `tauri-commands.ts` 末尾追加 4 个 invoke 封装，函数名 `*UserPrompt*` 前缀，`import type` | ✅ 见 `src/lib/tauri-commands.ts:802-839`；4 函数签名与 Architect § 6.2 + 后端 `commands/user_prompt.rs:108-176` 实际签名 1:1（参数包 `{ module }` / `{ module, text }` 对齐 Rust 函数命名参数） |
| **AC-3** | `types/index.ts` 追加 `export * from "./user-prompt";` | ✅ 见 `src/types/index.ts:58-60`；附 2 行注释说明为何用 `export *` 而非 `export type *`（因运行时常量 `PROMPT_MODULES`） |
| **AC-4** | `pnpm tsc --noEmit` 0 error，既有 dangling import 不修复 | ✅ 复跑 `pnpm tsc --noEmit` 退出码 0、零输出；PR-4 半成品 `promptStore.ts:40/65/71/77` 中的 `cmd.getPrompt/savePrompt/dryRunPrompt/resetPrompt` 仍是 dangling（本 task 不修） |
| **AC-5** | 不发起真实 IPC | ✅ 契约层不调用 invoke；测试中 `vi.mock("@tauri-apps/api/core")` 全程拦截 |

### 3. 关键发现
1. **字段映射完全对齐**：前端 9 个 camelCase 字段（`PromptInfo`）与后端 `#[serde(rename_all = "camelCase")] pub struct PromptInfo`（`commands/user_prompt.rs:34-44`）逐字段镜像，类型映射 `Option<String> ↔ string | null` / `Vec<String> ↔ string[]` / `usize ↔ number` 均正确。
2. **参数包形态完全对齐 task_002 实际签名**：后端 `save_user_prompt(module: String, text: String)`、`reset_user_prompt(module: Option<String>)`，前端 `{ module, text }` / `{ module }` 透传正确（Tauri 默认 camelCase 参数名映射，与 Rust 单词字段名无差异）。
3. **测试质量真实**：14 用例非占位——`vi.mock` 拦截 invoke，断言 `toHaveBeenCalledWith("命令名", 参数包)`；`Object.keys(info).sort()` 9 字段镜像验证 camelCase；`rejects.toBe("中文字符串")` 验证 Tauri `Result<T, String>` 错误透传。`it.each` 覆盖 4 module × `getUserPrompt`。
4. **PR-4 半成品零触碰**：`git status` 显示仅 `tauri-commands.ts`、`types/index.ts` 改动 + 2 个新建文件；`promptStore.ts` / `PromptEditor.tsx` / `commands/prompts.rs` 全部未触碰；ADR-005/R6 命名隔离严格执行（新前端用 `*UserPrompt*` 与旧 `getPrompt/savePrompt` 字面隔离）。

---

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | AC-1~5 全部满足；4 函数 invoke 命令名 + 参数包与 task_002 后端实际签名 1:1；类型 9 字段全镜像；`pnpm tsc --noEmit` 退出 0；14/14 测试 PASS。 |
| 安全性 | 20% | 5 | 契约层不引入 IPC 调用、无网络、无新依赖；错误传递保持后端 `Result<T, String>` 原样透传（不 swallow/wrap）；类型联合 `PromptModule` 将非法 module 字面量挡在编译期；与"防 Prompt 注入"职责无关（由后端 Layer A/B 担保，符合 ADR-003 / ADR-005 分层）。 |
| 代码质量 | 15% | 5 | 文件头部含追溯链注释（"真相来源：task_001_architect / output.md § 5.3"）；JSDoc 标注 9 字段语义、命名隔离 R6 来由、四道防线引用 ADR；`export *` vs `export type *` 注释解释为何需要前者；函数命名严格遵守 AC-2 约束（避免与 PR-4 字面冲突）。 |
| 测试覆盖 | 10% | 5 | 14 用例覆盖：① 常量顺序/标题（AC-1）；② 4 函数 invoke 透传（AC-2，含 4 module × getUserPrompt 全枚举 + 双形态 reset）；③ types 桶导出 reference equality（AC-3，用 `.toBe()` 钉死 tree-shaking 语义）；④ 9 字段 camelCase 镜像；⑤ null 边界；⑥ 字符串异常透传。`vi.mock` 真拦截，非占位 `expect(true).toBe(true)`。 |
| 架构一致性 | 10% | 5 | 目录结构与 Architect § 7 完全一致（`src/types/user-prompt.ts` + `tauri-commands.ts` 末尾追加）；命名 100% 严格 § 6.2；遵守 ADR-005 / R6（独立 store/types，不复用 PR-4）；类型 9 字段与 § 5.3 逐字段对齐，零自创字段；不引入新依赖。 |
| 可维护性 | 20% | 5 | 文件头部 + 各导出含 JSDoc 注释链回 ADR 与 PRD 段号，3 个月后另一 Agent 通过注释即可定位语义出处；常量与类型分离便于 P2 增/改 module（如新增 module 只需改 `PromptModule` 联合 + `PROMPT_MODULES` + `PROMPT_MODULE_TITLES` 三处）；测试中桶导出 `.toBe()` 引用相等防御 tree-shaking 优化破坏 re-export 语义；命名隔离前缀 `*UserPrompt*` 提示未来开发者勿与 PR-4 `getPrompt` 混淆。 |

**综合分：5.00 / 5**（加权计算 = 5×0.25 + 5×0.20 + 5×0.15 + 5×0.10 + 5×0.10 + 5×0.20 = 5.00）

---

## 总体判断

- [x] **PASS**
- [ ] FIX
- [ ] BLOCKER

**判定依据**：所有 5 个 AC 全部满足；类型字段 / 函数签名 / 参数包 / 函数命名全部与 Architect § 5.3 + § 6.2 + task_002 后端实际签名严格对齐；`pnpm tsc --noEmit` 零 error；14/14 真测试 PASS；PR-4 半成品零触碰；无 BLOCKER 与 MAJOR 问题。

---

## 问题列表

### BLOCKER（必须修复）
无。

### MAJOR（强烈建议修复）
无。

### MINOR（可选）

1. **`getUserPrompt` / `saveUserPrompt` 函数参数采用位置参数而非对象参数**
   - **代码位置**：`src/lib/tauri-commands.ts:819`、`:828`
   - **现状**：`getUserPrompt(module: PromptModule)` 与 `saveUserPrompt(module: PromptModule, text: string)` 为位置参数；与 input.md AC-2 给出的示例签名 `getUserPrompt(module: PromptModule)` 一致，但当未来加入第三个可选入参（如 `dryRun?: boolean`）时位置参数扩展性弱于对象参数。
   - **修复方向（可选）**：可保留现状（与 AC-2 完全一致），或在 task_006 store 包装层做 adapter。**不影响本 task PASS**。
   - **验证标准**：无需改动。

2. **测试用例中桶导出 `.toBe()` 引用相等的语义防御**
   - **代码位置**：`src/lib/__tests__/user-prompt.contract.test.ts:68-69`
   - **现状**：使用 `expect(PROMPT_MODULES_FROM_INDEX).toBe(PROMPT_MODULES)`（引用相等），Dev 在 output.md "需要 Reviewer 特别关注" § 3 已说明意图是钉死 tree-shaking 不重新构造数组。这是合理但相对严格的语义防御，未来若 Vite 切到带 inline 优化的 plugin 可能误报。
   - **修复方向（可选）**：可同时保留 `.toEqual()` 与 `.toBe()` 双断言，或加一条注释说明触发失败时的处置策略（"若失败说明 re-export 链断、不是 tree-shaking 问题"）。**不影响本 task PASS**。
   - **验证标准**：无需改动。

3. **类型测试用例中 `displayTitle` 取自 `PROMPT_MODULE_TITLES[module]`，但断言 `result === fixture` 没显式核对 displayTitle 实际从后端返回的中文值**
   - **代码位置**：`src/lib/__tests__/user-prompt.contract.test.ts:88-105`
   - **现状**：测试用 fixture 自造 PromptInfo（含 `displayTitle: PROMPT_MODULE_TITLES[module]`），但本 task 是契约层而非端到端，displayTitle 由后端写入（task_003 回填），所以这里只测"封装函数透传"而非"后端会返回什么"。语义正确。
   - **修复方向（可选）**：无需改动；displayTitle 实际值由 task_003 与 task_008 e2e 验证。
   - **验证标准**：无需改动。

---

## 给 Dev 的修复指引

不适用（PASS）。

---

## 自检清单（Reviewer）

- [x] 逐条检查了 5 个 AC 的满足情况，全部 ✅
- [x] 检查了 session_context.md § 6 领域审查重点（4 module 独立隔离 ✓；命名隔离 ✓；不引入新依赖 ✓；契约层不影响 prompt 注入路径 ✓）
- [x] 每个 MINOR 都给出了位置 + 修复方向（虽然无需修复）
- [x] 评分诚实：本 task 是纯类型契约层 + 透传封装，AC 极度具体可测，5/5 是合理评价而非"还不错"
- [x] 实跑 `pnpm tsc --noEmit`（exit 0）+ `pnpm test src/lib/__tests__/user-prompt.contract.test.ts`（14/14 PASS）
- [x] 交叉核对了 task_002 后端 `PromptInfo` 结构与 commands 签名（`src-tauri/src/commands/user_prompt.rs:34-44 / 108-176`），前端 9 字段 + 4 函数完全 1:1
- [x] `git status --short` 验证 PR-4 半成品文件零触碰

---

## 审查前验证记录

- [x] 测试结果存在且非空（`pnpm tsc --noEmit` exit 0 + Vitest 14/14 完整输出粘贴在 output.md § 测试结果）
- [x] 自测验证矩阵存在且正常路径全部 PASS（13 行矩阵，全 PASS）
- [x] 架构遵守声明已填写（4 项全 [x]，"偏离说明：无"）
