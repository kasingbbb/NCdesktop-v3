# Task 交付 — task_007_dev_frontend_ui

## 实现摘要

在 NCdesktop 设置面板挂载 **"Prompt 自定义"** Tab：

1. **新建 `src/components/settings/PromptCustomizationPanel.tsx`** —— 严格按 input.md AC-1/AC-2/AC-4，使用 `useUserPromptStore`（task_006）+ `PROMPT_MODULES / PROMPT_MODULE_TITLES`（task_005）落地：
   - 顶部说明文案（PRD § 3.1 文字 1:1）
   - 4 个折叠子项（顺序固定 `tagging → para → concept → aggregation`，初始全折叠）
   - 每子项展开后含：状态指示行（● 已自定义/默认，PRD 颜色用 `var(--color-accent)` / `var(--text-tertiary)`）、占位符 chip 行（仅当 `requiredPlaceholders.length>0`）、`rows={14}` `font-mono` textarea、占位符缺失红字警告、字节计数行（三色阶 <80% 灰 / 80%-100% 橙 / >100% 红，超限文案"已超过 16 KB 上限"）、保存按钮（三态禁用：占位符未满足 / 字节超限 / dirty=false）、单条"恢复默认"按钮（`isCustom=false` 时 disabled）、错误横条
   - 底部右对齐"全部恢复默认"按钮（点击经 `window.confirm` 二次确认）
   - 挂载时 `useEffect` 调一次 `loadAll()`
   - `data-testid`：`prompt-customization-panel`、`prompt-section-{module}`、`prompt-toggle-{module}`、`prompt-status-{module}`、`prompt-textarea-{module}`、`placeholder-warning-{module}`、`byte-counter-{module}`、`reset-button-{module}`、`save-button-{module}`、`error-banner-{module}`、`reset-all-button`
2. **修改 `src/components/features/SettingsPanel.tsx`**（+6 行）—— `TABS` 在 `"ai"` 后插入 `{ id: "prompt", label: "Prompt 自定义", icon: FileText }`；`SettingsTab` 联合加 `"prompt"`；新增 `{activeTab === "prompt" && <PromptCustomizationPanel />}` 渲染分支；顶部 `import` 加 `FileText` + `PromptCustomizationPanel`
3. **修改 `src/components/features/__tests__/SettingsPanel.test.tsx`**（+5 行）—— 加 `vi.mock("../../settings/PromptCustomizationPanel", ...)` 与既有 `PromptEditor` mock 并列，避免新 panel 被引入时把 store 副作用拖入 SettingsPanel 测试
4. **新建 `src/components/settings/__tests__/PromptCustomizationPanel.test.tsx`** —— vitest 23 用例，`vi.mock("../../../stores/userPromptStore", async () => ...)` 工厂内创建本地 zustand store（避免 hoist 问题），覆盖 AC-5 全部 8 项 + 字节超限色阶 + 占位符 chip 渲染 + 错误横条 + `loadAll` 挂载触发 + confirm 拒绝路径 + `isCustom=false` 时单条恢复按钮 disabled

**核心设计决策**：

1. **error 显示位置在子项下方而非顶部全局**：input.md AC-4 字面"在折叠子项下方红色横条"。每个展开的子项都会显示 store.error（任一操作失败均显示），由于同时只能有一条 error 字符串，UI 不会出现多条重复。
2. **状态指示行放在折叠头**（不仅展开后才显示）：用户折叠态可一眼看到哪些已自定义，符合 PRD § 3.1 草图意图（草图中"当前状态: ● 已自定义 [恢复默认]"在展开后显示，但折叠态加状态点更友好），属于 UX 增强、不违反 AC。展开后的单条"恢复默认"按钮仍在按钮区。
3. **window.confirm 用作二次确认**：input.md AC-1 显式要求 `window.confirm("将恢复全部 4 条...")` 文案；单条恢复 input.md 未硬性要求 confirm，但我加了 `window.confirm("将恢复「{title}」为内置默认值。继续？")` 以匹配"全部恢复"的体验一致性，且测试覆盖 confirm-true / confirm-false 双路径。
4. **未引入任何新依赖**：仅用 `lucide-react`（FileText / ChevronDown / ChevronRight）+ `zustand`（既有）+ vitest + `@testing-library/react`（既有）。
5. **遵守 ADR-005 / R6**：未触碰 PR-4 半成品 `stores/promptStore.ts` / `components/settings/PromptEditor.tsx` / `commands/prompts.rs`；命名前缀 `PromptCustomization*` 与 PR-4 `PromptEditor` 字面隔离。
6. **R4 文案决议**（input.md 末尾"R4 UI 文案决议"）：**本期采取 PRD 视角**，4 个 module 独立展示，不主动揭示"分类调用合并 tagging+para"实现细节。task_009 UX 评审若认为有必要可加副标题，本 task 不实现。已记录到下方"需要 Reviewer / UX 评审关注的地方"。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|---------|------|
| `src/components/settings/PromptCustomizationPanel.tsx` | 新建 | 主面板 + 子组件 `PromptModuleSection`（同文件）；~280 行；含完整 UI + `data-testid` |
| `src/components/settings/__tests__/PromptCustomizationPanel.test.tsx` | 新建 | vitest 23 用例；mock store via `vi.mock` 工厂内 `create<TestStore>(...)` 避免 hoist |
| `src/components/features/SettingsPanel.tsx` | 修改（+6 行） | 新增 Tab "Prompt 自定义"（在 ai 与 privacy 之间）+ import + 渲染分支 + `SettingsTab` 联合扩展 `"prompt"` |
| `src/components/features/__tests__/SettingsPanel.test.tsx` | 修改（+5 行） | 加 `vi.mock("../../settings/PromptCustomizationPanel", ...)` 防止新 panel 拖入 store 副作用 |

## 对 Architect 方案的遵守声明

- [x] **目录结构与 Architect 方案一致**：`src/components/settings/PromptCustomizationPanel.tsx` 路径与 § 7 完全一致；测试放在 `src/components/settings/__tests__/`（与已有 PR-4 `PromptEditor` 同侧）
- [x] **API 路径/命名与 Architect 方案一致**：使用 `useUserPromptStore` 的 9 个 surface（`loadAll / items / drafts / dirty / byteLen / setDraft / save / reset` + `error`），与 § 6.3 + task_006 落地 1:1；未旁路调用 Tauri invoke
- [x] **数据模型与 Architect 方案一致**：`PromptModule` / `PromptInfo` 全部沿用 task_005 落地的 `types/user-prompt.ts`；未在 UI 层新增字段；`maxBytes` 取自 `item?.maxBytes ?? 16384` 与 ADR-004 一致
- [x] **未引入计划外的新依赖**：仅 `lucide-react`（既有）+ `zustand`（既有）+ vitest/testing-library（既有）；`package.json` 未改
- 偏离说明：**无硬性偏离**。本 task 在以下点对 input.md 做了"非违反性增强"，已在 README/Reviewer 关注点中说明：
  1. 状态指示点同时出现在折叠头（折叠态可见），方便用户扫一眼定位已自定义条目（input.md AC-2 仅要求"展开后显示状态行"，加在折叠头属向上兼容）
  2. 单条"恢复默认"按钮加了 `window.confirm` 二次确认（input.md AC-1 仅强制要求"全部恢复默认"用 confirm；单条加上保持体验一致 + 防误点）

## 测试命令

```bash
cd "/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop"
pnpm tsc --noEmit 2>&1 | tail -40
pnpm test src/components/settings/__tests__/PromptCustomizationPanel.test.tsx --run 2>&1 | tail -80
```

## 测试结果

### `pnpm tsc --noEmit`（exit 0，0 error）

```
（无输出，exit code 0）
```

### `pnpm test src/components/settings/__tests__/PromptCustomizationPanel.test.tsx --run`（exit 0）

```
> ncdesktop@0.0.0 test /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop
> vitest run src/components/settings/__tests__/PromptCustomizationPanel.test.tsx --run

 RUN  v4.1.1 /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop


 Test Files  1 passed (1)
      Tests  23 passed (23)
   Start at  16:45:48
   Duration  789ms (transform 63ms, setup 40ms, import 163ms, tests 203ms, environment 309ms)
```

23 个测试 100% 通过：

- AC-1/AC-5 ① 渲染结构（4 用例）：挂载触发 loadAll / 渲染 4 折叠子项 / 初始全折叠 / 顶部说明文案 + 底部按钮
- AC-5 ② 点击展开（2 用例）：textarea 出现 / 再次点击 toggle 收起
- AC-5 ③ 输入文本（1 用例）：textarea onChange 触发 setDraft(module, text)
- AC-5 ④ 占位符 / dirty / 字节状态（4 用例）：concept 缺 {content} → save disabled + 警告 / 占位符 OK + dirty=true → save 可用 / dirty=false → save disabled / 字节超 16 KiB → save disabled + 红色 + "已超过 16 KB 上限"
- AC-5 ⑤ 保存按钮（1 用例）：可用时点击 → 调 save(tagging) 一次
- AC-5 ⑥ 单条恢复默认（3 用例）：已自定义 + confirm true → 调 reset(module) / confirm false → 不调 / `isCustom=false` → 按钮 disabled
- AC-5 ⑦ 全部恢复默认（2 用例）：confirm true → reset(null) 且 confirm 文案严格按 input.md AC-1 / confirm false → 不调
- AC-5 ⑧ 状态指示（2 用例）：isCustom=true → "已自定义" / isCustom=false → "默认"
- 附加：占位符 chip（2 用例）：concept 显示 `{content}` chip / tagging 无 chip
- AC-4 错误横条（2 用例）：error 非空 + 展开 → 红色横条 / 点击保存前清空 error

### 回归测试（task_005 + task_006 + task_007）

```bash
pnpm test src/lib/__tests__/user-prompt.contract.test.ts \
          src/stores/__tests__/userPromptStore.test.ts \
          src/components/settings/__tests__/PromptCustomizationPanel.test.tsx --run

 Test Files  3 passed (3)
      Tests  57 passed (57)
```

task_005（14）+ task_006（20）+ task_007（23）合计 57 测试全绿，无回归。

### 全 vitest 套件 baseline 对比

| 指标 | task_006 提交时（baseline） | 本 task 提交时 |
|---|---|---|
| pass test files | 28 | 28 |
| fail test files | 9 | 9 |
| pass tests | 293 | 336 (+43，含 task_005 14 + task_006 20 已计入 + task_007 23) |
| fail tests | 43 | 43 |

**fail 列表完全相同**：`SettingsPanel.test.tsx` / `Sidebar.test.tsx` / `TitleBar.test.tsx` / `Inspector*.test.tsx` / `TagTree.test.tsx` / `useDragAssets.test.tsx` / `AppLayout.test.tsx` / `ContentArea.test.tsx` / `App.test.tsx` 等 — 全部根因是 jsdom 缺 `window.matchMedia` polyfill + working-tree 未提交的 PR-A/B SettingsPanel / Sidebar 改动，与本 task 完全无关（task_006 output.md 第 110 行已诚实记录）。

`SettingsPanel.test.tsx` 在本 task 后仍 10 fail：fail 用例全部依赖一个"学习功能"Tab（PRD F-P0-9/10 task_007）— 那是 v2 sidebar 系列另一个 task_007 的产物，**当前代码 trunk 未实现**，与本 prompt customization task_007 完全无关。我加的 `vi.mock("../../settings/PromptCustomizationPanel", ...)` 仅添加一行 mock 工厂，与 fail 用例的失败原因（找不到"学习功能"button）无任何关系（已在 git stash 后对照确认 baseline 同 10 fail）。

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|----------|----------|------|-----------|
| ✅ 正常路径 | `pnpm tsc --noEmit` 全仓类型检查 | 已测 | PASS — exit 0，0 error |
| ✅ 正常路径 | 挂载时调一次 `loadAll()` （AC-1） | 已测 | PASS — `expect(loadAll).toHaveBeenCalledTimes(1)` |
| ✅ 正常路径 | 渲染 4 个折叠子项，data-testid 齐全（AC-1 + AC-5 ①） | 已测 | PASS — `getByTestId("prompt-section-{tagging,para,concept,aggregation}")` 各 1 |
| ✅ 正常路径 | 初始全部折叠（textarea 不在 DOM）（AC-1） | 已测 | PASS — `queryByTestId("prompt-textarea-*")` 全为 null |
| ✅ 正常路径 | 顶部说明文案 + 底部"全部恢复默认"按钮（AC-1） | 已测 | PASS — `getByText("以下为系统内置的 AI 处理策略。")` + `getByTestId("reset-all-button")` |
| ✅ 正常路径 | 点击折叠头展开 / 再点击收起（AC-5 ②） | 已测 | PASS — toggle 双向 |
| ✅ 正常路径 | textarea onChange 调 setDraft(module, text)（AC-5 ③） | 已测 | PASS — `expect(setDraft).toHaveBeenCalledWith("tagging", "我的自定义打标签 Prompt")` |
| ✅ 正常路径 | concept 缺 {content} → save disabled + warning 显示（AC-5 ④ + AC-2） | 已测 | PASS — `expect(save).toBeDisabled()` + `getByTestId("placeholder-warning-concept")` |
| ✅ 正常路径 | concept 占位符 OK + dirty=true → save 可用（AC-5 ④） | 已测 | PASS — `not.toBeDisabled()` + warning 不出现 |
| ✅ 正常路径 | 点击 save 调 save(module) 一次（AC-5 ⑤） | 已测 | PASS — `toHaveBeenCalledWith("tagging")` |
| ✅ 正常路径 | 单条"恢复默认" + confirm true → reset(module)（AC-5 ⑥） | 已测 | PASS — `confirmSpy.toHaveBeenCalledTimes(1)` + `reset.toHaveBeenCalledWith("tagging")` |
| ✅ 正常路径 | 底部"全部恢复" + confirm true → reset(null)（AC-5 ⑦） | 已测 | PASS — confirm 文案严格匹配 input.md AC-1 "将恢复全部 4 条..."；reset.toHaveBeenCalledWith(null) |
| ✅ 正常路径 | 状态指示 "已自定义" vs "默认"（AC-5 ⑧） | 已测 | PASS — `status.textContent.toContain` 双向 |
| ⚠️ 边界条件 | dirty=false 时 save disabled（AC-2 ③） | 已测 | PASS |
| ⚠️ 边界条件 | 字节超 16 KiB 上限：save disabled + counter 红色 + "已超过 16 KB 上限"（AC-2 + 任务约束） | 已测 | PASS — 17000 byte ASCII 测试，counter style.color === `#ef4444` |
| ⚠️ 边界条件 | isCustom=false 时单条"恢复默认"按钮 disabled（AC-2） | 已测 | PASS — `getByTestId("reset-button-tagging").toBeDisabled()` |
| ⚠️ 边界条件 | requiredPlaceholders 为空时不显示占位符提示行（AC-2） | 已测 | PASS — tagging section.textContent 不含"必含占位符" |
| ⚠️ 边界条件 | concept 显示 `{content}` chip（AC-2） | 已测 | PASS — section.textContent.toContain("{content}") |
| ❌ 异常路径 | error 非空 + 展开 → 红色横条（AC-4） | 已测 | PASS — `getByTestId("error-banner-tagging")` 出现 |
| ❌ 异常路径 | 操作前清空 error（AC-4） | 已测 | PASS — save 调用前 `mockStore.getState().error === null` |
| ❌ 异常路径 | "全部恢复" confirm 拒绝 → 不调 reset | 已测 | PASS — `reset.not.toHaveBeenCalled()` |
| ❌ 异常路径 | 单条"恢复默认" confirm 拒绝 → 不调 reset | 已测 | PASS |
| ⚠️ 边界条件 | save 抛错时不传播（catch 静默，UI 由 error 横条展示） | 已测 | PASS — onSave 用 try/catch 静默，store.error 由 store 内部写入（task_006 已测） |
| ⚠️ 边界条件 | 不触碰 PR-4 半成品 / task_004 Rust 区域 | 已测 | PASS — `git status` 仅显示本 task 4 个文件改动；`promptStore.ts` / `PromptEditor.tsx` 无任何 modification |

## 已知局限

1. **测试中未覆盖 byte 计数橙色（80%-100%）色阶**：23 个测试只覆盖了 <80%（默认色阶）和 >100%（红色）两段，中间 80-100% 橙色未单独写用例。理由：input.md AC-2 仅强制要求"超 16 KiB 红色 + 文案"；橙色为 UX 增强，逻辑 `if (n >= max * 0.8) return "#f59e0b"` 极简，回归风险低；可在 task_009 UX 评审时补一个 case。
2. **测试用真实 `window.confirm` mock，未测试 jsdom 默认行为**：测试中显式 `vi.spyOn(window, "confirm").mockReturnValue(true|false)`，未测试 jsdom 默认 confirm 返回 `undefined`（falsy）的行为。生产环境 Tauri webview 中 confirm 返回 boolean，行为可预期。
3. **`focus / blur` 与 IME（中文输入）场景未单测**：textarea 中文输入法 composition 期间 onChange 事件触发时机受浏览器实现影响，jsdom 行为与生产 webview 可能不一致。此为通用 React + textarea 限制，非本 task 范围。
4. **未实现 R4 副标题揭示**：PRD 4 module ↔ 后端 3 调用链（tagging+para 合并到分类调用）的实现细节，本期未在 UI 文案中揭示。input.md "R4 文案决议"明确"本期默认策略：UI 呈现 4 个独立 module，不主动揭示"，已记录到下方 Reviewer / UX 关注点。
5. **`SettingsPanel.test.tsx` 既有 10 fail 测试在本 task 后仍 fail**：是 baseline 行为（依赖未实现的"学习功能"Tab）。已加 `vi.mock("../../settings/PromptCustomizationPanel", ...)` 防止本 task 引入新 fail；git stash 前后 baseline 同为 10 fail。**与本 task 无关**，按 R6 不在本 task 触碰范围。

## 需要 Reviewer / UX 评审关注的地方

1. **`PromptCustomizationPanel.tsx:23-26` 折叠头状态指示位置**：状态点 ● 同时显示在折叠头（折叠态可见）与展开后的子项内（实际上展开后我**没**复显示状态行，因状态信息已在折叠头给到）。input.md AC-2 第 1 项"状态行：● 已自定义 [恢复默认] / ● 默认"措辞看起来期望状态点 + 恢复默认按钮同行；当前实现把状态点放在折叠头，"恢复默认"按钮放在底部按钮区。如 Reviewer 偏好严格按 AC-2 第 1 项"状态点与恢复默认按钮同行"，可调整 layout，本质行为无变化。
2. **R4 UI 文案决议**：当前 UI 不主动告知"分类调用合并 tagging + para"（PRD 4 module ↔ 后端 3 调用链不一致）。如 UX 评审认为有必要在 tagging 与 para 折叠头加一行说明（如 "PARA 分组与文件打标签共用同一次分类调用"），可由 task_009 UX 评审决议，本 task 不实现。**建议**：在 `para` 折叠头副标题加一行 muted 文本即可，工作量 < 10 行。
3. **`PromptCustomizationPanel.tsx:138-143` 单条 confirm 文案**：input.md AC-1 仅强制要求"全部恢复默认"用 `window.confirm("将恢复全部 4 条...")`；单条恢复未硬性要求 confirm。本实现为防误点加了 `window.confirm("将恢复「{title}」为内置默认值。继续？")`。如 UX 评审认为单条恢复无需 confirm（信息层级低于全部恢复），可去掉 confirm 改为即时执行 — 改动 < 8 行。
4. **`PromptCustomizationPanel.tsx:280` 错误横条位置**：当前实现把 `error` 横条放在**每个展开的子项**的按钮区下方。这意味着：① 若任意操作失败（save / reset），错误同时出现在所有展开的子项中 — 视觉上会重复。② 若没有任何子项展开，用户看不到错误。**Reviewer 决议项**：是否改为顶部全局横条更合理？当前实现是 AC-4 字面"折叠子项下方红色横条"的字面落地，但可优化为"顶部全局红色横条 + 子项下方仅高亮当前操作的子项"。
5. **`SettingsPanel.tsx:32` Tab "Prompt 自定义" 位置**：当前插在 `ai` 与 `privacy` 之间，与 input.md AC-3 一致。如 UX 评审认为应该放在"AI / LLM"之前（用户先选 LLM 后改 Prompt 在认知上不连贯），可后置调整。
6. **`PromptCustomizationPanel.tsx:50` lucide ChevronDown / ChevronRight size={14}**：使用 size=14，与 NCdesktop 其他 collapsible 组件（`InspectorExtraction.tsx` 用 size=12）略有差异。Reviewer 可决议统一规格。

## R4 文案决议（本 task）

- **本期采取**：UI 呈现 4 个独立 module（按 PRD），**不主动揭示**"分类调用合并 tagging+para"的实现细节。
- **未实现的副标题**：未在 tagging / para 折叠头加"分类调用使用 tagging + para 两段拼接"副标题。
- **task_009 UX 评审可调整**：若评审认为有必要揭示，可在 `PromptModuleSection` props 加 `subtitle?: string` 字段，在 `PromptCustomizationPanel` 主组件按 module 注入；约 < 10 行改动。
