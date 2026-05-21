# Task 交付 — task_002_T0_contracts

## 实现摘要

T0 是文档型 task，**不动产品代码**，仅产出 T1-T6 共用的契约基线 `contracts.md`。核心决策：

1. **IpcError shape 双向规范化**（§A）：Rust enum `#[serde(rename)]` 字面量与 TS 联合类型字面量逐项给出对照表，11 项 code 闭集每项 `details` schema 锁字段名 + 必填标注；序列化协议明示 `From<IpcError> for String` 走 `serde_json::to_string` + 兜底单行 JSON 防前端 parse 死。
2. **5 命令签名逐字搬运 PRD §5.1**（§B），并给出对应前端 camelCase wrapper 签名 + Tauri invoke payload key 列表（避免 T2 实现时 key 大小写漂移）；明示退役旧多素材 `move_asset_to_workspace_folder`。
3. **`__ROOT__` 三处单点**（§C）：入站 `resolve_relative_path("__ROOT__") → ""`、DB 永不含（`debug_assert!` 4 处入口）、出站 `list_*` 首行 + 4 命令返根目录场景仍用 `__ROOT__`。给出 Reviewer 红线 checklist。
4. **错误码 → 中文文案表**（§D）：11 项逐项给出文案模板 + 依赖 `details` 字段 + 缺字段降级策略；锁定 `E_FOLDER_DIRTY` 文案必须用 `details.now` 渲染，对接 ADR-010 删除 confirm 重弹 modal 行为。
5. **变更管控**（§E）：明示下游不得擅自改契约；必须回 T0 修订并由 Conductor 重新传导。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---|---|---|
| `sessions/conductor/tasks/task_002_T0_contracts/contracts.md` | 新建 | T0 契约主产物：§A IpcError shape / §B 5 命令签名 / §C `__ROOT__` 编解码 / §D 文案表 / §E 变更管控 / §F 既有代码对齐说明 |
| `sessions/conductor/tasks/task_002_T0_contracts/output.md` | 新建 | 本交付记录 |

> 产品代码（`NCdesktop/...`）**零改动**，符合 input.md「修改文件：无」。

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（T0 不创建任何代码目录；仅在 `sessions/conductor/tasks/task_002_T0_contracts/` 下生成文档）
- [x] API 路径/命名与 Architect 方案一致（5 命令签名逐字搬运 PRD §5.1 = Architect output.md「API 设计」节）
- [x] 数据模型与 Architect 方案一致（`IpcErrorCode` 11 项闭集 / `IpcError` / `DeleteReport` / `WorkspaceFolderEntry` 字段全部对齐 ADR-001 / ADR-004 / 既有代码）
- [x] 未引入计划外的新依赖（T0 无依赖变更；`trash` / `unicode-normalization` 归 T1）
- 偏离说明：无。

## 测试命令

```bash
# 文档型 task，无可执行测试；自检脚本如下：
ls -1 "/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/工作区增加文件夹功能_harness-kit/sessions/conductor/tasks/task_002_T0_contracts/"
# 期望输出：contracts.md / input.md / output.md

# 字段对照（人工执行）：
# 1. PRD §4.3 11 项 code ↔ contracts.md §A.2 / §A.3 / §A.4 / §D 各表
# 2. PRD §5.1 5 命令签名 ↔ contracts.md §B.1
# 3. ADR-004 三处单点 ↔ contracts.md §C.1 / §C.2 / §C.3
# 4. 既有 src/types/workspace.ts IpcErrorCode ↔ contracts.md §A.2
```

## 测试结果

```
$ ls -1 sessions/conductor/tasks/task_002_T0_contracts/
contracts.md
input.md
output.md

字段对照（逐项核对）：
- PRD §4.3 11 项 code（E_NAME_INVALID … E_INTERNAL）↔ contracts.md §A.2 / §A.3 rename 表 / §A.4 schema 表 / §D 文案表 → PASS（11 项闭集逐字一致，无新增 E_DEPTH_LIMIT/E_CYCLE）
- PRD §5.1 5 签名（create/rename/delete/move/count）↔ contracts.md §B.1 → PASS（参数名、参数类型、返回类型逐字一致）
- ADR-004 三处单点 ↔ contracts.md §C.1（resolve_relative_path）/ §C.2（debug_assert!）/ §C.3（list 首行 + 4 命令返根仍用 __ROOT__）→ PASS
- 既有 src/types/workspace.ts IpcErrorCode 11 项 ↔ contracts.md §A.2 → PASS（字符级一致）
- 既有 src-tauri/src/commands/workspace_folders.rs WorkspaceFolderEntry/DeleteReport 字段 ↔ contracts.md §B.1 「返回值数据结构」 → PASS
```

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|---|---|---|---|
| ✅ 正常路径 | AC-1 文档落盘（contracts.md + output.md 同目录） | 已测 | PASS — `ls` 输出含两文件 |
| ✅ 正常路径 | AC-2 IpcError shape 双向：TS shape + Rust shape + 11 项 code 闭集逐项 details schema | 已测 | PASS — §A.2/§A.3 字面量对照表逐项一致；§A.4 schema 表 11 行齐全，含 `必填` 标注 |
| ✅ 正常路径 | AC-2 红线：禁新增 `E_DEPTH_LIMIT` / `E_CYCLE` | 已测 | PASS — §A.4 + 顶部红线声明双重明示 |
| ✅ 正常路径 | AC-3 5 命令签名 + 前端 wrapper signature + DeleteReport 定义 | 已测 | PASS — §B.1 (Rust) / §B.2 (TS wrapper) / §B.1「返回值数据结构」含 DeleteReport |
| ✅ 正常路径 | AC-4 `__ROOT__` 三处单点 + assets 写路径 `debug_assert!` | 已测 | PASS — §C.1 / §C.2（含 `debug_assert!` 代码片段）/ §C.3 / §C.4 Reviewer checklist |
| ✅ 正常路径 | AC-5 文案表 11 项 + details 字段渲染规则 + 前端唯一来源声明 | 已测 | PASS — §D 11 行模板 + 渲染规则 4 条 + `E_FOLDER_DIRTY` 必用 `details.now` 明示 |
| ✅ 正常路径 | AC-6 红线声明（顶部 + §E 变更管控） | 已测 | PASS — 顶部「红线声明」4 条 + §E 变更管控四步流程 |
| ⚠️ 边界条件 | Rust ↔ TS 字面量字符级一致（含大小写、下划线） | 已测 | PASS — §A.3 提供 11 项对照表；TS shape 取自既有 `src/types/workspace.ts`（已对齐） |
| ⚠️ 边界条件 | `details` 缺必填字段时前端不抛二次错 | 已测 | PASS — §D 渲染规则第 3 条明示「降级为通用文案 + warn 上报」 |
| ⚠️ 边界条件 | `serde_json::to_string` 失败的兜底 JSON | 已测 | PASS — §A.3 `From<IpcError> for String` 给出兜底单行 JSON 字面量 |
| ❌ 异常路径 | T0 不动产品代码（既有 `src-tauri/src/commands/workspace_folders.rs` / `src/types/workspace.ts` 未触碰） | 已测 | PASS — 仅新建 2 文件于 harness-kit/sessions 下 |
| ❌ 异常路径 | 下游 task 擅自改契约的拦截路径 | 已测 | PASS — §E 明示「不允许『先实现再补契约』」+ 修订四步流程 |

## 已知局限

1. **`details` schema 仅文档化，运行时无强约束**：T1 实现 `IpcError` 时不强制 `details` 走 typed struct，仍是 `serde_json::Value`；如 T3 实现时违反 schema，需靠 T2 前端 `errorMessages` 测试 + Reviewer 复核兜底。可选升级路径：T1 为每个 code 定义一个 `details_xxx` typed struct + 构造函数（POST-MVP）。
2. **`E_NAME_INVALID.reason` 枚举闭集 5 项**（`slash` / `dot_prefix` / `whitespace` / `too_long` / `empty`）是本文档新增的子契约，PRD §4.3 未细分。下游 T1 `validate_folder_name` 必须严格按此 5 项分类，**不得**新增（如 `colon`、`reserved_char` 等子分类）。若实现时发现需要新增子分类，回 T0 修订。
3. **`Asset` 返回结构未在本文件锁字段**：`move_asset_to_workspace_folder` 返回类型沿用既有 `src-tauri/src/models.rs::Asset`，T0 不展开其字段表（避免与 models.rs 双源）；如 T3 / T4 / T6 需用到 Asset 某字段，以 `models.rs` 为权威。
4. **`E_TRASH_FAILED.reason` 仅 2 枚举值**（`still_exists` / `crate_error`），未细分 trash crate 内部错误（如 `permission_denied`）；若用户场景需要区分，回 T0 修订。

## 需要 Reviewer 特别关注的地方

1. **§A.3 Rust ↔ TS 字面量对照表**：T1 / T2 落地时，Rust enum `#[serde(rename = "...")]` 与 TS 字面量必须字符级一致。Reviewer 复核 T1 / T2 PR 时务必逐项 diff 此表。
2. **§A.4 `E_FOLDER_DIRTY.details.now` 必填**：这是前端 `errorMessages.E_FOLDER_DIRTY` 文案渲染依赖的关键字段；T3 实现 `delete_workspace_folder` 时不能漏。**Reviewer 应在 T3 PR 中专门 grep `E_FOLDER_DIRTY` 构造点验证 `details` 含 `now` 字段**。
3. **§C.2 `debug_assert!(!path.contains("__ROOT__"))` 4 处入口**：T1 落 `db/asset.rs::rename_path_prefix` 内一处；T3 落 `move_asset_to_workspace_folder` 内一处；既有 `delete_asset` / import 入口 T1 / T3 检视时各加一处。Reviewer 在 T1 / T3 PR 中需确认 4 处全到位。
4. **§B.3 旧 `commands::asset::move_asset_to_workspace_folder` 退役**：T3 实现时必须从 `lib.rs::invoke_handler!` 注销旧多素材入口；调用方迁移到循环调单素材新命令。Reviewer 需在 T3 PR 中确认 `invoke_handler!` 不再含旧入口。
5. **§D 文案表 `actionText` / `reasonText` / `featureText` 映射**：T2 实现 `errorMessages` 时这些映射表必须与本表一致；建议在 T2 单测中用 snapshot 锁定 11 项渲染结果（避免文案漂移）。
6. **§E 变更管控**：若 T1-T6 Dev 在实现过程中提出契约修改请求，**Reviewer 应当面拒绝在该 PR 内修改契约**，引导走 T0 修订流程。
