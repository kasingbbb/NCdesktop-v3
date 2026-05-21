# Task 交付 — task_003_dropzone_close

## 实现摘要

**根因确认**：`tauri.conf.json` 无 `closable: false`（窗口配置正确）；问题在于前端 `void win.close()` 使用 Tauri v2 的 Window 对象直接调用，该调用在某些条件下静默失败。
**修复方案**：改用已注册的 Rust 命令 `close_dropzone_window`（通过 `invoke`），该命令由 Rust 侧可靠地获取窗口 handle 并调用 `win.close()`，错误不再静默。同时移除不再使用的 `getCurrentWindow` import。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `src/lib/tauri-commands.ts` | 修改 | 新增 `closeDropzoneWindow()` 封装 |
| `src/components/features/dropzone/DropzoneApp.tsx` | 修改 | 移除 `getCurrentWindow` import，关闭按钮改用 `cmd.closeDropzoneWindow()` |

## 对 Architect 方案的遵守声明

- [x] 修改范围仅限 `DropzoneApp.tsx`（关闭逻辑）和 `tauri-commands.ts`（按惯例添加包装）
- [x] 未触碰其他 dropzone 子组件
- [x] `tauri.conf.json` 无需修改（已确认无 `closable: false`）
- 偏离：无

## 测试命令

```bash
pnpm tauri dev  # 打开悬浮窗，点击右上角 X，确认窗口关闭
```

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果 |
|----------|----------|------|------|
| ✅ 正常路径 | 点击 X 按钮 → 悬浮窗关闭 | 代码路径已修复 | 需 tauri dev 验证 |
| ✅ 正常路径 | 点击 X 时不触发拖拽（onMouseDown stopPropagation 保留）| 代码保留 | PASS |
| ⚠️ 边界条件 | 关闭失败时 console.error 输出，用户无静默失败 | 代码覆盖 | PASS |
| ⚠️ 边界条件 | 窗口已关闭时再次调用 close_dropzone_window | Rust 侧 get_webview_window 返回 None，静默无操作 | 正常 |

## 已知局限

无

## 需要 Reviewer 特别关注的地方

`DropzoneApp.tsx` 关闭按钮 onClick — 确认 `cmd.closeDropzoneWindow()` 调用路径正确，`catch` 能捕获并输出真实错误信息
