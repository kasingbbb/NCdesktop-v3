# Task 输入 — task_011_dev_dropzone_focus

## 目标

`DropzoneApp.tsx` 监听 Tauri 2 window focus/blur 事件，当主窗口聚焦时：① 浮窗 opacity 变 0.45 ② 浮窗位置退避到屏幕右下角；主窗失焦时恢复 opacity 1 与位置。同时：
- 去掉浮窗内的"缩放手柄"
- 拖动改为整条顶部 12px drag region（macOS 习惯）
- 关闭 X 放右上角
- hover 时 tooltip "拖入文件以快速导入"
- `settingsStore.dropzonePosition` 默认值改为 `{x: viewport.width-220, y: viewport.height-200}`（首次启动）

## 前置条件

- 依赖 task：**无**
- 必须先存在的文件/接口：
  - `src/components/features/dropzone/DropzoneApp.tsx`
  - `src/stores/settingsStore.ts`（确认 `dropzonePosition` 字段）
  - Tauri 2 已暴露 `getCurrent().onFocusChanged` API（已确认）

## 验收标准（Acceptance Criteria）

1. **AC-1**：主窗口聚焦时，浮窗 CSS opacity 为 0.45（用 CSS class 切换或 style，**不调用 native setOpacity**）
2. **AC-2**：主窗口失焦时（用户点击浮窗或其它窗口），浮窗 opacity 恢复 1
3. **AC-3**：监听器使用 Tauri 2 的 `getCurrent().onFocusChanged(...)` API；unlisten 函数在 `useEffect` cleanup 中调用
4. **AC-4**：浮窗位置：首次启动时（`dropzonePosition` 为默认值），定位到 `{x: viewport.width-220, y: viewport.height-200}`；用户拖动后位置覆盖默认值
5. **AC-5**：浮窗 UI 不再有"缩放手柄"DOM（grep DropzoneApp.tsx 等无 resize handle 类名）
6. **AC-6**：浮窗顶部有 12px 高的拖动区（CSS `cursor: move` 或 `data-tauri-drag-region` 属性）
7. **AC-7**：关闭 X 按钮在右上角；点击关闭浮窗（隐藏，不销毁窗口实例）
8. **AC-8**：浮窗 hover 时显示 tooltip "拖入文件以快速导入"（用现有 Tooltip 组件或 title 属性）
9. **AC-9**：单测覆盖：① focus → opacity 0.45 ② blur → opacity 1 ③ cleanup 调用 unlisten ④ default position 写入 store
10. **AC-10**：`pnpm check` + `pnpm lint` + `pnpm test` 全绿
11. **AC-11**：macOS 手测：拖动顶部 12px 区域可移动浮窗；右上 X 可关闭；hover 显示 tooltip；主窗聚焦时浮窗半透明退避到右下

## 技术约束

- **Tauri API 调用**：必须用 `@tauri-apps/api/window` 的 `getCurrent().onFocusChanged`；不通过 `event.listen` 间接监听
- **opacity 由 CSS 控制**：通过 className 切换或 inline style；**不调用 setOpacity**（避免跨平台问题，ADR-005）
- **位置写入 store**：用 `settingsStore.setDropzonePosition({x, y})`；如 setter 不存在则新增
- **drag region**：优先用 Tauri 2 的 `data-tauri-drag-region` 属性，浏览器环境降级为 `cursor: move`
- **不引新依赖**
- **viewport 获取**：用 `window.innerWidth / window.innerHeight`（不调 Tauri API 获取屏幕尺寸，因为浮窗本身是 Tauri 子窗）

## 参考文件

- `src/components/features/dropzone/DropzoneApp.tsx`（现有结构）
- `src/components/features/dropzone/DropzoneWindow.tsx`（缩放/位置相关）
- `src/stores/settingsStore.ts`
- `src/components/features/dropzone/__tests__/DropzoneApp.test.tsx`（如已存在则扩展）
- `product/prd/notecapt-v1.3-ui_prd_v1.md` §7.1 DZ-01 ~ DZ-04 + ADR-005

## 预估影响范围

- **修改文件**：
  - `src/components/features/dropzone/DropzoneApp.tsx`（focus/blur 监听 + opacity / position）
  - 可能：`src/components/features/dropzone/DropzoneWindow.tsx`（拖动 region + 关闭按钮）
  - `src/stores/settingsStore.ts`（默认 dropzonePosition + setter 如缺失）
  - 可能：`src/components/features/dropzone/__tests__/DropzoneApp.test.tsx`

- **新建文件**：可能上述测试

---

## Reviewer 重点关注项

- 监听器是否在 unmount 时正确 unlisten（防止内存泄漏）
- opacity 切换是否有 200ms 过渡（用 --duration-fast）
- 首次启动 dropzonePosition 真的是右下，而不是中心
- Tauri API import 路径与版本是否匹配（@tauri-apps/api/window vs window）
- macOS 手测必须实测（vitest 无法验证拖动）
