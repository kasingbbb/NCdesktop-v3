# Task 输入 — task_008_verify_cmda_focus

## 目标

验证并补齐 F-03 中 Cmd+A 的焦点分流逻辑。当前 `AssetListView.tsx` 的 keydown handler（约 L298-L310）已把 `leftPaneFocused` 放入依赖数组，但 handler 体内仍用统一 `displayAssets.map(...)` 作为全选集合，未按焦点切换到 `rawAssets` / `processedAssets`。

## 前置条件

- 现有文件：`src/components/features/AssetListView.tsx`
- 已存在 state：`leftPaneFocused`（L262）+ `onMouseEnter/Leave` 在左栏容器 L423-L424
- 已存在数据源：`rawAssets`、`processedAssets` 应在 useMemo 中已分流（若没有需先确认）

## 验收标准

1. **AC-1**：鼠标进入左栏后，按 Cmd+A，`selectedAssetIds` 等于 `new Set(rawAssets.map(a => a.id))`。
2. **AC-2**：鼠标进入右栏后，按 Cmd+A，`selectedAssetIds` 等于 `new Set(processedAssets.map(a => a.id))`。
3. **AC-3**：焦点位于 INPUT / TEXTAREA 时 Cmd+A 不触发全选（保留现有行为）。
4. **AC-4**：若 `rawAssets` 或 `processedAssets` 因 workspaceFolder 过滤需要筛选，全选作用于筛选后的集合（与现有 displayAssets 语义一致）。
5. **AC-5**：移除 deps 数组中冗余项（如分流后不再需要 displayAssets，dep 应同步更新）。

## 技术约束

- 不引入新 store 字段，沿用 `leftPaneFocused` 局部 state。
- 若 `rawAssets` 与 `processedAssets` 当前未分别 useMemo，先建立分流 useMemo 再消费。
- 不动 `selectAllAssets()` 现有 API（ADR-007）。

## 参考文件

- `src/components/features/AssetListView.tsx` L230-L310
- v2.1 PRD §F-03

## 预估影响范围

- 修改：`AssetListView.tsx`（±15 行 keydown handler + useMemo 校对）
