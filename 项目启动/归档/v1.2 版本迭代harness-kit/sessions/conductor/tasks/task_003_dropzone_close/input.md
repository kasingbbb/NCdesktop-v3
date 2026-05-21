# Task 输入 — task_003_dropzone_close

## 目标

修复 `DropzoneApp.tsx` 中悬浮窗关闭按钮点击无反应的 bug，确保点击 X 按钮后窗口在 macOS 生产环境下可靠关闭。

## 前置条件

- 依赖 task：无（独立，可与 task_001 并行）
- 必须先存在的文件：
  - `src/components/features/dropzone/DropzoneApp.tsx`
  - `src-tauri/tauri.conf.json`（需检查 dropzone window 配置）

## 验收标准（Acceptance Criteria）

1. **AC-1**：点击悬浮窗右上角 X 按钮，窗口关闭（在 `tauri dev` 模式下可验证）
2. **AC-2**：关闭按钮的 click handler 不再使用 `void` 静默吞掉错误，改为有错误处理
3. **AC-3**：`tauri.conf.json` 中 dropzone window 的 `closable` 字段值已确认/修正（若之前为 false 则改为 true）
4. **AC-4**：修复范围仅限 `DropzoneApp.tsx` 和 `tauri.conf.json`，不触碰其他 dropzone 子组件

## 技术约束

- 只修改 `DropzoneApp.tsx` 的关闭相关逻辑（第 240–249 行附近）和 `tauri.conf.json`
- 不改变悬浮窗的其他交互行为（拖拽、缩放、展开/收起）
- 错误处理：`win.close()` 改为 `.then(() => {}).catch(err => console.error('[dropzone] close failed', err))`

## 诊断清单（Dev 执行顺序）

1. 先查 `src-tauri/tauri.conf.json`：找 dropzone/DropzoneWindow 的 window 配置，确认 `closable` 是否为 `false`
2. 查 `DropzoneApp.tsx` 第 208 行附近：`getCurrentWindow()` 调用位置是否正确
3. 查关闭按钮 onClick（第 246 行附近）：`void win.close()` → 改为有错误处理的版本
4. 如果 `closable: false`，改为 `closable: true` 并重新测试

## 参考文件

- `src/components/features/dropzone/DropzoneApp.tsx` — 关闭按钮实现（第 240–249 行）
- `src-tauri/tauri.conf.json` — window 配置（搜索 "dropzone" 或 "DropzoneWindow"）
- Tauri v2 文档：`Window::close()` API

## 预估影响范围

- 修改文件：
  - `src/components/features/dropzone/DropzoneApp.tsx`（关闭逻辑，约 5 行）
  - `src-tauri/tauri.conf.json`（如需修改 closable 字段）
- 不新建文件

## Conductor 复杂度评估

**S 级**（单点 bug 修复，影响范围极小）
→ Reviewer 关注点：修复是否覆盖根因（config + 代码双检查）
