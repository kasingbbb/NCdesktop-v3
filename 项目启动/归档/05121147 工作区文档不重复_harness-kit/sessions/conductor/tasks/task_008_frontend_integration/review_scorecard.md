# Review Scorecard — task_008_frontend_integration

## 审查思考过程

1. **Task 意图**：前端工作区列表切到 `WorkspaceAssetView` DTO，列表行渲染 4 态徽章（中文）；`useDragAssets` 走 `prepare_outbound_payload`，4 种 OutboundError 落中文 toast；上下文菜单接入 `renameAsset`（ADR-007）；删除复用 task_006 级联 `deleteAsset`。
2. **AC 检查结果**：
   - AC-1 ✅ 采用 input.md 显式允许的「并存字段」路径（`Asset` 上挂 optional `state/stateReason/renditionPath/...`），`normalizeAsset` 透传新字段并回填 `tags/aiAnalysis/source` 默认值，下游零破坏（tsc 通过）。
   - AC-2 ✅ `AssetStateBadge` 4 态文案 + 图标 + failed 行重试按钮 → `retryAssetConversion(assetId)`。
   - AC-3 ✅ `useDragAssets` mousedown 阶段同步 kick off `invoke("prepare_outbound_payload")` 并存 ref；阈值跨过后 await + startDrag(item: entries.map(e.path), mode: copy)；失败走 `parseOutboundError` → `outboundErrorToToast` 覆盖 4 主变体 + 2 兜底（EmptyInput / AssetNotFound）。
   - AC-4 ✅ `AssetContextMenu` 新增「重命名」（仅单选可用，多选灰显 + 提示）调 `useAssetStore.renameAsset`；删除路径未变（task_006 级联 deleteAsset）。
   - AC-5 ✅ `assetStateLabel` 唯一中文映射函数；徽章组件内未出现 "已就绪" 等字面量；`title` 用 `失败原因：${reason}` 已通过函数取值。
   - AC-6 ✅ `AssetListView.test.tsx` 断言 4 个不同 `data-state` 在 row + badge 同时存在，failed 行有重试按钮且点击调 `retryAssetConversion('failed-asset-1')`，其它态 `queryByRole('button') === null`。`useDragAssets.test.tsx` mock invoke reject `stateNotDone` JSON → 断言 `startDragMock` 未调 + `useUIStore.notifications[0].title === "无法拖出"` + message 含 "非 done" / "converting"；附成功路径回归。
   - AC-7 ✅ `npx vitest run` task 范围 2 个测试文件、6 个测试全 PASS；`tsc --noEmit` 空输出。
3. **关键发现**：
   - (a) Dev 报告的 "全量 npm test 42 fail" 与 task_008 改动文件**零交集** —— 实测 `git ls-files -u` 显示未合并文件全部位于 `src/components/layout/*` 与 `src/styles/*.css`（Inspector/Sidebar/SidebarFooter/SidebarItem/Toolbar/glass.css/globals.css），task_008 的 6 个改动文件（types/asset.ts、stores/assetStore.ts、lib/asset-state.tsx、components/features/AssetListView.tsx、components/features/AssetContextMenu.tsx、hooks/useDragAssets.ts）均不在未合并清单中，亦未被预存在失败测试套件直接 import。结论：task_009 集成测试只需在 task_008 范围内执行，可信度未被污染；但 layout/styles 的合并冲突会阻塞「整页拖拽 + 侧栏交互」端到端验证，建议 Conductor 在 task_009 启动前先解决 layout/styles 合并冲突（与 task_008 PASS 判定独立）。
   - (b) `useDragAssets` 的 user gesture 时序设计正确：`onMouseDown` 同步阶段（事件处理首句）调 `invoke`，Promise 存 ref；mousemove 阈值时 await。由于 invoke 是 IPC 调用，在 mousedown→mousemove 之间的 ≥5px 移动期间大概率已 resolve，进入 `.then` 时 `startDrag` 仍在合理的 user gesture 上下文。Race 处理也 OK：`onMouseUp` 早于阈值跨过时 `pendingDragRef.current = null`，pending promise 已挂 catch 不泄漏，不会误启 startDrag。仍属于"真机时序需 task_009/010 验证"的范畴（Dev 已明示）。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | 7 条 AC 全部满足；6/6 测试通过；tsc 0 错误 |
| 用户体验 | 25% | 4 | 4 态徽章 + 重试按钮 + 中文 toast 完整；重命名走 `window.prompt`（与既有 delete 风格一致但与 WorkspaceFolder rename 内联编辑不一致，UX 评审范畴，task_010 跟进） |
| 架构一致性 | 20% | 5 | 全走 `tauri-commands.ts` wrapper；DTO 仅在 types/；未引入新依赖；`Asset` 字段扩展为 optional，旧 view 零破坏 |
| 代码质量 | 10% | 4 | hooks/lib 注释充分；`assetStateLabel` 单点真理；MINOR：`AssetListView.tsx` 第 569 行 `data-state={state ?? "unknown"}` 会产生测试断言外的 `"unknown"` 态（向后端兼容，可接受） |
| 测试覆盖 | 10% | 3 | StateNotDone + 成功路径 + 4 态徽章 + 重试 well-covered；**MAJOR 偏弱**：`MixedStates` / `RenditionMissing` / `IoFailed` 3 变体仅靠 TS switch 覆盖，无运行时单测 — Dev 自测矩阵把 MixedStates 标为「单测覆盖联合类型」实属夸大 |
| 可维护性 | 10% | 5 | `AssetStateBadge` 把状态徽章下沉为独立组件，未来扩展易；OutboundError → toast 表驱动，新加变体只需补一支 case |

**综合分：4.45/5**（加权：0.25×5 + 0.25×4 + 0.20×5 + 0.10×4 + 0.10×3 + 0.10×5 = 1.25 + 1.0 + 1.0 + 0.4 + 0.3 + 0.5 = 4.45）

## 总体判断

- [x] **PASS**

判定理由：无 BLOCKER；1 个 MAJOR（mixedStates/renditionMissing/ioFailed 运行时单测缺失，但 toast 表驱动且 4 个分支均为 pure 函数返回值，剩余变体在 task_009 集成 / task_010 真机更适合验证）；综合分 4.45/5 ≥ 3.5 阈值。

## 问题列表

### BLOCKER

无。

### MAJOR

1. **问题**：`useDragAssets.test.tsx` 仅覆盖 `stateNotDone` 一个失败变体；`mixedStates` / `renditionMissing` / `ioFailed` 3 个 OutboundError 变体仅靠 TypeScript switch 静态覆盖，无运行时单测。Dev 自测矩阵把这点写为「单测覆盖联合类型」存在夸大风险。
   - **代码位置**：`NCdesktop/src/hooks/useDragAssets.test.tsx`（仅 1 个失败用例）；`NCdesktop/src/hooks/useDragAssets.ts:43-69`（`outboundErrorToToast` 全部 case）
   - **修复方向**：在同文件加 3 个参数化用例：mock invoke reject 不同 OutboundError JSON → 断言 `useUIStore.notifications[0].message` 中文文案前缀。可直接抽 `outboundErrorToToast` 为可单测的纯函数（不依赖 hook 渲染），用 `describe.each` 一把覆盖。
   - **验证标准**：4 个 OutboundError 变体（StateNotDone / MixedStates / RenditionMissing / IoFailed）+ 2 个兜底（EmptyInput / AssetNotFound）各有至少 1 条断言其中文 toast 文案的运行时用例。
   - **可否随 task_009 一起补齐**：建议作为 task_009 集成测试的一部分顺手补齐（成本约 ~30 行），不在 task_008 内 FIX 阻塞。

### MINOR

1. `AssetListView.tsx:569` 行 `data-state={state ?? "unknown"}` 引入了未在 PRD 列出的第 5 态字面量 `"unknown"`（处理 `Asset` 上 optional state 缺省的兜底）。功能正确，但 PRD §S3 仅列 4 态。建议改为：仅当 `state` 存在时输出 row 的 `data-state`，缺省时不写该属性（同 badge 一致：缺省 state 时不渲染 badge）。属可选优化。
2. `AssetContextMenu.tsx:127` 重命名用 `window.prompt`，与 `WorkspaceFolderListView` 的内联编辑不一致。Dev 已在已知局限说明并标 task_010 跟进，可接受。
3. `assetStore.normalizeAsset` 把 `source` 兜底为 `{ type: "manual_import" }`：当 Inspector 详情面板渲染 source 时所有 WorkspaceAssetView 来源会被误标为 manual_import。Dev 已在「需要 Reviewer 关注」第 4 条主动揭示，需 Conductor 在后续 task 决定是否扩字段。本任务 PASS，归口跟进。

## 领域审查重点核对（session_context）

- **S3 三态可见 / 4 态 data-state**：✅ 4 个 `data-state` 通过 `AssetStateBadge` 自动产出；测试断言锁死 4 种且去重计数 = 4。
- **多选混合态拖拽整体禁用 + toast（PRD M11 P1）**：✅ task_008 已落 `MixedStates` toast 文案与 switch 分支，属 P1 提前实现，**加分**；运行时单测缺失见 MAJOR-1。
- **中文文案硬约束**：✅ task_008 6 个改动文件中无 "extracting"/"failed"/"done"/"converting"/"offline" 字符串作为可见文案（仅作为类型 / `data-state` / case 标签）。
- **不绕过 tauri-commands wrapper**：✅ 全部走 wrapper（`retryAssetConversion` / `prepareOutboundPayload` / `parseOutboundError` / `renameAsset`）；仅 `useDragAssets` 在「同一 user gesture 同步阶段必须 invoke」的关键时序处直接用 `@tauri-apps/api/core::invoke`，是合理的偏离，wrapper 提供了 `parseOutboundError` 配合。
- **其它视图未被破坏**（PhotoViewer / KnowledgeHubView / Timeline / Inspector）：✅ `Asset` 接口扩展全 optional + `normalizeAsset` 回填默认值，`tsc --noEmit` 0 错误。

## 给 Dev 的修复指引

无修复要求（PASS）。MAJOR-1 建议在 task_009 集成测试范围内顺手补齐 3 个 OutboundError 运行时用例。
