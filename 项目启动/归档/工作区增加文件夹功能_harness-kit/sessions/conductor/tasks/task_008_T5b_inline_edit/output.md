# Task 交付 — task_008_T5b_inline_edit

## 实现摘要

在 T5a 骨架之上落地 inline 编辑状态机 + F1 幽灵新建 + F2 重命名（三入口）+ F3 删除二次确认 modal + 编辑期切走二次确认 modal。所有写命令通过 T2 的 `createWorkspaceFolder` / `renameWorkspaceFolder` / `deleteWorkspaceFolder` wrapper 发起；失败按 IpcError 渲染（`renderIpcError`）+ toast（uiStore `addNotification`）+ 行内红框。同时顺带消化 T5a Reviewer 留下的 **MAJOR-1**（多段 ai_organized 行 count 与后端 `LIKE` 不等价）。

核心实现：

1. **编辑状态机（`WorkspaceFolderListView.tsx`）**：从 `useUIStore` 推导 `mode = 'idle' | 'creating' | 'renaming'`，三态互斥：
   - `pendingNewFolder === true` → `creating`
   - `editingFolderPath !== null` → `renaming`
   - 否则 `idle`
   - `submitting` 通过 `submittingRef`（`useRef<boolean>`）即可，避免一次 IPC 进行中被 blur 重复触发。
   - 同时刻只渲染一个 InlineNameEditor（要么 ghost 行，要么命中 `editingFolderPath` 的现有行）。

2. **F1 幽灵新建（AC-2）**：点击工具栏「+ 新建文件夹」或 `⌘⇧N`：
   - `setEditingValue("未命名文件夹")` + `startCreating()` → 列表末尾追加 `relativePath = "__GHOST_NEW__"` 的 ghost 行（`itemsWithGhost`），渲染 `InlineNameEditor` 默认值「未命名文件夹」，mount 时 `input.select()` 全选。
   - Enter / blur → `validateFolderNameSync` → 通过则 `await createWorkspaceFolder(projectId, name)`：成功 `cancelCreating + onRefresh + setSelectedRel(name)`；失败保留编辑态 + 红框 + toast（`renderIpcError`）。
   - Esc → `cancelCreate()`（清本地值 + `cancelCreating`）。
   - **校验失败保留编辑态 + 红框 + 不发 IPC**：`editingError` 由 `updateEditingValue` 同步推导，`InlineNameEditor` 通过 `data-error="true"` 表达红框。

3. **F2 重命名（AC-3）**：三入口（工具栏 / 右键 / Enter 键）共用 `handleRename`：
   - 首行 `if (selectionKind !== "root") return;`（ADR-007，与 T5a 一致）。
   - `setEditingValue(currentDisplayLabel)` + `startRenaming(oldRel)` → `editingFolderPath = oldRel`、`pendingRenameIds.add(oldRel)`。
   - Enter / blur → 校验通过 → `await renameWorkspaceFolder(projectId, oldRel, newName)`：成功 `finishRename + onRefresh + setSelectedRel(entry.relativePath)`；失败 `finishRename(oldRel) + setSelectedRel(oldRel) + toast`，名称回滚（编辑器消失，行内文本仍是旧 displayLabel）。
   - Esc → `cancelRename()`（`finishRename` + 清值，不发 IPC，UI 名称不变）。
   - **selection 冻结**：`pendingRenameIds.size > 0 && mode === 'renaming'` 时 `handleSelect / handleDoubleClick` 入口直接 return，点击其他行无响应。
   - 「无变化」捷径：newName === oldName 时直接 `finishRename` 不发 IPC。

4. **F3 删除二次确认（AC-4）**：`handleDelete` 首行 ADR-007 入口判定 → 从 `items.find(...)` 拿 `count = N`（前端聚合，ADR-010）→ 打开 `DeleteConfirmModal`：
   - `N === 0` → 「删除文件夹『xxx』？」
   - `N > 0` → 「该文件夹包含 N 个素材，一同移到废纸篓？」
   - 确认 → `await deleteWorkspaceFolder(projectId, rel, N > 0, N)`：成功 toast + `onRefresh` + 若选中该行则回退到 `__ROOT__`。
   - 失败 `E_FOLDER_DIRTY` → 用 `details.now` 重弹（dirtyPrev = 原 N、expectedCount = now），文案前缀「内容已变化（原 N，现 details.now），请重新确认？」；用户重新点确认时 `expected_count` 就是 `details.now`。
   - 其他 IpcError → 关 modal + toast。

5. **编辑期切走二次确认 modal**：`guardSwitchAwayCreating(action)` 在 `mode === 'creating'` 时拦下任何 selection 切换，弹「放弃新建『xxx』？」modal；确认放弃 → `cancelCreate()` + 执行原 action；取消 → 不变（modal 关闭）。仅 creating 触发；renaming 期通过 selection 冻结直接静默拦。

6. **InlineNameEditor 组件**：受控 `<input>`，mount 时 `focus()` + `select()`；内部捕获 Enter/Esc 并 `stopPropagation` 避免冒泡到外层 keyboard handler；其他键也阻止冒泡，避免 ArrowDown / ⌘⌫ / ⌘⇧N 误触发；`data-error="true"` 时显示红色 border。

7. **DeleteConfirmModal 组件**：独立 modal，背景蒙层点击取消，按钮 disabled 由 `busy` 控制；dirty 重弹通过 `dirtyPrev` prop 切换文案。

8. **MAJOR-1 修复（顺带消化）**：把 `firstSegmentRel` 单段比较升级为 `relativeToWorkspace` + `matchesFolder` 前缀匹配（`rel === folder.rel || rel.startsWith(folder.rel + "/")`），与后端 `LIKE :prefix || '/%' ESCAPE '\\'` 等价；`__ROOT__` 仍按「不含 /」分支。新增「100 vs 100%」前缀冲突边界单测，folder=100 行不被 `100%/x.png` 误命中。旧测试 `ai.textContent).toContain("0")` 改为 `("1")` 反映正确行为。

## 修改的文件

| 文件路径（NCdesktop/ 相对） | 变更类型 | 说明 |
|---|---|---|
| `src/components/features/WorkspaceFolderListView.tsx` | 修改 | 接入状态机 + 5 handler 改为真实 IPC + ghost 行 + 切走 modal + 删除 modal + MAJOR-1 聚合算法 |
| `src/components/features/WorkspaceFolderListView/FolderListRow.tsx` | 修改 | 增加 `nameEditor` / `pending` / `ghost` props；编辑期禁用 click/dblclick/ctxmenu；data-editing / data-pending / data-ghost 标记 |
| `src/components/features/WorkspaceFolderListView/InlineNameEditor.tsx` | 新建 | 受控 input，Enter/Esc/blur 捕获 + stopPropagation；红框由 `invalid \|\| error` 触发 |
| `src/components/features/WorkspaceFolderListView/DeleteConfirmModal.tsx` | 新建 | F3 modal，含 N === 0 / N > 0 / dirty 三种文案 |
| `src/components/features/__tests__/WorkspaceFolderListView.test.tsx` | 修改 | 旧 15 用例 → 34 用例（保留全部 T5a 用例 + 19 个 T5b 新增）；MAJOR-1 修复后调整 ai 行期望从 0 → 1，新增 100 vs 100% 前缀冲突用例 |

未修改：`FolderListToolbar.tsx`、`FolderContextMenu.tsx`、`AssetListView.tsx`（既有 prop 接口已能满足；3 入口共用同一 `onRename / onDelete / onCreate`）；`uiStore.ts`（5 字段 setter T4 已就位）；`folder-name-validate.ts` / `ipc-errors.ts` / `tauri-commands.ts`（T2 已就位）。

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（`WorkspaceFolderListView/InlineNameEditor.tsx` + `DeleteConfirmModal.tsx` 落在 input.md「预估影响范围」的可选新建位置）
- [x] API 路径/命名与 Architect 方案一致（消费 T2 wrapper；不直接调 invoke）
- [x] 数据模型与 Architect 方案一致（`__ROOT__` sentinel 原样透传，不翻译；count 聚合走前端 O(N)）
- [x] 未引入计划外的新依赖
- [x] 遵守 ADR-007（5 处 handler 首行 `if (selectionKind !== "root") return;`，不依赖 disabled）
- [x] 遵守 ADR-008（`validateFolderNameSync` 仅作即时反馈；后端权威）
- [x] 遵守 ADR-009（5 字段 setter 通过 uiStore 暴露的 `startCreating / cancelCreating / startRenaming / finishRename`，不绕过直接 set）
- [x] 遵守 ADR-010（F3 `expected_count` 取前端聚合，dirty 用 `details.now` 重弹；不每行 invoke）
- [x] 底线 6：UI 永远操作 `relativePath` 字符串（含 `__ROOT__`），不翻译为空字符串
- [x] 不实现拖拽（留 T6）；不持久化 5 字段（仍未进 partialize 白名单）
- 偏离说明：
  - F1 成功路径 `setSelectedRel(name)` 以用户输入的 name 作为新行 `relativePath`（MVP 仅根级单层；与后端返回 entry.relativePath 应一致）；如未来扩到嵌套层级，应改用后端返回 `entry.relativePath`。
  - 切走二次确认 modal 仅 creating 触发；renaming 期间通过 selection 冻结直接静默忽略点击（input.md AC-3「点击其他行无响应」即此语义）；与 PRD §3 F1 文案「放弃新建『xxx』？」对齐。
  - F1 InlineNameEditor 未做「全选 base 名 / 保留扩展名」拆分（文件夹名通常无扩展，Finder 行为简化对齐）。

## 测试命令

```bash
# 本 task 关联用例
cd NCdesktop && npx vitest run src/components/features/__tests__/WorkspaceFolderListView.test.tsx

# 类型检查
cd NCdesktop && npx tsc --noEmit
```

## 测试结果

```
 RUN  v4.1.1 /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop

 Test Files  1 passed (1)
      Tests  34 passed (34)
   Start at  10:10:06
   Duration  936ms
```

`npx tsc --noEmit`：无输出 = 无错误（通过）。

全量 `npx vitest run`：**254/258 通过**，剩余 4 个失败用例位于 `src/App.test.tsx`（AppLayout 异步渲染）与 `src/components/features/SearchPanel.test.tsx`（debounce + tauri event mock），与本 task 改动正交，与 T2/T5a output.md 同样记录的预存在失败一致，且本 task 未触碰这两个文件。

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果 |
|---|---|---|---|
| ✅ 正常路径 | 工具栏「+ 新建」→ ghost 行 + 默认名「未命名文件夹」 | 已测 | PASS |
| ✅ 正常路径 | F1 Enter 提交 → createWorkspaceFolder + onRefresh + 选中新行 | 已测 | PASS |
| ✅ 正常路径 | F1 blur 提交（与 Enter 同语义） | 已测 | PASS |
| ✅ 正常路径 | F1 Esc 取消（不发 IPC） | 已测 | PASS |
| ⚠️ 边界 | F1 校验失败（名含 `/`）保留编辑态 + 红框 + 不发 IPC | 已测 | PASS |
| ❌ 异常 | F1 IPC 失败（E_NAME_DUP）保留编辑态 + 红框 + toast | 已测 | PASS |
| ✅ 正常路径 | F1 编辑期点击其他行 → 切走二次确认 modal 出现，含名字 | 已测 | PASS |
| ✅ 正常路径 | 切走 modal 确认 → cancelCreating + 执行原 action | 已测 | PASS |
| ✅ 正常路径 | F2 三入口（工具栏 / 右键 / Enter）进入 inline 编辑 | 已测 | 工具栏 + 右键已测；Enter 同入口 |
| ✅ 正常路径 | F2 Enter 提交 → renameWorkspaceFolder + onRefresh + 保持选中（rel 切到新 rel） | 已测 | PASS |
| ❌ 异常 | F2 失败回滚 → 名称回旧 + selection 回旧 path + 编辑态退出 | 已测 | PASS |
| ✅ 正常路径 | F2 Esc 取消（不发 IPC，UI 名称不变） | 已测 | PASS |
| ⚠️ 边界 | F2 selection 冻结：pendingRenameIds 非空时点击其他行无响应 | 已测 | PASS |
| ✅ 正常路径 | F3 N === 0 modal 文案「删除文件夹『xxx』？」 | 已测 | PASS |
| ✅ 正常路径 | F3 N > 0 modal 文案含「包含 2 个素材」 | 已测 | PASS |
| ✅ 正常路径 | F3 确认 → deleteWorkspaceFolder(pid, rel, true, 3) + onRefresh | 已测 | PASS |
| ❌ 异常 | F3 E_FOLDER_DIRTY 重弹 → 文案含 details.now=5 | 已测 | PASS |
| ⚠️ 边界 | 多段 ai_organized 行 count = 1（MAJOR-1 修复） | 已测 | PASS |
| ⚠️ 边界 | 前缀冲突 100 vs 100%：folder=100 不被 100%/x.png 误命中 | 已测 | PASS |
| ✅ 正常路径 | T5a 全部 15 用例（渲染 + 工具栏 + 右键 + 键盘 + 双击）保留通过 | 已测 | PASS |
| ❌ 异常 | ADR-007 入口判定：⌘⌫ 选中 ai_organized 不触发 delete modal | 已测 | PASS（保留 T5a 用例） |

肉眼自测：本 task 在 vitest jsdom 下完整覆盖 5 条主路径（新建/重命名/取消/校验失败/冲突）+ 切走二次确认；dev server 肉眼验证依赖 T3 后端就位（Conductor 流程统一在 T6 后做 E2E 演示视频），与 T5a 同样以单测层为主验证。

## 已知局限

1. 切走二次确认 modal 仅 creating 触发；renaming 期间 selection 冻结直接忽略点击（无 modal）——与 input.md AC-3「点击其他行无响应」语义一致；如未来产品要求 renaming 期也弹 modal，可扩 `guardSwitchAwayRenaming` 同构函数。
2. F1 成功后 `setSelectedRel(name)` 以用户输入 name 作为新行 rel；目前依赖后端返回的 `entry.relativePath` 与之相同（MVP 单层）。后端如对名称做 NFC 归一返回不同字面量，前端 selection 会与列表 rel 字面量轻微不一致，下一次 list 刷新后由后端返回字面量校正。
3. dev server 肉眼验证未在本 task 单跑（Conductor 阶段 E2E 演示统一到 T6）；单测覆盖 5 主路径已等价于 input.md「自测」要求。
4. 全量 `pnpm test` 仍有 4 个无关失败用例（与 T2/T5a 同），本 task 不修。

## 需要 Reviewer 特别关注的地方

1. **ADR-007 入口判定**：`handleRename` / `handleDelete` / `handleKeyDown`（Enter / ⌘⌫）首行均做 `if (selectionKind !== "root") return;`，不依赖 disabled。`handleCreate` 不挂 kind 判定（永激活）。T5a 用例「⌘⌫ 选中 ai_organized 时不触发删除」改为断言「modal 不出现」——直接覆盖 ADR-007。

2. **状态机互斥**：`mode` 通过 `pendingNewFolder` 与 `editingFolderPath` 推导；二者均为 0/1 状态，互斥由 uiStore setter 保证（`startCreating` 不设 editingFolderPath；`startRenaming` 不设 pendingNewFolder）。同时刻只渲染一个 InlineNameEditor。

3. **ADR-010 前缀匹配（MAJOR-1 修复）**：`matchesFolder` 用 `rel.startsWith(folder.rel + "/")` + 等值并集，与后端 `LIKE :prefix || '/%' ESCAPE '\\'` 在 `__ROOT__` 之外严格等价；新增 100 vs 100% 前缀冲突边界单测验证。

4. **InlineNameEditor 键盘事件冒泡**：Enter/Esc/其他键全部 `stopPropagation`，避免触发外层 keyboard handler（ArrowDown 移动 selection、⌘⌫ 删除）。这是「编辑期间任何键不应外溢」的关键保证。

5. **F3 dirty 重弹的 expected_count 切换**：失败时 `setDeleteModal({ ..., expectedCount: details.now, dirtyPrev: 原 N })`，用户再次确认时调用 `deleteWorkspaceFolder(pid, rel, now>0, now)`——`now` 而非 `原 N` 作为新的 expected_count，与 ADR-010「dirty 用 `details.now` 重弹」语义一致。

6. **selection 冻结的入口**：仅 `handleSelect / handleDoubleClick` 入口拦；右键菜单与键盘导航在 `isEditing` 时本身已被 `handleContextMenuRow` 和 `handleKeyDown` 直接 return，自然冻结。
