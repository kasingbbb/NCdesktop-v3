# Review Scorecard — task_004_T2_frontend_ipc

## 审查思考过程

1. **Task 意图**：搭建前端 IPC 与错误层 — IpcError TS 类型、11 项错误码闭集、parseIpcError/invokeWithIpcError、11 项中文文案渲染表、5 个 camelCase wrapper、folder-name 同步校验。所有字面量、payload key、details schema、文案渲染语义必须与 T0 contracts.md §A.2/§A.4/§B.2/§D 字符级一致。

2. **AC 逐条检查**：
   - **AC-1 类型补全（IpcErrorCode 联合 / IpcError / DeleteReport）**：✅
     - `src/types/workspace.ts` L16–27 与 §A.2 11 项字面量字符级一致；L34–38 IpcError shape；L41–43 DeleteReport。`tsc --noEmit` 通过。
   - **AC-2 ipc-errors.ts（isIpcError / parseIpcError / invokeWithIpcError / errorMessages / renderIpcError）**：✅
     - `isIpcError` 用 `IPC_ERROR_CODE_SET` 运行时校验闭集（L38）、`message:string`、`details` 可选对象（L40）。
     - `parseIpcError` 优先级：对象直返 → string JSON.parse → 兜底 E_INTERNAL（L53–73），含 `String(raw)` 兜底分支。
     - `invokeWithIpcError` `try await invoke / catch throw parseIpcError(e)`（L79–88），异常侧总是 IpcError shape。
     - `errorMessages` 11 项全部实现，details 缺字段降级 + `console.warn("ipc_error_details_missing: ...")`。
     - `renderIpcError(err)` 便捷渲染（L234）。
   - **AC-3 5 个 camelCase wrapper**：✅（未改动，但实际打开 `src/lib/tauri-commands.ts` L178–243 与 §B.2 表逐项核对：`{projectId,name}` / `{projectId,relativePath,newName}` / `{projectId,relativePath,confirmNonEmpty,expectedCount}` / `{assetId,targetRelativePath}` / `{projectId,relativePath}` 字符级一致，全部走 `invokeWithIpcError<T>`）。
   - **AC-4 folder-name-validate.ts**：✅ 5 reason 闭集 `blank|has_slash|leading_dot|too_long|reserved`；UTF-8 字节用 `new TextEncoder().encode(name).length`（L54）；保留字 `organized` 单点；文件头注释明示「后端为权威」。
   - **AC-5 前端单测全绿**：✅ `npx vitest run` 实测 2 文件 / 35 用例全过；测试断言行为（substring / mock.calls / 引用相等），非快照。
   - **AC-6 文案表唯一来源声明**：✅ 文件头注释 L3–12 红线声明「`errorMessages` 是用户可见文案唯一来源；`message` 仅日志/上报，禁止直接展示」。

3. **关键发现**：
   - 11 项 code 三处闭集（联合类型 / `IPC_ERROR_CODES` 数组 / `IPC_ERROR_CODE_SET` Set）双向一致，并有单测断言 `size === 11`。
   - 渲染规则 4 条硬约束全部按契约落地（`E_FOLDER_DIRTY` 用 `now`；`E_NOT_FOUND` 根目录分支；缺 details 降级 + warn；`E_PATH_ESCAPE` 防泄漏）均有专门单测。
   - 「未改动」声明经实际打开 `tauri-commands.ts` / `workspace.ts` 核对属实，无需改动。

---

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | 11 项 code、4 条渲染规则、5 wrapper payload key、UTF-8 字节边界全部按契约落地；35 用例全绿；T0 §A.2/§A.4/§B.2/§D 字符级一致。 |
| 安全性 | 25% | 5 | `E_PATH_ESCAPE` 文案不带 `requestedPath`，单测明确断言不含 `/etc/passwd`；`E_TRASH_FAILED` / `E_INTERNAL` 也不暴露 path/where；后端 `message` 不进 UI；details 缺字段降级而非二次抛错避免 UI 误吞。 |
| 代码质量 | 15% | 5 | 命名清晰，三张映射表（reason/action/kind/feature）提取为模块顶部常量；`warnDetailsMissing` 抽出统一上报；文件头红线注释完备；零 lint 隐患（仅一处必要的 `eslint-disable no-console`）。 |
| 测试覆盖 | 20% | 5 | 35 用例覆盖 isIpcError 闭集 11 项 + 非法形态、parseIpcError 5 分支（a/b/c/c'/d）、errorMessages 11 项完整渲染 + 根目录分支 + 防泄漏 + 5 处缺字段降级 + warn 调用次数、invokeWithIpcError 3 路径、validateFolderNameSync 5 reason + 优先级 + UTF-8 边界 255/256/中×86。 |
| 架构一致性 | 10% | 5 | 目录与 ADR-001/ADR-007 一致；不引入运行时新依赖；wrapper 全走 `invokeWithIpcError<T>`；payload key camelCase 与 §B.2 表字符级一致；`__ROOT__` 由前端原样透传给后端归一（与 ADR-004 一致）。 |
| 可维护性 | 5% | 5 | 红线注释明示扩展路径（"新增 code 必须先回 T0"）；映射表外部化便于后续微调文案；前端 5 reason 与后端 5 reason 名差异在 doc 内显式说明（前端 closed set 不等同后端）。 |

**综合分：5.0/5**（加权计算 = 5 × 1.00）

---

## 总体判断

- [x] **PASS**
- [ ] FIX
- [ ] BLOCKER

判定依据：无 BLOCKER；无 MAJOR；2 条 MINOR 建议（非阻塞）；35 用例全绿；契约对齐严格；6 维全部 5/5。

---

## 问题列表

### BLOCKER（必须修复，否则不可能 PASS）

无。

### MAJOR（强烈建议修复）

无。

### MINOR（可选；不阻塞 PASS）

1. **E_NAME_INVALID 文案微调**：
   - **当前实现**：`名称「${name}」不合法（${reasonText}）`（ipc-errors.ts L149）
   - **契约模板**：`名称不合法（{reasonText}）`（contracts.md §D #1）
   - **说明**：实现把 `name` 拼进了模板，比契约模板更友好（PRD 验收和 T2 单测都依赖 `name` 出现在文案里），属于 §D「T2 实现时可微调，但字段名/渲染语义不变」允许范围。**保留即可**；如要严格逐字一致，可改回不带 name 的版本，但要同步调整 ipc-errors.test.ts 中对 `a/b` 出现的断言（L155）。

2. **前端 5 reason 与后端 5 reason 名不同的口径备注**：
   - **现状**：`validateFolderNameSync` reason = `blank | has_slash | leading_dot | too_long | reserved`；后端 `E_NAME_INVALID.reason` = `slash | dot_prefix | whitespace | too_long | empty`（§A.4 #1）。
   - **影响**：UI 层若想在前端 reason 命中后直接复用 `errorMessages.E_NAME_INVALID(reason映射)`，需要做一次本地 → 后端口径的转换；目前 folder-name-validate.ts 文件头注释已说明前端是 UI 反馈、后端权威，但没有给映射表。
   - **建议（可选）**：在 folder-name-validate.ts 末尾追加一张 `FRONTEND_TO_BACKEND_REASON` 常量映射（`blank→empty`、`has_slash→slash`、`leading_dot→dot_prefix`、`too_long→too_long`、`reserved`→无对应/走 E_NAME_RESERVED），供 T5a 表单层快速渲染中文。**不阻塞当前 task**，可在 T5a 一并补。

3. **`parseIpcError` 的 `String(raw)` try/catch**（ipc-errors.ts L66–71）：
   - 实际上对 `null`/`undefined`/number/boolean，`String()` 永远不会抛；对象的 `String()` 抛错也极罕见（需自定义 `toString` 抛错）。这层 try/catch 是过度防御。**保留也无害**，纯风格问题。

---

## 给 Dev 的修复指引

**判定为 PASS，无需修复**。Dev 可继续推进 T5a/T5b（store/UI）。

如 Conductor 要求严格逐字契约，可选择性处理上方 MINOR-1（影响 1 行代码 + 1 行测试断言）。其余无动作。

---

## 4 个无关失败用例的核验结论

实际打开 `NCdesktop/src/App.test.tsx`（L29–58）与 `NCdesktop/src/components/features/SearchPanel.test.tsx`（L1–110）后核对：

| 失败用例 | 文件:行 | 根因 | 与 T2 是否相关 |
|---|---|---|---|
| `App > renders AppLayout by default` | App.test.tsx L31 | AppLayout 渲染 / `window.location.pathname` mock 时序 | **无关**（T2 未触碰 App.tsx / AppLayout / uiStore / libraryStore / projectStore） |
| `SearchPanel > performs search after internal debounce` | SearchPanel.test.tsx L51 | 200ms 内 debounce + `searchHoisted.performSearch` mock 时序 | **无关**（T2 未触碰 SearchPanel / useSearchStore） |
| `SearchPanel > calls onNavigate and logs when item is selected` | SearchPanel.test.tsx L64 | 同上 debounce 链路依赖 | **无关** |
| `SearchPanel > navigates with keyboard Enter` | SearchPanel.test.tsx L77 | 同上 debounce 链路依赖 | **无关** |

**核验结论**：4 个失败用例全部位于 `App.test.tsx` / `SearchPanel.test.tsx`，根因为 debounce 异步等待 + mock 时序问题（既存历史 issue），与 T2 改动文件（`ipc-errors.ts` / `folder-name-validate.ts` / `tauri-commands.ts` / `types/workspace.ts`）**无任何代码依赖关系**。Dev 声明属实。

本 task 新增/覆写的 35 用例（ipc-errors.test.ts 28 + folder-name-validate.test.ts 7）独立运行 **35/35 全绿**（实测 `npx vitest run` Duration 628ms）。
