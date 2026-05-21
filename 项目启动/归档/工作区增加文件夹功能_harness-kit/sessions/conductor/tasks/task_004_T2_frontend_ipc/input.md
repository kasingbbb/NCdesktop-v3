# Task 输入 — task_004_T2_frontend_ipc

## 目标
搭建前端 IPC 与错误层：在 `src/types/workspace.ts` 追加 `IpcError` / `IpcErrorCode` / `DeleteReport` 类型；在 `src/lib/ipc-errors.ts` 实现 `parseIpcError` / `invokeWithIpcError` / 11 项 `errorMessages` 中文文案表；在 `src/lib/tauri-commands.ts` 增加 5 个 camelCase wrapper（`createWorkspaceFolder` / `renameWorkspaceFolder` / `deleteWorkspaceFolder` / `moveAssetToWorkspaceFolder` / `countFolderAssets`）；新增 `src/lib/folder-name-validate.ts` 同步即时校验函数。

## 前置条件
- 依赖 task：task_002_T0_contracts（contracts.md §A / §B / §D）
- 必须先存在的文件/接口：
  - `sessions/conductor/tasks/task_002_T0_contracts/contracts.md`
  - `NCdesktop/src/types/workspace.ts`（既有 `WorkspaceFolderEntry`）
  - `NCdesktop/src/lib/tauri-commands.ts`（既有 IPC wrapper 框架）
- 本 task 可**与 T1 并行**（仅依赖 T0 契约，不依赖 T1 代码）。

## 验收标准（Acceptance Criteria）

1. **AC-1 类型补全**：`src/types/workspace.ts` 追加：
   - `IpcErrorCode` 联合类型（11 项闭集，字符级一致于 contracts.md §A.2）
   - `IpcError { code: IpcErrorCode; message: string; details?: Record<string, unknown> }`
   - `DeleteReport { trashed: number }`
   `tsc --noEmit` 通过。
2. **AC-2 `ipc-errors.ts`**：实现 `isIpcError(unknown) is IpcError` 类型守卫（用 `IPC_ERROR_CODE_SET` 运行时校验 code 字面量）+ `parseIpcError(raw)` 还原（先判 IpcError 对象；其次 `JSON.parse` string；parse 失败降级 `E_INTERNAL`，`message` 为原 string）+ `invokeWithIpcError<T>(cmd, args)` 包装（成功透传 T；失败 throw IpcError）+ `errorMessages: Record<IpcErrorCode, (details?) => string>`（contracts.md §D 11 项逐字搬运；`E_FOLDER_DIRTY` 必须用 `details.now`）+ `renderIpcError(err)` 便捷渲染。
3. **AC-3 5 个 camelCase wrapper**：`src/lib/tauri-commands.ts` 追加：
   ```ts
   createWorkspaceFolder(projectId, name) → Promise<WorkspaceFolderEntry>
   renameWorkspaceFolder(projectId, relativePath, newName) → Promise<WorkspaceFolderEntry>
   deleteWorkspaceFolder(projectId, relativePath, confirmNonEmpty, expectedCount) → Promise<DeleteReport>
   moveAssetToWorkspaceFolder(assetId, targetRelativePath) → Promise<Asset>
   countFolderAssets(projectId, relativePath) → Promise<number>
   ```
   全部走 `invokeWithIpcError<T>`；调用方 catch 拿到的就是 `IpcError`（非裸 string）。
4. **AC-4 `folder-name-validate.ts`**：纯字符串校验函数 `validateFolderNameSync(name) → { ok: boolean; reason?: 'has_slash' | 'leading_dot' | 'blank' | 'too_long' | 'reserved' }`。注释明示「后端 `validate_folder_name` 是最终权威，本函数仅作 UI 即时反馈」。
5. **AC-5 前端单测**：`pnpm test` 全绿。新增 `src/lib/__tests__/ipc-errors.test.ts`：
   - `parseIpcError` 处理：(a) 已是 IpcError 对象直接返；(b) JSON string 还原；(c) 非法 JSON 降级 `E_INTERNAL`；(d) 非 string 非 IpcError → `E_INTERNAL`；(e) 11 项 code 都能通过 `isIpcError` 守卫。
   - `errorMessages.E_FOLDER_DIRTY({ old: 3, now: 5 })` 中文文案包含 `"3"` 与 `"5"`。
   - `errorMessages.E_NAME_INVALID({ name: "a/b" })` 包含 `"a/b"`。
6. **AC-6 文案表唯一来源声明**：`ipc-errors.ts` 文件头注释明示「本表是用户可见文案的唯一来源；后端 `IpcError.message` 字段仅日志/上报，禁止直接展示」。

## 技术约束
- session_context §5：前端 camelCase；不引入 i18n。
- 不引入运行时新依赖（仅 TS 类型 + 纯函数 + `@tauri-apps/api/core::invoke`）。
- `invokeWithIpcError` 内 `try { await invoke } catch (e) { throw parseIpcError(e) }`；不要把 IpcError 转为 Error 子类，前端 catch 应用 `isIpcError` 守卫判别。
- 11 项 code 联合类型与运行时 set 必须双向一致；新增 code 必须先回 T0 修订 contracts.md。

## 参考文件
- `sessions/conductor/tasks/task_002_T0_contracts/contracts.md` §A.2 / §B / §D
- `sessions/conductor/tasks/task_001_architect/output.md` ADR-001 / ADR-007（前端 handler 入口）
- 既有代码：
  - `NCdesktop/src/types/workspace.ts`
  - `NCdesktop/src/lib/tauri-commands.ts`（既有 `listProjectWorkspaceFolders` / `revealProjectWorkspaceFolder` / `getProjectWorkspaceRoot` 三个 read wrapper 不动）
  - `NCdesktop/src/types/asset.ts`（`Asset` 类型，move wrapper 返回）

## 预估影响范围
- 新建文件：
  - `NCdesktop/src/lib/ipc-errors.ts`
  - `NCdesktop/src/lib/folder-name-validate.ts`
  - `NCdesktop/src/lib/__tests__/ipc-errors.test.ts`
- 修改文件：
  - `NCdesktop/src/types/workspace.ts`（+ IpcError / IpcErrorCode / DeleteReport）
  - `NCdesktop/src/lib/tauri-commands.ts`（+ 5 camelCase wrapper）
