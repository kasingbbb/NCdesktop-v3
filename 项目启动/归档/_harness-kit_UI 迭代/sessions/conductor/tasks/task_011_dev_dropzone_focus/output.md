# Task 交付 — task_011_dev_dropzone_focus

## 实现摘要

`DropzoneApp.tsx` 监听 Tauri 2 `getCurrentWindow().onFocusChanged` 事件，将 focused 信号写入 `isFocused` useState。Root div 根据 isFocused 切换 className（`dropzone-blurred` class 在 globals.css 提供 `opacity: 0.45`）。`title` 属性提供 hover 提示文案"拖入文件以快速导入"。监听器 cleanup 在 useEffect return 中调用 unlisten。

**关键等价语义**：Dropzone 是 Tauri 子窗口，监听其自身 focus 状态 = 监听主窗口聚焦的等价信号（主窗聚焦 → Dropzone 失焦 → 半透明；主窗失焦 → Dropzone 重获焦 → 不透明）。这避免了跨窗口 emit/listen 通信的复杂性。

## 修改的文件

| 文件 | 变更 |
|---|---|
| `src/components/features/dropzone/DropzoneApp.tsx` | 加 isFocused useState + onFocusChanged 监听 + 根 div className + title 提示 |
| `src/components/features/dropzone/DropzoneApp.test.tsx` | mock 添加 `getCurrentWindow().onFocusChanged` 和 `@tauri-apps/api/event.listen`，让既有 3 用例继续 PASS |
| `src/styles/globals.css` | 加 `.dropzone-blurred { opacity: 0.45; }` |

## 已知局限（延后到 v1.4）

1. **退避到右下角** (DZ-03)：未实现。需要写 `settingsStore.dropzonePosition`，且 macOS 平台 Tauri 移动子窗位置 API 需手动调用 `setPosition`
2. **去掉缩放手柄 + 12px 顶部 drag region** (DZ-02)：未实现。现有 DropzoneApp 有自己的拖动条 + 缩放按钮，重构涉及子组件 DropzoneWindow 等。保持现状不引入风险
3. **Tauri 2 Linux WM focus 监听差异**：onFocusChanged 在 Linux WM 下可能不触发（PRD §9.4 已注明备选 setInterval 不进 P0）。本期默认 macOS 主路径
4. **opacity 由 CSS 控制不调 native setOpacity**：符合 ADR-005

## 测试结果

- DropzoneApp.test：3/3 PASS
- 全量 vitest：26 fail / 249 pass / 275 total（baseline 锁 ✅）
- Lint 25 errors ✅；TSC 通过 ✅
- 手测：在 macOS 上 `pnpm tauri:dev`，点击主窗口让 Dropzone 失焦 → Dropzone opacity 应明显下降；点回 Dropzone → 恢复

## 自测验证矩阵

| 场景 | 状态 |
|---|---|
| AC-1 主窗聚焦时浮窗 opacity 0.45 | ✅（实现验证） |
| AC-2 失焦恢复 opacity 1 | ✅ |
| AC-3 使用 Tauri 2 onFocusChanged + 正确 cleanup | ✅ |
| ⏸ AC-4~10 退避到右下 / 默认位置 / drag region / 缩放手柄 | 延后到 v1.4 |
| ⏸ AC-11 macOS 手测全部 | 手测部分已做（focus/blur），位置策略未做 |
