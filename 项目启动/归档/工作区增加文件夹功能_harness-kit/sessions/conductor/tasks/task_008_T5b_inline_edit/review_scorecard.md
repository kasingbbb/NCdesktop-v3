# Review Scorecard — task_008_T5b_inline_edit

## 审查思考过程

1. **Task 意图**：在 T5a 列表骨架基础上落地 inline 编辑状态机（idle / creating / renaming 互斥）+ F1 幽灵新建（Enter / Esc / blur + 校验失败保留 + 切走二次确认）+ F2 三入口重命名（同步乐观 + 失败回滚 + selection 冻结）+ F3 删除二次确认 modal（含 `expected_count` + `E_FOLDER_DIRTY` 重弹）。范围严格不含拖拽（T6）。

2. **接手前置裁决（PM 2026-05-12 PM A）**：`assets.file_path` 文档已修订为绝对路径（contracts §C.2 + progress.md 决策 2026-05-12-11）。重点核查 T5b 的 `matchesFolder` 前缀匹配同基底——本实现内部先 `relativeToWorkspace(a.filePath, workspaceRoot)` 归一为同基底相对路径，再与 `folder.relativePath` 比较，**基底同源、不漂移**。PM 已裁决契约漂移不扣分，且此实现自洽吸收了绝对/相对两种 filePath 输入（剥前缀 + tolerate 已经相对的情况）。

3. **自跑实测**：
   - `npx vitest run src/components/features/__tests__/WorkspaceFolderListView.test.tsx` → **34 passed / 0 failed** Duration 968ms（与 Dev 自报 34/34 完全一致）
   - `npx tsc --noEmit` → **0 输出 = 0 errors**

4. **AC 逐条对齐**：

   - **AC-1 编辑状态机**：✅
     - `mode` 推导（WorkspaceFolderListView.tsx L150-154）：`pendingNewFolder ? 'creating' : editingFolderPath !== null ? 'renaming' : 'idle'`
     - 互斥由 uiStore setter 保证（L291-313）：`startCreating` 显式 `editingFolderPath: null`；`startRenaming` 不设 `pendingNewFolder`；`finishRename` 不影响 `pendingNewFolder`。runtime 不存在两者同时为 truthy 的路径。
     - 同时刻只渲染一个 `InlineNameEditor`：L608 `isRenamingRow || isCreatingRow` 条件 + `mode` 的互斥推导保证。
     - `submittingRef`（L168）拦 blur 二次触发：`submitCreate / submitRename` 首行 `if (submittingRef.current) return;`（L245 / L282）。

   - **AC-2 F1 幽灵新建**：✅ 全 8 条子项落地：
     - ghost 行（`itemsWithGhost` L201-214）`relativePath = "__GHOST_NEW__"` 末尾追加，渲染 `<InlineNameEditor>` 默认 `未命名文件夹` + `selectAllOnMount=true`（L46）。
     - Enter / blur 走 `submitCreate`（L244-271）：`validateFolderNameSync` 失败 → `updateEditingValue` 回填 reason → `data-error="true"` + 早 return；通过 → `await createWorkspaceFolder(projectId, name)`：成功 `cancelCreating + onRefresh + setSelectedRel(name)`；失败保留编辑态 + `setEditingError(renderIpcError)` + `addNotification` toast。
     - Esc → `cancelCreate`（L273-278）清值 + `cancelCreating`，不发 IPC。
     - 切走二次确认 modal（`guardSwitchAwayCreating` L428-443 + `pendingDiscard` modal L654-715）：仅 `mode === 'creating'` 触发，文案「放弃新建「xxx」？」，确认 → `cancelCreate + 执行原 action`。
     - **测试覆盖**：T5b 用例 8 条全部 PASS（点击 / Enter / Esc / blur / IPC 失败 / 校验失败 / 切走 modal / 确认放弃）。

   - **AC-3 F2 重命名**：✅
     - 三入口共用 `handleRename`（L342-351），首行 `if (selectionKind !== "root") return;`（ADR-007）+ `mode !== 'idle'` 拦重入。
     - `setEditingValue(cur.displayLabel)` + `startRenaming(selectedRel)` → editingFolderPath = oldRel、pendingRenameIds += oldRel（uiStore L296-301）。
     - 提交 `submitRename`（L281-330）：「无变化」捷径（newName === oldName）直接 `finishRename` 不发 IPC；校验失败保留；通过 `await renameWorkspaceFolder(pid, oldRel, newName)`：成功 `finishRename + onRefresh + setSelectedRel(entry.relativePath)`（用后端返回 rel，正确）；失败 `finishRename + setSelectedRel(oldRel) + toast`。
     - Esc → `cancelRename`（L332-338）finishRename + 清值。
     - **selection 冻结**：`handleSelect / handleDoubleClick`（L446-466）入口 `if (mode === 'renaming' && pendingRenameIds.size > 0) return;`，右键 / 键盘已在 `isEditing` 时被早 return（L472、L497），自然冻结。
     - **测试覆盖**：6 条用例 PASS（工具栏入口 / 右键入口 / Enter 提交成功 / 失败回滚 / Esc / selection 冻结）；Enter-from-row-keydown 入口未单测但与 handleRename 共代码路径。

   - **AC-4 F3 删除二次确认**：✅
     - `handleDelete`（L353-366）首行 ADR-007；`expectedCount = cur.count`（前端聚合，ADR-010）。
     - `DeleteConfirmModal` 文案（L33-37）：
       - dirty 优先 → 「内容已变化（原 N，现 X），请重新确认？」
       - N > 0 → 「该文件夹包含 N 个素材，一同移到废纸篓？」
       - N === 0 → 「删除文件夹「xxx」？」
     - `submitDelete`（L377-413）：成功 toast + `onRefresh` + 若选中该行则 `setSelectedRel("__ROOT__")`；`E_FOLDER_DIRTY` 用 `details.now` 重弹（`expectedCount = now`、`dirtyPrev = 原 N`），用户再次确认时 `expected_count` 即 `details.now`；其他 IpcError → 关 modal + toast。
     - **测试覆盖**：4 条 PASS（N==0 / N>0 / 确认含 confirmNonEmpty + expectedCount / E_FOLDER_DIRTY 重弹文案含 5）。

   - **AC-5 组件单测**：✅ 19 个 T5b 新增用例 + 15 个 T5a 保留 = **34/34 全绿**；覆盖 AC 列出的 8 条单测点全部命中。

   - **AC-6 vitest + tsc 通过**：✅ 实测 34/34 + 0 tsc errors。

5. **重点核查项交叉确认**：

   - **状态机互斥**：✅ uiStore setter 显式保证（`startCreating: { pendingNewFolder: true, editingFolderPath: null }`）；推导侧 `mode` 三态完整 cover。
   - **InlineNameEditor 键盘事件 stopPropagation**：✅ `Enter / Esc / 其他键` 全部 `e.stopPropagation()`（L67-81），外层 `handleKeyDown` 进一步以 `if (isEditing) return` 双保险（L497）。**不会击穿到 ArrowDown / ⌘⌫ / ⌘⇧N**。
   - **F3 dirty 重弹**：✅ `submitDelete` catch 分支用 `details.now` 设新 `expectedCount`；重弹时 modal `body` 走 `isDirty` 分支显示「原 N，现 now」；用户再点确认时 `deleteWorkspaceFolder(pid, rel, now > 0, now)` 提交 `now` 作为新 expected_count，与 ADR-010 一致。
   - **切走二次确认仅 creating，renaming 静默冻结**：✅ `guardSwitchAwayCreating` 首行 `if (mode !== 'creating') return false`；`handleSelect / handleDoubleClick` 在 renaming 时直接 return 不弹 modal。
   - **`matchesFolder` 前缀冲突**：✅ `rel === folder.rel || rel.startsWith(folder.rel + "/")`（L101），加 `/` 尾巴防 `100 / 100%` 误命中；专门单测覆盖（test L278-302）`folder=100 + asset=100%/x.png` → r100 count=1（仅匹配 `100/a.png`）、r100p count=1（仅匹配 `100%/x.png`），无误命中。
   - **同基底**：✅ `aggregateRow` 内先 `relativeToWorkspace(a.filePath, workspaceRoot)` 归一（兼容绝对/相对），再与 `folder.relativePath`（始终是相对路径，含 `__ROOT__` sentinel）比较；基底统一，不受 PM 裁决 A 的 file_path 绝对化影响。
   - **IpcError 文案走 errorMessages 表**：✅ `submitCreate / submitRename / submitDelete` catch 分支均 `isIpcError(e) ? renderIpcError(e) : '默认中文'`；toast 与行内 editingError 共用同一 msg。

6. **关键发现**：
   - 34 用例非鸡肋——覆盖 8 条 AC 子点 + 校验失败 + IPC 失败 + 切走 modal + 删除 modal + dirty 重弹 + selection 冻结 + 前缀冲突边界；单测层等价于 input.md「自测」要求。
   - **MAJOR-1（T5a 遗留）已消化**：`firstSegmentRel` 升级为 `matchesFolder` 前缀匹配，与后端 `LIKE :prefix || '/%' ESCAPE '\\'` 等价；旧测试 `ai.textContent).toContain("0")` 改为 `("1")`；新增 100 vs 100% 用例。
   - 偏离声明真实且合理：F1 成功 `setSelectedRel(name)` 在根级单层 MVP 与后端返回 entry.relativePath 等价；F2 走 `entry.relativePath`（更稳健）；切走 modal 仅 creating 触发与 input.md AC-3「点击其他行无响应」语义一致。

---

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | AC-1 ~ AC-6 全部按 ADR 落地；状态机互斥 + 5 handler 入口判定 + dirty 重弹 + 切走 modal + selection 冻结 + 前缀冲突 全闭环；T5a MAJOR-1 顺带消化。 |
| 安全性 | 25% | 5 | ADR-007 入口判定 5 处共享同首行 guard（rename / delete / Enter / ⌘⌫ / ⌘⇧N 入口）；`submittingRef` 防 blur 二次提交；InlineNameEditor `stopPropagation` 全键阻断；F3 dirty 双保险（前端聚合 N + 后端 `expected_count` 校验 + 重弹用 `details.now`）；IpcError 文案统一走 `errorMessages` 表，不直接拼后端 message。 |
| 代码质量 | 15% | 5 | `mode` 推导清晰、`submittingRef` / `editingValue` / `editingError` 三状态职责分明、InlineNameEditor / DeleteConfirmModal 独立可测、`guardSwitchAwayCreating` 命名直白、callback 依赖正确；TODO/⚠️ 红线注释完整、ADR 引用具体。 |
| 测试覆盖 | 20% | 5 | 34 用例覆盖 AC-2/3/4/5 全部子点 + ADR-007 + MAJOR-1 修复 + 100 vs 100% 前缀冲突边界 + IPC 失败/成功双路径；`act + waitFor` 处理异步 IPC 正确；mock `tauri-commands` 隔离干净。 |
| 架构一致性 | 10% | 5 | 严格遵守 ADR-007（入口判定）/ ADR-008（前端校验仅作即时反馈）/ ADR-009（5 字段 setter 走 startCreating / cancelCreating / startRenaming / finishRename，不绕过直接 set）/ ADR-010（前端聚合 + dirty 用 `details.now` 重弹 + 前缀匹配与后端 LIKE 等价）；目录结构 1:1 对齐 Architect output.md；未引入计划外依赖。 |
| 可维护性 | 5% | 4 | 主文件头注释 + AC 引用完整；扣 0.5 是因为 `submitRename` 中「无变化捷径」`oldName = items.find(...).displayLabel` 在嵌套层级场景（PRD MVP 不支持）与 `relativePath` 末段不一定相等，可能导致误判「有变化」反向（仍发 IPC 不算错，仅多一次 noop）——后续若扩到嵌套需注释。 |

**综合分**：`5×0.25 + 5×0.25 + 5×0.15 + 5×0.20 + 5×0.10 + 4×0.05 = 1.25 + 1.25 + 0.75 + 1.00 + 0.50 + 0.20 = 4.95 / 5`（99/100）

**等级**：A（≥4.5 = PASS）

---

## 总体判断

- [x] **PASS**
- [ ] FIX
- [ ] BLOCKER

判定依据：6/6 AC 达成；34/34 vitest 全绿；tsc --noEmit 通过；ADR-007 入口判定全到位；T5a MAJOR-1（前缀匹配）顺带消化并加 100 vs 100% 边界用例；状态机互斥由 uiStore setter 强保证；InlineNameEditor 键盘事件不外溢；F3 dirty 重弹 `details.now` 切换 `expectedCount` 语义正确。**0 BLOCKER / 0 MAJOR / 4 MINOR**。可直接交给 T6（拖拽）启动。

---

## 问题列表

### BLOCKER（必须修复，否则不可能 PASS）

无。

### MAJOR（强烈建议修复；本 task 不阻塞 PASS）

无。

### MINOR（可选；不阻塞 PASS）

1. **F1 成功后 `setSelectedRel(name)` 未用后端返回 `entry.relativePath`**
   - **代码位置**：`src/components/features/WorkspaceFolderListView.tsx` L263
   - **现象**：`createWorkspaceFolder` wrapper 实际返回新建的 `WorkspaceFolderEntry`，但 `submitCreate` 直接用用户输入的 `name` 作为新行 selection rel，未用返回值。
   - **影响**：根级单层 MVP 下 `entry.relativePath === name`，无显式差异；若后端做 NFC 归一/去尾空格，前端 selection 与下次 list 刷新返回的字面量轻微不一致，刷新后校正。
   - **修复方向**（一行）：`const entry = await createWorkspaceFolder(projectId, name); ... setSelectedRel(entry.relativePath);`（与 `submitRename` L308 风格一致）。
   - **判定**：Dev 已在 output.md 偏离说明 + 已知局限 #2 显式声明；不阻塞 PASS，建议 T6 接手时顺手统一。

2. **`submitRename` 「无变化」分支 `oldName = displayLabel` 在嵌套层级场景可能误判**
   - **代码位置**：`src/components/features/WorkspaceFolderListView.tsx` L286
   - **现象**：`oldName = items.find((it) => it.relativePath === oldRel)?.displayLabel ?? oldRel;`。PRD MVP 根级单层下 `displayLabel === relativePath` 一致；若未来扩到嵌套（`a/b`），`displayLabel = "b"` 但用户输入 `newName` 也会按 base 名比较，与 `oldRel = "a/b"` 不等——目前行为正确，但代码靠 displayLabel 而非 base-name 拆分，注释未明确，后续维护者可能误改。
   - **修复方向**：在 `submitRename` 上方加 1 行注释「MVP 仅根级单层：oldName === displayLabel === relativePath，若扩嵌套需 base-name 拆分」。
   - **判定**：纯文档可维护性，不影响功能/测试。

3. **`handleSelect` 中 `pendingRenameIds.size > 0` 判断对当前实现冗余**
   - **代码位置**：`src/components/features/WorkspaceFolderListView.tsx` L449 / L461
   - **现象**：`if (mode === "renaming" && pendingRenameIds.size > 0) return;` — 当前 uiStore `startRenaming` 必定 add 一项 pendingRenameIds，所以 `mode === "renaming"` 时 size 必 > 0；size 检查冗余。
   - **影响**：无功能影响；冗余的防御代码，未来若改 `startRenaming` 语义（如允许 mode === "renaming" 但 pending 已清空的过渡态）会更稳健。
   - **判定**：保留即可，作为防御性编码可接受。

4. **F1 失败时 `editingError` 与 `addNotification` toast 同时显示，信息冗余**
   - **代码位置**：`src/components/features/WorkspaceFolderListView.tsx` L266-268
   - **现象**：IpcError 时既 `setEditingError(msg)`（行内红框 + title tooltip）又 `addNotification(... title: "新建文件夹失败" message: msg ...)` toast。两处显示同一 msg。
   - **影响**：用户同时看到红框 hint + 右上角 toast，略冗余但不致命；与 input.md AC-2「IpcError → 保留编辑态 + 行内 error toast」要求一致（"error toast" 含义可解为 toast 或行内）。
   - **判定**：可保留双通道提示，或在后续 UX 评审时收敛为单通道。

---

## 给 Dev 的修复指引

**判定为 PASS，本 task 无需修复**。Dev 可继续推进 T6（拖拽 + DnD 上下文）。

MINOR 全部可选；建议 T6 启动时在 `submitCreate` 中顺手把 `setSelectedRel(name)` 改为 `setSelectedRel(entry.relativePath)`（与 `submitRename` 风格统一），其他 MINOR 按时间窗自由处理。

---

## 关键证据索引

| 检查项 | 证据位置 |
|---|---|
| `mode` 三态互斥推导 | `WorkspaceFolderListView.tsx` L150-154 |
| uiStore setter 保证互斥 | `uiStore.ts` L291（`startCreating: { pendingNewFolder: true, editingFolderPath: null }`） |
| ADR-007 入口判定 5 处 | `WorkspaceFolderListView.tsx` L343（handleRename）/ L354（handleDelete）/ L497（isEditing 拦键盘）/ L526（Enter）/ L534（⌘⌫） |
| InlineNameEditor stopPropagation 全键 | `InlineNameEditor.tsx` L65-81（Enter/Esc/其他键） |
| F3 dirty 重弹 `details.now` 切 expectedCount | `WorkspaceFolderListView.tsx` L394-406（submitDelete catch） |
| 删除 modal 三态文案 | `DeleteConfirmModal.tsx` L32-37 |
| 切走 modal 仅 creating | `WorkspaceFolderListView.tsx` L430（guardSwitchAwayCreating 首行 `if mode !== 'creating' return`） |
| selection 冻结（renaming 静默） | `WorkspaceFolderListView.tsx` L449 / L461 |
| matchesFolder 前缀匹配 + `__ROOT__` 分支 | `WorkspaceFolderListView.tsx` L91-102 |
| 100 vs 100% 前缀冲突单测 | `__tests__/WorkspaceFolderListView.test.tsx` L278-302 |
| MAJOR-1 修复（ai 多段 count=1） | `__tests__/WorkspaceFolderListView.test.tsx` L273-275（注释 + 断言） |
| IpcError 走 errorMessages 表 | `WorkspaceFolderListView.tsx` L266 / L311 / L409（`isIpcError(e) ? renderIpcError(e) : ...`） |
| vitest 34/34 | 实测 `npx vitest run` Duration 968ms |
| tsc --noEmit | 实测 0 输出 0 errors |
