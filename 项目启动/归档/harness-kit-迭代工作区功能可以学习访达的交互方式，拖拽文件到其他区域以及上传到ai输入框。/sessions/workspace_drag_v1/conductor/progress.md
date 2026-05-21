# Conductor Progress — workspace_drag_v1

## 当前状态
STATE: SHIPPED
当前 Task: —（P0 MVP 已验收 + 打包，P1 待规划）
更新时间: 2026-04-26

---

## 已完成阶段

- [x] session_context.md 填写完毕
- [x] 复杂度评级：L
- [x] Debate 完成（4 层完整，含 v2.1 修正）
- [x] PRD v2.1 产出，Conductor 桥接摘要已附加
- [x] **task_001_architect DONE** — 技术方案 output.md 已产出，6 个 P0 Task input.md 已写入
- [x] **task_002 DONE** — 删除 `draggable:true`，恢复 startDrag 路径
- [x] **task_003 DONE** — 左栏 list(L476) + grid(L526) 两处挂载 `makeDragProps`
- [x] **task_004 DONE** — 左栏 Cmd+Click 多选 + `leftPaneFocused` Cmd+A 焦点区分
- [x] **task_005 DONE** — Rust `move_asset_to_workspace_folder`（两阶段原子回滚 + 路径越界防护）
- [x] **task_006 DONE** — `AssetContextMenu.tsx`（约280行），左右栏4处 onContextMenu 集成
- [x] **验收通过** — 右键菜单功能正常，拖拽到 Finder 正常
- [x] **发布打包** — `pnpm tauri build` 成功

---

## 验收后 Bug 修复记录（非 task，Hotfix）

| 问题 | 根因 | 修复 |
|------|------|------|
| 空白窗口 | `useEffect` 引用 `leftPaneFocused`/`rawAssets`/`processedAssets`，但三者在 L204/L260 声明，晚于 useEffect 的 L170，触发 const TDZ ReferenceError | 将 useEffect 移至 `rawAssets` useMemo 之后 |
| 拖拽无效（鼠标离开卡片即失效） | `onMouseMove` 绑在卡片元素上，鼠标拖出边界后事件断流 | 改为 `onMouseDown` 时动态挂 `window.mousemove/mouseup` |
| startDrag 报错 "drag image not found" | `icon: ""` 被插件当文件路径解析 | 新增 Rust 命令 `get_drag_icon_path`，dev 模式用 `env!("CARGO_MANIFEST_DIR")/icons/32x32.png`，release 用 `resource_dir()` |
| 打包失败 | `AssetContextMenu.tsx` 中 `currentParentPath` 声明但未使用（tsc strict） | 删除该行 |

---

## 已完成 Task 队列（P0 MVP）

| Task ID | 描述 | 状态 |
|---------|------|------|
| task_002_fix_drag_bug | 修复 startDrag 失效 | ✅ DONE |
| task_003_left_pane_drag | 左栏补全 makeDragProps | ✅ DONE |
| task_004_left_pane_multiselect | 左栏多选 + Cmd+A 焦点区分 | ✅ DONE |
| task_005_rust_move_command | Rust 原子 move 命令 | ✅ DONE |
| task_006_context_menu | AssetContextMenu 组件 | ✅ DONE |

## 待执行 Task 队列（P1）

| Task ID | 描述 | 状态 | 前置 |
|---------|------|------|------|
| task_007_copy_text | BatchToolbar"复制文本" + read_asset_text_content | PENDING | P0 SHIPPED |
| task_008_drag_spike | 拖拽到文件夹 Spike（子方案 3A） | PENDING | P0 SHIPPED |

---

## 关键决策记录

- [2026-04-26] Debate 完成，确立独立操作语义（撤销 v2.0 方案 A 约束）
- [2026-04-26] 确认 startDrag 失效根因：draggable:true 导致 Web DnD 截断 onMouseMove
- [2026-04-26] 确认 MVP 路径：右键菜单（绕开 startDrag vs Web DnD 冲突），拖拽内部落点为 P1 Spike
- [2026-04-26] Architect：DB 更新在所有 rename 成功后统一提交，保证磁盘/DB 原子一致性
- [2026-04-26] Architect：不修改 selectAllAssets()，用局部 leftPaneFocused state 区分 Cmd+A 焦点
- [2026-04-26] Hotfix：`get_drag_icon_path` Rust 命令用 cfg!(debug_assertions) 分离 dev/release 路径

---

## 状态转移日志

[2026-04-26] STATE: INIT → DEBATE | 原因: L 复杂度，启动 4 层完整 Debate | 风险: 无
[2026-04-26] STATE: DEBATE → PRD_COMPLETE | 原因: Debate v2.1 完成，PRD 已产出 | 风险: P1 Spike 结果未知
[2026-04-26] STATE: PRD_COMPLETE → READY_FOR_ARCHITECTURE | 原因: PRD 桥接摘要通过自检 | 风险: 低
[2026-04-26] STATE: READY_FOR_ARCHITECTURE → DEVELOPING | 原因: Architect task_001 完成，6 个 P0 Task 写入 | 风险: 低
[2026-04-26] STATE: DEVELOPING → P0_COMPLETE | 原因: task_002~006 全部 DONE | 风险: 待人工验收
[2026-04-26] STATE: P0_COMPLETE → SHIPPED | 原因: 验收通过 + 4 个 Hotfix + pnpm tauri build 成功 | 风险: 无
