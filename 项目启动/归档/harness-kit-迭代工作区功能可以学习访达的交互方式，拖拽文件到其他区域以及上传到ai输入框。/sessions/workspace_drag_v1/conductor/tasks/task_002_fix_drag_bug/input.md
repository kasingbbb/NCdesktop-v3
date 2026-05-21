# Task 输入 — task_002_fix_drag_bug

## 目标

删除 `useDragAssets.ts` 中 `makeDragProps` 返回值里的 `draggable: true as const`，解除 HTML5 Web DnD 对 `onMouseMove` 事件链的拦截，使 `startDrag`（OS 级文件拖拽）恢复正常触发。

## 前置条件

- 依赖 task：无
- 必须先存在的文件：`src/hooks/useDragAssets.ts`

## 验收标准（Acceptance Criteria）

1. **AC-1**：右栏拖拽一个 `.md` 文件到 macOS Finder 下载文件夹，松手后文件被物理复制，无"幽灵元素"拖影。
2. **AC-2**：右栏多选 2 个文件后拖拽，松手后 2 个文件均被复制到目标位置。
3. **AC-3**：`src/hooks/useDragAssets.ts` 中不存在 `draggable: true` 或 `draggable: true as const`。

## 技术约束

- 仅修改 `src/hooks/useDragAssets.ts` 第 36 行，删除 `draggable: true as const,` 这一行（含逗号）。
- 保留 `onDragStart` handler（Web DnD 降级通道），不移除。
- 不修改 `makeDragProps` 的其他返回字段（`onMouseDown`、`onMouseMove`、`onMouseUp`）。
- 不修改任何 Rust 代码、store 或其他组件。

## 参考文件

- `src/hooks/useDragAssets.ts`（改动位置：第 36 行 `draggable: true as const,`）
- Architect output.md §2.1：根因分析与修复说明

## 预估影响范围

- 新建文件：无
- 修改文件：`src/hooks/useDragAssets.ts`（删除 1 行）
