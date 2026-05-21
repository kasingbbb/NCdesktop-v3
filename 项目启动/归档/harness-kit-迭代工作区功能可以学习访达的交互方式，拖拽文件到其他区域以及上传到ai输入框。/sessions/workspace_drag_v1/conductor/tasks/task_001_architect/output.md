# Architect 技术方案 — workspace_drag_v1

**版本**：v1.0  
**日期**：2026-04-26  
**状态**：APPROVED（Architect 自签）

---

## 一、总体设计原则

1. **最小改动原则**：6 个 Task 覆盖 P0 MVP，无一行超出 PRD 范围的重构。
2. **独立操作语义**：所有操作（拖拽/移动/删除）仅作用于显式选中集合，Rust 命令不追查关联文件。
3. **原子性保证**：Rust `move_asset_to_workspace_folder` 实现 try-rollback 模式，无孤儿文件。
4. **路径安全**：所有目标路径在 `fs::rename` 前必须 `canonicalize()` 并验证前缀在 `workspace_root` 内。
5. **与现有模式对齐**：前端 Hooks / Store / 组件命名与现有 `useDragAssets`、`useRubberBandSelect`、`useAssetStore` 模式保持一致。

---

## 二、核心架构决策

### 2.1 draggable:true 移除（Task 002）

**根因**：`useDragAssets.ts` 第 36 行 `draggable: true as const` 激活 HTML5 Web DnD，`mousedown → mousemove` 事件链被浏览器接管，`onMouseMove` 不再触发，`startDrag` 从未被调用。

**修复**：删除 `draggable: true as const` 这一键值对，保留 `onDragStart` 作为 Web DnD 降级通道（暂保留，不移除，避免意外副作用）。

**影响范围**：仅 `src/hooks/useDragAssets.ts` 第 36 行，改动量：1 行。

### 2.2 左栏 makeDragProps 接入（Task 003）

左栏 rawAssets 卡片当前无任何拖拽 props。需要在两处（list viewMode 约 L460，grid viewMode 约 L497）的卡片元素上添加 `{...makeDragProps(a.id)}`。

`useDragAssets` hook 已接收 `assets` 参数，但目前只传入了 `assets`（全量）。确认：`assets` 包含 rawAssets 和 processedAssets，因此 `resolveFilePaths` 对 rawAssets 的 `filePath` 也能正确解析，**不需要修改 hook 本身**。

### 2.3 左栏多选 + Cmd+A 焦点区分（Task 004）

**当前问题**：左栏卡片只有 `onClick` 单选逻辑，无 `metaKey` 判断。Cmd+A 全选调用 `useAssetStore.getState().selectAllAssets()`，该方法目前全选 `assets`（全量），无栏位区分。

**设计**：
- 在 `AssetListView` 中新增 `leftPaneFocused` boolean state，由左栏容器的 `onMouseEnter`/`onMouseLeave` 驱动。
- `handleKey` 中 Cmd+A 分支：`leftPaneFocused` → `setSelectedAssetIds(new Set(rawAssets.map(a=>a.id)))` ；否则 `setSelectedAssetIds(new Set(processedAssets.map(a=>a.id)))`。
- 左栏卡片 `onClick` 增加 `e.metaKey || e.ctrlKey` 判断，调用 `toggleSelectAsset(a.id)`（与右栏逻辑对齐）。

**不修改** `useAssetStore` 的 `selectAllAssets` 方法，避免影响其他调用方。

### 2.4 Rust move_asset_to_workspace_folder（Task 005）

**新函数位置**：`src-tauri/src/commands/asset.rs`（追加，不新建文件）

**原子性策略**：维护 `Vec<(PathBuf, PathBuf)>` 已移动记录，任一 rename 失败后逆向 rename 所有已记录对，返回 `Err`。DB 更新在**所有文件 rename 成功后统一进行**（不逐文件写入 DB），从而保证 DB/磁盘一致性。

**路径安全**：
```
workspace_root = workspace::project_workspace_dir(&project_id)?  // 绝对路径
target_dir = if relative_path == "__ROOT__" { workspace_root.clone() }
             else { workspace_root.join(relative_path) }
canonical_target_dir = target_dir.canonicalize() 或 create_dir_all 后再 canonicalize
assert canonical_target_dir.starts_with(&workspace_root.canonicalize())
```

**注册**：在 `src-tauri/src/lib.rs` 的 `invoke_handler![]` 宏中追加 `commands::asset::move_asset_to_workspace_folder`。

**TS 包装**：在 `src/lib/tauri-commands.ts` 追加 `moveAssetToWorkspaceFolder`（见 PRD F-06 接口定义）。

### 2.5 AssetContextMenu 组件（Task 006）

**新建文件**：`src/components/features/AssetContextMenu.tsx`

**数据流**：
- 触发：卡片 `onContextMenu` → 设置 `contextMenu: { x, y, assetId, pane: 'left'|'right' }` state（在 `AssetListView` 中 useState）
- 渲染：在 `AssetListView` JSX 末尾 portal 渲染 `<AssetContextMenu>`，点击 document 关闭
- 子菜单数据：复用 `workspaceFolders` state（已在 `AssetListView` 中维护）
- 移动调用：`moveAssetToWorkspaceFolder(targetIds, relativePath, projectId)` → `loadWorkspaceFolders()` + store `fetchAssets()` + toast

**操作目标计算**：
```
targetIds = contextMenu.pane 的 assetId 是否在 selectedAssetIds 中
  ? Array.from(selectedAssetIds)   // 批量操作选中集合
  : [contextMenu.assetId]          // 单个右键
```

---

## 三、改动文件清单

| 文件 | 类型 | Task | 改动说明 |
|------|------|------|----------|
| `src/hooks/useDragAssets.ts` | 修改 | 002 | 删除 `draggable: true as const`（第 36 行）|
| `src/components/features/AssetListView.tsx` | 修改 | 003 | 左栏 list/grid 卡片添加 `{...makeDragProps(a.id)}` |
| `src/components/features/AssetListView.tsx` | 修改 | 004 | 左栏卡片 Cmd+Click；新增 `leftPaneFocused` state；Cmd+A 焦点区分 |
| `src-tauri/src/commands/asset.rs` | 修改 | 005 | 追加 `move_asset_to_workspace_folder` 函数 |
| `src-tauri/src/lib.rs` | 修改 | 005 | invoke_handler 注册新命令 |
| `src/lib/tauri-commands.ts` | 修改 | 005 | 追加 `moveAssetToWorkspaceFolder` TS 包装 |
| `src/components/features/AssetContextMenu.tsx` | 新建 | 006 | 右键菜单组件（含二级子菜单）|
| `src/components/features/AssetListView.tsx` | 修改 | 006 | 挂载 contextMenu state + 渲染 AssetContextMenu |

**不修改文件**：
- `src/stores/`（assetStore、uiStore 不新增 slice）
- `src/hooks/useRubberBandSelect.ts`（右栏框选保持原状）
- `src-tauri/src/db/asset.rs`（复用现有 `update_name_and_path`）

---

## 四、接口设计

### Rust 命令签名
```rust
#[tauri::command]
pub fn move_asset_to_workspace_folder(
    database: State<'_, Database>,
    asset_ids: Vec<String>,
    target_relative_path: String,
    project_id: String,
) -> Result<(), String>
```

### TS 包装
```typescript
export async function moveAssetToWorkspaceFolder(
  assetIds: string[],
  targetRelativePath: string,
  projectId: string
): Promise<void>
```

### AssetContextMenu Props
```typescript
interface AssetContextMenuProps {
  x: number;
  y: number;
  assetId: string;
  pane: 'left' | 'right';
  selectedAssetIds: Set<string>;
  workspaceFolders: WorkspaceFolderEntry[];
  projectId: string;
  onClose: () => void;
  onMoved: () => void;
}
```

---

## 五、测试策略

| Task | 测试类型 | 测试点 |
|------|----------|--------|
| 002 | 手动 | 右栏拖 .md 到 Finder，验证有文件落下（非幽灵） |
| 003 | 手动 | 左栏拖 PDF 到桌面，验证文件落下；多选后拖 3 个均落下 |
| 004 | 手动 | 左栏 Cmd+Click 多选高亮；左栏焦点 Cmd+A 全选左栏；右栏焦点 Cmd+A 全选右栏 |
| 005 | Rust 单元测试 | 正常移动；越界路径 Err；中途失败回滚验证（mock fs 或 tmp 目录） |
| 006 | 手动 | 右键左栏 → 出现"移到文件夹"；右键右栏 → 出现"移到文件夹"；多选后批量移动；删除只删选中文件 |

---

## 六、Task 依赖图

```
task_001（本文档）
    ├── task_002（修复 drag bug）─────────────────────────────────┐
    ├── task_003（左栏 drag）←── 依赖 task_002                    │
    ├── task_004（左栏多选）←── 无前置，可与 task_002/003 并行    │
    └── task_005（Rust 命令）←── 无前置，可与 task_002/003/004 并行
            └── task_006（context menu）←── 依赖 task_005         │
                                                                   │
所有 P0 完成 → task_007/task_008（P1）                            │
```

**并行策略**：task_002、task_004、task_005 无相互依赖，可并行启动不同 Dev Agent。task_003 须在 task_002 DONE 后启动。task_006 须在 task_005 DONE 后启动。

---

## 七、风险缓解

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| `onDragStart` 保留后仍触发 Web DnD 干扰 | 低 | 中 | task_002 验收时如有干扰可再移除 `onDragStart` |
| 左栏焦点状态不准确（鼠标快速移动） | 低 | 低 | 仅影响 Cmd+A 焦点区分，不影响核心拖拽功能 |
| `canonicalize()` 在目标目录不存在时返回 Err | 中 | 中 | 先 `create_dir_all` 后再 `canonicalize`（见 task_005 实现规格） |
| 多文件 rename 中途宕机（OS 崩溃） | 极低 | 高 | 超出单进程回滚能力范围，不在 MVP scope 内处理 |
