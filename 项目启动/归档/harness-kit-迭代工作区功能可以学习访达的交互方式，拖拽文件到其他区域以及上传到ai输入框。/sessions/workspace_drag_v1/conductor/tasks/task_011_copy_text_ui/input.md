# Task 输入 — task_011_copy_text_ui

## 目标

在右栏增加"复制文本"入口（`BatchToolbar` + `AssetContextMenu` 右栏菜单），调用 task_010 的 `readAssetTextContent`，用 `\n---\n` 拼接后写剪贴板，并 Toast 提示。

## 前置条件

- 依赖：**task_010 DONE**
- 既有：`src/components/features/assets/BatchToolbar.tsx` L80- 的工具条 JSX
- 既有：`src/components/features/AssetContextMenu.tsx`（已支持左右栏区分 `pane`）

## 验收标准

1. **AC-1**：`BatchToolbar` 在右栏 selectedAssetIds.size ≥ 1 时显示"复制文本"按钮（带 icon，例如 `lucide-react` 的 `FileText` 或 `ClipboardCopy`）。
2. **AC-2**：点击按钮 → 调用 `readAssetTextContent(ids)` → `\n---\n` 拼接 → `navigator.clipboard.writeText` → Toast "已复制 N 个文件的文本内容到剪贴板"。
3. **AC-3**：返回的某条为空串时，对应位置插入"[尚无文本内容]"占位，不打断其余文件。
4. **AC-4**：剪贴板 API 失败 → Toast type:'error'，duration: 4000ms。
5. **AC-5**：`AssetContextMenu` 当 `pane === 'right'` 时在"在 Finder 中显示"上方插入"复制文本内容"菜单项；点击行为与按钮一致；左栏菜单不出现此项。
6. **AC-6**：右键单文件且未在 selectedAssetIds 中 → 仅复制该文件；右键已选中文件 → 复制整个选中集合（沿用现有 targetIds 计算）。

## 技术约束

- 不引入新依赖；icon 用项目内既有 `lucide-react`。
- 多选限制：单次操作 ≤ 50 文件（防止剪贴板/UI 卡顿），超出则 Toast 警告并截断。
- 与 BatchToolbar 现有 type:'success' Toast 风格一致。

## 参考文件

- `src/components/features/assets/BatchToolbar.tsx` L80-L162
- `src/components/features/AssetContextMenu.tsx` L300-L344（"在 Finder 中显示"上方插入位置）
- `src/lib/tauri-commands.ts`（task_010 新增的 `readAssetTextContent`）
- v2.1 PRD §F-04 右栏菜单表 + §F-07

## 预估影响范围

- 修改：`BatchToolbar.tsx`（+30 行：按钮 + handler）
- 修改：`AssetContextMenu.tsx`（+25 行：菜单项 + handler，仅右栏渲染）
