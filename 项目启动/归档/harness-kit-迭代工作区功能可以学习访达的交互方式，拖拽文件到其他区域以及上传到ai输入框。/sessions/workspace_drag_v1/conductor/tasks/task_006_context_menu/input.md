# Task 输入 — task_006_context_menu

## 目标

新建 `AssetContextMenu.tsx` 右键菜单组件，并将其集成到 `AssetListView.tsx` 中，实现左右栏卡片的右键菜单（含"移到文件夹"二级子菜单），支持对选中集合的批量移动操作。

## 前置条件

- 依赖 task：**task_005 必须 DONE**（`move_asset_to_workspace_folder` 命令与 `moveAssetToWorkspaceFolder` TS 包装已就绪）
- 必须先存在的文件/接口：
  - `src/lib/tauri-commands.ts`：`moveAssetToWorkspaceFolder` 已导出
  - `src/components/features/AssetListView.tsx`：`workspaceFolders` state 已维护
  - `src/components/features/WorkspaceFolderStrip.tsx`：参考 `WorkspaceFolderEntry` 类型

## 验收标准（Acceptance Criteria）

1. **AC-1**：右键左栏任意原件，弹出菜单包含"移到文件夹 ▶"和"在 Finder 中显示"和"删除"三项。
2. **AC-2**：右键右栏任意转化文件，弹出菜单包含"移到文件夹 ▶"和"在 Finder 中显示"和"删除"三项。
3. **AC-3**："移到文件夹 ▶" hover 后展开二级子菜单，列出当前项目所有 WorkspaceFolder 子目录（含根目录"/"）；文件当前所在目录对应项灰显。
4. **AC-4**：点击二级子菜单中某目标文件夹，仅触发右键卡片（或选中集合）的移动，完成后资产列表刷新，Toast 提示成功。
5. **AC-5**：右键时若有多个文件被选中（`selectedAssetIds.has(assetId)` 为 true），菜单操作对整个选中集合生效；右键未选中的文件时仅对该单文件操作。
6. **AC-6**：右键 PDF 原件 → "移到文件夹" → 选择子文件夹，**只有该 PDF 移动**，其关联 Markdown 文件保持原位。
7. **AC-7**：点击菜单外区域或按 Esc 键，菜单关闭。

## 技术约束

### 新建组件 Props
```typescript
interface AssetContextMenuProps {
  x: number;
  y: number;
  assetId: string;
  pane: 'left' | 'right';
  selectedAssetIds: Set<string>;
  workspaceFolders: WorkspaceFolderEntry[];
  projectId: string;
  currentFilePath: string;      // 用于判断当前所在文件夹，灰显对应子菜单项
  onClose: () => void;
  onMoved: () => void;          // 移动完成后回调：触发 loadWorkspaceFolders + fetchAssets
}
```

### 操作目标计算逻辑
```typescript
const targetIds = selectedAssetIds.has(assetId)
  ? Array.from(selectedAssetIds)
  : [assetId];
```

### AssetListView 集成

在 `AssetListView.tsx` 中：
1. 新增 state：
   ```typescript
   const [contextMenu, setContextMenu] = useState<{
     x: number; y: number; assetId: string; pane: 'left' | 'right';
     filePath: string;
   } | null>(null);
   ```
2. 左右栏卡片均添加 `onContextMenu`：
   ```typescript
   onContextMenu={(e) => {
     e.preventDefault();
     setContextMenu({ x: e.clientX, y: e.clientY, assetId: a.id, pane: 'left'/'right', filePath: a.filePath });
   }}
   ```
3. 在组件 JSX 末尾渲染 `AssetContextMenu`（无需 portal，直接渲染在 root div 中，position: fixed）。

### 菜单行为规格

- **删除**：调用现有 `deleteAsset`（或 `useAssetStore` 中对应方法），弹 `confirm` 对话框，仅删除 `targetIds` 中的文件。
- **在 Finder 中显示**：调用 `revealProjectWorkspaceFolder`（现有命令）。
- **移到文件夹**：二级子菜单项点击后调用 `moveAssetToWorkspaceFolder(targetIds, relativePath, projectId)`，完成后调用 `onMoved()`。
- 子菜单项灰显条件：`WorkspaceFolderEntry.relativePath` 与 `currentFilePath` 所在目录的相对路径匹配时灰显。
- 性能要求：菜单出现 ≤ 100ms（不做异步数据加载，直接使用已有 `workspaceFolders` state）。

### 样式约束

- 使用 position:fixed，z-index 高于现有所有元素（≥ 1000）。
- 参考现有 CSS Variables（`--bg-primary`、`--border-primary`、`--brand-navy` 等）。
- 不引入新的 CSS 框架或动画库。

## 参考文件

- `src/components/features/WorkspaceFolderStrip.tsx`（`WorkspaceFolderEntry` 类型、`listProjectWorkspaceFolders` 调用方式）
- `src/components/features/AssetListView.tsx`（L200-226 `workspaceFolders` state 维护，右栏卡片 onContextMenu 挂载位置）
- `src/lib/tauri-commands.ts`（`moveAssetToWorkspaceFolder`、`revealProjectWorkspaceFolder`）
- Architect output.md §2.5

## 预估影响范围

- 新建文件：`src/components/features/AssetContextMenu.tsx`（约 120 行）
- 修改文件：`src/components/features/AssetListView.tsx`（约 +20 行：contextMenu state、onContextMenu handlers、组件渲染）
