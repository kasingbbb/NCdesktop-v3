# Review Scorecard — task_007_dev_frontend_ui

## 审查思考过程

### 1. Task 意图

为 NCdesktop 「设置面板」新增 **"Prompt 自定义"** Tab，挂载 `PromptCustomizationPanel`：4 个折叠子项（tagging / para / concept / aggregation）+ textarea + 状态指示 + 占位符提示 + 字节计数（三色阶）+ 单条「恢复默认」+ 底部「全部恢复默认」+ 错误横条 + mount 时调一次 `useUserPromptStore.loadAll()`。复用 task_006 落地的 zustand store；严格隔离 PR-4 半成品（ADR-005 / R6）；非技术用户友好（session_context § 6）。

### 2. AC 检查结果

- **AC-1**（主结构 / 顶部说明 / 4 折叠 / 底部按钮 / `loadAll` 挂载触发 / data-testid）✅ — 顶部 3 行说明文案、4 折叠（顺序 `PROMPT_MODULES`、初始全折叠）、`useEffect` 一次性 `loadAll`、底部「全部恢复默认」按钮 + confirm 文案完全匹配 input.md 字面。但顶部说明文案 **第三行用半角逗号** "如不确定**,**请保持默认值。"（PRD/input.md 均为全角 **"，"**），MINOR 文案瑕疵，不阻断功能。
- **AC-2**（单 module 行为：状态行 / 占位符 chip / textarea / 警告 / 字节计数 / 按钮）✅ — 状态点 ● 放在折叠头（output.md 已声明属"非违反性增强"，AC-2 措辞"状态行：● 已自定义 [恢复默认]"未硬性要求二者必同行，但状态点+「恢复默认」按钮被拆到两处。功能 / 数据流均正确，标记为 MINOR 关注项。
- **AC-3**（SettingsPanel.tsx Tab 集成）✅ — `TABS` 在 `"ai"` 后追加 `{ id: "prompt", label: "Prompt 自定义", icon: FileText }`；类型联合扩展；渲染分支正确。
- **AC-4**（错误显示）✅ — 错误横条在每个展开子项下方红色；保存 / 恢复操作前 `useUserPromptStore.setState({ error: null })` 清空；测试用例 `AC-4 错误横条` 双断言通过。位置在子项下方而非顶部全局，作者已在 output.md 标注为"Reviewer 决议项"。AC-4 字面落地，标记 MINOR 关注。
- **AC-5**（vitest 23 测试覆盖 8 项 + 附加 chip / 错误 / loadAll / confirm 双路径）✅
- **AC-6**（`pnpm tsc --noEmit` exit 0 + 单文件 test 全绿 23/23）✅ — 实跑验证通过。

### 3. 关键发现

1. **三层隔离严守**：`git status` 显示 `promptStore.ts` / `PromptEditor.tsx` 0 改动；`commands/prompts.rs` 不在改动列表（PR-4 半成品完全保留）。`PromptCustomization*` 命名与 `PromptEditor` 字面区隔，ADR-005 / R6 严格遵守。
2. **顶部说明第三行半角逗号** (PromptCustomizationPanel.tsx:121)：PRD / input.md 均要求全角 `，`。MINOR，文案精确度问题，测试未捕获（断言只覆盖前两行）。
3. **错误横条位置与状态点位置** 是作者已识别并主动标注给 Reviewer 决议的项。错误横条采用 AC-4 字面"折叠子项下方"，状态点放折叠头是 UX 增强。属设计选择，不阻断 PASS。

---

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | AC-1~AC-6 全满足；23/23 测试通过；保存按钮三态禁用、字节三色阶、占位符警告、confirm 两路径全部到位。文案瑕疵不影响功能。 |
| 安全性 | 20% | 4 | 命名隔离严守 ADR-005 / R6；操作前清空 error 防误显；window.confirm 二次确认（含单条）防误点。轻微扣分：confirm 在 Tauri webview 是 native dialog，但仍依赖 webview 实现，未做 fallback，但本期 PRD 范围未要求。 |
| 代码质量 | 15% | 5 | 单文件 399 行结构清晰：分 `PromptCustomizationPanel`（主）+ `PromptModuleSection`（子）+ 2 个 helper（`checkPlaceholdersOk` / `byteColor`）；细粒度 selector（每个字段单独 hook）；JSDoc 头部完整；CSS token 一致；命名清晰。 |
| 测试覆盖 | 10% | 5 | 23 用例覆盖 AC-5 全 8 项 + 字节超限 + 占位符 chip + 错误横条 + loadAll mount 触发 + confirm 双路径 + isCustom=false 单条 disabled。mock 策略合理（vi.mock 工厂内创建真实 zustand store）。已知未测 80%-100% 橙色色阶，逻辑极简，回归风险低。 |
| 架构一致性 | 10% | 5 | 目录结构与 Architect § 7 1:1；data-testid 完整；使用 task_006 store 的 9 个 surface 与 § 6.3 严格对齐；CSS token 用 NCdesktop 现有变量；未引入新依赖。 |
| 可维护性 | 20% | 5 | JSDoc 解释每个设计决策（含 ADR-005 / R6 隔离声明）；inline 注释标注 AC 编号；output.md 主动揭示 6 项 Reviewer 关注点，便于交接；变量命名清晰；helper 函数职责单一。 |

**综合分：4.80/5**（加权 0.25×5 + 0.20×4 + 0.15×5 + 0.10×5 + 0.10×5 + 0.20×5）

---

## 总体判断

- [x] **PASS**

无 BLOCKER，无 MAJOR，仅 3 项 MINOR（文案瑕疵 + 设计选择项），综合分远超 3.5/5 阈值。

---

## 问题列表

### BLOCKER（必须修复，否则不可能 PASS）
无。

### MAJOR（强烈建议修复）
无。

### MINOR（可选修复）

1. **顶部说明文案第三行使用半角逗号**：
   - **代码位置**：`/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/components/settings/PromptCustomizationPanel.tsx:121`
   - **现状**：`<p>如不确定,请保持默认值。</p>`（半角 `,`）
   - **期望**：`<p>如不确定，请保持默认值。</p>`（全角 `，`，与 PRD / input.md 一致）
   - **建议**：1 字符替换。同时建议把第三行加入测试断言以防回归。
   - **影响**：纯文案精确度，对非技术用户无感知；不影响 PASS。

2. **状态点 ● 在折叠头，单条「恢复默认」按钮在展开后的按钮区**：
   - **代码位置**：`PromptCustomizationPanel.tsx:256-266`（状态点）/ `:347-362`（恢复默认按钮）
   - **现状**：状态点 ● 已自定义 / ● 默认 在折叠头（即便折叠态也可见）；「恢复默认」按钮仅在展开后的按钮区。
   - **AC-2 文字**："状态行：● 已自定义 [恢复默认] / ● 默认"暗示状态点与按钮同行。
   - **作者声明**：output.md "需要 Reviewer 关注点 1" 已主动揭示。理由：折叠态可一眼定位已自定义条目，提升 UX。
   - **建议**：保留当前实现（UX 增强 + 测试覆盖到位）；若 task_009 UX 评审认为需严格 AC 字面，可在二轮微调。
   - **影响**：UX 选择，不阻断功能。

3. **错误横条放在每个展开子项下方而非顶部全局**：
   - **代码位置**：`PromptCustomizationPanel.tsx:381-394`
   - **现状**：任意操作失败时，每个**展开**的子项都会显示同一份 `store.error`；若无任何子项展开，用户看不到错误。
   - **AC-4 文字**：字面要求"在折叠子项下方红色横条"，当前实现 100% 字面落地。
   - **作者声明**：output.md "需要 Reviewer 关注点 4" 已主动揭示并提出"顶部全局横条 + 子项下方仅高亮当前操作子项"备选方案。
   - **建议**：保留当前实现（AC-4 字面）；若 task_009 UX 评审认为顶部全局横条更合理，可在二轮微调。考虑到只能同时显示一条 store.error，重复显示成本可接受。
   - **影响**：UX 选择，不阻断功能。

---

## R4 文案决议核查

**作者声明**：output.md 末尾"R4 文案决议"明确"本期采取 PRD 视角，UI 呈现 4 个独立 module，**不主动揭示**"分类调用合并 tagging+para"的实现细节"，并标记 "task_009 UX 评审可调整"。

**Reviewer 复核**：与 Architect output.md § R4 一致，符合 PRD 视角；副标题预留扩展点（约 < 10 行）已在 output.md 注明。**通过**。

---

## 给 Dev 的修复指引

**不适用**（PASS 状态，无强制修复项）。

若 Conductor 决定在 task_009 UX 评审前先修复 MINOR-1（半角逗号），改动范围：
- `PromptCustomizationPanel.tsx:121` 单字符替换
- `PromptCustomizationPanel.test.tsx` 顶部说明文案断言新增第三行（约 +1 行）

预计 < 3 行变更，2 分钟内完成。

---

## Reviewer 自检清单

- [x] 逐条检查 AC-1~AC-6（全 ✅）
- [x] 领域审查重点：非技术用户友好 / 错误提示清晰度（confirm 二次确认、错误横条、占位符警告中文文案、字节计数三色阶 — 全到位）
- [x] R6 / ADR-005 合规验证（`git status` 实跑确认 `promptStore.ts` / `PromptEditor.tsx` / `commands/prompts.rs` 0 改动）
- [x] `pnpm tsc --noEmit` exit 0 实跑
- [x] `pnpm test src/components/settings/__tests__/PromptCustomizationPanel.test.tsx --run` 23/23 实跑
- [x] task_004 Rust 区域未触碰（git diff 范围核查）
- [x] R4 文案决议核查
- [x] MINOR 给出具体文件:行号 + 修复建议（即便非强制）
- [x] 评分诚实（未因"还不错"就给满 5/5：安全性 4/5 反映 confirm 未做 fallback 的轻微保留）
