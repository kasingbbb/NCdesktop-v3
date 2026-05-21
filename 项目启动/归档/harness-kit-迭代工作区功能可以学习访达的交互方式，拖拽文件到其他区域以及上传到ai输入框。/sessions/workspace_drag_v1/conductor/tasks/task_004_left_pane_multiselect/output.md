# Task Output — task_004_left_pane_multiselect

## 实现摘要

在 `AssetListView.tsx` 中完成以下五处改动，为左栏 rawAssets 卡片补全 Cmd+Click 多选能力，并按焦点区分 Cmd+A 全选范围：

1. **新增 `leftPaneFocused` state**（L203）：`const [leftPaneFocused, setLeftPaneFocused] = useState(false)`
2. **修改 `handleKey` Cmd+A 分支**（L176-183）：根据 `leftPaneFocused` 分别全选 `rawAssets` 或 `processedAssets`，移除了对 `useAssetStore.getState().selectAllAssets()` 的调用；同步更新 `useEffect` 依赖数组加入 `leftPaneFocused, rawAssets, processedAssets, setSelectedAssetIds`
3. **左栏容器 div 增加鼠标事件**（L440-441）：`onMouseEnter={() => setLeftPaneFocused(true)}` / `onMouseLeave={() => setLeftPaneFocused(false)}`
4. **左栏 list 模式卡片**（约 L468-499）：onClick 改为 Cmd+Click 判断调用 `toggleSelectAsset`/`selectAsset`；新增 `multiSelected` 变量，背景色改为 `var(--brand-navy-10)`，轮廓改为 `2px solid var(--brand-navy)`
5. **左栏 grid 模式卡片**（约 L511-540）：同上改动，`<button>` onClick 改为 Cmd+Click 判断；新增 multiSelected 高亮样式

## 修改文件表格

| 文件 | 类型 | 改动描述 |
|------|------|----------|
| `src/components/features/AssetListView.tsx` | 修改 | 新增 state、修改 handleKey、左栏容器鼠标事件、list/grid 卡片 onClick + multiSelected 样式 |

## 架构遵守声明

- 未修改 `useAssetStore`、任何 hook 文件或 Rust 代码
- 未调用 `useAssetStore.getState().selectAllAssets()`
- `toggleSelectAsset`、`selectAsset`、`setSelectedAssetIds` 均来自 `useAssetStore()` 解构（L136-143），直接使用
- TypeScript 编译 `npx tsc --noEmit` 零错误

## 自测验证矩阵

| 验收标准 | 状态 | 说明 |
|----------|------|------|
| AC-1：左栏 list 模式 Cmd+Click 多选 + 高亮 | 满足 | onClick 加入 metaKey/ctrlKey 判断，`multiSelected` outline 已添加 |
| AC-2：左栏 grid 模式 Cmd+Click 多选 + 高亮 | 满足 | grid `<button>` onClick 同样改造，multiSelected 样式一致 |
| AC-3：鼠标在左栏按 Cmd+A 仅全选 rawAssets | 满足 | `leftPaneFocused=true` 时 `setSelectedAssetIds(new Set(rawAssets.map(a=>a.id)))` |
| AC-4：鼠标在右栏按 Cmd+A 仅全选 processedAssets | 满足 | `leftPaneFocused=false`（默认）时全选 processedAssets，原行为不退步 |
| AC-5：左栏多选后拖拽均被带出（联动 task_003） | 待运行时验证 | `makeDragProps` 读取 `selectedAssetIds`，多选卡片 id 均进入 set，拖拽逻辑与右栏一致 |
| 不修改 store/hook | 满足 | 仅修改 AssetListView.tsx |
