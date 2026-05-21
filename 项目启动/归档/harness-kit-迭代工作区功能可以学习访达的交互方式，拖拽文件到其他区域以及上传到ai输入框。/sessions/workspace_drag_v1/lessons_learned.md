# Lessons Learned — workspace_drag_v1
**日期**：2026-04-26

---

## 一、Tauri OS 级拖拽的三个陷阱

### 陷阱 1：draggable="true" 与 startDrag 互斥

HTML5 的 `draggable="true"` 属性会让浏览器接管 mousedown → mousemove 事件序列，替换为 dragstart → drag 序列。一旦激活，原始 `onMouseMove` 不再触发，`startDrag`（需要 mousemove 达到阈值才调用）永远不会被执行。

**规则**：在同一个元素上使用 `@crabnebula/tauri-plugin-drag` 的 `startDrag` 时，绝对不能同时设置 `draggable="true"`。

### 陷阱 2：onMouseMove 必须绑在 window，不能绑在元素上

将 mousemove 绑在卡片元素上时，鼠标按住后稍微移出边界，事件就断流。正确做法：

```typescript
onMouseDown: (e) => {
  // 记录起点
  window.addEventListener("mousemove", onMouseMove);  // 全局追踪
  window.addEventListener("mouseup", onMouseUp);
}
```

**规则**：凡是需要跨元素追踪鼠标轨迹的交互（拖拽、resize、滑块），mousemove 必须注册到 window。

### 陷阱 3：startDrag 的 icon 参数不接受空字符串

`icon: ""` 会被插件 Rust 侧当做文件路径解析，找不到文件报 "drag image not found"。

**解决方案**：写一个 Rust 命令返回图标的绝对路径，用 `cfg!(debug_assertions)` 分离 dev / release：

```rust
// dev: CARGO_MANIFEST_DIR = src-tauri/（编译时常量，总是正确）
// release: app_handle.path().resource_dir()（打包后的资源目录）
```

---

## 二、Tauri 2.x dev 模式的资源路径陷阱

`@tauri-apps/api/path` 的 `resourceDir()` / `resolveResource()` 在 dev 模式下返回 `target/debug/`，而不是源码目录。这个目录在正常开发流程中不包含静态资源文件。

**规则**：需要在 dev 模式下访问静态资源（图标等），使用 Rust 端 `env!("CARGO_MANIFEST_DIR")` 而非前端的 path API。

---

## 三、React Hook 的 const TDZ 崩溃

```typescript
// 错误：useEffect 在 const 声明之前引用它
useEffect(() => {
  if (leftPaneFocused) { ... }  // ← 引用
}, [leftPaneFocused]);

const [leftPaneFocused, setLeftPaneFocused] = useState(false);  // ← 声明在后
```

`const` 有暂时性死区（TDZ），在同一作用域内声明之前访问会抛 ReferenceError。组件在渲染时崩溃 → 空白窗口，且没有明显的错误提示（React 错误边界会静默吞掉）。

**规则**：useEffect 的依赖项所对应的 state/变量，必须在 useEffect 调用之前声明。**检查顺序：先声明，后使用。**

---

## 四、多 Agent 并行开发的协调问题

本次有多个 Agent 同时修改 `AssetListView.tsx`（task_003 加 makeDragProps，task_004 加多选逻辑）。两个 Agent 各自拿到文件的不同时间点的快照，可能产生冲突或遗漏。

**观察**：task_004 的 useEffect 被放在了错误的位置（比依赖变量的声明更早），这是 Agent 拿到旧版文件快照、未感知到 task_003 改动的副作用。

**规则**：
- 并行 Agent 修改同一文件时，后启动的 Agent 必须先 Read 文件确认当前状态
- 有顺序依赖的文件修改应串行而非并行
- Conductor 在汇总时要对关键文件做最终一致性检查

---

## 五、tsc dev 不报错但 build 报错

`npx tsc --noEmit` 在 dev 模式下使用宽松配置，而 `vite build` 调用 `tsc -b` 使用严格配置（包括 `noUnusedLocals`）。未使用变量在 dev 不报错，在 build 报错。

**规则**：打包前用 `tsc -b`（与 build 脚本一致）而非 `tsc --noEmit` 做最终检查，或直接跑 `pnpm build` 确认。

---

## 六、Harness Conductor 流程有效性评估

**有效的部分**：
- Debate → PRD → Architect → Dev 的分层结构，让复杂问题的分析质量明显提升
- Task input.md 的验收标准让 Dev Agent 有明确的完成定义
- progress.md 作为状态真相，断点续传无需重新理解上下文

**需要改进的部分**：
- Dev Agent 交付后缺少"运行时验证"环节（tsc 通过 ≠ 功能正确）
- 验收（QA）应该是独立 task，而不是让用户手动测试后再 hotfix
- 多 Agent 并行修改同一文件时需要 Conductor 做合并检查，而不是假设各 Agent 独立无冲突
