# Task 交付 — task_004_T2_frontend_ipc

## 实现摘要

按 contracts.md §A/§B/§D 完成前端 IPC 封装：

1. **类型层（`src/types/workspace.ts`）**：追加 `IpcErrorCode`（11 项闭集字面量联合）、`IpcError`、`DeleteReport { trashed: number }`，并把 `WorkspaceFolderEntry.kind` 补足 `WorkspaceFolderKind = 'root' | 'root_import' | 'ai_organized'`（保留 `| string` 宽松回退以不破坏既有调用方）。
2. **错误解包层（`src/lib/ipc-errors.ts`，新建）**：
   - `isIpcError(unknown)` 类型守卫（基于 11 项闭集 Set）；
   - `parseIpcError(unknown)`：已是 IpcError → 直接返回；string → `JSON.parse` 还原并校验；其余 / parse 失败 / code 不在闭集 → 兜底 `{ code: 'E_INTERNAL', message: String(raw), details: undefined }`；
   - `invokeWithIpcError<T>(cmd, args)`：try/catch invoke，catch 处 `throw parseIpcError(e)`，调用方拿到的失败**始终是 IpcError 对象**；
   - `errorMessages`：11 项中文文案（逐字搬运 contracts.md §D.2），`E_FOLDER_DIRTY` 用 `details.now` 渲染，`E_NAME_DUP/INVALID/RESERVED` 拼 `details.name`，`E_PATH_ESCAPE` 拼 `details.relative_path`，其余固定文案；
   - `renderIpcError(IpcError)` 便捷函数。
3. **5 个 camelCase wrapper（追加到 `src/lib/tauri-commands.ts`）**：`createWorkspaceFolder` / `renameWorkspaceFolder` / `deleteWorkspaceFolder` / `moveAssetToWorkspaceFolder`（**收敛为单素材签名** `(assetId, targetRelativePath) → Asset`）/ `countFolderAssets`，全部走 `invokeWithIpcError<T>`，参数 camelCase 与 contracts.md §B.2 精确对齐；`__ROOT__` sentinel 原样透传，不在 wrapper 翻译。
4. **调用方适配**：`AssetContextMenu.tsx` 旧调用 `moveAssetToWorkspaceFolder(assetIds, target, projectId)` 改为对 `targetIds` 数组的 `for...of` 逐一调用，符合 input.md §技术约束「本期调用方逐一调用」。
5. **单测**：`src/lib/__tests__/ipc-errors.test.ts`，16 个用例全绿。

核心设计决策：
- `parseIpcError` 对 "JSON 解析成功但 code 不在闭集" 也走兜底 `E_INTERNAL`，保护闭集契约；
- `isIpcError` 不依赖 `instanceof`，纯结构判别，满足跨边界 throw 后的判别需求；
- `WorkspaceFolderEntry.kind` 用联合 + `| string` 而非纯字面量联合，避免破坏既有 `revealProjectWorkspaceFolder` 等流程对 `kind` 的 string 用法。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---|---|---|
| `src/types/workspace.ts` | 修改 | 追加 `IpcErrorCode` / `IpcError` / `DeleteReport` / `WorkspaceFolderKind` |
| `src/lib/ipc-errors.ts` | 新建 | `isIpcError` / `parseIpcError` / `invokeWithIpcError` / `errorMessages` / `renderIpcError` |
| `src/lib/tauri-commands.ts` | 修改 | 追加 5 个 camelCase wrapper；旧 `moveAssetToWorkspaceFolder` 收敛为单素材签名 |
| `src/components/features/AssetContextMenu.tsx` | 修改 | 适配单素材 `moveAssetToWorkspaceFolder`（for...of 逐一调用） |
| `src/lib/__tests__/ipc-errors.test.ts` | 新建 | 16 个用例覆盖 AC-1 / AC-4 |

## 对 Architect 方案的遵守声明

- [x] 命名与 contracts.md §B.2 精确一致（5 wrapper 名称、参数名、参数顺序、参数类型）
- [x] `IpcError` shape 与 contracts.md §A.2 一致（`code` / `message` / `details?`）
- [x] 11 项错误码闭集逐字一致；不引入 `E_DEPTH_LIMIT` / `E_CYCLE` 等 MVP 红线外 code
- [x] errorMessages 文案与 contracts.md §D.1 / §D.2 逐字一致；`E_FOLDER_DIRTY` 使用 `details.now` 渲染（PRD §4.2、ADR-010）
- [x] `__ROOT__` 在 wrapper 不翻译，原样透传（contracts.md §C.1 入站契约）
- [x] 未引入新运行时依赖（仅 TS 类型 + 纯函数 + 既有 `@tauri-apps/api/core`）
- [x] 未顺手重构既有 tauri-commands.ts 其他 wrapper
- 偏离说明：
  - 旧 `moveAssetToWorkspaceFolder(assetIds[], target, projectId)` 按 input.md §技术约束改为单素材签名。**该 break 是 input.md 明确要求**，已同步适配 `AssetContextMenu.tsx`（项目内唯一调用方）；
  - `WorkspaceFolderEntry.kind` 类型从 `string` 收紧为 `WorkspaceFolderKind | string`（联合保留 string fallback），属类型补强，不重命名既有字段，符合 prompt 红线「仅在缺时补齐」。

## 测试命令

```bash
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop && pnpm tsc --noEmit
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop && pnpm test --run
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop && pnpm test --run src/lib/__tests__/ipc-errors.test.ts
```

## 测试结果

### `pnpm tsc --noEmit`

```
EXIT=0   （无输出，全部通过）
```

### `pnpm test --run`（全量）

```
 Test Files  2 failed | 23 passed (25)
      Tests  4 failed | 193 passed (197)
   Duration  7.58s
```

**4 个失败均为本任务前已存在的预存测试**（与本任务无关）：

```
FAIL src/App.test.tsx > App Component > renders AppLayout by default
FAIL src/components/features/SearchPanel.test.tsx > SearchPanel Component > performs search after internal debounce
FAIL src/components/features/SearchPanel.test.tsx > SearchPanel Component > calls onNavigate and logs when item is selected
FAIL src/components/features/SearchPanel.test.tsx > SearchPanel Component > navigates with keyboard Enter
```

未触及 `App.tsx` / `SearchPanel.tsx` / 相关测试文件；失败原因为既有 `jsdom` 下 `@tauri-apps/api` 事件订阅未 mock，与 task_004 修改无关。

### `pnpm test --run src/lib/__tests__/ipc-errors.test.ts`（本任务用例）

```
 RUN  v4.1.1
 Test Files  1 passed (1)
      Tests  16 passed (16)
   Duration  343ms
```

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|---|---|---|---|
| ✅ 正常路径 | 11 项 code 均能产出非空中文文案 | 已测 | PASS（CJK 字符断言通过） |
| ✅ 正常路径 | `E_FOLDER_DIRTY({old:2, now:3})` 文案含 "3" 且含 "现" | 已测 | PASS |
| ✅ 正常路径 | `E_NAME_DUP({name:"参考"})` 文案含 "参考" | 已测 | PASS |
| ✅ 正常路径 | `E_NAME_INVALID` 拼 `details.name`、`E_PATH_ESCAPE` 拼 `details.relative_path` | 已测 | PASS |
| ✅ 正常路径 | `invokeWithIpcError` 成功透传数据 | 已测 | PASS（mock invoke 解析对象原样返回） |
| ✅ 正常路径 | `invoke` 抛合法 JSON `{code:'E_NAME_DUP',...}` → 包装抛 IpcError，`isIpcError(e) === true` 且 `e.code === 'E_NAME_DUP'` | 已测 | PASS |
| ⚠️ 边界条件 | `parseIpcError` 收到已是 IpcError 对象 | 已测 | PASS（identity 返回） |
| ⚠️ 边界条件 | JSON 字符串但 code 不在 11 项闭集 | 已测 | PASS（兜底 E_INTERNAL，message 保留原始 JSON） |
| ⚠️ 边界条件 | 非 string 非对象输入（如数字 42） | 已测 | PASS（`String(42)` → message） |
| ❌ 异常路径 | `invoke` 抛非 JSON 字符串 `"boom"` | 已测 | PASS（兜底 `{code:'E_INTERNAL', message:'boom'}`） |
| ❌ 异常路径 | `isIpcError` 拒绝缺字段 / 非法 code / null / string | 已测 | PASS |
| ⚠️ 边界条件 | TS 编译（含调用方适配 `AssetContextMenu.tsx`） | 已测 | PASS（`pnpm tsc --noEmit` 0 错误） |
| ❌ 异常路径 | 端到端真实 invoke（Tauri runtime） | 未测 | 需 `tauri dev`，超出本 task 范围，留给 T5/T6 集成测试 |

## 已知局限

- 4 个预存测试失败（App.test / SearchPanel.test）与本任务无关，未顺手修复以遵守「不重构既有代码」红线；已在测试结果节标注。
- `invokeWithIpcError` 未做 retry / timeout 处理；按 input.md「仅 TS 类型 + 纯函数」要求，重试策略归于上层 Hook（T5）。
- `WorkspaceFolderEntry.kind` 保留 `| string` 宽松回退；未来若所有调用方都改用 `WorkspaceFolderKind` 字面量后，可由后续 task 收紧。

## 需要 Reviewer 特别关注的地方

1. **`AssetContextMenu.tsx:100` 的 for...of 逐一调用**：旧实现是后端一次性 atomic 多素材移动，新前端逐一调用**不再原子**——若中途某个失败，前面已成功的不会回滚。input.md 明确说明本期允许（「单素材拖入」场景为主），但 Reviewer 请确认这与 PRD §5.1 一致。
2. **`parseIpcError` 对 "JSON 解析成功但 code 不在闭集" 的处理**：选择降级为 `E_INTERNAL`（而非保留原始 code）。这是为了**强制闭集契约**，但会丢失后端真实 code 信息（仅 message 保留原始 JSON）。请确认这一选择与 contracts.md §A.1 ADR-001 fallback 语义一致。
3. **`isIpcError` 的 `details` 校验**：要求 `details` 要么 `undefined` 要么是 object，不接受 `null`。后端 serde `skip_serializing_if = "Option::is_none"` 不会序列化 None，所以这里不应出现 null；但若后端日后改 schema，需同步更新本守卫。
4. **`WorkspaceFolderEntry.kind: WorkspaceFolderKind | string`**：保留 string 宽松回退是为了不破坏既有调用方对 kind 的 string 比较（如 `kind === 'ai_organized'` 字符串比较仍成立）。若 Reviewer 倾向严格化，可在后续 task 一并收紧。
