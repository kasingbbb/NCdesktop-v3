# Task 输入 — task_009_T6_drag_drop

## 目标
实现 F4 单素材拖拽：HTML5 DnD + dragenter 计数器；drop target 高亮 `var(--accent-emphasis)` 2px inset；`__ROOT__` 双向合法；drop 到 `ai_organized` 前端阻止 + toast；drop 到正在编辑行禁止 + toast；完成 `test_exdev_two_phase` + `test_delete_dirty_recount` 两个 Rust 集成测试；产 PR 截图 + 10s GIF。

## 前置条件
- 依赖 task：task_005_T3_write_commands（DONE）、task_006_T4_count_state_integration（DONE）、task_008_T5b_inline_edit（DONE）
- 必须先存在的文件/接口：
  - `move_asset_to_workspace_folder` 单素材命令 + EXDEV copy-first
  - `WorkspaceFolderListView` 含 inline 编辑状态机（暴露 `isEditing(row)`）
  - `uiStore.dragOverPath` setter
  - `delete_workspace_folder` 完整事务内 recount 流程
  - `moveAssetToWorkspaceFolder` 单素材 wrapper（T2 产出）

## 验收标准（Acceptance Criteria）
1. **AC-1 集成测试 `test_exdev_two_phase`**：`cargo test --manifest-path NCdesktop/src-tauri/Cargo.toml --test workspace_folder_integration test_exdev_two_phase` PASS — 通过 `safe_rename::simulate_exdev` cfg 或 mock layer 触发 EXDEV，验证执行顺序 = copy_dir → fsync → rename(tmp→final) → BEGIN/UPDATE/COMMIT → remove src；模拟 `remove src` 失败仅产生 `cleanup_pending` 日志、DB 与目标物理状态正确。
2. **AC-2 集成测试 `test_delete_dirty_recount`**：`cargo test ... test_delete_dirty_recount` PASS — confirm 拿到 N=2 后，并发线程往目标目录塞 1 个文件再 invoke delete(expected_count=2)，后端事务内 recount = 3，返 `E_FOLDER_DIRTY{old:2, now:3}`；DB 与物理目录均未删除。
3. **AC-3 drop 双向**：单素材从右栏拖入 root 行 → 后端物理 rename + DB 同事务前缀替换；从该 root 行素材列表拖回 `__ROOT__` 行 → 同样事务移动；两个方向均成功（PRD §3 F4 / Debate §2）。
4. **AC-4 drop 高亮**：`dragenter` 时目标行渲染 `boxShadow: inset 0 0 0 2px var(--accent-emphasis)`；`dragleave` 计数器归零才清除；浅/深色模式下手动验证可见性（PRD §3 F4 / §4.5 / R9）。
5. **AC-5 ai_organized 拦截**：drop 到 `ai_organized` 行 → 前端 `preventDefault` 不发 IPC + toast「AI 归类目录受保护，不可手动移入」；单测覆盖（PRD §3 F4 / 底线 1）。
6. **AC-6 编辑行禁止**：drop 到 `editingFolderPath === row.relativePath` 的行 → 禁止图标光标 + toast「目标正在编辑中」，不打断编辑（PRD §3 F4 / Debate §4）。
7. **AC-7 dragenter 计数器**：子元素冒泡不抖动；单测验证 enter→over→leave 子节点切换时 `dragOverPath` 保持稳定。
8. **AC-8 PR 截图 + 10s GIF**：在 PR 描述中附（a）新列表浅/深主题截图；（b）10 秒 GIF 演示「新建 → 重命名 → 拖入 → 删除」四连（PRD §6.4 / §8 交付门槛）。
9. **AC-9 验收 §6.4 手动 1-6 全过**：在 PR 描述勾选 PRD §6.4 的 6 项手动验收清单（含越界单测、ai_organized 直 invoke、`⌘⌫` 二次确认）。
10. **AC-10 全绿**：`pnpm test` + `cargo test` 全绿；`pnpm tsc --noEmit` 无 error。

## 技术约束
- **DnD 栈**：仅用 HTML5 DnD（`onDragEnter / onDragOver / onDragLeave / onDrop`）+ `useRef<number>(0)` dragenter 计数器，**不接 `tauri://drag-drop`**（R7 / ADR-011）。
- **drop 高亮**：用 `boxShadow: inset 0 0 0 2px var(--accent-emphasis)`，**不要整行反色**（PRD §3 / 桥接摘要 R9 / session_context §10）。
- **双向 `__ROOT__`**：drop 目标 kind ∈ `{root, root_import}` 合法，`ai_organized` 拦截（PRD §3 F4 / Debate §2）。
- **编辑互斥**：通过 T5b 暴露的 `isEditing(row)` 判定（不直接读 `editingFolderPath`，避免重复逻辑）。
- **IPC 仅单素材**：本期一次拖一个 asset（PRD §3 F4 / ADR API）。
- **集成测试**：使用 tempfile 工作区；EXDEV 测试用 cfg flag 或注入式 mock 触发；不要依赖真实跨卷环境。
- **写通道锁**：拖拽 drop 链路最终也走 `move_asset_to_workspace_folder` → write_guard，无须前端再加锁。
- **commit**：中文 Conventional Commits；本 task PR 即"整体 PR"（其他 task 已并入），最终 PR 描述需含 GIF + 截图 + §6.4 勾选（PRD §8）。

## 参考文件
- 既有：
  - `NCdesktop/src/components/features/AssetListView.tsx`（右栏素材卡片，drag source 现有实现）
  - `NCdesktop/src/components/features/WorkspaceFolderListView.tsx`（T5a/T5b 产出）
  - `NCdesktop/src-tauri/src/utils/safe_rename.rs`（T1 产出，含 EXDEV simulate cfg）
  - `NCdesktop/src-tauri/tests/workspace_folder_integration.rs`（T4 产出，本 task 追加 2 个测试）
- 契约：`sessions/conductor/tasks/task_002_T0_contracts/contracts.md`
- 方案：output.md ADR-002（EXDEV）、ADR-007（编辑互斥+kind 判定）、ADR-011（DnD 栈）、§风险登记表 R1/R7/R9

## 预估影响范围
- 新建文件：无（在 T5a/T5b 组件上扩 DnD 监听）
- 修改文件：
  - `NCdesktop/src/components/features/WorkspaceFolderListView/FolderListRow.tsx`（加 `onDragEnter/Over/Leave/Drop` + 高亮 style）
  - `NCdesktop/src/components/features/WorkspaceFolderListView.tsx`（dragenter 计数器、toast 派发）
  - `NCdesktop/src/components/features/__tests__/WorkspaceFolderListView.test.tsx`（追加 drop 拦截 / 编辑行禁止 / 计数器稳定性用例）
  - `NCdesktop/src-tauri/tests/workspace_folder_integration.rs`（追加 `test_exdev_two_phase` + `test_delete_dirty_recount`）
- PR 产物：截图 + 10s GIF（附在 PR 描述）
