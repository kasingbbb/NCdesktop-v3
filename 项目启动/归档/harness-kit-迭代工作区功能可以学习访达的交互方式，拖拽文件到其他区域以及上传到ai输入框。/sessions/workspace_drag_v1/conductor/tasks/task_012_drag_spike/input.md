# Task 输入 — task_012_drag_spike

## 目标

执行 v2.1 PRD §F-08 的 Spike：验证 Tauri webview 中"延迟 startDrag"方案的三个通过条件。产出 Spike 报告，**不实施正式落点逻辑**。

## 前置条件

- P0 已 SHIPPED（无依赖代码）
- 阅读 PRD §F-08 + Debate Layer 1（`debate/session_001/`）

## Spike 通过条件（PRD 已定义）

1. Tauri webview 中 Web DnD 的窗口级 `dragleave` 是否可靠触发？
2. `WorkspaceFolderStrip` 横条作为 Web DnD drop target 是否正常工作？
3. `startDrag` 在 app 内落下时 `DropzoneApp` 是否**不被**误触发？

## 验收标准

1. **AC-1**：临时在 `src/main.tsx` 或新建 `src/devtools/dragSpike.ts` 注入 instrumentation（`window.addEventListener('dragstart' | 'dragleave' | 'drop', console.log)`），覆盖三条件的事件路径。
2. **AC-2**：手动用例（在 dev build 中）覆盖：
   - a) 从右栏卡片拖出窗口外 → 观察 dragleave 是否触发；
   - b) 从右栏卡片拖到 `WorkspaceFolderStrip` 某个横条上 → 观察 drop target 是否命中；
   - c) 用 `startDrag` 拖到 app 内 `DropzoneApp` 区域 → 观察是否误触发上传。
3. **AC-3**：产出 `sessions/workspace_drag_v1/conductor/tasks/task_012_drag_spike/spike_report.md`，对三条件每条给出"通过/失败 + 证据 + 截图/log"。
4. **AC-4**：报告结尾给出决策建议：
   - 三条件全通过 → 立项 task_013 正式实现；
   - 任一失败 → 维持现有右键菜单，本 Spike 关闭，**不提交 instrumentation 代码到主干**（仅保留报告）。
5. **AC-5**：清理：Spike 结束后从 dev 入口移除 instrumentation（git diff 应只剩 spike_report.md）。

## 技术约束

- **不修改** `useDragAssets.ts` 与 `startDrag` 现有路径。
- **不修改** `DropzoneApp` / `WorkspaceFolderStrip` 业务逻辑（只能挂只读监听器）。
- 探针挂载位置统一在 dev-only 入口，release build 应自动剥离（`if (import.meta.env.DEV)` 守卫）。

## 参考文件

- `src/hooks/useDragAssets.ts`（startDrag 调用点）
- `src/components/features/WorkspaceFolderStrip.tsx`
- `src/components/features/dropzone/`（DropzoneApp）
- v2.1 PRD §F-08

## 预估影响范围

- 临时新增：探针文件 ~30 行（Spike 后删除）
- 持久产出：`spike_report.md`
