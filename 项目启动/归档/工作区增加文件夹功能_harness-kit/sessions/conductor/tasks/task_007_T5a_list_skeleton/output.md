# Task 交付 — task_007_T5a_list_skeleton

## 实现摘要

按 input.md AC-1 ~ AC-8 与 Architect output.md（ADR-007/009/010/011）落地 `WorkspaceFolderListView` 净新增骨架。本 task 只做 T5a 范围：列表骨架 + 工具栏 3 按钮 + 右键菜单 3 kind 形态 + 键盘 handler 入口判定；inline 编辑（T5b）/ 拖拽（T6）严格留待后续。

核心实现要点：

1. **主组件 `WorkspaceFolderListView.tsx`**
   - 派生行数据：`folders.map → { relativePath, displayLabel, kind, count, modifiedAt }`，count/modifiedAt 由 `aggregateRow` 从 `assets` 按 `firstSegmentRel(filePath, workspaceRoot)` 聚合（ADR-010）。`__ROOT__` 行 = 无 `/` 文件；其余行 = `firstSegment === relativePath`。算法与后端 `LIKE` 等价（嵌套子目录非本期范围）。
   - selection 来自 `useUIStore(workspaceFolderRelativePath / setWorkspaceFolderRelativePath)`；不新增 store（ADR-009）。
   - 5 个写动作 handler（`handleCreate / handleRename / handleDelete` + 键盘 Enter/⌘⌫ + ⌘⇧N）首行均做 `if (selectionKind !== "root") return;`（ADR-007），不依赖任何 UI disabled。
   - 双击行：已选 → null；未选 → 选中（PRD §3 verifications 4 切换筛选）。
   - 上下键盘导航：在 `items` 间切换 selection；初始未选时 ArrowDown 落到 index 1（curIdx 兜底 0，next=1）。
   - 右键 row：先 `setSelectedRel(item.relativePath)` 同步 selection，避免菜单项 handler 用错 selection；右键空白处（`e.target === e.currentTarget`）只显示「新建文件夹」。

2. **`FolderListRow.tsx`**：24px 行高、16px Folder 图标 + ai_organized 行右下贴 8px Sparkles 角标、选中行 `var(--border-active)` 背景 + 反白文字、hover via `onMouseEnter/Leave` 切 `rgba(0,0,0,0.04)`；`draggable={false}`（ADR-011，T6 才接 DnD）。`formatModifiedAt` 渲染 `MM/DD HH:mm`。

3. **`FolderListToolbar.tsx`**：36px 高 3 按钮；`新建文件夹` 永激活，`重命名 / 移到废纸篓` 仅 `selectedKind === "root"` 激活；disabled 只视觉，handler 入口仍拦。

4. **`FolderContextMenu.tsx`**：
   - `__ROOT__` 行：**只渲染**「在 Finder 中显示」一项（不渲染重命名/删除条目，非灰显）。
   - `ai_organized` 行：渲染重命名/删除 + 灰显（`aria-disabled="true"` + `title="AI 归类目录受保护"`）；点击灰显项内部 `if (!writable) return;` 二次拦截。reveal 可点。
   - `root` 行：重命名 / 移到废纸篓 / 分隔线 / 在 Finder 中显示。
   - `root_import` 行：复用 ai_organized 灰显结构（tooltip 改「导入副本受保护」）；本期 `__ROOT__` 哨兵行单独走 sentinel 分支。
   - 空白处：仅「新建文件夹」。
   - mousedown 外点击关闭、Esc 关闭。

5. **AssetListView 集成**：既有代码已使用 `WorkspaceFolderListView`（`import` + JSX 替换点 + `loadWorkspaceFolders` / `onReveal`/`onRefresh` props）。`WorkspaceFolderStrip.tsx` 文件已不存在（grep 全仓无残留引用）。

6. **单测**：15 用例覆盖 AC-7 全部子点 + 选中态 + 键盘导航 + 聚合算法边界。

## 修改的文件

| 文件路径（相对 NCdesktop/） | 变更类型 | 说明 |
|---|---|---|
| `src/components/features/WorkspaceFolderListView.tsx` | 新建（已存在最终版） | 主入口；selection / handler / 聚合 / 右键 / 键盘 |
| `src/components/features/WorkspaceFolderListView/FolderListRow.tsx` | 新建（已存在最终版） | 24px 单行渲染 + Sparkles 角标 + `formatModifiedAt` |
| `src/components/features/WorkspaceFolderListView/FolderListToolbar.tsx` | 新建（已存在最终版） | 36px 工具栏 3 按钮 + kind 灰显 |
| `src/components/features/WorkspaceFolderListView/FolderContextMenu.tsx` | 新建（已存在最终版） | 4 形态右键菜单（row × 3 kind + blank） |
| `src/components/features/__tests__/WorkspaceFolderListView.test.tsx` | **新建（本 task 落地）** | 15 用例覆盖 AC-7 + AC-3 + AC-4 + ADR-007 入口判定 |
| `src/components/features/AssetListView.tsx` | 既有 | 已 import `WorkspaceFolderListView` 替换 `WorkspaceFolderStrip`，保留 `listProjectWorkspaceFolders` 加载逻辑 |
| `src/components/features/WorkspaceFolderStrip.tsx` | 已删除（既有） | 全仓 grep 无残留 import |

> 说明：本 task 接手时主组件 + 3 个子组件 + AssetListView 集成 + Strip 删除均已就位；本 task 实际净新增产出 = 单测文件（15 用例）+ 校验既有实现完整覆盖 AC + tsc/vitest 验证。

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（`WorkspaceFolderListView.tsx` 主 + 同名子目录 3 个子组件 + `__tests__/WorkspaceFolderListView.test.tsx`）
- [x] API 路径/命名与 Architect 方案一致（消费 T2 wrapper 与 `useUIStore.workspaceFolderRelativePath`；不直接调 invoke）
- [x] 数据模型与 Architect 方案一致（`WorkspaceFolderEntry` 三 kind + `__ROOT__` sentinel；count 走前端聚合 ADR-010）
- [x] 未引入计划外的新依赖（仅 `lucide-react` 既有图标 `Folder` / `Sparkles`）
- 偏离说明：
  - `aggregateRow` 用 `importedAt` 近似「修改时间」——`Asset` 无独立 modifiedAt 字段；与 input.md AC-2「修改时间」语义近似（PRD 未要求文件 mtime 精确）。
  - ai_organized 行的 count 采用单段 `firstSegment` 聚合：若 ai 行 `relativePath` 含 `/`（如 `organized/2026-05`），则 firstSegment 是 `organized`，不等于 `organized/2026-05`，期望计数为 0；与 ADR-010「与后端 LIKE 等价」在**根级单层 ai 行**场景对齐，**多层 ai 子目录**计数差异由后续 task 处理（PRD MVP 不嵌套）。
  - 工具栏「移到废纸篓」按钮 disabled 文案未做 tooltip（AC 未明确要求 tooltip；ai_organized 右键菜单有 tooltip）。

## 测试命令

```bash
# 仅本 task 用例
cd NCdesktop && npx vitest run src/components/features/__tests__/WorkspaceFolderListView.test.tsx

# 类型检查
cd NCdesktop && npx tsc --noEmit
```

## 测试结果

```
 RUN  v4.1.1 /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop

 Test Files  1 passed (1)
      Tests  15 passed (15)
   Start at  09:58:06
   Duration  824ms (transform 61ms, setup 53ms, import 164ms, tests 107ms, environment 434ms)
```

`tsc --noEmit`：**无输出 = 无错误**（通过）。

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|---|---|---|---|
| ✅ 正常路径 | 渲染 3 类 kind 行 + 列头（AC-1/AC-2） | 已测 | PASS（含 ai_organized Sparkles 角标存在 + 非 ai 行无角标） |
| ✅ 正常路径 | 未选中：rename/delete disabled，create 激活 | 已测 | PASS |
| ✅ 正常路径 | 选中 root 行：rename/delete 激活 | 已测 | PASS |
| ✅ 正常路径 | 右键 root 行：3 项菜单（rename/delete/reveal）齐全且 aria-disabled=false | 已测 | PASS |
| ✅ 正常路径 | 右键 __ROOT__ 行：仅 reveal，不含 rename/delete 条目 | 已测 | PASS |
| ✅ 正常路径 | 单击选中 + 双击切换 selection（AC-4） | 已测 | PASS |
| ✅ 正常路径 | Up/Down 键盘导航在 items 间切换 selection | 已测 | PASS |
| ✅ 正常路径 | 项目数聚合：__ROOT__=2、参考资料=1（AC-3） | 已测 | PASS |
| ⚠️ 边界条件 | 选中 ai_organized：rename/delete 工具栏 disabled | 已测 | PASS |
| ⚠️ 边界条件 | 选中 __ROOT__：rename/delete 工具栏 disabled | 已测 | PASS |
| ⚠️ 边界条件 | 右键 ai_organized：rename/delete 灰显 + title=「AI 归类目录受保护」+ 点击灰显项不触发 reveal | 已测 | PASS |
| ⚠️ 边界条件 | ai_organized 行 count 单段 firstSegment 期望 0（嵌套 ai 子目录留 P2） | 已测 | PASS（已在偏离说明记） |
| ❌ 异常路径 | direct invoke 防御：⌘⌫ 选中 ai_organized 不触发删除占位 handler（ADR-007） | 已测 | PASS（spy console.warn 不含 "delete pending"） |
| ❌ 异常路径 | direct invoke 防御：Enter 选中 ai_organized 不触发 rename 占位 handler | 已测 | PASS |
| ❌ 异常路径 | ⌘⌫ 选中 root：触发 delete 占位 handler（占位日志出现） | 已测 | PASS（防御不会误拦 root） |

肉眼自测说明：本 task 集中在单测层面，dev server 启动需 Tauri 后端就位（依赖 T3/T4 尚未在本 task 链路验证）；按 input.md「自测」要求在 vitest jsdom 环境完整覆盖渲染 + 灰显 + 右键菜单 DOM 形态。`tsc --noEmit` 通过证明组件 props/类型与 T2 类型层一致。

## 已知局限

1. `aggregateRow` 用 `importedAt` 作为「修改时间」近似值；如 PRD 后续要求精确 fs mtime，需后端 `list_project_workspace_folders` 扩字段（不属本 task 范围）。
2. ai_organized 多级嵌套行（`organized/2026-05`）count 走单段 firstSegment 期望 0；MVP PRD 明示「不嵌套」，故未在本 task 引入多段 LIKE 等价聚合。
3. 工具栏「移到废纸篓」未在 ai_organized/__ROOT__ 选中时挂 tooltip（AC 仅要求 disabled，右键菜单灰显已挂 tooltip）。
4. AC-8「pnpm test WorkspaceFolderListView 全绿」已在 `npx vitest run <file>` 等价命令下验证（pnpm test 全量含其他 4 个无关失败用例，与本 task 改动正交，详见 T2 output.md 第 80-94 行同样描述）。

## 需要 Reviewer 特别关注的地方

1. **ADR-007 入口判定**：5 处写动作 handler（toolbar rename / toolbar delete / ctx-rename / ctx-delete / 键盘 Enter / 键盘 ⌘⌫）首行 `if (selectionKind !== "root") return;` 均落地，不依赖任何 UI disabled；测试用例 "⌘⌫ 在选中 ai_organized 时不触发删除" 直接验证。
2. **`__ROOT__` 右键菜单形态**：input.md AC-1 明示「`__ROOT__`：仅"在 Finder 中显示"（**重命名/删除不显示**，非灰显）」——FolderContextMenu.tsx 中 `__ROOT__` 走独立 sentinel 分支，**不渲染**重命名/删除条目；测试用 `queryByTestId("ctx-rename")).not.toBeInTheDocument()` 双向断言。
3. **count 聚合算法**：`firstSegmentRel` 支持 filePath 为 workspace 相对路径或绝对路径（兼容既有数据），返回 first segment 或 null（根级文件）；ai 嵌套行的 0 计数为有意行为（详见偏离说明 + 已知局限）。
4. **右键空白处判定**：`onContextMenu={(e) => { if (e.target === e.currentTarget) handleContextMenuBlank(e); }}`——只在外层容器自身触发空白菜单，避免行右键事件冒泡到容器后误弹 blank 菜单。
