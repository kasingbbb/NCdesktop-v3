# Task 输入 — task_004_left_pane_multiselect

## 目标

为左栏 rawAssets 卡片补全 Cmd+Click 多选能力，并新增 `leftPaneFocused` 焦点状态以区分 Cmd+A 全选作用域（左栏 → 全选 rawAssets；右栏 → 全选 processedAssets）。

## 前置条件

- 依赖 task：无（可与 task_002、task_005 并行开发）
- 必须先存在的文件/接口：
  - `src/components/features/AssetListView.tsx`
  - `useAssetStore`：提供 `setSelectedAssetIds`、`toggleSelectAsset`、`clearSelection`

## 验收标准（Acceptance Criteria）

1. **AC-1**：左栏 list 模式下 Cmd+Click 第 2、3 个原件，3 个卡片均显示高亮（`multiSelected` outline）；再次 Cmd+Click 已选中卡片，取消该卡片高亮。
2. **AC-2**：左栏 grid 模式下 Cmd+Click 多选行为同 AC-1。
3. **AC-3**：鼠标在左栏区域内按 Cmd+A，仅全选 rawAssets（右栏不受影响，processedAssets 选中数量不变）。
4. **AC-4**：鼠标在右栏区域内按 Cmd+A，仅全选 processedAssets（现有行为不退步）。
5. **AC-5**：左栏 Cmd+Click 多选后执行拖拽，选中的多个文件均被拖出（与 task_003 AC-4 联动验证）。

## 技术约束

- 在 `AssetListView` 函数体内新增 `const [leftPaneFocused, setLeftPaneFocused] = useState(false)`。
- 左栏容器 div 添加 `onMouseEnter={() => setLeftPaneFocused(true)}` 和 `onMouseLeave={() => setLeftPaneFocused(false)}`。
- 修改 `handleKey` 中的 Cmd+A 分支：
  ```typescript
  if ((e.metaKey || e.ctrlKey) && e.key === "a") {
    e.preventDefault();
    if (leftPaneFocused) {
      setSelectedAssetIds(new Set(rawAssets.map((a) => a.id)));
    } else {
      setSelectedAssetIds(new Set(processedAssets.map((a) => a.id)));
    }
  }
  ```
- **不调用** `useAssetStore.getState().selectAllAssets()`（避免影响其他调用方）。
- 左栏卡片 `onClick` 增加 Cmd+Click 判断，与右栏逻辑保持一致：
  ```typescript
  onClick={(e) => {
    if (e.metaKey || e.ctrlKey) {
      toggleSelectAsset(a.id);
    } else {
      selectAsset(a.id);
    }
  }}
  ```
- 左栏卡片高亮样式：复用右栏已有的 `multiSelected` 条件样式（`outline: 2px solid var(--brand-navy)`）。
- list viewMode（约 L460）和 grid viewMode（约 L497）两处均需修改。
- 不修改任何 store、hook 或 Rust 代码。

## 参考文件

- `src/components/features/AssetListView.tsx`（L166 现有 makeDragProps 调用，L168-183 现有 handleKey，L460 list 左栏，L497 grid 左栏，L593-729 右栏卡片 multiSelected 参考实现）
- Architect output.md §2.3

## 预估影响范围

- 新建文件：无
- 修改文件：`src/components/features/AssetListView.tsx`（约 +10 行：useState、onMouseEnter/Leave、handleKey 修改、两处卡片 onClick 修改 + 高亮样式）
