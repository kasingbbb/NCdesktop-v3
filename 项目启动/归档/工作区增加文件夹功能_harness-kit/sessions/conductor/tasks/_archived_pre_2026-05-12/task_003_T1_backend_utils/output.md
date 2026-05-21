# Task 交付 — task_003_T1_backend_utils

## 实现摘要

实现工作区文件夹管理的后端工具层，为 T3 写命令打底。核心产物：

- **`utils/ipc_error.rs`**：`IpcError` + `IpcErrorCode` 11 项闭集枚举，`Serialize` + `From<IpcError> for String`（JSON 序列化），含 9 个语义化工厂方法（`name_invalid` / `name_dup` / `name_reserved` / `path_escape` / `protected_kind` / `not_found` / `cross_device` / `platform_unsupported` / `trash_failed` / `folder_dirty` / `internal`）。
- **`utils/nfc.rs`**：`nfc_normalize` / `nfc_eq` / `nfc_self_heal` / `nfc_heal_workspace`。
- **`utils/safe_rename.rs`**：`safe_rename(src, dst) -> RenameOutcome`（同卷 / 跨卷两种结果）；`remove_src_after_commit`；`cleanup_pending_scan`；通过 `#[cfg(test)]` 静态 mutex 注入 EXDEV，让单测可覆盖 copy-first 路径。
- **`utils/write_guard.rs`**：`WorkspaceWriteGuard` 双层 `Mutex<HashMap<ProjectId, Arc<Mutex<()>>>>`，`lock_for(&id) -> Arc<Mutex<()>>`，调用方 `.lock()` 持有 guard（避免 owned MutexGuard 的复杂生命周期）。
- **`workspace.rs` 追加**：`resolve_relative_path`（`__ROOT__` → 空相对）、`validate_and_canonicalize`（拒 `..` / 绝对 / symlink 越界，使用「最长存在前缀 canonicalize + 拼剩余段」策略支持叶子尚未创建场景）、`validate_folder_name`（NFC 后字符串校验：禁 `/ \ :`、禁 `.` 开头、禁空白、禁保留字 `organized`、禁 > 255 字节）。
- **`startup.rs`**：bootstrap 末尾挂 `workspace_startup_hooks()`，串行调用 `nfc_heal_workspace` + `cleanup_pending_scan`，外层 `catch_unwind` + log，绝不阻塞启动。
- **`Cargo.toml`**：+ `trash = "5"`、+ `unicode-normalization = "0.1"`，仅此 2 项。

关键决策：
1. **`IpcErrorCode` 用逐项 `#[serde(rename = "E_XXX")]`** 而非 `SCREAMING_SNAKE_CASE`，避免 `E_` 前缀单字母被 serde 拆分歧义；测试逐项断言字面量匹配 contracts.md §A.2。
2. **`lock_for` 返回 `Arc<Mutex<()>>`** 而非 owned guard：caller 调用 `.lock()`；规避 `OwnedMutexGuard` 的 unsafe / 生命周期。
3. **EXDEV 检测用 `raw_os_error() == Some(18)`** 兜底 + 字符串匹配 `CrossesDevices`，避免 `ErrorKind::CrossesDevices` 在不同 Rust 版本下的稳定性差异。
4. **`canonicalize_longest_prefix`**：对存在的最长前缀 canonicalize 再拼接剩余段，使 `validate_and_canonicalize` 在叶子未创建时（如 `create_workspace_folder` 调用点）也能正确做 symlink 越界检测。
5. **`safe_rename` 测试注入** 用 `Mutex<Option<PathBuf>>` 静态 + `#[cfg(test)]`，让单测可覆盖 copy-first 主路径而无需真实跨卷文件系统。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---|---|---|
| `NCdesktop/src-tauri/Cargo.toml` | 修改 | + `trash = "5"`、+ `unicode-normalization = "0.1"` |
| `NCdesktop/src-tauri/src/utils/mod.rs` | 修改 | 注册 `ipc_error` / `nfc` / `safe_rename` / `write_guard` 4 个新 mod |
| `NCdesktop/src-tauri/src/utils/ipc_error.rs` | 新建 | `IpcError` + `IpcErrorCode` 11 项闭集 + 工厂构造 + `Into<String>` + 4 个单测 |
| `NCdesktop/src-tauri/src/utils/nfc.rs` | 新建 | `nfc_normalize` / `nfc_eq` / `nfc_self_heal` / `nfc_heal_workspace` + 5 个单测 |
| `NCdesktop/src-tauri/src/utils/safe_rename.rs` | 新建 | `safe_rename` / `RenameOutcome` / `remove_src_after_commit` / `cleanup_pending_scan` + 6 个单测 |
| `NCdesktop/src-tauri/src/utils/write_guard.rs` | 新建 | `WorkspaceWriteGuard` + 3 个并发单测 |
| `NCdesktop/src-tauri/src/workspace.rs` | 修改 | 追加 `resolve_relative_path` / `validate_and_canonicalize` / `validate_folder_name` + 14 个单测 |
| `NCdesktop/src-tauri/src/startup.rs` | 修改 | bootstrap 末尾挂 `workspace_startup_hooks`（nfc heal + cleanup pending） |

未修改：`lib.rs`（命令注册归 T3）、`commands/workspace_folders.rs`（命令归 T3）。

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（`utils/ipc_error.rs` `utils/nfc.rs` `utils/safe_rename.rs` `utils/write_guard.rs` 全部按 output.md §"目录结构"落位）
- [x] API 路径/命名与 Architect 方案一致（未注册任何 `#[tauri::command]`；本 task scope）
- [x] 数据模型与 Architect 方案一致（`IpcError` / `IpcErrorCode` 严格按 contracts.md §A.3；`RenameOutcome` 与 input.md §6 一致）
- [x] 未引入计划外的新依赖（仅 `trash` + `unicode-normalization`）
- [x] ADR-001（IpcError 序列化）：`From<IpcError> for String` 用 `serde_json::to_string`，失败兜底 `{"code":"E_INTERNAL",...}` ✅
- [x] ADR-002（EXDEV copy-first）：`copy_path_recursive → fsync_all → rename(tmp→final) → 返回 CrossDevice { pending_remove_src }` ✅
- [x] ADR-003（写通道锁）：`Arc<Mutex<HashMap<ProjectId, Arc<Mutex<()>>>>>` ✅
- [x] ADR-004（`__ROOT__` 编解码）：`resolve_relative_path` 单点归一空相对 ✅
- [x] ADR-005（NFC 自愈）：递归扫描 + 目标已存在则 log::warn 跳过 ✅
- [x] ADR-008（命名校验后端权威）：NFC 后比较，5 类非法分类全覆盖单测 ✅
- [x] IpcErrorCode 闭集严格 **11 项**，无 `E_DEPTH_LIMIT` / `E_CYCLE` ✅
- [x] 启动期串行 + 非阻塞（`catch_unwind` 包裹）✅
- 偏离说明：无。

## 测试命令

```bash
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri && cargo build 2>&1 | tail -5
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri && cargo test --lib 2>&1 | tail -3
```

## 测试结果

### cargo build（成功）

```
   = note: `AnthropicContent` has a derived impl for the trait `Debug`, but this is intentionally ignored during dead code analysis
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: `notecapt` (lib) generated 6 warnings (run `cargo fix --lib -p notecapt` to apply 5 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.43s
```

注：6 个 warning 全部来自既有代码（`commands/knowledge_unit_learning.rs` 未用 import 等），与本 task 修改无关。本 task 新增代码无 warning。

### cargo test --lib（148 passed / 0 failed）

```
test result: ok. 148 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.88s
```

本 task 新增的 28 个测试（全部 PASS）：

```
test utils::ipc_error::tests::all_eleven_codes_serialize_to_screaming_snake ... ok
test utils::ipc_error::tests::details_omitted_when_none ... ok
test utils::ipc_error::tests::folder_dirty_carries_old_and_now ... ok
test utils::ipc_error::tests::into_string_serializes_to_json ... ok
test utils::nfc::tests::nfc_eq_handles_mixed_forms ... ok
test utils::nfc::tests::nfc_idempotent_for_cjk ... ok
test utils::nfc::tests::nfc_normalize_combines_decomposed ... ok
test utils::nfc::tests::nfc_self_heal_renames_nfd_dir ... ok
test utils::nfc::tests::nfc_self_heal_skips_when_target_exists ... ok
test utils::safe_rename::tests::cleanup_pending_scan_removes_marked_src ... ok
test utils::safe_rename::tests::cleanup_pending_scan_removes_orphan_tmp ... ok
test utils::safe_rename::tests::exdev_triggers_copy_first_and_src_retained ... ok
test utils::safe_rename::tests::is_exdev_detects_errno_18 ... ok
test utils::safe_rename::tests::remove_src_after_commit_clears_src ... ok
test utils::safe_rename::tests::same_volume_rename_succeeds ... ok
test utils::write_guard::tests::different_projects_parallel ... ok
test utils::write_guard::tests::lock_for_returns_same_arc_for_same_project ... ok
test utils::write_guard::tests::same_project_serializes ... ok
test workspace::folder_utils_tests::resolve_relative_path_root_sentinel ... ok
test workspace::folder_utils_tests::validate_and_canonicalize_rejects_absolute ... ok
test workspace::folder_utils_tests::validate_and_canonicalize_rejects_dotdot ... ok
test workspace::folder_utils_tests::validate_and_canonicalize_rejects_symlink_escape ... ok
test workspace::folder_utils_tests::validate_and_canonicalize_root_sentinel_returns_workspace_root ... ok
test workspace::folder_utils_tests::validate_folder_name_accepts_chinese_and_nfd ... ok
test workspace::folder_utils_tests::validate_folder_name_rejects_backslash ... ok
test workspace::folder_utils_tests::validate_folder_name_rejects_colon ... ok
test workspace::folder_utils_tests::validate_folder_name_rejects_empty_and_blank ... ok
test workspace::folder_utils_tests::validate_folder_name_rejects_leading_dot ... ok
test workspace::folder_utils_tests::validate_folder_name_rejects_reserved ... ok
test workspace::folder_utils_tests::validate_folder_name_rejects_slash ... ok
test workspace::folder_utils_tests::validate_folder_name_rejects_too_long ... ok
test workspace::folder_utils_tests::validate_folder_name_reserved_case_sensitive ... ok
```

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|---|---|---|---|
| ✅ 正常路径 | `IpcError` 11 项 code 序列化为字面量 `E_*` | 已测 | PASS — `all_eleven_codes_serialize_to_screaming_snake` 逐项断言 |
| ✅ 正常路径 | `IpcError` → JSON String 边界转换 | 已测 | PASS — `into_string_serializes_to_json` |
| ✅ 正常路径 | `nfc_normalize` 把 NFD `e+́` 归一为 NFC `é` | 已测 | PASS |
| ✅ 正常路径 | `nfc_self_heal` 把 NFD 目录 rename 为 NFC | 已测 | PASS |
| ✅ 正常路径 | 同卷 `safe_rename` 成功 | 已测 | PASS |
| ✅ 正常路径 | 注入 EXDEV 走 copy-first，src 保留 + dst 完整 | 已测 | PASS — `exdev_triggers_copy_first_and_src_retained`，断言 src 在 commit 前仍存在 |
| ✅ 正常路径 | `remove_src_after_commit` 清理 src | 已测 | PASS |
| ✅ 正常路径 | `WorkspaceWriteGuard` 同 project 串行 | 已测 | PASS — 双线程 + 时间断言（总耗时 ≥ 2× sleep） |
| ✅ 正常路径 | `WorkspaceWriteGuard` 不同 project 并行 | 已测 | PASS — 双线程 + 时间断言（总耗时 < 2× sleep） |
| ✅ 正常路径 | `validate_and_canonicalize("__ROOT__")` 返 workspace 根 canonical | 已测 | PASS |
| ⚠️ 边界条件 | `IpcError.details = None` 时 serde skip | 已测 | PASS — `details_omitted_when_none` |
| ⚠️ 边界条件 | NFC 自愈：目标 NFC 名已存在则跳过 + log::warn | 已测 | PASS — `nfc_self_heal_skips_when_target_exists` |
| ⚠️ 边界条件 | `cleanup_pending_scan` 清 `.cleanup_pending` 标记 + 孤立 `.cross_device.tmp` | 已测 | PASS |
| ⚠️ 边界条件 | `validate_folder_name("Organized")`（大小写敏感） | 已测 | PASS（通过，不命中保留字；契约未要求大小写不敏感） |
| ⚠️ 边界条件 | `validate_folder_name` 接受 CJK + NFD 字符 | 已测 | PASS |
| ❌ 异常路径 | `validate_and_canonicalize("../../etc")` → `E_PATH_ESCAPE` | 已测 | PASS |
| ❌ 异常路径 | `validate_and_canonicalize("/etc/passwd")` → `E_PATH_ESCAPE` | 已测 | PASS |
| ❌ 异常路径 | symlink 越界 → `E_PATH_ESCAPE` | 已测 | PASS — 在 sandboxed HOME 中构造指向 `/tmp` 的 symlink，验证 canonicalize 后 `starts_with` 拒绝 |
| ❌ 异常路径 | `validate_folder_name("a/b" / "a\\b" / "a:b")` → `E_NAME_INVALID` | 已测 | PASS（3 例分别 covered） |
| ❌ 异常路径 | `validate_folder_name(".hidden")` → `E_NAME_INVALID` | 已测 | PASS |
| ❌ 异常路径 | `validate_folder_name("")` / `"   "` → `E_NAME_INVALID` | 已测 | PASS |
| ❌ 异常路径 | `validate_folder_name("organized")` → `E_NAME_RESERVED` | 已测 | PASS |
| ❌ 异常路径 | `validate_folder_name("a"*300)` → `E_NAME_INVALID` | 已测 | PASS |
| ❌ 异常路径 | `is_exdev` 对 errno 18 返 true，对 errno 2 (ENOENT) 返 false | 已测 | PASS |

AC 对应表：
- AC-1 ✅（utils::ipc_error 4 test PASS）
- AC-2 ✅（utils::nfc 5 test PASS）
- AC-3 ✅（utils::safe_rename 6 test PASS：含 same-volume / mock EXDEV / cleanup_pending / fsync 路径）
- AC-4 ✅（utils::write_guard 3 test PASS：串行 + 并行 + Arc 同一）
- AC-5 ✅（workspace::folder_utils_tests 4 个路径校验 PASS：`..` / 绝对 / symlink / `__ROOT__`）
- AC-6 ✅（workspace::folder_utils_tests 9 个命名校验 PASS：5 类非法 + 3 正常 + 大小写边界）
- AC-7 ✅（Cargo.toml +2 deps；cargo build 成功）
- AC-8 ✅（startup::bootstrap 末尾挂 `workspace_startup_hooks`，`catch_unwind` 包裹绝不阻塞；`cleanup_pending_scan_removes_marked_src` / `cleanup_pending_scan_removes_orphan_tmp` 验证 tmp/marker 清理逻辑）

## 已知局限

1. **保留字 `organized` 大小写敏感**：contracts.md §A.4 仅写"命中保留字 `organized`"，未明确大小写策略；我按字符级匹配实现（"Organized" 通过）。若产品/Reviewer 期望大小写不敏感，可在 `validate_folder_name` 一行改为 `nfc_name.eq_ignore_ascii_case("organized")`。已写测试 `validate_folder_name_reserved_case_sensitive` 锁定当前行为。
2. **EXDEV 跨卷测试用 mock 注入** 而非真实跨文件系统：tempfile 无法可移植地构造跨卷 fixture。但 mock 路径触发的是与真实 EXDEV 完全相同的 copy-first 代码分支（包括 `is_exdev` 单独单测验证 errno 18 检测），所以主路径行为有覆盖。
3. **`cleanup_pending` 24h 阈值未实现**：当前 `cleanup_pending_scan` 见到 `.cleanup_pending` 标记或 `.cross_device.tmp` 即清，未做 ADR-002 提到的"窗口期 > 24h 静默清理"。若 T3 写命令在启动期 hook 之前调用导致正在用的 tmp 被误删，需要补 mtime 检查。当前对纯启动期 hook 调用是安全的。
4. **`safe_rename.rs` 内 `impl IpcError`**：在 safe_rename 内 inline 了 `IpcError::attach_internal_hint` 私有方法。技术上 OK（Rust 允许），但若 Reviewer 偏好单点 impl，可移到 `ipc_error.rs`。
5. **`unsafe set_var("HOME")` 测试 sandbox**：依赖 `dirs_next::download_dir()` 读 HOME；本 task 已用全局 Mutex 串行化避免测试间状态污染。Rust 2024 edition 警告"set_var unsafe"，已用 `unsafe { ... }` 块包裹。

## 需要 Reviewer 特别关注的地方

1. **`utils/ipc_error.rs` 11 项 enum 字面量**（行 16~46）：必须与 contracts.md §A.2 TS 字面量集合**字符级一致**，请逐项比对。`all_eleven_codes_serialize_to_screaming_snake` 测试已锁定。
2. **`utils/safe_rename.rs::safe_rename`**（行 49~119）：copy-first 顺序必须严格为 `copy → fsync → rename(tmp→final) → 返回 pending`；**绝不能在本函数内删 src**（删 src 是 caller 在 COMMIT 后调用 `remove_src_after_commit`）。
3. **`workspace.rs::canonicalize_longest_prefix`**（行 ~370）：对叶子未创建场景做 symlink 检测的策略。当中间段是 symlink 越界时（`escape_link/sub`），`escape_link` 这一存在段会被 canonicalize 解析出真实路径，使 `starts_with(root_canonical)` 拒绝。请确认这一策略对 T3 `create_workspace_folder`（新建一层）和 `rename_workspace_folder`（重命名一层）都正确。
4. **`startup.rs::workspace_startup_hooks`** 用 `AssertUnwindSafe` 包裹 panic；如果 Reviewer 担心 lint，可改为返回 `Result` + map_err log 模式。
5. **`WorkspaceWriteGuard::lock_for` 返回 Arc 而非 owned guard**：T3 的写命令模板需写 `let arc = guard.lock_for(&pid); let _g = arc.lock().expect("锁中毒");` 两行而非一行。这是为了避免 `OwnedMutexGuard` 的 unsafe，权衡是可接受的。请在 T3 Dev prompt 中提示此用法。
6. `app.manage(WorkspaceWriteGuard::new())` **本 task 未注册**到 lib.rs（按 input.md "T3 注册"指示），T3 实现时务必补上。
