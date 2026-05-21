# Task 输入 — task_004_T2_frontend_ipc

## 目标
在 `src/lib/tauri-commands.ts` 加 5 个 camelCase IPC 包装函数，新建 `src/lib/ipc-errors.ts`（`IpcError` TS 类型 + `errorMessages` 文案表 + `invokeWithIpcError<T>` 解包器），并在 `src/types/workspace.ts` 追加 `DeleteReport` 等类型。

## 前置条件
- 依赖 task：task_002_T0_contracts（必须先 DONE，使用其 contracts.md 文案表与字段 schema）
- 必须先存在的文件/接口：
  - `NCdesktop/src/lib/tauri-commands.ts` 既有 invoke wrapper 模式
  - `NCdesktop/src/types/workspace.ts` 既有 `WorkspaceFolderEntry`
  - contracts.md（错误码 + 文案表）

## 验收标准（Acceptance Criteria）
1. **AC-1**：`pnpm test ipc-errors` 单测 PASS（覆盖：11 个 code 各传 mock details 渲染出非空中文文案；`errorMessages.E_FOLDER_DIRTY({old:2, now:3})` 返回含 "3" 的字符串；JSON.parse 失败 fallback 为 `E_INTERNAL`）。
2. **AC-2**：5 个 camelCase wrapper 全部加在 `src/lib/tauri-commands.ts`：`createWorkspaceFolder`、`renameWorkspaceFolder`、`deleteWorkspaceFolder`、`moveAssetToWorkspaceFolder`（**收敛为单素材签名 `(assetId: string, targetRelativePath: string) → Promise<Asset>`**）、`countFolderAssets`；每个 wrapper 内部走 `invokeWithIpcError<T>(cmd, args)`。
3. **AC-3**：`types/workspace.ts` 新增 `IpcErrorCode` union（11 项）、`IpcError` interface、`DeleteReport { trashed: number }`；TS `pnpm tsc --noEmit` PASS。
4. **AC-4**：`invokeWithIpcError<T>` 单测：mock Tauri invoke reject 一个合法 IpcError JSON string，函数 reject 出 `IpcError` 对象；mock reject 非 JSON 字符串，fallback 出 `{code:'E_INTERNAL', message: <原始>}`。
5. **AC-5**：`pnpm test` 全绿（不引入新失败）。

## 技术约束
- 命令名 camelCase ↔ snake_case 严格对齐（前端 camelCase，后端 snake_case；PRD §5）。
- 不使用 i18n 框架，文案直接中文常量（Debate §3）。
- 旧 `moveAssetToWorkspaceFolder(assetIds: string[], targetRelativePath, projectId): Promise<void>`（见 `NCdesktop/src/lib/tauri-commands.ts:170`）**收敛为单素材**：移除 `assetIds` Array 与 `projectId` 参数（project_id 由 asset_id 反推，符合 PRD §5.1）；本期调用方（`AssetListView` 单素材拖入）逐一调用即可。
- `invokeWithIpcError<T>` 必须用 try/catch + `JSON.parse(err.toString())`；解析失败时返 `{code:'E_INTERNAL', message: err.toString()}`（ADR-001）。
- 不引入任何运行时新依赖（仅 TS 类型 + 纯函数）。
- 不顺手改无关 wrapper。

## 参考文件
- 现有：`NCdesktop/src/lib/tauri-commands.ts:82-180`（既有工作区 wrapper + 旧 move wrapper）、`NCdesktop/src/types/workspace.ts`
- 契约：`sessions/conductor/tasks/task_002_T0_contracts/contracts.md`（文案表与字段 schema 直接对齐）
- 方案：output.md ADR-001（IpcError 解包）、§API 设计

## 预估影响范围
- 新建文件：
  - `NCdesktop/src/lib/ipc-errors.ts`
  - `NCdesktop/src/lib/__tests__/ipc-errors.test.ts`
- 修改文件：
  - `NCdesktop/src/lib/tauri-commands.ts`（+5 wrapper；旧 moveAssetToWorkspaceFolder 改签名）
  - `NCdesktop/src/types/workspace.ts`（+`IpcError` / `IpcErrorCode` / `DeleteReport`）
