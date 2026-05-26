# Task 输入 — task_007_dev_frontend_ui

## 目标

新建 `src/components/settings/PromptCustomizationPanel.tsx`（4 个可折叠子项 + 文本编辑框 + 状态指示 + 保存/恢复按钮 + 字节计数 + 占位符提示）；并在 `SettingsPanel.tsx` 新增 Tab "Prompt 自定义"挂载它。

## 前置条件

- 依赖 task：`task_006_dev_frontend_store` 必须 DONE（提供 `useUserPromptStore`）
- 必须先存在的文件/接口：
  - `src/stores/userPromptStore.ts`
  - `src/types/user-prompt.ts`（`PROMPT_MODULES` / `PROMPT_MODULE_TITLES`）

## 验收标准（Acceptance Criteria）

1. **AC-1（PromptCustomizationPanel 主结构）** — 新建 `src/components/settings/PromptCustomizationPanel.tsx`：
   - 顶部：说明文案
     ```
     以下为系统内置的 AI 处理策略。
     修改后将影响对应功能的输出结果。
     如不确定，请保持默认值。
     ```
   - 中部：4 个折叠子项，顺序固定 = `PROMPT_MODULES`，标题用 `PROMPT_MODULE_TITLES[module]`；折叠状态由组件内部 `useState<Record<PromptModule, boolean>>` 管理，初始全部折叠
   - 底部：右对齐按钮"全部恢复默认"，点击弹 `window.confirm("将恢复全部 4 条 Prompt 为内置默认值，已有自定义会丢失。继续？")` 后调 `reset(null)`
   - 顶部加 `data-testid="prompt-customization-panel"`，每个子项加 `data-testid={"prompt-section-" + module}`
   - 挂载时调 `useUserPromptStore().loadAll()`（用 `useEffect`，仅在 mount 时）

2. **AC-2（单个 module 折叠子项）** — 每个子项展开后包含：
   - **状态行**：`● 已自定义 [恢复默认]` / `● 默认`（颜色用 `var(--accent-*)` / `var(--text-tertiary)`，与 NCdesktop 现有 UI token 一致）
   - **占位符提示行**：若 `item.requiredPlaceholders.length > 0`，显示 `"必含占位符：{content}"` 列出每个 required 占位符（chip 样式）
   - **textarea**：
     - `value = drafts[module]`
     - `onChange` 调 `setDraft(module, e.target.value)`
     - rows ≥ 14；字体等宽 `font-mono`
     - 占位符未满足时下方红字提示 `"缺少必含占位符：{content}（保存按钮已禁用）"`，对应 `data-testid={"placeholder-warning-" + module}`
   - **字节计数行**：右下角 `{n} / {maxBytes} 字节`，颜色：n < 80% maxBytes 灰；80-100% 橙；>100% 红
   - **按钮区**：右对齐
     - `[恢复默认]` 调 `reset(module)`；按钮 disabled 条件：`item.isCustom === false`
     - `[保存]` 调 `save(module)`；disabled 条件：① 占位符未满足 ② byteLen > maxBytes ③ `dirty[module] === false`

3. **AC-3（SettingsPanel.tsx 集成）** — 修改 `src/components/features/SettingsPanel.tsx`：
   - `TABS` 数组在 `"ai"` 之后插入 `{ id: "prompt", label: "Prompt 自定义", icon: <某个合适 lucide icon, 如 FileText> }`
   - 类型 `SettingsTab` 加 `"prompt"`
   - 渲染区追加 `{activeTab === "prompt" && <PromptCustomizationPanel />}`
   - 顶部 `import { PromptCustomizationPanel } from "../settings/PromptCustomizationPanel";`

4. **AC-4（错误显示）** — 任意 save / reset 失败时，显示 `error` 字段（来源 `userPromptStore.error`）在折叠子项下方红色横条；用户再次点击操作时清空（调用前 set error: null）

5. **AC-5（vitest 组件测试）** — 在 `src/components/settings/__tests__/PromptCustomizationPanel.test.tsx` 新建测试：
   - mock `userPromptStore`
   - 覆盖：① 加载时显示 4 个折叠条 ② 点击展开第一个 ③ 输入文本触发 setDraft + dirty=true ④ 缺占位符时 save 按钮 disabled ⑤ 点击保存调 save(module) ⑥ 点击单条"恢复默认"调 reset(module) ⑦ 点击底部"全部恢复"经 confirm 后调 reset(null) ⑧ 状态指示显示"已自定义" vs "默认"
   - 既有 `SettingsPanel.test.tsx` 中 `vi.mock("../../settings/PromptEditor", ...)` 与本 task **无关**，本 task 新增的 PromptCustomizationPanel **不被该 mock 覆盖**；SettingsPanel.test.tsx 因新增 Tab 可能需要补一行 `vi.mock("../../settings/PromptCustomizationPanel", () => ({ PromptCustomizationPanel: () => <div data-testid="prompt-custom-panel-mock" /> }));`（若 SettingsPanel.test.tsx 因新 Tab 导入失败时）

6. **AC-6（前端类型检查 + 测试全绿）** — `pnpm tsc --noEmit` 通过；`pnpm test` 全表 PASS

## 技术约束

- **代码规范**：
  - UI token 用 `var(--text-primary)` / `var(--surface-elevated)` / `var(--border-primary)` / `var(--accent-*)` 等 NCdesktop 现有 CSS 变量
  - lucide-react 图标（与 SettingsPanel.tsx 既有图标库一致）
  - **不引入新 UI 库**（如 `radix-ui` / `headless-ui`）
- **Architect 方案约束**：
  - 不复用 `src/components/settings/PromptEditor.tsx`（PR-4 半成品，详见 ADR-005 / R6）
  - 4 module 顺序固定 = `PROMPT_MODULES`（tagging → para → concept → aggregation）
  - PRD § 3.1 UI 草图作为参考，但用 NCdesktop 现有视觉风格实现

## 参考文件

**必读**：
- PRD § 3.1（UI 草图）
- Architect output.md `§ 7`（目录结构）
- Architect output.md `§ R4`（PRD 4 module ↔ 后端 3 调用链的 UI 文案责任）

**代码参考（必读）**：
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/components/features/SettingsPanel.tsx` — Tab 范式 + UI token 用法 + `SettingRow` / `ToggleSwitch` helper
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/components/features/__tests__/SettingsPanel.test.tsx` — 现有测试范式（mock 子组件）
- `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/components/settings/PromptEditor.tsx` — 不复用但可参考其 textarea + 按钮 + dryRun 状态显示模式

## 预估影响范围

- **新建文件**：
  - `src/components/settings/PromptCustomizationPanel.tsx`
  - `src/components/settings/__tests__/PromptCustomizationPanel.test.tsx`
- **修改文件**：
  - `src/components/features/SettingsPanel.tsx`（新增 Tab）
  - `src/components/features/__tests__/SettingsPanel.test.tsx`（如需要，补 mock）
- **预估变更**：~600 行（含测试 ~250 行）
