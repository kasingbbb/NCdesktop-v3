# Task 输入 — task_009_move_toast

## 目标

为 `AssetContextMenu` 的"移到文件夹"操作补 Toast 反馈（v2.1 PRD §F-05 行为规格："完成后…显示 Toast"）。

## 前置条件

- 既有：`src/components/features/AssetContextMenu.tsx`（L94 `handleMoveToFolder`）
- 既有通知 API：`useUIStore.addNotification`（`BatchToolbar` 已是消费方，参考其 type/title/message/duration 字段）

## 验收标准

1. **AC-1**：移动单文件成功 → Toast 显示"已移到 {folder.displayLabel}"（type: success, duration: 2500ms）。
2. **AC-2**：移动多文件成功 → Toast 显示"已移动 N 个文件到 {folder.displayLabel}"。
3. **AC-3**：Rust 命令返回 Err → Toast 显示"移动失败：{err}"（type: error, duration: 4000ms），菜单不关闭以便重试。
4. **AC-4**：Toast 必须在 `onMoved()` 调用前后任一时机触发（推荐成功路径先 addNotification 再 onMoved，失败路径不 onMoved）。
5. **AC-5**：移动期间（`moving = true`）保持现有禁用态，避免重复触发。

## 技术约束

- 不引入新通知组件 / 不改 UI shell。
- `addNotification` 通过 `useUIStore.getState()` 获取（与既有 `useAssetStore.getState()` 风格一致），或经 props 注入。

## 参考文件

- `src/components/features/AssetContextMenu.tsx` L94-L110
- `src/components/features/assets/BatchToolbar.tsx` L30-L42（addNotification 用法示例）

## 预估影响范围

- 修改：`AssetContextMenu.tsx`（±10 行）
