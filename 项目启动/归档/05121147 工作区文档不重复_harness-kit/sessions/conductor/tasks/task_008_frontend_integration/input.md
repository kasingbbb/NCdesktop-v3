# Task 输入 — task_008_frontend_integration

## 目标
前端切到 `WorkspaceAssetView` DTO：AssetListView 显示四态徽章（中文）、非 done / 混合态禁用拖拽、失败行加重试按钮、useDragAssets 走 `prepare_outbound_payload`。

## 前置条件
- 依赖 task：task_003（DTO + get_assets 切流）、task_004（renameAsset）、task_005（prepareOutboundPayload）、task_006（retryAssetConversion + delete 级联）
- 必须先存在的文件/接口：
  - `src/types/workspaceAsset.ts`（task_003）
  - `src/lib/tauri-commands.ts` 中的 `renameAsset` / `prepareOutboundPayload` / `retryAssetConversion`（task_004/005/006）

## 验收标准（AC）
1. **AC-1**：`src/stores/assetStore.ts` 把 `assets: Asset[]` 改为 `assets: WorkspaceAssetView[]`（或加并存字段，过渡期保留兼容）。所有读取 `asset.filePath / asset.name / asset.type` 的位置改为消费 WorkspaceAssetView 同名字段。
2. **AC-2**：`AssetListView.tsx` 替换 `extractionStatusBadge` 为 `assetStateBadge(state)`，4 态文案：
   - `done` → 「已就绪」 + 绿色 CheckCircle2
   - `converting` → 「转化中」 + 旋转 Loader2
   - `failed` → 「失败」 + AlertCircle + 「重试」按钮（调 `retryAssetConversion(assetId)`）
   - `offline` → 「离线待转化」 + WifiOff 图标（lucide 已有）
3. **AC-3**：`useDragAssets.ts` 改造：
   - mousedown 阈值越过后，先 `await invoke('prepare_outbound_payload', { assetIds })`
   - 成功 → `startDrag({ item: entries.map(e => e.path), icon })`
   - 失败 → 解析 `OutboundError` JSON：
     - `StateNotDone` → toast「非 done 态资产无法拖出（当前：xxx）」
     - `MixedStates` → toast「多选包含非 done 态资产（N 条），无法整体拖出」
     - `RenditionMissing` → toast「未找到转化后的 MD 文件，请先重试转化」
     - `IoFailed` → toast「拖拽准备失败：<reason>」
4. **AC-4**：`AssetContextMenu.tsx` 重命名菜单项调用 `renameAsset(assetId, newName)` 而非旧的 `updateAsset(asset)`。删除调用现有 `deleteAsset(assetId)`（task_006 已升级为级联）。
5. **AC-5**：所有可见文案中文（不出现 "extracting" / "failed" 等英文枚举值）；状态枚举到文案的映射集中在一个工具函数 `assetStateLabel(state: AssetState): string`。
6. **AC-6**：单测：
   - `AssetListView.test.tsx`：渲染 4 态资产 → 断言 4 个不同 `data-state` 出现；failed 行有"重试"按钮
   - `useDragAssets.test.ts`（如不存在，可以新建轻量）：mock invoke 返回 `StateNotDone` → 断言 startDrag 未被调用，toast 被触发
7. **AC-7**：`pnpm test`（vitest）全部通过；现有快照若不一致需更新。

## 技术约束
- 前端 DTO 形状仅在 `src/types/`，组件不重塑（session_context §5）。
- 用户可见文案统一中文（硬约束）。
- 不绕过 `tauri-commands.ts` wrapper 直接 invoke。
- 不引入新 UI 库；状态徽章用现有 lucide-react 图标。
- 不修改 dropzone 内部组件（导入命令签名未变，DropzoneApp.tsx 不需要动）。

## 参考文件
- `src/components/features/AssetListView.tsx`（既有 extractionStatusBadge 改造起点，第 69–100 行）
- `src/hooks/useDragAssets.ts`（既有 mousedown 阈值 + startDrag 流程）
- `src/components/features/AssetContextMenu.tsx`
- `src/types/asset.ts`（保留作为非工作区视图）
- `task_001_architect/output.md` §ADR-008 / §五 数据流

## 预估影响范围
- 新建文件：无（task_003 已建 types/workspaceAsset.ts）
- 修改文件：
  - `src/stores/assetStore.ts`
  - `src/components/features/AssetListView.tsx`
  - `src/components/features/AssetContextMenu.tsx`
  - `src/hooks/useDragAssets.ts`
  - `src/lib/tauri-commands.ts`（如 task_004/005/006 未补全则在此补全 wrapper）
  - 相关测试文件
- 估算变更：~700 行（含 ~200 行测试）
