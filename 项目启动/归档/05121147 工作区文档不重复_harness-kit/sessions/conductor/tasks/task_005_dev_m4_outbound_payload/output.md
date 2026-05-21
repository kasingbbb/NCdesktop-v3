# Task 交付 — task_005_dev_m4_outbound_payload

## 实现摘要

实现 Tauri 命令 `prepare_outbound_payload(asset_ids)`，把多选 done 态资产的 canonical
markdown rendition 通过 `fs::hard_link` → 跨卷 `fs::copy` fallback 投影到
`~/Library/Caches/NCdesktop/outbound/{asset_id}/<sanitized>.md`，返回 `OutboundEntry[]`。
错误以 `OutboundError` 联合体序列化为 JSON 字符串通过 `Err(String)` 通道上抛，前端
`tauri-commands.ts` 提供 `prepareOutboundPayload` + `parseOutboundError` 配套 wrapper。

核心设计决策：
1. **状态判定走纯函数**：复用 `db::asset::list_root_assets` + `compute_asset_state`，
   commands 层不拼任何 SQL（硬约束）。多选时按 `project_id` 分组批量查询，再用
   `Path::exists()` 在命令层 stat rendition / source。
2. **asset_id 鲁棒**：通过 `db::asset::resolve_asset_pair`（task_004）把 derivative.id
   也解算回 root，与未来知识中枢点击 .md 条目场景对齐。
3. **EXDEV 处理**：照抄 `dropzone.rs::try_rename_or_copy_remove` 模式，直接用
   `io::ErrorKind::CrossesDevices`（Rust 1.94 已稳定，dropzone 已在用）。
4. **sanitize 严格按 PRD §4.4**：单遍替换 → UTF-8 字节截断对齐 char_boundary →
   尾随 `.`/空格 → Windows 保留字（大小写无关） → 空值兜底；6 个规则各有专测。
5. **缓存目录幂等**：每次 `remove_dir_all` → `create_dir_all`，避免上一次 sanitize
   不同的旧文件残留导致前端 startDrag 多文件污染。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `NCdesktop/src-tauri/src/commands/outbound.rs` | 新建 | 命令实现 + `sanitize_outbound_filename` + `OutboundError` + 12 单测 |
| `NCdesktop/src-tauri/src/commands/mod.rs` | 修改 | 追加 `pub mod outbound;` |
| `NCdesktop/src-tauri/src/lib.rs` | 修改 | `invoke_handler!` 末尾追加 `commands::outbound::prepare_outbound_payload`（不重排既有命令） |
| `NCdesktop/src/lib/tauri-commands.ts` | 修改 | 追加 `OutboundEntry` / `OutboundError` 类型 + `prepareOutboundPayload` / `parseOutboundError` |

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（`commands/outbound.rs` 新建文件，未触碰其它模块归属）
- [x] API 路径/命名与 Architect 方案一致（`prepare_outbound_payload` / `OutboundEntry` 与 input.md AC-1 字段一致；`camelCase` JSON）
- [x] 数据模型与 Architect 方案一致（复用 `WorkspaceAssetView`/`AssetState`/`AssetListJoinRow`，无新 DB 表 / 字段）
- [x] 未引入计划外的新依赖（仅用已有 `dirs-next`、`tempfile`(dev)、`serde_json`）
- 偏离说明：无。ADR-005（缓存目录策略）、ADR-007（resolve_asset_pair）、ADR-008（NSStringPboardType 留 spike）均按文档严格执行。

## 测试命令

```bash
cd NCdesktop/src-tauri && cargo test -p notecapt --lib commands::outbound
npm --prefix NCdesktop run check
```

## 测试结果

```
   Compiling notecapt v0.1.0 (/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri)
warning: unused import: `PathBuf`
  --> src/commands/dropzone.rs:10:23
   |
10 | use std::path::{Path, PathBuf};
   |                       ^^^^^^^
   |
   = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: unused variable: `client`
   --> src/llm/chat.rs:109:5
（…仓库既有 warning，与本 task 无关，已省略 …）
warning: `notecapt` (lib test) generated 5 warnings (run `cargo fix --lib -p notecapt --tests` to apply 4 suggestions)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 5.39s
     Running unittests src/lib.rs (target/debug/deps/app_lib-8c2b6ae4be6c948e)

running 12 tests
test commands::outbound::tests::outbound_error_serializes_to_camel_case_json ... ok
test commands::outbound::tests::sanitize_replaces_slash_and_backslash ... ok
test commands::outbound::tests::sanitize_strips_control_chars_and_del ... ok
test commands::outbound::tests::sanitize_preserves_cjk_and_emoji ... ok
test commands::outbound::tests::classify_state_all_done_passes ... ok
test commands::outbound::tests::sanitize_trailing_dot_or_space_appends_underscore ... ok
test commands::outbound::tests::classify_state_single_non_done_returns_state_not_done ... ok
test commands::outbound::tests::classify_state_mixed_returns_mixed_states_with_offending ... ok
test commands::outbound::tests::sanitize_truncates_long_utf8_and_appends_asset_id_suffix ... ok
test commands::outbound::tests::sanitize_windows_reserved_appends_underscore ... ok
test commands::outbound::tests::reset_outbound_dir_is_idempotent_and_empties_existing_files ... ok
test commands::outbound::tests::link_or_copy_rendition_happy_path_creates_file_with_same_content ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 113 filtered out; finished in 0.00s
```

前端 `npm run check`：

```
> ncdesktop@0.0.0 check
> tsc --noEmit
```

无输出（无 TS 错误）。

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|----------|----------|------|-----------|
| ✅ 正常路径 | sanitize 6 个规则分支（/\\、控制字符、CJK/emoji、Windows 保留字、尾随 dot/space、长 UTF-8 截断+id 后缀） | 已测 | PASS（6 个单测全绿） |
| ✅ 正常路径 | `link_or_copy_rendition` happy path：hardlink 成功并落盘 | 已测 | PASS（tempdir 内验证字节相等） |
| ✅ 正常路径 | `reset_outbound_dir` 二次调用清空旧文件 | 已测 | PASS（写入 stale.md 后再调用，stale.md 不存在） |
| ✅ 正常路径 | `classify_state` 全 done → Ok | 已测 | PASS |
| ⚠️ 边界条件 | sanitize 截断后字符边界对齐（3 字节字符 "好" × 100 = 300B） | 已测 | PASS（结果只含完整 "好"，长度 ≤ 209 字节，后缀 `_<id8>`） |
| ⚠️ 边界条件 | Windows 保留字大小写无关（CON / lpt3） | 已测 | PASS（两者均追加 `_`） |
| ❌ 异常路径 | 单选非 done → `StateNotDone { state, message }` JSON | 已测 | PASS（state 字段为 `"converting"` 小写） |
| ❌ 异常路径 | 多选混合（done/failed/done/offline）→ `MixedStates { offending: [a2, a4] }` | 已测 | PASS（offending 顺序与输入对齐） |
| ❌ 异常路径 | `OutboundError` JSON 字段名 camelCase（`kind` / `assetId`） | 已测 | PASS（`outbound_error_serializes_to_camel_case_json`） |
| ⚠️ 边界条件 | 跨卷 fallback（`CrossesDevices` → `fs::copy`） | 未测 | 单机测试无法构造跨卷场景；走与 dropzone 同源 pattern，依赖 `ErrorKind::CrossesDevices` 在 Rust 1.94 稳定（dropzone.rs 已用） |
| ⚠️ 边界条件 | 命令端到端 happy path（real Database + State<…>） | 未测 | 需 Tauri runtime / 真 SQLite 初始化；当前覆盖在状态分类 + IO 落盘两个纯函数层，集成留给 reviewer 手测 / 后续 E2E |
| ❌ 异常路径 | rendition_path 在 DB 有但磁盘缺失 → `RenditionMissing` | 未测 | 同上，需要 Database state；逻辑在 `prepare_outbound_payload` 第 3 步 `Path::new(p).exists()` 分支显式覆盖 |

## 已知局限

1. **命令本体未做集成测试**：`prepare_outbound_payload` 依赖 `tauri::State<Database>`，
   在 lib 测试中构造一个真实 Database + 多 asset 行成本较高（需要 schema migration +
   pipeline_tasks 行 + 真实 markdown 文件）。当前选择是把核心逻辑拆为纯函数
   （`sanitize_outbound_filename` / `classify_state` / `reset_outbound_dir` /
   `link_or_copy_rendition`）单独覆盖，端到端路径靠 reviewer 手动拖出验证。
2. **跨卷 copy fallback 无自动化测试**：与 dropzone 同源问题；CI 单机无法挂第二个
   文件系统。改靠代码同源模式 + dropzone 历史回归保证。
3. **多 project 混合调用**：`collect_state_inputs` 会按 project_id 分组批量查询。
   若用户从多个 project 同时多选拖出，会触发 N（project 数）次 `list_root_assets`，
   性能可控但非最优；按 PRD §4.4 工作区限定单 project 多选语义，此分支几乎不会触发。
4. **`state` 字段反射**：`classify_state` 里把 `AssetState` 通过 `serde_json::to_value`
   提取小写名（`"converting"`/`"failed"`/`"offline"`），仰仗 `#[serde(rename_all = "lowercase")]`
   不变；若未来 `AssetState` 改命名策略需同步调整测试。

## 需要 Reviewer 特别关注的地方

1. **`collect_state_inputs` 的 SQL 复用策略**（commands/outbound.rs 第 ~330 行）：
   是否同意"按 project_id 分组复用 `list_root_assets`"作为"不在 commands/ 拼 SQL"
   的实现路径？相比新增 `db::asset::get_join_row_by_id`，本方案无需新增 db 函数，
   但代价是若入参 asset_ids 跨多 project 会触发多次 N=工作区资产数的全量查询。
2. **`OutboundError` 的 `kind` 字段值**（commands/outbound.rs 第 ~50 行）：
   每个 variant 上的 `#[serde(rename = "stateNotDone")]` 等是否符合前端期望命名？
   `tauri-commands.ts` 的 `OutboundError` 联合类型与之一一对齐，可直接 grep 对照。
3. **缓存目录路径**（`{cache_root}/NCdesktop/outbound/{asset_id}/`）：
   macOS 上展开为 `~/Library/Caches/NCdesktop/outbound/{asset_id}/`，符合 ADR-005
   与 input.md AC-3。Linux 下展开为 `~/.cache/NCdesktop/outbound/...`，可用但非
   重点平台；Windows 暂未考虑（NCdesktop 当前仅 macOS）。
4. **sanitize 截断后再次走"尾随 dot/space"检查的顺序**：当前实现是先截断（步骤 4-5），
   再做尾随 dot/space（步骤 6）。极端 case：display_name = `"a".repeat(199) + "."`，
   长度 200 字节恰好不截断，末尾的 `.` 会触发追加 `_` → 总长 201 字节，仍 < 255。
   规则顺序与 PRD §4.4 描述一致；未在测试中覆盖该 corner，依赖代码自检。
