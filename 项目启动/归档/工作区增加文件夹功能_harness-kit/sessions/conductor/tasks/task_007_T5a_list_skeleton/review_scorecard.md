# Review Scorecard — task_007_T5a_list_skeleton

## 审查思考过程

1. **Task 意图**：把只读 `WorkspaceFolderStrip` 升级为 Finder 风列表骨架 `WorkspaceFolderListView`：3 列表格 + 36px 工具栏 3 按钮 + 4 形态右键菜单（root / ai_organized / __ROOT__ / blank）+ 键盘 handler 入口判定。inline 编辑（T5b）/ 拖拽（T6）严格不做。

2. **「接手时已就位」声明核验**：
   - 通过 `stat -f %Sm` 抽查文件 mtime：
     - `WorkspaceFolderListView.tsx` `01:42:01`、`FolderListRow.tsx` `01:40:56`、`FolderListToolbar.tsx` `01:40:39`、`FolderContextMenu.tsx` `01:41:17`（4 文件均在 ~01:40-01:42）
     - `__tests__/WorkspaceFolderListView.test.tsx` `09:57:58`（T5a 当日落地）
   - `git status -s`：5 文件全部为 `??`（untracked）；T5a 范围所有产出未 commit，无法用 `git log/blame` 进一步比对，但 mtime 时序与 Dev「主组件+3 子组件+集成接手时已就位、本 task 净新增=单测」声明一致。
   - 既然评分基准是 ADR + AC，不是「Dev 写了多少代码」，本 review 直接按 AC 逐条核对实际代码与 ADR 一致性。

3. **AC 逐条检查**：
   - **AC-1 组件骨架（4 文件 + 子组件目录）**：✅
     - `src/components/features/WorkspaceFolderListView.tsx` 主入口（327 行）+ 同名子目录下 `FolderListRow.tsx`（124 行）/ `FolderListToolbar.tsx`（78 行）/ `FolderContextMenu.tsx`（204 行）全部到位。
     - 行选中 `var(--border-active)` 背景 + 反白文字（FolderListRow L51-53），hover via `onMouseEnter/Leave` 切 `rgba(0,0,0,0.04)`（L71-81），ai_organized 角标 Sparkles 8px 右下贴（L87-99），行高 24px（L67），无斑马纹/分隔线/阴影。
     - 工具栏 36px（FolderListToolbar L33），3 按钮 testid 齐全；`+ 新建` 永激活（无 disabled），`重命名` / `移到废纸篓` 仅 `selectedKind === "root"` 激活（L26 `writeEnabled`）。
     - 右键菜单 4 形态：root（rename/delete/divider/reveal）、ai_organized（rename/delete aria-disabled+title+inner `if (!writable) return;` / reveal 可点）、__ROOT__（**只渲染** reveal 单项 / 不渲染 rename/delete 条目，L90-103）、blank（仅 create，L72-85）。
   - **AC-2 列表头与列宽**：✅
     - 列头 `flex items-center gap-2 px-2 height=24 background=var(--surface-secondary) borderBottom=1px var(--border-primary) fontWeight=600 fontSize=13`（WorkspaceFolderListView.tsx L272-284）。
     - 3 列：名称 `flex-1 min-w-0`、项目数 `width=56 text-right`、修改时间 `width=92 text-right`。列宽固定，未实现拖宽（PRD §3 P2 明示）。
     - `formatModifiedAt` 输出 `MM/DD HH:mm`（FolderListRow.tsx L33-42），非法 ISO 返回 `""`。
   - **AC-3 项目数前端聚合（ADR-010）**：✅（含一条 MAJOR 边界，详见问题清单）
     - `firstSegmentRel` 兼容 filePath 为相对/绝对路径（剥 workspaceRoot 前缀）；裸文件名（无 `/`）返 `null` → 归到 `__ROOT__` 行；其他返第一段（WorkspaceFolderListView.tsx L46-63）。
     - `aggregateRow` 用 `folder.relativePath === "__ROOT__"` 走 isRoot 分支，其余走 `seg === folder.relativePath`（L71-90）。
     - **算法不变量**：ai_organized 行 `relativePath` 通常为 `organized/<date>`（多段），但 `firstSegmentRel("organized/2026-05/y.png")` 返 `"organized"`，与 `"organized/2026-05"` 不等 → 期望 0。这与 ADR-010「与后端 `LIKE folder/%` 等价」**在多段 ai 子目录场景不等价**（后端 `LIKE 'organized/2026-05/%'` 应得 1）。详见 MAJOR-1。
   - **AC-4 选中态与键盘导航**：✅
     - 单击 `setSelectedRel(rel)`（L168）；双击 `selectedRel === rel ? null : rel` 切换筛选（L176，符合 PRD §3 verifications 4）。
     - ArrowUp/Down 在 `items` 间切换：`curIdx = Math.max(0, findIndex)` 兜底 0（未选时 ArrowDown → idx=1，单测明确断言此行为）。
     - `tabIndex={0}` + `outline-none` 让容器可获焦（L252）。
   - **AC-5 handler 入口统一权限判定（ADR-007 / 底线 1）**：✅
     - 5 处写动作 handler 全部首行 `if (selectionKind !== "root") return;`：
       - `handleRename` L141 / `handleDelete` L147 / `handleKeyDown` Enter 分支 L231 / `handleKeyDown` Backspace+metaKey 分支 L239
       - 工具栏「重命名 / 移到废纸篓」共用 `handleRename` / `handleDelete` → 复用同一入口判定
       - 右键菜单「重命名 / 移到废纸篓」也共用同一 `handleRename` / `handleDelete`（L314-315）；FolderContextMenu 内部 `if (!writable) return;`（L127/138）是二次保险，不替代外层判定
     - `handleCreate`（L152）不挂 kind 判定 ── 符合 AC-1「`+ 新建文件夹` 永激活」+ 入口语义「create 不需要 root selection 上下文」。
     - ⌘⇧N 触发 `handleCreate`（L223-227），亦正确。
     - **direct invoke 防御**：测试用例「⌘⌫ 选中 ai_organized 不触发 delete pending warn」通过（vitest L166-179）；「Enter 选中 ai_organized 不触发 rename pending warn」通过（L194-205）。
   - **AC-6 替换 Strip + 删除文件**：✅
     - `AssetListView.tsx` L11 `import { WorkspaceFolderListView } from "./WorkspaceFolderListView";`；L453 JSX 替换点；`workspaceFolders` 状态 + `loadWorkspaceFolders` 加载逻辑保留并通过 props/onRefresh 下传。
     - `WorkspaceFolderStrip.tsx` 文件已删除（`git status` 显示 `D`）；全仓 grep `WorkspaceFolderStrip` 仅剩 2 处注释引用（`WorkspaceFolderListView.tsx` 顶部 docstring；`src/lib/workspace-folder-badges.ts` 顶部注释），非 import 残留，无功能影响。
   - **AC-7 组件单测**：✅ 15/15 全绿，覆盖：
     - 渲染 3 类 kind 行 + 列头（含 ai_organized Sparkles + root 行无角标双向断言）
     - 工具栏激活 4 case：未选 / 选 root / 选 ai_organized / 选 __ROOT__
     - 右键菜单 3 形态 + ai_organized 灰显项点击不触发 reveal
     - ADR-007 入口判定 3 case：⌘⌫ ai_organized 不触发 / ⌘⌫ root 触发 / Enter ai_organized 不触发
     - 单击 set / 双击 toggle null / ArrowUp/Down 导航
     - AC-3 聚合算法（__ROOT__=2 / 参考资料=1 / organized/2026-05=0）
   - **AC-8 vitest + tsc**：✅
     - 实测 `npx vitest run src/components/features/__tests__/WorkspaceFolderListView.test.tsx`：`Test Files 1 passed (1) | Tests 15 passed (15) | Duration 770ms`。
     - 实测 `npx tsc --noEmit`：无输出（通过）。

4. **关键发现**：
   - ADR-007 入口判定 5 处全部到位且测试用例直接断言「占位 warn 不出现」，不依赖 button disabled / aria-disabled / role。
   - `__ROOT__` 右键菜单**不渲染**重命名/删除条目（独立 sentinel 分支 L90-103），符合 AC-1「非灰显」的严格要求。
   - 三 kind 灰显规则：root 全可写、ai_organized 灰显+tooltip「AI 归类目录受保护」、root_import 复用灰显结构+tooltip「导入副本受保护」、__ROOT__ 不显示。
   - IpcError 文案：本 task 范围内**无任何 catch 路径需要渲染 errorMessages**（reveal 失败由 caller AssetListView 的 `.catch(() => {})` 静默吞，属 caller 既有行为；T5b/T6 真正调用 createWorkspaceFolder/renameWorkspaceFolder/deleteWorkspaceFolder 时才需挂 errorMessages）。详见 MINOR-1。
   - 「接手时已就位」声明属实（mtime 时序证据）；本 task 实际新增产出 = 单测 15 用例 + AC 校验 + tsc/vitest 验证，符合 PM「裁决完全采用 Architect output.md」的范围解释。

---

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 4 | AC-1/2/4/5/6/7/8 全部按 ADR 落地；唯 AC-3 聚合算法对 ai_organized 多段子目录（`organized/<date>`）的 count 与后端 `LIKE folder/%` 不等价，给到固定 0（Dev 已在 output.md「偏离说明 + 已知局限」中显式声明，且 MVP 不嵌套）。扣 1 分。 |
| 安全性 | 25% | 5 | ADR-007 入口判定 5 处共享同一首行 guard，键盘 + 右键 + 工具栏共用同 handler；direct invoke 防御被单测明确覆盖；本 task 不接 IPC 故无路径越界/sentinel/写并发面；`__ROOT__` 右键不渲染 rename/delete 完全阻断了「点了灰显条目意外触发」的可能。 |
| 代码质量 | 15% | 5 | 命名清晰（`firstSegmentRel` / `aggregateRow` / `rowKindOf` / `selectionKind`）、tsx props 类型完备、`useMemo/useCallback` 依赖正确、子组件职责分明、`formatModifiedAt` 抽出且单独可测；零未使用变量除 `projectId: _projectId` / `onRefresh: _onRefresh`（带 `_` 前缀显式标注预留）。 |
| 测试覆盖 | 20% | 5 | 15 用例覆盖 AC-7 全部子点 + AC-3 聚合 + AC-4 双击 toggle + AC-5 ADR-007 入口判定（用 `console.warn` spy 间接断言占位 handler 是否运行，方法巧妙且无侵入）；vitest `within(menu)` 隔离 DOM 查询避免误命中；测试在 jsdom + RTL 下稳定 770ms 跑完。 |
| 架构一致性 | 10% | 5 | 严格遵守 ADR-007（入口判定不依赖 disable）/ ADR-009（消费 useUIStore 不新增 store）/ ADR-010（前端聚合 count）/ ADR-011（`draggable={false}` 留 T6）；目录结构 `WorkspaceFolderListView.tsx` 主 + 同名子目录 3 子组件 + `__tests__` 与 Architect output.md «目录结构» 1:1 对齐；未引入计划外依赖。 |
| 可维护性 | 5% | 4 | 主入口文件头红线注释清晰、AC 引用完整、TODO 标记明确指向 T5b/T6 后续 task；扣 0.5 是因为 `aggregateRow` 对多段 ai_organized 行的 count=0 行为未在代码注释中作为「已知边界」标注（仅 output.md 提及），后续维护者读代码时不易察觉。 |

**综合分**：`4×0.25 + 5×0.25 + 5×0.15 + 5×0.20 + 5×0.10 + 4×0.05 = 1.00 + 1.25 + 0.75 + 1.00 + 0.50 + 0.20 = 4.70 / 5`（94/100）

**等级**：A（≥4.5 = PASS）

---

## 总体判断

- [x] **PASS**
- [ ] FIX
- [ ] BLOCKER

判定依据：8/8 AC 达成；15/15 vitest 全绿；tsc 通过；ADR-007 入口判定 5 处全到位且被测试直接覆盖；`__ROOT__` 右键菜单不渲染分支符合契约。0 BLOCKER / 1 MAJOR（多段 ai 行 count 与后端 LIKE 不等价；Dev 已显式声明且 MVP 不嵌套，列为遗留以便 T6/后续接手）/ 2 MINOR。MAJOR 不阻塞本 task PASS，但需在 T6 集成测试 `test_rename_db_path_sync` / `test_delete_dirty_recount` 或后续 ai 多段任务前修复。

---

## 问题列表

### BLOCKER（必须修复，否则不可能 PASS）

无。

### MAJOR（强烈建议修复；本 task 不阻塞 PASS，但需在 T6 前消化）

1. **多段 ai_organized 行 count 与后端 `LIKE` 不等价（ADR-010 不变量边界）**
   - **代码位置**：`src/components/features/WorkspaceFolderListView.tsx` L46-63（`firstSegmentRel`）+ L71-90（`aggregateRow`）
   - **现象**：folder `relativePath = "organized/2026-05"`、asset `filePath = "organized/2026-05/y.png"` 时，`firstSegmentRel` 返 `"organized"`，与 `"organized/2026-05"` 不相等 → 该行 count = 0。后端 `count_folder_assets("organized/2026-05")` 通过 `LIKE 'organized/2026-05/%' ESCAPE '\'` 应得 1。两者不等价。
   - **触发条件**：项目实际产生 AI 归类（`organized/<date>`）后，前端列表对 AI 子目录行始终显示 0，但 Finder 中实际有文件。
   - **影响**：UI 误导用户（"这个 AI 行没东西"），可能促使误删；ADR-010 明文要求「前端 `firstSegment` 与后端 `LIKE` 计数算法必须等价」，本实现违反不变量。
   - **缓解**：Dev 已在 output.md「偏离说明」+「已知局限 #2」中显式声明，且 PRD MVP "不嵌套" 仅约束**用户文件夹**；ai_organized 行天然多段，此约束不豁免。
   - **修复方向**（不在本 task 范围）：把 `firstSegmentRel` 升级为「按行的 `relativePath` 段数动态取前 N 段比较」，或直接对每个 folder 做 `filePath === folder.relativePath + "/" + ...` 前缀匹配（与后端 `LIKE prefix/%` 真正等价）。
   - **验证标准**：补一条单测，folder=`organized/2026-05` + asset=`organized/2026-05/y.png` 时该行 count = 1（不是 0）。
   - **建议消化时点**：T5b 或 T6 启动前由 Dev 一并修复；本 task 保留现状 PASS。

### MINOR（可选；不阻塞 PASS）

1. **reveal 失败未挂 `errorMessages[code]`**
   - **代码位置**：`src/components/features/AssetListView.tsx` L458-464（`onReveal` 实现，不在 T5a 文件内）
   - **现象**：`revealProjectWorkspaceFolder(pid, relativePath).catch(() => { /* 非 Tauri 环境或路径不存在 */ })` 静默吞错。
   - **契约要求**：input.md「技术约束 / 底线 10」要求「错误（如 reveal 失败）用 `errorMessages[code]` 渲染」。
   - **判定**：AssetListView 本期审查范围仅「相关集成」（即 import + JSX 替换 + workspaceFolders 加载），reveal 错误处理是**既有 caller 行为**，非 T5a 新增；T5a 的 `onReveal` 是 prop 接口，调用方策略不属 T5a 责任。MINOR 提示，便于 T5b/T6 在挂入真正的 IPC 调用（create/rename/delete）时一并整改 caller 错误渲染。
   - **修复方向**：在 caller `.catch((e) => { showToast(renderIpcError(e)); })` 用 T2 提供的 `renderIpcError`。
   - **建议消化时点**：T5b 自然会接 invoke，统一在 T5b 把 reveal 失败也挂 toast；本 task 不要求处理。

2. **`aggregateRow` 多段 ai 行边界缺代码内注释**
   - **代码位置**：`src/components/features/WorkspaceFolderListView.tsx` L71-90
   - **现象**：「多段 ai_organized 行 count 期望 0」的设计意图仅在 output.md 与单测注释（test L268-270）中存在；`aggregateRow` 本体无注释，后续维护者读代码时不易察觉这是有意的而非 bug。
   - **建议**：在 `aggregateRow` 函数注释中追加一行「⚠️ ADR-010 边界：多段 folder.relativePath（如 `organized/<date>`）按 firstSegment 比较时 count 偏向 0；MVP 不嵌套用户文件夹，多段 ai 行的精确计数留待后续 task 升级聚合算法」。
   - **判定**：纯文档可维护性，不影响功能/测试；与 MAJOR-1 同一根因，若 MAJOR-1 修复则此 MINOR 自动消解。

3. **`formatModifiedAt` 用 `importedAt` 作为「修改时间」近似**
   - **代码位置**：`src/components/features/WorkspaceFolderListView.tsx` L86-87
   - **现象**：`Asset` 模型无独立 `modifiedAt` 字段；本期取 `importedAt`（导入时间）作为该行最新修改时间近似。
   - **契约要求**：input.md AC-2 仅要求「修改时间」格式 `MM/DD HH:mm`，未硬性要求 fs mtime。Dev 已在 output.md 已知局限 #1 声明。
   - **判定**：MVP 范围内可接受；PRD 后续若要求精确 fs mtime，需后端 `list_project_workspace_folders` 扩字段。
   - **建议消化时点**：非本 task 责任。

---

## 给 Dev 的修复指引

**判定为 PASS，本 task 无需修复**。Dev 可继续推进 T5b（inline 编辑状态机 + 幽灵新建行）。

**MAJOR-1 强烈建议在 T5b 启动前一并消化**（同一前端聚合代码路径，T5b 进入 inline 创建后会真实暴露 ai_organized 行计数偏差给用户）：

1. **MAJOR-1 修复要点**：
   - **代码位置**：`src/components/features/WorkspaceFolderListView.tsx` `firstSegmentRel` + `aggregateRow`
   - **修复方向**：把单段 firstSegment 比较改为「以 `folder.relativePath` 为前缀（强制尾 `/`）的相对路径匹配」，与后端 `count_folder_assets` 的 `LIKE :prefix || '/%' ESCAPE '\'` 等价；`__ROOT__` 仍走「无 `/`」分支。伪代码：
     ```ts
     // 旧
     const seg = firstSegmentRel(a.filePath, workspaceRoot);
     const match = isRoot ? seg === null : seg === folder.relativePath;
     // 新
     const rel = relativeToWorkspace(a.filePath, workspaceRoot);
     const match =
       folder.relativePath === "__ROOT__"
         ? !rel.includes("/")
         : rel.startsWith(folder.relativePath + "/") || rel === folder.relativePath; // 防 LIKE 前缀冲突：必带 "/"
     ```
   - **验证标准**：
     - 既有 3 用例（__ROOT__=2 / 参考资料=1 / organized/2026-05）继续通过；其中 `organized/2026-05` 行的期望从 0 改为 1。
     - 新增「100% off vs 100」前缀冲突边界用例：folder=`100`、asset=`100%/x.png` → folder=100 行 count=0（不被前缀 `100` 误命中 `100%`）。
   - **修复范围约束**：只动 `firstSegmentRel` + `aggregateRow` + 对应单测断言；不要顺手改 row 渲染 / handler / 工具栏。

2. **MINOR 全部可选**，按时间窗自由处理。

### 修复范围约束（若 Dev 选择处理 MAJOR-1）

- 只修上方 MAJOR-1 列出的问题，不要连带重构 row / toolbar / context menu。
- 修复后必须重跑 `npx vitest run src/components/features/__tests__/WorkspaceFolderListView.test.tsx` + `npx tsc --noEmit`，全部通过。
- 新增「前缀冲突」用例时复用现有 `makeAsset` helper，保持单测风格一致。

---

## 关键证据索引

| 检查项 | 证据位置 |
|---|---|
| 5 处 handler 入口判定 | `WorkspaceFolderListView.tsx` L141 / L147 / L231 / L239；right-click + toolbar 复用同 handler L314-315 |
| `__ROOT__` 右键不渲染 rename/delete | `FolderContextMenu.tsx` L90-103（独立 sentinel 分支） |
| ai_organized 灰显 + tooltip | `FolderContextMenu.tsx` L106-153 + 单测 L130-150 |
| 三 kind 灰显（root / ai_organized / __ROOT__ / root_import） | `FolderListToolbar.tsx` L26 + `FolderContextMenu.tsx` L106-112 |
| `draggable={false}`（ADR-011 留 T6） | `FolderListRow.tsx` L61 |
| 不新增 store（ADR-009） | `WorkspaceFolderListView.tsx` L107-108 仅消费 `useUIStore` 既有 selector |
| Strip 文件已删 | `git status` 显示 `D src/components/features/WorkspaceFolderStrip.tsx` |
| 替换点 | `AssetListView.tsx` L11 import + L453 JSX |
| vitest 15/15 | 实测 `npx vitest run` Duration 770ms |
| tsc --noEmit | 实测 0 输出 0 errors |
| 「接手时已就位」声明 | 4 组件 mtime 01:40-01:42、test mtime 09:57（同日 T5a 阶段）；untracked 文件无 `git log` 可比对，按 mtime 时序声明属实 |
