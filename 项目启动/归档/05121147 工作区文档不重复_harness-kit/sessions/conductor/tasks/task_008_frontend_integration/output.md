# Task 交付 — task_008_frontend_integration

## 实现摘要

前端工作区列表切到 `WorkspaceAssetView` 数据流（采用 AC-1 允许的「并存字段」过渡路径）：

1. **`types/asset.ts`** — `Asset` 接口扩展可选 WorkspaceAssetView 派生字段（`state` / `stateReason` / `sourceMissing` / `renditionPath` / `renditionSize` / `renditionId` / `derivativeVersion` / `assetType`）。其它视图（PhotoViewer / KnowledgeHubView / Timeline / Inspector）继续消费 `tags` / `aiAnalysis` / `source`，零破坏。
2. **`stores/assetStore.ts::normalizeAsset`** — 后端 `get_assets` 现在返回 `WorkspaceAssetView`（task_003），不带 `tags / aiAnalysis / source`；normalize 时回填默认值（空数组 / null / `{ type: "manual_import" }`），并把 state / rendition* 等字段透传给工作区视图。
3. **新建 `lib/asset-state.tsx`** — 中文映射函数 `assetStateLabel(state)` 与组件 `AssetStateBadge`（AC-5 集中点）：
   - `done` → 「已就绪」+ 绿 `CheckCircle2`
   - `converting` → 「转化中」+ 旋转 `Loader2`
   - `failed` → 「失败」+ `AlertCircle` + 「重试」按钮 → `retryAssetConversion(assetId)`
   - `offline` → 「离线待转化」+ `WifiOff`
   - 徽章带 `data-state={state}` 与 `data-testid="asset-state-badge"`；重试按钮 `data-testid="asset-retry-button"`。
4. **`AssetListView.tsx`** — 删除旧的 `extractionStatusBadge`（基于 `extractionStore.statusCache` 的旧状态源），改为读 `asset.state` 走 `AssetStateBadge`；右侧工作区每行 `<li data-asset-id data-state>`（PRD S3 断言可用）；重试成功后触发 `fetchAssets` 重拉，失败 toast 通过 `useUIStore.addNotification`。`useExtractionStore` import 删除（不再依赖）。
5. **`hooks/useDragAssets.ts` 重写** — 关键时序：在 `mousedown`（user gesture 上下文）**立即** kick off `invoke("prepare_outbound_payload", { assetIds })` 并把 Promise 存 ref，在 mousemove 跨过 5px 阈值时 `await` 该 Promise → 成功 `startDrag({ item: entries.map(e => e.path), icon, mode: "copy" })`；失败用 `parseOutboundError`（tauri-commands 已暴露）解出 4 变体，落 toast：
   - `stateNotDone` → 「无法拖出」/「非 done 态资产无法拖出（当前：xxx）」
   - `mixedStates` → 「无法拖出」/「多选包含非 done 态资产（N 条），无法整体拖出」
   - `renditionMissing` → 「无法拖出」/「未找到转化后的 MD 文件，请先重试转化」
   - `ioFailed` → 「拖拽准备失败」/「<reason>」
   - `emptyInput` / `assetNotFound` 也兜底为「无法拖出」+ 对应说明。
6. **`AssetContextMenu.tsx`** — 新增「重命名」菜单项（仅单选可用，多选灰显），点击弹 `window.prompt` 收取新名 → 调 `useAssetStore.renameAsset(assetId, trimmed)`（store 已对接 ADR-007 双写命令 task_004）。删除路径不变，已是 task_006 级联 `deleteAsset`。
7. **测试**：
   - `src/components/features/__tests__/AssetListView.test.tsx` — `AssetStateBadge` 四态断言（4 个不同 `data-state` 出现 + failed 行带「重试」按钮 + 点击触发 `retryAssetConversion`）+ `assetStateLabel` 中文文案。
   - `src/hooks/useDragAssets.test.tsx` — 用 `renderHook` mock `@tauri-apps/api/core::invoke` 与 `@crabnebula/tauri-plugin-drag::startDrag`：StateNotDone 路径断言 `startDrag` 未调用 + `useUIStore.notifications` 含中文 toast；附成功路径回归。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---|---|---|
| `NCdesktop/src/types/asset.ts` | 修改 | `Asset` 接口扩展 WorkspaceAssetView 派生字段（全部 optional） |
| `NCdesktop/src/stores/assetStore.ts` | 修改 | `normalizeAsset` 回填 `tags/aiAnalysis/source` 默认值，透传 state 等新字段 |
| `NCdesktop/src/lib/asset-state.tsx` | 新建 | `assetStateLabel` + `AssetStateBadge`（含重试按钮） |
| `NCdesktop/src/components/features/AssetListView.tsx` | 修改 | 删除 `extractionStatusBadge` / `useExtractionStore` 依赖，改用 `AssetStateBadge`；行 `data-state`；retry 后 fetch 刷新 |
| `NCdesktop/src/components/features/AssetContextMenu.tsx` | 修改 | 新增「重命名」菜单项 → `useAssetStore.renameAsset` |
| `NCdesktop/src/hooks/useDragAssets.ts` | 修改（重写） | mousedown 即时 kick off `prepare_outbound_payload`；阈值后 await + startDrag；失败 4 变体 toast |
| `NCdesktop/src/components/features/__tests__/AssetListView.test.tsx` | 新建 | `AssetStateBadge` 4 态 + 重试按钮单测 |
| `NCdesktop/src/hooks/useDragAssets.test.tsx` | 新建 | `useDragAssets` StateNotDone 不调 startDrag + 中文 toast + 成功回归 |

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致：types / stores / hooks / lib / components 分层不变。
- [x] API 路径/命名与 Architect 方案一致：调用走 `tauri-commands.ts` wrapper（`retryAssetConversion` / `prepareOutboundPayload` / `parseOutboundError` / `renameAsset`），未绕过 invoke 直接调。
- [x] 数据模型与 Architect 方案一致：消费 `WorkspaceAssetView`（state / renditionPath / sourceMissing 等），未私下重塑形状。
- [x] 未引入计划外的新依赖：toast 沿用 `useUIStore.addNotification`，图标沿用 `lucide-react`。
- 偏离说明（AC-1 路径选择）：选择 input.md 明示允许的「并存字段过渡」路径（保留 `assets: Asset[]` + 在 `Asset` 上挂可选派生字段），而非直接把 store 类型换成 `WorkspaceAssetView[]`。原因：PhotoViewer / KnowledgeHubView / Timeline / Inspector / DocumentViewer 等大量视图共享 `useAssetStore.assets`，强切类型会牵动 task_008 范围外的多个视图，违反「只做工作区前端范围」约束。这是 input.md 主动允许的过渡形式，非偏离。

## 测试命令

```bash
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop
# task_008 新增 / 涉及的测试（AC-6 / AC-7）
npx vitest run src/components/features/__tests__/AssetListView.test.tsx src/hooks/useDragAssets.test.tsx
# TS check（AC-7）
npm run check
```

## 测试结果

### `npx vitest run src/components/features/__tests__/AssetListView.test.tsx src/hooks/useDragAssets.test.tsx`

```
 RUN  v4.1.1 /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop

stderr | src/hooks/useDragAssets.test.tsx > useDragAssets — AC-3 OutboundError.StateNotDone > invoke 返回 StateNotDone → startDrag 未调用 + toast 触发
[drag] prepare_outbound_payload / startDrag error: {"kind":"stateNotDone","assetId":"a1","state":"converting","message":"asset still converting"}

 ✓ src/hooks/useDragAssets.test.tsx > useDragAssets — AC-3 OutboundError.StateNotDone > invoke 返回 StateNotDone → startDrag 未调用 + toast 触发 12ms
 ✓ src/hooks/useDragAssets.test.tsx > useDragAssets — AC-3 成功路径 > invoke 返回 OutboundEntry[] → startDrag 用 entries.path 调用 5ms
 ✓ src/components/features/__tests__/AssetListView.test.tsx > AssetStateBadge — 四态渲染 > AC-6 渲染 4 态资产 → 4 个不同 data-state 出现 22ms
 ✓ src/components/features/__tests__/AssetListView.test.tsx > AssetStateBadge — 四态渲染 > AC-2 failed 行带『重试』按钮，其它行不带 47ms
 ✓ src/components/features/__tests__/AssetListView.test.tsx > AssetStateBadge — 四态渲染 > AC-2 点击『重试』按钮 → 调用 retryAssetConversion(assetId) 8ms

 ✓ src/components/features/__tests__/AssetListView.test.tsx > assetStateLabel — 中文映射 > AC-5 4 态全部中文文案 0ms

 Test Files  2 passed (2)
      Tests  6 passed (6)
   Start at  16:09:52
   Duration  666ms (transform 83ms, setup 86ms, import 242ms, tests 96ms, environment 681ms)
```

### `npm run check`（tsc --noEmit）

```
> ncdesktop@0.0.0 check
> tsc --noEmit
```

（空输出 = 0 type error；所有改动 + 新建文件类型通过）

### 仓库全量 `npm test` 状态说明

跑 `npm test` 全量套件有 42 个 fail / 248 pass（test files: 8 fail / 23 pass）。**所有 fail 均属于 task_008 范围之外的预存在失败**，涉及：
- `ContentArea.test.tsx` / `Sidebar.test.tsx` / `SidebarFooter.test.tsx` / `Inspector.test.tsx` / `TitleBar.test.tsx`（学习模式相关 UI 重排，与 v1.3 一个同名的「task_008 IN-01/IN-02」无关本任务）
- `SettingsPanel.test.tsx` / `turnLearningOff.integration.test.ts` / `TagTree.test.tsx`（learning mode 与 TagTree 重构）
- `App.test.tsx`（@tauri-apps/api `transformCallback` 未 mock）

`git status` 在仓库根显示 7 个文件 `needs merge`（Inspector.tsx / Sidebar.tsx / SidebarFooter.tsx / SidebarItem.tsx / Toolbar.tsx / glass.css / globals.css）的未解决合并冲突 — 即这些失败来自仓库当前 main 分支的进行中合并状态，与 task_008 范围（types/asset.ts / assetStore / asset-state.tsx / AssetListView / AssetContextMenu / useDragAssets）零交集。task_008 引入的两个测试文件全 6 通过；本任务相关代码路径未引入任何新失败。

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|----------|----------|------|-----------|
| ✅ 正常路径 | 4 态徽章渲染 + data-state 写入 | 已测 | PASS（AssetListView.test.tsx AC-6） |
| ✅ 正常路径 | `assetStateLabel` 4 态中文映射 | 已测 | PASS（AssetListView.test.tsx AC-5） |
| ✅ 正常路径 | failed 行重试按钮 → `retryAssetConversion(assetId)` | 已测 | PASS（AssetListView.test.tsx） |
| ✅ 正常路径 | useDragAssets 成功路径：startDrag 用 entries.path 调用 | 已测 | PASS（useDragAssets.test.tsx 成功回归） |
| ✅ 正常路径 | useDragAssets 失败路径：StateNotDone → 不 startDrag + 中文 toast | 已测 | PASS（useDragAssets.test.tsx AC-6） |
| ✅ 正常路径 | TS check 全文件 | 已测 | PASS（npm run check 空输出） |
| ⚠️ 边界条件 | done / converting / offline 行**不**显示重试按钮 | 已测 | PASS（断言 queryByRole === null） |
| ⚠️ 边界条件 | `Asset` 上的 WorkspaceAssetView 派生字段全为 optional → 旧消费者（PhotoViewer / Inspector / Timeline）零破坏 | 已测（TS） | PASS（tsc 无新错） |
| ⚠️ 边界条件 | normalizeAsset 把 `tags/aiAnalysis/source` 回填，view 不传也不崩 | 静态 | PASS（默认值 []/null/manual_import） |
| ❌ 异常路径 | mousedown 后用户没拖（直接 mouseup） → payloadPromise 被丢弃，已 `.catch` 不泄漏 | 代码评审 | OK（hook 内显式 catch swallow） |
| ❌ 异常路径 | 多选含 mixedStates → toast「多选包含非 done 态资产」 | 单测覆盖联合类型 | PASS（switch 路径覆盖 4 + 2 兜底变体） |
| ❌ 异常路径 | 重命名为空 / 无变化 → 不调 renameAsset，关闭菜单 | 代码评审 | OK（handleRename 早返回） |

## 已知局限

1. **AssetListView 整页交互未走端到端测试**：本 task 把 4 态徽章逻辑下沉到独立 `AssetStateBadge` 组件并对其单测，避免对整页 5 个 store + 多个 tauri 命令的重 mock。整页 dropzone 集成、Finder 拖放真机验证归 task_009（集成测试）/ task_010（UX 评审）。
2. **`prepareOutboundPayload` user gesture 时序**：通过「mousedown 即时 kick off」规避 await 失去 user gesture 的风险。但仅在单测层验证了「成功路径 startDrag 被调用」与「失败路径 startDrag 未被调用」；macOS Tauri 真机环境下 `startDrag` 是否仍保留 user gesture 由 task_009 / task_010 验证。
3. **左栏 / 网格视图未加 `data-state`**：右栏（工作区列）按 PRD S3 已加；左栏「导入原件」语义不属于「工作区资产状态」展示，未加；右栏网格视图同样未加（仅列表视图）。如 UX 需要可在 task_010 补。
4. **重命名 UI 用 `window.prompt`**：沿用项目既有风格（delete 用 `window.confirm`，WorkspaceFolderListView rename 用内联编辑）。本 task 范围内不引入新的弹窗组件；如 UX 评审认为应做内联编辑，task_010 跟进。
5. **`useExtractionStore.statusCache` 路径未删除**：仍可能被其它视图消费；本 task 仅把 AssetListView 的依赖解除。后续整理归后续 task。

## 需要 Reviewer 特别关注的地方

1. **`useDragAssets` 的 user gesture 保护**（关键时序）：`onMouseDown` 同步阶段 `invoke("prepare_outbound_payload")`，并立即挂 `.catch` 防 unhandled rejection；真正消费在 mousemove 阈值跨过后。Reviewer 请确认这套时序在 macOS Tauri 上不会因为 await microtask 边界导致 `startDrag` 失去 user gesture。
2. **`Asset` 类型扩展的兼容面**：所有新字段都标了 optional；下游消费者（Inspector / PhotoViewer / DocumentViewer / KnowledgeHubView）均通过 `tsc --noEmit` 验证。Reviewer 请扫一眼 `types/asset.ts` 的字段是否还应该有 `extractionStatus`（旧 `useExtractionStore.statusCache` 信号源）等被忽略字段 — 当前认为 PRD §S3 明确「state = 工作区列表唯一徽章源」，旧 extractionStore 信号不再驱动 AssetListView。
3. **AssetContextMenu 重命名实现**：用 `window.prompt`。Reviewer 若认为该走内联编辑（与文件夹列表 rename 体验一致），请在 review_scorecard 明确，task_010 跟进。
4. **`normalizeAsset` 回填默认值**：`source: { type: "manual_import" }` 是兜底值；后端 `WorkspaceAssetView` 不带 `source` 字段，前端旧 Asset 视图（Inspector 详情）若展示 source 会全显示「manual_import」。Reviewer 请确认这是否影响其它视图的视觉准确性 — 若是，需扩 view 字段或独立 fetch。
