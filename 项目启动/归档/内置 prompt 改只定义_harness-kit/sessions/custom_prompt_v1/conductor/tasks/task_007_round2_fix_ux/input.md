# Task 输入 — task_007_round2_fix_ux

## 目标

按 task_009 UX 评审产出的 "可选 fix list（< 50 行改动）"，对 PromptCustomizationPanel + userPromptStore 做最小范围的 UX 修复，修掉 3 项 MAJOR、落地 R4 方案 B、顺手清理若干 MINOR。

## 前置条件

- 依赖 task：`task_007_dev_frontend_ui`（已 PASS 4.80/5）+ `task_009_ux_review`（已 PASS）
- 必须先存在的文件：
  - `src/components/settings/PromptCustomizationPanel.tsx`（task_007 产物）
  - `src/components/settings/__tests__/PromptCustomizationPanel.test.tsx`（task_007 产物）
  - `src/stores/userPromptStore.ts`（task_006 产物）
  - `src/stores/__tests__/userPromptStore.test.ts`（task_006 产物）

## 验收标准（Acceptance Criteria）

来源：`tasks/task_009_ux_review/output.md` § 可选 fix list（10 项）。

### MAJOR（必修，3 项）

1. **AC-1（saving spinner — fix #2 / 对应 task_009 MAJOR-3）**
   - **位置**：`PromptCustomizationPanel.tsx:138-145`（PromptItem 组件内）
   - **改造**：在每个 PromptItem 组件内引入 `const [saving, setSaving] = useState(false)`；`handleSave` 包裹 `setSaving(true)` … `finally setSaving(false)`；保存按钮在 saving=true 时显示 spinner（用既有图标库的 Loader2 + spin 动画或 CSS）+ 禁用
   - **验证**：vitest 加 1 个测试：mock store.save 返回延迟 Promise，断言点击保存后按钮显示 spinner 文案并 disabled，resolve 后恢复

2. **AC-2（无障碍属性 aria-disabled / aria-label — fix #3 + #4 / 对应 task_009 MAJOR-2）**
   - **位置**：`PromptCustomizationPanel.tsx:363-378`（保存按钮）+ `:301-314`（textarea）
   - **改造**：
     - 保存按钮追加 `aria-disabled={saveDisabled}` + `title={saveDisabled ? "无未保存修改" : undefined}`（实际禁用原因按代码逻辑写）
     - textarea 追加 `aria-label={`${title} 的 Prompt 编辑区`}`
   - **验证**：vitest 加 1 个测试：渲染后 `screen.getByRole('button', { name: /保存/i })` 有 `aria-disabled` 属性；textarea 用 `getByLabelText('文件打标签 的 Prompt 编辑区')` 可定位

3. **AC-3（错误横条去重 — fix #5 + #10 / 对应 task_009 MAJOR-1）**
   - **位置**：`userPromptStore.ts:99-104`（error 字段类型）+ `PromptCustomizationPanel.tsx:382-394`（错误横条渲染）
   - **改造**：
     - **store**：`error: string | null` → `error: { module: PromptModule | null, message: string } | null`；`save(module)` 失败时写入 `{ module, message }`，`loadAll()` / `resetAll()` 失败时 `module: null`（全局错误）
     - **UI**：错误横条仅在 `error.module === module || error.module === null` 时渲染；全局错误（null）保留在顶部
   - **验证**：vitest 改既有"错误横条显示"测试为：`save(tagging)` 失败时仅 tagging 子项显示错误；`save(concept)` 失败时仅 concept 显示

### R4 方案 B（必落地，1 项）

4. **AC-4（para / tagging 折叠头副标题 — fix #6）**
   - **位置**：`PromptCustomizationPanel.tsx:228-267`（PromptItem 折叠头部分）
   - **改造**：PromptItem props 追加可选 `subtitle?: string`；折叠头 title 下方渲染一行 muted 副标题（如有）
   - **落地副标题文案**（来自 task_009 R4 方案 B）：
     - tagging：`与「PARA 分组」共用同一次分类调用，两者同时生效`
     - para：`与「文件打标签」共用同一次分类调用，两者同时生效`
     - concept / aggregation：无副标题
   - **验证**：vitest 加 1 个测试：渲染后 tagging 与 para 子项都能查到对应文案；concept / aggregation 没有副标题

### MINOR（顺手修，5 项；如时间紧可降级仅做 #1 #7）

5. **AC-5（全角逗号 — fix #1 / MINOR-1）**：`PromptCustomizationPanel.tsx:121` 的 `,` 改成 `，`
6. **AC-6（字节超限警告独立行 — fix #7 / MINOR-3）**：超过 16 KiB 时 "已超过 16 KB 上限" 文案独立一行 + ⚠ 图标（lucide AlertTriangle 14px）
7. **AC-7（chevron size 一致 — fix #8 / MINOR-5）**：`<ChevronDown size={14}>` 改 `size={12}`（与 NCdesktop 其他折叠组件一致；如确认其他位置就是 14 则保持 14）
8. **AC-8（dirty 守卫 — fix #9 / MINOR-6）**：`SettingsPanel.tsx:53` onClose / `:89` setActiveTab 处加 dirty 守卫：`if (Object.values(useUserPromptStore.getState().drafts/dirty).some(Boolean)) confirm("有未保存修改，确定离开？")`
9. **AC-9（必跑测试）**：
   - `pnpm tsc --noEmit` 0 error
   - `pnpm test src/components/settings/__tests__/PromptCustomizationPanel.test.tsx src/stores/__tests__/userPromptStore.test.ts --run` 全绿
   - 不引入 vitest 全套件新的 fail（baseline 与 task_006/007 一致）

## 技术约束

- **代码改动总量目标**：~40-45 行，**不超过 80 行**（超过则降级为路径 B：跳过本 task 直接进 task_010）
- **不修改后端**：所有 fix 仅在前端 TS/TSX
- **不修改 PR-4 半成品** `stores/promptStore.ts` / `components/settings/PromptEditor.tsx` / `commands/prompts.rs`（R6/ADR-005）
- **不修改 task_002~008 已 PASS 的产物的接口签名**：`PromptInfo` / Tauri command 签名 / 4 module 字面量等保持不变
- **store error 字段类型变更属于内部 surface 变更**：会被消费端（PromptCustomizationPanel）跟随更新；这是允许的内部重构
- **不引入新依赖**：spinner 用 lucide-react 的 `Loader2`（确认 NCdesktop 已用 lucide-react）
- **文案中文，温和**
- **不修改 progress.md**

## 参考文件

**必读**：
- task_009 output.md 末尾 fix list（验收映射的真相来源）：`sessions/custom_prompt_v1/conductor/tasks/task_009_ux_review/output.md`（行 299-318）
- handoff_contracts § 3：`core/handoff_contracts.md`
- Dev 系统提示词：`roles/conductor/dev/prompt.md`

**代码参考**：
- 待改文件：
  - `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/components/settings/PromptCustomizationPanel.tsx`
  - `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/components/settings/__tests__/PromptCustomizationPanel.test.tsx`
  - `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/stores/userPromptStore.ts`
  - `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/stores/__tests__/userPromptStore.test.ts`
  - `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src/components/features/SettingsPanel.tsx`
- spinner 图标参考：grep `Loader2` / `lucide-react` 看既有用法

## 预估影响范围

- **修改文件**：
  - `src/components/settings/PromptCustomizationPanel.tsx`（主战场，~25-30 行）
  - `src/components/settings/__tests__/PromptCustomizationPanel.test.tsx`（+3~4 测试）
  - `src/stores/userPromptStore.ts`（~5-10 行：error 字段类型 + save/loadAll/resetAll 写入路径）
  - `src/stores/__tests__/userPromptStore.test.ts`（既有"error 透传"测试改造）
  - `src/components/features/SettingsPanel.tsx`（+5 行 dirty 守卫，可选）
- **新建文件**：无
- **预估变更**：40-45 行（含测试 ~15 行）

## 工作量自检阀门（重要）

实现前计划协议结束后，先估算总变更行数：
- 若 ≤ 80 行：继续执行
- 若 > 80 行：**ESCALATE 给 Conductor**，输出"总量预估"+"建议降级 / 拆分方案"，等待指示
