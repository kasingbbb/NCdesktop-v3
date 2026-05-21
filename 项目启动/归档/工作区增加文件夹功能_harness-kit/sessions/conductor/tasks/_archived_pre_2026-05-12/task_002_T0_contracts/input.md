# Task 输入 — task_002_T0_contracts

## 目标
冻结本期前后端契约：`IpcError` JSON shape、5 个 Tauri 命令签名、`__ROOT__` 编解码规则、错误码 ↔ 中文文案映射表，产出 `contracts.md` 单文档作为后续 T1-T6 的唯一引用基线。

## 前置条件
- 依赖 task：无
- 必须先存在的文件/接口：
  - PRD `product/prd/workspace_folder_mgmt_prd_v1.md` §4.3 / §5.1 / §5.2
  - Debate 结论 `sessions/workspace_folder_mgmt/debate/session_001/debate_conclusions.md` §2 / §3 / §5
  - 现有 `NCdesktop/src/types/workspace.ts`、`NCdesktop/src-tauri/src/commands/workspace_folders.rs`（理解既有 `WorkspaceFolderEntry` 形态）

## 验收标准（Acceptance Criteria）
1. **AC-1**：产出 `sessions/conductor/tasks/task_002_T0_contracts/contracts.md`，包含 4 节：(a) IpcError JSON shape + 11 项 code 闭集；(b) 5 命令 Rust 签名（完全复制 PRD §5.1，逐字一致）；(c) `__ROOT__` 编解码契约（入站→空字符串、DB 不存 sentinel、出站保留）；(d) `errorMessages[code](details) → string` 文案表（11 项 × 中文文案，含 `E_FOLDER_DIRTY` 用 `details.now` 渲染）。
2. **AC-2**：每个 error code 的 `details` 字段 schema（如 `E_FOLDER_DIRTY: {old: u32, now: u32}`、`E_NAME_DUP: {name: string}`）明确列出。
3. **AC-3**：在 contracts.md 末尾附"消费方核对清单"：T1 / T2 / T3 / T4 各自需要从本文档抽取的字段，便于下游 Dev 反查。
4. **AC-4**：文档通过 grep 自检：`grep -c "E_NAME_INVALID\|E_NAME_DUP\|E_NAME_RESERVED\|E_PATH_ESCAPE\|E_PROTECTED_KIND\|E_NOT_FOUND\|E_CROSS_DEVICE\|E_PLATFORM_UNSUPPORTED\|E_TRASH_FAILED\|E_FOLDER_DIRTY\|E_INTERNAL"` 至少各命中 1 次。

## 技术约束
- 命令签名**完全沿用 PRD §5.1，禁止改 arity 或参数名**（见 PRD 不可妥协底线 §5.1）。
- `__ROOT__` 是 UI/IPC sentinel，**严禁出现在 DB**（底线 6）。
- 错误码闭集 11 项，**禁止新增**（底线 10；MVP 不嵌套，无 `E_DEPTH_LIMIT` / `E_CYCLE`）。
- 文案为中文常量，无 i18n（Debate §3）。
- 本 task **不写代码**；仅产 markdown。

## 参考文件
- PRD §4.3 `IpcError` 类型块
- PRD §5.1 5 命令签名块
- PRD §5.2 uiStore 新增字段
- Debate §3 错误模型
- Debate §2 `__ROOT__` Canonical 契约
- output.md ADR-001（IpcError 序列化协议）、ADR-004（`__ROOT__` 编解码）

## 预估影响范围
- 新建文件：
  - `sessions/conductor/tasks/task_002_T0_contracts/contracts.md`
- 修改文件：无（不进 NCdesktop 仓库）
