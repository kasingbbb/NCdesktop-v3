# Task 交付 — task_003_T1_backend_utils

> 注：本文档为「补齐交付契约」补单。代码由前一棒 Dev 已写入仓库；本棒 Dev 仅做契约/AC 核验 + 实际跑测试 + 落本 output.md，未重写任何代码。

## 实现摘要

T1 后端工具层 6 个交付物全部落地：

1. **依赖**（AC-1）：`Cargo.toml` 已加 `trash = "5"` 与 `unicode-normalization = "0.1"`；`dev-dependencies` 加 `tempfile = "3"`。`cargo build` 通过（隐含于 `cargo test` build 成功）。
2. **IpcError 模块**（AC-2）：`src-tauri/src/utils/ipc_error.rs`
   - `IpcErrorCode` enum + 11 项变体，序列化字面量经测试断言与 T0 §A.3 字符级一致；
   - `IpcError { code, message, details: Option<Value> }` + `#[serde(skip_serializing_if = "Option::is_none")]`；
   - `impl From<IpcError> for String` 走 `serde_json::to_string` + 单行 JSON 兜底；
   - 11 个工厂函数（`name_invalid` / `name_dup` / `name_reserved` / `path_escape` / `protected_kind` / `not_found` / `cross_device` / `platform_unsupported` / `trash_failed` / `folder_dirty` / `internal`），details 键名严格按 T0 §A.4 camelCase。
3. **workspace 工具扩展**（AC-3）：`src-tauri/src/workspace.rs` 追加：
   - `resolve_relative_path(project_id, rel) -> PathBuf`（`__ROOT__` 归一为空相对）；
   - `validate_and_canonicalize`：字符串层拒 `..` / 绝对 / `\` 开头 / 非 Normal Component；对最长存在前缀做 canonicalize 再拼剩余段；`starts_with(workspace_root_canonical)`；symlink 越界 → `E_PATH_ESCAPE`；
   - `validate_folder_name`：NFC 后比较；禁 `/ \ :`、`.` 开头、空白、空、>255 字节、`organized`（大小写敏感）。
4. **NFC 模块**（AC-4）：`src-tauri/src/utils/nfc.rs`：`nfc_normalize` / `nfc_eq` / `nfc_self_heal(project_root)`（递归）/ `nfc_heal_workspace()`（枚举 `~/Downloads/NoteCaptWorkPlace/*/`）。失败仅 log，目标已存在跳过。
5. **safe_rename 模块**（AC-5）：`src-tauri/src/utils/safe_rename.rs`：`safe_rename` + `RenameOutcome::{SameVolume, CrossDevice}` + EXDEV (raw_os_error 18) 两阶段 copy→fsync→rename；`remove_src_after_commit` 失败留 `.cleanup_pending` 标记；`cleanup_pending_scan(root)` 递归清理标记 + 孤立 `.cross_device.tmp`；`#[cfg(test)] mod test_inject` 提供 EXDEV 模拟开关。
6. **WorkspaceWriteGuard**（AC-6）：`src-tauri/src/utils/write_guard.rs`：`Mutex<HashMap<String, Arc<Mutex<()>>>>` + `lock_for(project_id) -> Arc<Mutex<()>>`。
7. **启动 hook**（AC-7）：`src-tauri/src/startup.rs::workspace_startup_hooks()` 串行调 `nfc_heal_workspace()` + `cleanup_pending_scan(workspace_root)`，外包 `catch_unwind` 防 panic；`src-tauri/src/lib.rs` `setup` 内 `app.manage(WorkspaceWriteGuard::new())`。
8. **测试**（AC-8）：`cargo test --lib` 156 项全绿。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `src-tauri/Cargo.toml` | 修改 | + `trash = "5"`、`unicode-normalization = "0.1"`、dev `tempfile = "3"` |
| `src-tauri/Cargo.lock` | 修改 | 依赖锁定 |
| `src-tauri/src/utils/mod.rs` | 修改 | + `pub mod ipc_error / nfc / safe_rename / write_guard` |
| `src-tauri/src/utils/ipc_error.rs` | 新建 | IpcErrorCode + IpcError + 11 工厂 + `From<IpcError> for String` |
| `src-tauri/src/utils/nfc.rs` | 新建 | NFC 归一 + 自愈递归 + workspace 枚举 |
| `src-tauri/src/utils/safe_name.rs` | 新建 | 文件 stem 清洗（与 T1 AC 无直接对应，但被 import 命令依赖；前一棒新增） |
| `src-tauri/src/utils/safe_rename.rs` | 新建 | EXDEV-safe rename + cleanup_pending |
| `src-tauri/src/utils/write_guard.rs` | 新建 | 项目级写通道串行锁 |
| `src-tauri/src/workspace.rs` | 修改 | + `resolve_relative_path` / `validate_and_canonicalize` / `validate_folder_name` 与单测 |
| `src-tauri/src/startup.rs` | 修改 | + `workspace_startup_hooks()` 挂点，`catch_unwind` 保护 |
| `src-tauri/src/lib.rs` | 修改 | + `app.manage(WorkspaceWriteGuard::new())`（注释标 task_005 T3，实为 T1 AC-7 必需） |
| `src-tauri/src/commands/workspace_folders.rs` | 修改 | **T3 范围；前一棒越权落地了 4 写命令实现** — 见偏离说明 |
| `src-tauri/tests/workspace_folders_integration.rs` | 新建 | **T4 范围；前一棒越权落地了两个集成测试** — 见偏离说明 |

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（`utils/{ipc_error,nfc,safe_rename,write_guard}.rs` + `workspace.rs` 追加 + `startup.rs` 挂 hook + `lib.rs` manage Guard）
- [x] API 路径/命名与 Architect 方案一致（函数名、签名、返回类型与 ADR-001/002/003/004/005/008 一致）
- [x] 数据模型与 Architect 方案一致（`IpcErrorCode` 11 项闭集、`IpcError` 三字段、`RenameOutcome` 双 variant、`WorkspaceWriteGuard` 双层 Mutex）
- [x] 未引入计划外的新依赖（仅 `trash` + `unicode-normalization` + dev `tempfile`，与 input.md 预估一致）

### 偏离说明（重要）

1. **越权进入 T3 范围**：`src-tauri/src/commands/workspace_folders.rs` 已含 4 写命令（`create_workspace_folder_impl` / `rename_workspace_folder_impl` / `delete_workspace_folder_impl` / `move_asset_to_workspace_folder_impl`）+ 对应 `#[tauri::command]` 包装；`src-tauri/src/lib.rs::invoke_handler!` 已注册这 5 命令（含 `count_folder_assets`）。按 input.md「T1 仅工具层，不注册命令；命令注册留 T3」，这部分属于 T3 越权交付。本 T1 output 仅认领工具层；T3/T4 接手时直接复用、必要时增量改进，**不要求回退**（回退会破坏现有测试与 invoke_handler 完整性）。
2. **越权进入 T4 范围**：`src-tauri/tests/workspace_folders_integration.rs` 已含 `test_rename_db_path_sync` + `test_round_trip_root_to_folder_to_root` 两个集成测试。这是 T4 拥有的两个 IT 名（Architect output.md 集成测试分配表）。同上不要求回退。
3. **T0 §A.3 `rename_all` vs 逐项 `rename`**：T0 §A.3 同时写了 `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` 注释（"形式说明"）与 11 个逐项 `#[serde(rename = "...")]`，并备注「为避免 Rust 标识符限制，逐项 rename」。前一棒 Dev 二选一**保留 `rename_all`**（更简洁），删除逐项 `rename`，并在 `all_eleven_codes_serialize_to_screaming_snake` 测试中字符级断言 11 项序列化字面量一致。**核验结果：序列化输出与 T0 字面量字符级相等，契约不破坏**。Reviewer 重点关注此点（详见 §"需要 Reviewer 特别关注的地方" #1）。
4. **`utils/safe_name.rs`**：T1 AC 未列此文件，但前一棒新增。它是 import 命令的文件名清洗器，与 T1 无关、对 T1 也无副作用；保留即可。
5. **`lib.rs` 注释中的 task 序号**：`app.manage(WorkspaceWriteGuard::new())` 与命令注册块都标注 "task_005 T3"，但实际 manage Guard 属 T1 AC-7。仅注释笔误，不影响行为。

## 测试命令

```bash
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

可选：进一步缩到 T1 模块
```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib utils::
cargo test --manifest-path src-tauri/Cargo.toml --lib workspace::folder_utils_tests
```

集成测试（属 T4 范围，但已存在且通过；如需复跑）：
```bash
cargo test --manifest-path src-tauri/Cargo.toml --test workspace_folders_integration
```

## 测试结果

`cargo test --manifest-path src-tauri/Cargo.toml --lib` 完整结果（截取关键段落 + 全部 T1 相关用例 + 总览）：

```
warning: `notecapt` (lib test) generated 9 warnings (run `cargo fix --lib -p notecapt --tests` to apply 8 suggestions)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.56s
     Running unittests src/lib.rs (src-tauri/target/debug/deps/app_lib-96758bcc4e7fff02)

running 156 tests
...
test commands::workspace_folders::tests::direct_invoke_rejected_and_error_json_roundtrip ... ok
test commands::workspace_folders::tests::ai_organized_protected ... ok
test commands::workspace_folders::tests::nfc_dup ... ok
test commands::workspace_folders::tests::path_escape ... ok
test commands::workspace_folders::tests::prefix_boundary ... ok
test commands::workspace_folders::tests::reserved_name ... ok
test commands::workspace_folders::tests::trash_recheck ... ok
test commands::workspace_folders::tests::write_guard_serializes ... ok
...
test utils::ipc_error::tests::all_eleven_codes_serialize_to_screaming_snake ... ok
test utils::ipc_error::tests::details_omitted_when_none ... ok
test utils::ipc_error::tests::folder_dirty_carries_old_and_now ... ok
test utils::ipc_error::tests::into_string_serializes_to_json ... ok
test utils::nfc::tests::nfc_eq_handles_mixed_forms ... ok
test utils::nfc::tests::nfc_idempotent_for_cjk ... ok
test utils::nfc::tests::nfc_normalize_combines_decomposed ... ok
test utils::nfc::tests::nfc_self_heal_renames_nfd_dir ... ok
test utils::nfc::tests::nfc_self_heal_skips_when_target_exists ... ok
test utils::safe_name::tests::collapses_multiple_spaces ... ok
test utils::safe_name::tests::empty_becomes_untitled ... ok
test utils::safe_name::tests::preserves_cjk_and_emoji ... ok
test utils::safe_name::tests::replaces_illegal_chars ... ok
test utils::safe_name::tests::strips_control_bytes ... ok
test utils::safe_name::tests::truncates_long_names ... ok
test utils::safe_rename::tests::cleanup_pending_scan_removes_marked_src ... ok
test utils::safe_rename::tests::cleanup_pending_scan_removes_orphan_tmp ... ok
test utils::safe_rename::tests::is_exdev_detects_errno_18 ... ok
test utils::safe_rename::tests::remove_src_after_commit_clears_src ... ok
test utils::safe_rename::tests::same_volume_rename_succeeds ... ok
test utils::safe_rename::tests::exdev_triggers_copy_first_and_src_retained ... ok
test utils::write_guard::tests::lock_for_returns_same_arc_for_same_project ... ok
test utils::write_guard::tests::different_projects_parallel ... ok
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
...

test result: ok. 156 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.95s
```

**T1 模块测试统计**（直接归属本 task）：

| 模块 | 测试数 | 全部通过 |
|---|---|---|
| `utils::ipc_error::tests` | 4 | ✅ |
| `utils::nfc::tests` | 5 | ✅ |
| `utils::safe_rename::tests` | 6 | ✅ |
| `utils::write_guard::tests` | 3 | ✅ |
| `utils::safe_name::tests` | 6 | ✅（非 T1 AC 范围，附带通过） |
| `workspace::folder_utils_tests` | 14 | ✅ |
| `workspace::scope_tests` | 6 | ✅（既有 + 兼容验证） |
| 总览 | **156 passed; 0 failed** | ✅ |

警告：9 项 warning，全部为既有代码 unused import / mut / dead_code 类杂项（与 T1 改动无直接耦合，不阻塞）；其中 `commands/workspace_folders.rs` 3 处 `let mut conn` 不需要 `mut`，属于 T3 范围的代码质量问题。

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|----------|----------|------|-----------|
| ✅ 正常路径 | AC-1 依赖落地：`cargo build` 通过 | 已测 | PASS（由 `cargo test --lib` build 阶段通过隐含证明） |
| ✅ 正常路径 | AC-2 11 项 code 序列化字面量与 T0 §A.3 字符级一致 | 已测 | PASS — `all_eleven_codes_serialize_to_screaming_snake` |
| ✅ 正常路径 | AC-2 `From<IpcError> for String` 可被 `serde_json::from_str` 还原 | 已测 | PASS — `into_string_serializes_to_json` |
| ✅ 正常路径 | AC-2 `details=None` 时 skip 序列化 | 已测 | PASS — `details_omitted_when_none` |
| ✅ 正常路径 | AC-3 `resolve_relative_path("__ROOT__")` 归一空相对 | 已测 | PASS — `resolve_relative_path_root_sentinel`（断言不含 `__ROOT__` 字面量） |
| ✅ 正常路径 | AC-3 `validate_and_canonicalize` 对 `__ROOT__` 返 workspace 根 canonical | 已测 | PASS — `validate_and_canonicalize_root_sentinel_returns_workspace_root` |
| ⚠️ 边界条件 | AC-3 `validate_folder_name`：CJK / NFD 形式应通过 | 已测 | PASS — `validate_folder_name_accepts_chinese_and_nfd` |
| ⚠️ 边界条件 | AC-3 保留字大小写敏感：`"Organized"` 通过 | 已测 | PASS — `validate_folder_name_reserved_case_sensitive` |
| ⚠️ 边界条件 | AC-5 `is_exdev` 识别 errno=18 / 不误识别 ENOENT=2 | 已测 | PASS — `is_exdev_detects_errno_18` |
| ⚠️ 边界条件 | AC-5 EXDEV 注入：src **保留**直到 caller commit；dst 完整 | 已测 | PASS — `exdev_triggers_copy_first_and_src_retained` |
| ⚠️ 边界条件 | AC-6 同 project 多次 `lock_for` 返同一 `Arc` | 已测 | PASS — `lock_for_returns_same_arc_for_same_project` |
| ⚠️ 边界条件 | AC-6 不同 project 并发 → 并发上限=2 | 已测 | PASS — `different_projects_parallel`（带耗时断言） |
| ❌ 异常路径 | AC-3 `../../etc` → `E_PATH_ESCAPE` | 已测 | PASS — `validate_and_canonicalize_rejects_dotdot` |
| ❌ 异常路径 | AC-3 `/etc/passwd` 绝对路径 → `E_PATH_ESCAPE` | 已测 | PASS — `validate_and_canonicalize_rejects_absolute` |
| ❌ 异常路径 | AC-3 symlink 越界 → `E_PATH_ESCAPE` | 已测 | PASS — `validate_and_canonicalize_rejects_symlink_escape`（unix 限定） |
| ❌ 异常路径 | AC-3 `organized` → `E_NAME_RESERVED` | 已测 | PASS — `validate_folder_name_rejects_reserved` |
| ❌ 异常路径 | AC-3 5 类非法名（slash/backslash/colon/dot_prefix/empty/blank/too_long）→ `E_NAME_INVALID` | 已测 | PASS — 6 个 `validate_folder_name_rejects_*` |
| ❌ 异常路径 | AC-5 同 project 并发 → 并发上限=1 | 已测 | PASS — `same_project_serializes`（耗时 ≥ 2*80ms） |
| ❌ 异常路径 | AC-5 `cleanup_pending_scan` 清 marker + 孤立 tmp | 已测 | PASS — `cleanup_pending_scan_removes_marked_src` + `cleanup_pending_scan_removes_orphan_tmp` |
| ⚠️ 边界条件 | AC-4 NFD `"e\u{0301}"` 自愈为 NFC `"é"` | 已测 | PASS — `nfc_self_heal_renames_nfd_dir` |
| ⚠️ 边界条件 | AC-4 NFC 目标已存在 → 跳过 + warn（不 panic） | 已测 | PASS — `nfc_self_heal_skips_when_target_exists` |
| ⚠️ 边界条件 | AC-4 CJK idempotent | 已测 | PASS — `nfc_idempotent_for_cjk` |
| ✅ 正常路径 | AC-7 `setup` 内 `app.manage(WorkspaceWriteGuard::new())` 已挂 | 静态核验 | PASS — `lib.rs:66` |
| ✅ 正常路径 | AC-7 启动期挂 `nfc_heal_workspace` + `cleanup_pending_scan`，外包 `catch_unwind` 防 panic | 静态核验 | PASS — `startup.rs:107-123` |
| ✅ 正常路径 | AC-8 `cargo test --lib` 全绿 | 已测 | PASS — 156 passed; 0 failed |

## 已知局限

1. **AC-3 reason 闭集映射**：`validate_folder_name` 把 `/ \ :` 三种字符全部映射到 `reason="slash"`（T0 §A.4 #1 闭集只有 slash，未细分 backslash/colon）；这与 T0 一致，但前端无法区分具体字符。**符合契约，无需修复**。
2. **AC-3 `validate_folder_name` 走 NFC normalize 后再做 byte length 校验**：实现里 `.len() > 255` 是 NFC 后 UTF-8 字节长度，与 T0「>255 字节」表述一致。
3. **测试期 HOME sandbox**：`workspace::folder_utils_tests::with_sandboxed_home` 使用 `std::env::set_var("HOME", ...)`（unsafe；进程级状态）+ static `Mutex` 串行。这种做法在 Rust 2024 edition 中是 unsafe API；当前项目 edition=2021，编译通过。Reviewer 若有更优 sandbox 方案可在 review 反馈。
4. **集成测试 `test_round_trip_root_to_folder_to_root` 含 `eprintln!` DEBUG 段**（行 218-251）：是一段 inline 复现，用于前一棒调试 move_asset 流程。功能上不影响测试通过，但留下 DEBUG 噪音。**属 T4 范围，留给 T4 阶段清理**。
5. **`commands/workspace_folders.rs` 中 3 处 `let mut conn` 不需要 `mut` 警告**：T3 范围代码质量瑕疵；与 T1 无关。
6. **`safe_rename::IpcError::attach_internal_hint`**：在 `safe_rename.rs` 内为 `IpcError` 增加了一个 `pub(super)` 不可见的 `fn attach_internal_hint`（通过 `impl IpcError` 添加，可见性默认 module-private）。这是工具层为自用便利的扩展，未污染 `ipc_error.rs` 公共 API。若 Reviewer 偏好严格归位，可以挪进 `ipc_error.rs` 作为 `pub(crate) fn`。当前不阻塞。

## 需要 Reviewer 特别关注的地方

1. **【高】T0 §A.3 `rename_all` vs 逐项 `rename` 二选一处理结果**：
   - **背景**：T0 §A.3 同时给出 `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` 注释（标"形式说明"）与 11 个逐项 `#[serde(rename = "...")]`，备注「为避免 Rust 标识符限制，逐项 rename」。
   - **前一棒选择**：保留 `rename_all = "SCREAMING_SNAKE_CASE"`，删除逐项 `rename`。理由：PascalCase variant 名 `ENameInvalid` 经 SCREAMING_SNAKE_CASE 推导 = `E_NAME_INVALID`（serde 默认推导规则），与 T0 §A.3 表格 11 项字面量字符级一致。
   - **核验证据**：`utils::ipc_error::tests::all_eleven_codes_serialize_to_screaming_snake` 对 11 项做 `serde_json::to_string` + `assert_eq!` 字符级断言全部通过。
   - **Reviewer 决策点**：是否接受这种"二选一"简化？若严格按 T0 文字"逐项 rename"，需要把 11 个 `#[serde(rename = "...")]` 加回去（功能等价、字面量一致）。从契约保护角度，前一棒做法风险点是「未来若改了 variant 名（如改 `ENameInvalid` → `EInvalidName`），SCREAMING_SNAKE_CASE 会自动产出新字面量而契约破坏不会被发现」。**建议**：保留 `rename_all` 简洁版 + 加一条 doc-comment 警告"修改 variant 名必须同步检查序列化字面量"（前一棒已在 enum doc-comment 第 14-16 行写到）；或者按 Reviewer 偏好加回逐项 rename 做双保险。
2. **【高】越权范围**：T1 实际交付包含了 T3 的 4 写命令实现 + T4 的 2 个集成测试 + invoke_handler 注册。**Reviewer 在评 T1 时应只评 T1 AC**；其余等 T3 / T4 单独评审时再回看。本 task 不要求回退。
3. **【中】启动 hook 的 `catch_unwind`**：`startup::workspace_startup_hooks` 用 `catch_unwind(AssertUnwindSafe(...))` 包了 `nfc_heal_workspace` 与 `cleanup_pending_scan` 调用。这与 input.md「失败仅 log，不阻塞启动」对齐，但 `AssertUnwindSafe` 是 unwind-safety 兜底，并非强保证。模块内部已确保只 `log::warn` 不抛错，理论上不需要 `catch_unwind`；保留属防御编程。可接受。
4. **【中】`canonicalize_longest_prefix` 实现**：对最长存在前缀做 canonicalize 再拼剩余段（用于 create 时叶子不存在的场景）。已能 catch 中间段 symlink 越界（unix 单测覆盖）。**注意**：若叶子之前的最长存在前缀本身就是 workspace_root，则剩余段就是叶子名；这种场景的拼接结果未必能 canonicalize（叶子不存在），但 `starts_with(workspace_root_canonical)` 仍能正确判越界。已被 `validate_and_canonicalize_rejects_dotdot` / `_rejects_absolute` 覆盖。
5. **【中】`validate_folder_name` reason 闭集映射**：T0 §A.4 #1 `reason` 枚举为 `"slash"|"dot_prefix"|"whitespace"|"too_long"|"empty"`。实现里 `/ \ :` 全映射到 `"slash"`，`""` → `"empty"`，全空白 → `"whitespace"`，>255 → `"too_long"`，`.开头` → `"dot_prefix"`，**未出现 T0 闭集外的 reason 值**。OK。
6. **【低】`utils::safe_name` 模块**：该模块由前一棒 Dev 顺手加入但不在 T1 AC，本 task 不认领；Reviewer 评 T1 时可跳过这 6 个测试。
7. **【低】lib.rs 注释笔误**：`app.manage(WorkspaceWriteGuard::new())` 注释标 `task_005 T3`，实际是 T1 AC-7。可在 T3 review 时顺手修注释。
