# Review Scorecard — task_006_dev_frontend_store

## 审查思考过程

### 1. Task 意图
新建 `src/stores/userPromptStore.ts`（zustand store），桥接 UI 与 task_005 落地的 Tauri 契约层，承担 4 个 module 的加载 / 草稿编辑 / dirty 派生 / 保存 / 恢复默认 / UTF-8 字节计数。命名前缀严守 `userPrompt`，与 PR-4 半成品 `promptStore.ts` 字面 + 语义隔离（ADR-005 / R6）。

### 2. AC 检查结果

| AC | 状态 | 代码位置 |
|---|---|---|
| AC-1 store 形状 + 初始骨架 | ✅ | `userPromptStore.ts:73-104`（state 5 字段 + actions 5 个）+ `:30-58`（emptyItems/Drafts/Dirty 工厂） |
| AC-2 loadAll：4 module 全装载、drafts = userText ?? defaultText、错误透传 | ✅ | `userPromptStore.ts:106-128`（`drafts[m] = info.userText ?? info.defaultText` 在 line 119；错误 `String(e) + loading:false` 在 line 126） |
| AC-3 setDraft：dirty 重算公式 `text !== (userText ?? defaultText ?? "")` + 不发 IPC | ✅ | `userPromptStore.ts:130-138` + `:68` `effectiveText` 函数；测试 line 217-223 用 4 个 mock not.toHaveBeenCalled 显式断言 |
| AC-4 save：成功路径 saveUserPrompt → getUserPrompt → items 刷新 + dirty 归零；错误时不动本地态 + throw | ✅ | `userPromptStore.ts:140-157`（成功 line 143-151；错误 line 152-156 仅 `error` + throw，不调 getUserPrompt） |
| AC-5 reset：null → loadAll；module → getUserPrompt + `drafts = fresh.defaultText` | ✅ | `userPromptStore.ts:159-179`（line 162 `module === null` 分支 → `get().loadAll()`；line 165-174 单条分支同步 defaultText） |
| AC-6 byteLen：UTF-8 `TextEncoder().encode().length` | ✅ | `userPromptStore.ts:181-184`；测试 6 用例覆盖空/ASCII/中文/emoji/混合/与 TextEncoder 等价 smoke |
| AC-7 vitest 单测 + 全 mock IPC | ✅ | `__tests__/userPromptStore.test.ts:18-23` `vi.mock("../../lib/tauri-commands")`；20 用例全 PASS（实跑确认） |
| AC-8 stores/index.ts re-export | ✅ | `stores/index.ts:10` `export { useUserPromptStore } from "./userPromptStore";` |

### 3. 关键发现

1. **AC 字面对齐严密**。dirty 公式（input.md § AC-3 第 51 行 `text !== (items[module]?.userText ?? items[module]?.defaultText ?? "")`）通过 `effectiveText` 抽出后在 setDraft / loadAll / save / reset 四处统一应用——同公式四处一致，避免散落 mutation。save 失败时不调 getUserPrompt（避免覆盖用户正在编辑的草稿）有明显设计意图且测试显式断言（`mockGetUserPrompt.not.toHaveBeenCalled()`）。
2. **R6 / ADR-005 合规严格**。`git status` 显示 `src/stores/promptStore.ts` / `src/components/settings/PromptEditor.tsx` 均**未修改**；新建 `userPromptStore.ts` + 测试 + index.ts re-export 1 行均为命名前缀 `userPrompt`，与 PR-4 半成品字面 + 语义隔离。后端 `commands/user_prompt.rs::PromptInfo`（9 字段 serde camelCase）与前端 `src/types/user-prompt.ts::PromptInfo`（9 字段 camelCase）字面对齐；store `save` 调 `cmd.saveUserPrompt(module, draft)` 与 tauri-commands.ts:828 的两位置参签名完美对齐。
3. **实跑验证一致**：`pnpm exec tsc --noEmit` exit 0（与 Dev 自述一致，`tsconfig.app.json` 96 baseline error 均与本 task 无关）；`vitest run src/stores/__tests__/userPromptStore.test.ts` 20/20 PASS 实跑确认。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | AC-1~AC-8 全部满足；正常路径 / 错误路径 / 边界（save 失败不动本地、reset(null) 与 reset(module) 双分支、effectiveText 两源 userText/defaultText）全覆盖；20/20 测试 PASS 实跑确认 |
| 安全性 | 20% | 5 | 错误处理用 `String(e)` 一致透传后端中文消息（不吞 stack）；store 无任何网络 IO（全部经既有 IPC 层）；无 SQL/Prompt 注入面（store 不接触 DB 与 LLM）；隐私底线（数据不离机）由后端把控，store 仅是状态容器 |
| 代码质量 | 15% | 5 | 单一职责（state + actions，无 UI 状态污染）；DRY（emptyItems/Drafts/Dirty 工厂 + effectiveText 抽函数）；JSDoc 完备（含设计要点解释 dirty 口径 / save 错误策略 / byteLen 后端口径对齐）；DEV 期骨架与 PROMPT_MODULES 一致性 console.warn 断言低成本防错位 |
| 测试覆盖 | 10% | 5 | 20 用例覆盖 AC-1~AC-6 全部 + 异常路径（loadAll/save/reset 三 IPC 错误）+ 边界（save 失败不动 drafts/items 引用相等断言 / setDraft 非 mutate）+ byteLen 与 TextEncoder 等价 smoke（防实现回归）；mock 范式干净（4 IPC 函数全 mock，零真实 IPC） |
| 架构一致性 | 10% | 5 | 严格按 task_001 § 6.3 store 形状；目录与 § 7 一致；命名前缀 `userPrompt` 严守 ADR-005；不修改 PR-4 `promptStore.ts` / `PromptEditor.tsx`（git status 确认）；store schema 与 task_005 落地的 types/契约函数一对一 |
| 可维护性 | 20% | 5 | 设计要点（dirty 口径、save 错误策略、byteLen 后端口径）在头部 JSDoc 段显式说明；effectiveText 抽函数让公式在 4 个 action 一致应用；DEV 期断言提供 task_005 扩展 PromptModule 时的早期警报；测试 fixture（makeInfo / makeFullList）解耦 module 字面量，便于后续扩展 |

**综合分：5.00/5**（加权 = 5×0.25 + 5×0.20 + 5×0.15 + 5×0.10 + 5×0.10 + 5×0.20）

## 总体判断

- [x] **PASS**
- [ ] FIX
- [ ] BLOCKER

## 问题列表

### BLOCKER（必须修复，否则不可能 PASS）
无。

### MAJOR（强烈建议修复）
无。

### MINOR（可选）

1. **`__tests__/userPromptStore.test.ts:62` `const INITIAL = useUserPromptStore.getState()` 模块顶层取值**
   - 该模式取的是模块加载时的初始快照（vi.mock 已生效，beforeEach 尚未执行），用于 AC-1 断言 `create()` 默认值。当前实现成立，但若未来加 persist 中间件（rehydrate 异步），快照会先于 rehydrate，断言会失败。建议加注释或改用 `useUserPromptStore.getInitialState?.()` 等持久化兼容写法。当前**不阻塞**，仅是未来扩展性提示。

2. **`userPromptStore.ts:190-200` DEV 期骨架一致性断言**
   - `import.meta.env?.DEV` 包裹的 `console.warn` 在 vitest 环境中可能 truthy；当前不会破坏测试，但若未来扩展 PROMPT_MODULES 联合且骨架未同步，console.warn 不会让测试 fail。Dev 自述"考虑过升级为 throw"。当前 warn 是合理保守选择，**建议在 task_010 Architecture Guard 时复核是否需要升级**。

3. **`__tests__/userPromptStore.test.ts:41` fixture `displayTitle: module`**
   - 测试简化用 module 字面量本身作 displayTitle（如 `"tagging"`）而非真实中文标题 `"文件打标签"`。store 内部不读 displayTitle，测试断言也不涉及该字段，简化合理；若 Reviewer 偏好真实标题可用 `PROMPT_MODULE_TITLES[module]`，但不影响测试覆盖度。

## 给 Dev 的修复指引

不适用（PASS）。

## 自检清单核对

- [x] 我是否逐条检查了 AC 满足情况？ ✅（AC-1~AC-8 全 ✅，每项给出代码位置）
- [x] 我是否检查了 session_context.md 的领域审查重点？ ✅（4 module 独立隔离 + 用户内容传递后端 + 防 prompt 注入由后端把控 + 隐私底线）
- [x] BLOCKER / MAJOR 是否给出修复方向 + 验证标准？ ✅（无 BLOCKER / MAJOR，3 个 MINOR 均为提示性建议）
- [x] 评分是否诚实？ ✅（综合 5.00/5 反映"AC 字面对齐严密 + 实跑全绿 + 架构合规 + 测试质量高"的客观事实；6 维均 5/5 因每维都无可挑剔，强行降分会失真）
- [x] 如果判定 FIX，修复指引是否清晰？ N/A（PASS）

## 实跑验证记录

```bash
$ cd ".../NCdesktop/项目启动/NCdesktop"
$ pnpm exec tsc --noEmit
exit_code=0  # 全绿

$ pnpm test src/stores/__tests__/userPromptStore.test.ts --run
RUN  v4.1.1
Test Files  1 passed (1)
     Tests  20 passed (20)
  Duration  618ms
```

**git status 合规验证**：
- `src/stores/promptStore.ts` —— 未修改（不在 `git status --porcelain` 输出）
- `src/components/settings/PromptEditor.tsx` —— 未修改（不在 `git status --porcelain` 输出）
- `src/stores/userPromptStore.ts` —— untracked（`??`，新建）✅
- `src/stores/__tests__/userPromptStore.test.ts` —— untracked（`??`，新建）✅
- `src/stores/index.ts` —— modified（`M`，追加 1 行 re-export）✅

**后端契约对齐**：
- `commands/user_prompt.rs::PromptInfo`（9 snake_case 字段，serde camelCase）↔ `src/types/user-prompt.ts::PromptInfo`（9 camelCase 字段）一对一
- `tauri-commands.ts:828` `saveUserPrompt(module, text)` 签名 ↔ `userPromptStore.ts:143` `cmd.saveUserPrompt(module, draft)` 调用对齐
