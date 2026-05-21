# Session Context Archive — workspace_drag_v1
**归档时间**：2026-04-26（P0 SHIPPED）

---

## 本次迭代做了什么

为 NCdesktop（Tauri 2.x + React 18 + Rust）的两栏文件视图补全了 Finder 式文件拖拽和文件夹调度能力。

### 最终交付的能力

| 功能 | 入口 | 实现文件 |
|------|------|----------|
| 右栏文件拖到 Finder（Bug 修复） | 按住拖拽 | `useDragAssets.ts` |
| 左栏原件拖到 Finder（新增） | 按住拖拽 | `AssetListView.tsx` |
| 左栏 Cmd+Click 多选 | 键盘 + 点击 | `AssetListView.tsx` |
| Cmd+A 焦点区分全选 | 键盘 | `AssetListView.tsx` |
| 右键"移到文件夹"（左右栏均支持） | 右键菜单 | `AssetContextMenu.tsx` |
| 移到子文件夹二级菜单 | 右键菜单 | `AssetContextMenu.tsx` |
| Rust 原子 move 命令 | 内部调用 | `asset.rs` |

---

## 改动文件清单（最终落地）

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `src/hooks/useDragAssets.ts` | 修改 | 删除 draggable:true；改 window 全局 mousemove；get_drag_icon_path 命令取图标 |
| `src/components/features/AssetListView.tsx` | 修改 | 左栏拖拽 props；左栏多选；leftPaneFocused；contextMenu state；useEffect 移位 |
| `src/components/features/AssetContextMenu.tsx` | 新建 | 右键菜单组件（约280行） |
| `src/lib/tauri-commands.ts` | 修改 | moveAssetToWorkspaceFolder TS 包装 |
| `src-tauri/src/commands/asset.rs` | 修改 | move_asset_to_workspace_folder 命令（两阶段原子回滚） |
| `src-tauri/src/commands/settings.rs` | 修改 | get_drag_icon_path 命令 |
| `src-tauri/src/lib.rs` | 修改 | 注册两个新命令 |

---

## Hotfix（验收阶段发现，不在原始 Task 中）

1. **空白窗口** — `useEffect` 在 `const` TDZ 之前引用了变量，React 组件崩溃
2. **拖拽失效（鼠标离开卡片）** — `onMouseMove` 绑在元素上而非 window
3. **startDrag "drag image not found"** — `icon: ""` 被当路径解析；`resourceDir()` dev 模式返回 `target/debug/`（无图标）
4. **打包失败** — `currentParentPath` 未使用变量，tsc strict 报错

---

## 不变的边界（硬约束，全程遵守）

- 两栏布局（AssetListView 双栏结构）未重构
- startDrag（OS 级拖拽）能力未退步
- Rust 命令独立操作语义：不追查关联文件
- 路径安全：canonicalize 后验证在 workspace root 内

---

## P1 遗留（下次迭代起点）

- task_007：BatchToolbar"复制文本" + `read_asset_text_content` Rust 命令
- task_008：拖拽到 app 内文件夹 Spike（子方案 3A，验证 Web DnD drop target 可行性）
