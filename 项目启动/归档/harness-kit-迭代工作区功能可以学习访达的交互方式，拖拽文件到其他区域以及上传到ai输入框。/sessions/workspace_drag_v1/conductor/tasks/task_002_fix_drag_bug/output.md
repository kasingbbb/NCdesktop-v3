# Task 输出 — task_002_fix_drag_bug

## 实现摘要

从 `src/hooks/useDragAssets.ts` 的 `makeDragProps` 返回值中删除了 `draggable: true as const,` 这一行（原第 36 行）。

**根因**：当 DOM 元素设置了 `draggable="true"` 属性时，浏览器会在鼠标按下并移动时接管 HTML5 Web DnD 流程，将 `mousemove` 事件替换为 `dragstart` → `drag` 序列，导致 `onMouseMove` 无法收到事件，`startDrag`（OS 级 Tauri 文件拖拽）因此无法被触发。

**修复**：删除 `draggable: true as const,` 后，DOM 元素不再声明为 HTML5 可拖拽元素，浏览器不会拦截 `mousemove` 事件链，`onMouseMove` 可正常触发，OS 级拖拽 `startDrag` 恢复工作。`onDragStart` handler 保留作为 Web DnD 降级通道，当 OS 级拖拽不可用时仍可使用 `dataTransfer`。

---

## 修改的文件

| 文件路径 | 改动类型 | 说明 |
|---|---|---|
| `src/hooks/useDragAssets.ts` | 删除 1 行 | 删除 `makeDragProps` 返回值中的 `draggable: true as const,`（原第 36 行） |

---

## 架构遵守声明

- 仅修改了 `src/hooks/useDragAssets.ts`，严格遵守技术约束中的"仅修改该文件"要求。
- 未修改任何 Rust 代码、Tauri 命令、store 或其他 React 组件。
- `onDragStart`、`onMouseDown`、`onMouseMove`、`onMouseUp` 四个 handler 均完整保留，逻辑无变更。
- 未新建任何文件。

---

## 自测验证矩阵

### AC-3（代码静态检查）— 已自动完成

| 检查项 | 预期 | 实际 |
|---|---|---|
| 文件中存在 `draggable: true` | 不存在 | 不存在 ✓ |
| 文件中存在 `draggable: true as const` | 不存在 | 不存在 ✓ |
| `onDragStart` handler 完整保留 | 存在 | 存在 ✓ |
| `onMouseDown` handler 完整保留 | 存在 | 存在 ✓ |
| `onMouseMove` handler 完整保留 | 存在 | 存在 ✓ |
| `onMouseUp` handler 完整保留 | 存在 | 存在 ✓ |

### AC-1（单文件 OS 级拖拽）— 需手动验证

**步骤：**
1. 启动应用（`pnpm tauri dev`）。
2. 在右栏找到任意一个 `.md` 文件，单击选中。
3. 在该文件上按住鼠标左键，拖动超过 5px 阈值。
4. 将光标移动到 macOS Finder 的"下载"文件夹上，松开鼠标。

**预期结果：**
- 文件被物理复制到下载文件夹。
- 无"幽灵元素"拖影（即浏览器 HTML5 DnD 的半透明拖影消失）。
- 应用中原文件保持不变（复制而非移动）。

### AC-2（多选文件 OS 级拖拽）— 需手动验证

**步骤：**
1. 启动应用（`pnpm tauri dev`）。
2. 在右栏通过 Cmd+点击 多选 2 个文件。
3. 在其中一个已选文件上按住鼠标左键，拖动超过 5px 阈值。
4. 将光标移动到 macOS Finder 的目标文件夹，松开鼠标。

**预期结果：**
- 2 个文件均被复制到目标文件夹。
- 无"幽灵元素"拖影。
- 应用中原文件均保持不变。

---

## 交付状态

- AC-3（代码层面）：**满足** — 已通过文件内容验证确认。
- AC-1（单文件拖拽）：**待手动验证** — 需要在 macOS + Tauri 运行时环境中测试。
- AC-2（多选拖拽）：**待手动验证** — 需要在 macOS + Tauri 运行时环境中测试。
