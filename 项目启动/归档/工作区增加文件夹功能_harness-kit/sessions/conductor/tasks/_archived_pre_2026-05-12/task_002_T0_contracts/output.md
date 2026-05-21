# Task 交付 — task_002_T0_contracts

## 实现摘要

T0 是纯文档契约冻结 task，不动 NCdesktop 仓库代码。产出单文件 `contracts.md`，按 input.md AC-1 完成 4 节内容：(a) IpcError JSON shape + 11 项 code 闭集 + 序列化协议（Rust/TS 双侧定义）；(b) 5 个 Tauri 命令 Rust 签名，逐字复制 PRD §5.1 且未改动 arity/参数名/类型；(c) `__ROOT__` 编解码契约（入站 `"__ROOT__"` → 空字符串 `""`、DB 永不存 sentinel、出站 list 恢复 `"__ROOT__"`、`debug_assert!(!path.contains("__ROOT__"))` 强约束）；(d) `errorMessages` 中文文案表 11 项，含 `E_FOLDER_DIRTY` 用 `details.now` 渲染、`E_NAME_DUP` 用 `details.name` 渲染。

核心设计决策：
1. 错误码闭集严格 11 项，**显式声明禁止** `E_DEPTH_LIMIT` / `E_CYCLE` / 递归 count 等 MVP 外契约（input.md 红线 3）。
2. `details` schema 表格化（AC-2）：必填字段与 optional 字段分列，下游 `From<E> for IpcError` 实现即可对表落字段。
3. 增加 (B.3)「错误码 ↔ 命令矩阵」表，把 11 code × 5 命令的可达性显式标 ✓/—，T3 实现单测时可直接照表覆盖分支。
4. 消费方清单（AC-3）按 T1/T2/T3/T4 分组列出"应从本文档抽哪些字段"，避免下游 Dev 二次读 PRD 时重新解释偏离。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `sessions/conductor/tasks/task_002_T0_contracts/contracts.md` | 新建 | 契约基线单文档，4 节 + 消费方核对清单 |
| `sessions/conductor/tasks/task_002_T0_contracts/output.md` | 新建 | 本交付文档 |

NCdesktop 仓库代码：**无任何改动**（input.md 已明确 T0 纯文档）。

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（本 task 不动代码目录，仅产 markdown 到 sessions/conductor/tasks/）
- [x] API 路径/命名与 Architect 方案一致：5 命令签名**逐字复制** PRD §5.1，与 Architect output.md「API 设计」一节完全一致；前端 camelCase wrapper 命名遵循 ADR-005 / Architect 方案
- [x] 数据模型与 Architect 方案一致：`IpcError` Rust/TS 形态遵循 ADR-001；`DeleteReport { trashed: u32 }` 与 PRD §5.1 注释一致；`__ROOT__` 契约遵循 ADR-004 三条入站/DB/出站规则
- [x] 未引入计划外的新依赖（纯文档）
- 偏离说明：无。`IpcErrorCode` Rust 端通过 `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` 让枚举判别字符串与 TS 字面量字符级对齐，是为兑现 ADR-001 序列化协议的必要约定，未引入新决策。

## 测试命令

```bash
grep -c "E_NAME_INVALID\|E_NAME_DUP\|E_NAME_RESERVED\|E_PATH_ESCAPE\|E_PROTECTED_KIND\|E_NOT_FOUND\|E_CROSS_DEVICE\|E_PLATFORM_UNSUPPORTED\|E_TRASH_FAILED\|E_FOLDER_DIRTY\|E_INTERNAL" /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/工作区增加文件夹功能_harness-kit/sessions/conductor/tasks/task_002_T0_contracts/contracts.md

# 附加 per-code 自检：
for c in E_NAME_INVALID E_NAME_DUP E_NAME_RESERVED E_PATH_ESCAPE E_PROTECTED_KIND E_NOT_FOUND E_CROSS_DEVICE E_PLATFORM_UNSUPPORTED E_TRASH_FAILED E_FOLDER_DIRTY E_INTERNAL; do
  echo "$c=$(grep -c "$c" /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/工作区增加文件夹功能_harness-kit/sessions/conductor/tasks/task_002_T0_contracts/contracts.md)"
done
```

## 测试结果

```
49
---per-code---
E_NAME_INVALID=5
E_NAME_DUP=4
E_NAME_RESERVED=4
E_PATH_ESCAPE=4
E_PROTECTED_KIND=4
E_NOT_FOUND=4
E_CROSS_DEVICE=4
E_PLATFORM_UNSUPPORTED=4
E_TRASH_FAILED=4
E_FOLDER_DIRTY=7
E_INTERNAL=5
```

总命中 49（AC-4 要求 ≥11）；11 个 code 各自命中 ≥4 次，均 > 1，AC-4 PASS。

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|----------|----------|------|-----------|
| ✅ 正常路径 | AC-1：4 节齐全（IpcError shape+11 code / 5 命令签名 / `__ROOT__` 编解码 / errorMessages 文案表） | 已测 | PASS — contracts.md 4 节标题分别为 `(a)/(b)/(c)/(d)`，人工逐节核对完整 |
| ✅ 正常路径 | AC-2：每个 code 的 details schema 明确列出 | 已测 | PASS — A.4 表格 11 行，每行有 `details schema` 列与「必填字段」列 |
| ✅ 正常路径 | AC-3：T1/T2/T3/T4 消费方核对清单 | 已测 | PASS — 文末「消费方核对清单」分 4 节，每节列具体引用小节号（如 A.3 / B.2 / C.1 / D.1） |
| ✅ 正常路径 | AC-4：grep 11 code 各 ≥1 次 | 已测 | PASS — 总 49，最低 `E_NAME_DUP` 等仍达 4 次 |
| ✅ 正常路径 | 5 命令签名逐字一致（红线 1） | 已测 | PASS — 与 PRD §5.1 行级 diff：命令名/参数名/参数顺序/参数类型/返回类型全部一致；`relative_path` / `expected_count: u32` / `confirm_non_empty: bool` 未改 |
| ⚠️ 边界条件 | `E_FOLDER_DIRTY` 文案使用 `details.now` 渲染 | 已测 | PASS — D.1 第 10 行明确「必须用 details.now 渲染」；D.2 参考实现读取 `d?.now` |
| ⚠️ 边界条件 | `E_NAME_DUP` 文案使用 `details.name` 渲染 | 已测 | PASS — D.1 第 2 行 + D.2 实现读取 `d?.name` |
| ⚠️ 边界条件 | `__ROOT__` 三向契约（入站归一、DB 不存、出站恢复） | 已测 | PASS — (c) 节 C.1 / C.2 / C.3 三小节分别覆盖；C.2 包含 `debug_assert!` 代码片段 |
| ❌ 异常路径 | 是否混入 MVP 外契约（如 `E_DEPTH_LIMIT` / 递归 count） | 已测 | PASS — 通篇 grep 无 `E_DEPTH_LIMIT` / `E_CYCLE`；文档开头红线声明显式禁止 |
| ❌ 异常路径 | 是否新增/缩减 code 闭集 | 已测 | PASS — A.4 表 1-11 项与 PRD §4.3 联合字面量一一对应，无增减 |

## 已知局限

1. **`WorkspaceFolderEntry` / `Asset` 既有类型未在本文档重新定义**：T0 范围仅冻结新增契约（IpcError、5 命令签名、`__ROOT__`、文案表），既有类型沿用 `src/types/workspace.ts` / `src-tauri/src/commands/workspace_folders.rs`。若 T3 实施时发现既有 `WorkspaceFolderEntry` 缺字段（如 `kind` 枚举值），需回到 T0 增补；目前 PRD 与 Architect output 未识别此缺口。
2. **`E_INTERNAL` 的 `details.hint` 字段语义弱**：仅约定可选 + 仅日志用，未规定具体字符串集合。MVP 范围内不展示给用户故影响低；若后续需要细分 INTERNAL 原因（如 DB busy / serde 失败），应回 T0 修订。
3. **`E_CROSS_DEVICE` 与 EXDEV copy-first 的语义边界**：A.4 说明该 code 仅在 copy 阶段失败时返回，rename 自身 EXDEV 走两阶段不返此码；T3 实现时需严格区分，单测应覆盖「rename EXDEV 成功走 copy-first」与「copy 阶段失败返 E_CROSS_DEVICE」两条路径。

## 需要 Reviewer 特别关注的地方

1. **(b) 5 命令签名逐字一致**：请把 contracts.md (b) 节与 PRD §5.1 / Architect output.md「API 设计」做字符级 diff，确认 `project_id` / `relative_path` / `target_relative_path` / `expected_count: u32` / `confirm_non_empty: bool` 等参数名与类型未被改动。这是 input.md 红线 1。
2. **(a) A.4 表第 1、2、3、10 行的 details 必填字段**：分别为 `name`、`name`、`name`、`{ old, now }`。下游文案渲染依赖这些字段必存在；如果 Reviewer 认为应改为 optional，需在 T0 而非 T2 阶段调整。
3. **(c) C.2 `debug_assert!` 而非 `assert!` 的选择**：参考 ADR-004「debug 构建命中即为契约违反必须修」。Reviewer 请确认这是否与底线 6「严禁 `__ROOT__` 入 DB」的力度匹配；如需 release 构建也强约束，应升级为 `assert!` 或返回 `E_INTERNAL`，但这会偏离 ADR-004 原文。
4. **A.3 `IpcErrorCode` Rust 枚举判别字符串**：依赖 `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` 让 `ENameInvalid` 序列化为 `"E_NAME_INVALID"`。该约定在 Architect output 中未明写，但是是兑现 ADR-001「TS code 字面量集合 = Rust 枚举判别字符串」的必要补充。Reviewer 请确认是否接受这一最小补强。
5. **(B.3) 错误码 ↔ 命令矩阵的 ✓/— 标注**：此表为本 task 主动新增（input.md 未要求但有助于下游单测覆盖）。Reviewer 如认为越界，可要求降级为附录或删除；标注本身依据 PRD §5.1 各命令语义推导，未引入新决策。
