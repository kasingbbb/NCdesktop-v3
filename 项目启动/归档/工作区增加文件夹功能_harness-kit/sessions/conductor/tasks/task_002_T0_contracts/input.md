# Task 输入 — task_002_T0_contracts

## 目标
冻结本期所有下游 task 共用的契约基线：`IpcError` JSON shape、11 项错误码闭集、5 个新 Tauri 命令签名、`__ROOT__` sentinel 编解码规则、错误码 ↔ 中文文案表，产出 `contracts.md` 作为 T1-T6 不可改动的唯一权威。

## 前置条件
- 依赖 task：无（本 task 是所有后续 task 的前置）
- 必须先存在的文件/接口：
  - `product/prd/workspace_folder_mgmt_prd_v1.md`（§4.3 / §5.1 / §5.2）
  - `sessions/workspace_folder_mgmt/debate/session_001/debate_conclusions.md`（§2 `__ROOT__` Canonical / §3 错误模型 / §5 5 IPC）
  - Architect output.md（ADR-001 / ADR-004）

## 验收标准（Acceptance Criteria）

1. **AC-1 文档落盘**：在 `sessions/conductor/tasks/task_002_T0_contracts/` 下产出 `contracts.md`，且 `output.md`（交付记录）按 handoff_contracts §3 字段齐全。
2. **AC-2 IpcError shape 双向规范**：`contracts.md §A` 同时给出 TS shape 与 Rust shape，并明确序列化协议（Rust enum `#[serde(rename = "...")]` 字面量 = TS 联合类型字面量，字符级一致），11 项 code 闭集逐项列出 `details` schema + 必填字段。**禁止**新增 `E_DEPTH_LIMIT` / `E_CYCLE` 等非 MVP 码。
3. **AC-3 5 命令签名定稿**：`contracts.md §B` 逐字搬运 PRD §5.1 的 5 个 Rust `#[tauri::command]` 签名，并给出对应前端 camelCase wrapper signature（含返回类型、错误抛出契约）；`DeleteReport` 字段定义在此处。
4. **AC-4 `__ROOT__` 编解码规则**：`contracts.md §C` 给出三处单点：(1) 入站归一 `resolve_relative_path("__ROOT__") → ""`；(2) DB `assets.file_path` 永不含 `__ROOT__`；(3) 出站 `list_project_workspace_folders` 首行仍返 `__ROOT__`。明示 `assets` INSERT/UPDATE 入口必须 `debug_assert!(!path.contains("__ROOT__"))`。
5. **AC-5 文案表逐字定稿**：`contracts.md §D` 给出 11 项 code → 中文文案模板（含 `details` 字段渲染规则，例如 `E_FOLDER_DIRTY` 必须用 `details.now`），明示前端文案唯一来源 = 本表，后端 `message` 仅日志/上报。
6. **AC-6 红线声明**：`contracts.md` 顶部说明本文件是 T1-T6 唯一权威；下游不得擅自改动；如需变更必须回到 T0 修订本文档再传导。

## 技术约束
- session_context §5：前端 TS + React 19 + Zustand；命名前端 camelCase / 后端 snake_case；不引入 i18n。
- ADR-001：Tauri v2 invoke 边界 `Err(String)`，通过 `serde_json::to_string(&IpcError)` 序列化为单行 JSON。
- ADR-004：`__ROOT__` 仅 UI/IPC sentinel，永不入 DB。
- PRD §4.3：错误码 11 项闭集，不增不减。

## 参考文件
- `product/prd/workspace_folder_mgmt_prd_v1.md` §4.3 / §5.1 / §5.2
- `sessions/workspace_folder_mgmt/debate/session_001/debate_conclusions.md` §2 / §3 / §5
- `sessions/conductor/tasks/task_001_architect/output.md` ADR-001 / ADR-004 / API 设计 / 数据模型
- 既有代码参考（仅作 shape 对齐参考，本 task 不改代码）：
  - `NCdesktop/src/types/workspace.ts`（既有 `WorkspaceFolderEntry`、`WorkspaceFolderKind`）
  - `NCdesktop/src-tauri/src/commands/workspace_folders.rs`（既有 `WorkspaceFolderEntry` Rust struct）

## 预估影响范围
- 新建文件：
  - `sessions/conductor/tasks/task_002_T0_contracts/contracts.md`
  - `sessions/conductor/tasks/task_002_T0_contracts/output.md`
- 修改文件：无（本 task 仅产出契约文档，不动产品代码）
