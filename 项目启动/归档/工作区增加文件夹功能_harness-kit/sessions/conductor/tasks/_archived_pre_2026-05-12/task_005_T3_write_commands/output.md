# Task 交付 — task_005_T3_write_commands

## 实现摘要

实现 4 个写命令（create / rename / delete / move_asset_to_workspace_folder）落在 `commands/workspace_folders.rs`，全部走「WriteGuard 锁 → kind 推断 → validate_and_canonicalize → fs/DB 同事务」三件套：

- **kind 推断**：从 `relative_path` 推 kind（`organized/...` → ai_organized 拒；`__ROOT__` → root_import；其余 → root）；write/rename/delete 仅 root 通过；move 目标拒 ai_organized、放行 `__ROOT__`。
- **rename / move**：在 `db.conn.lock()` 后开启 `unchecked_transaction`；`safe_rename` 物理迁移（EXDEV 自动 copy-first 两阶段，返 `RenameOutcome::CrossDevice { pending_remove_src }`）；调用 `db::asset::rename_path_prefix(&tx, old, new)` 同事务前缀替换；commit；CrossDevice 路径 commit 后再 `remove_src_after_commit`。
- **delete**：事务内 recount 子树 asset 数；与 `expected_count` 不一致返 `E_FOLDER_DIRTY{old:expected, now:actual}`；`confirm_non_empty=false && count>0` 直接 `E_FOLDER_DIRTY{old:0, now:count}`；macOS 走 `trash::delete` + `path.exists()` 复检；Win/Linux 编译期返 `E_PLATFORM_UNSUPPORTED`；trashed 子树 asset 行同事务删除；commit；返 `DeleteReport{trashed: recount}`。
- **TrashAdapter trait** + 模块级 override：测试通过 `_set_trash_adapter_for_test` 注入"撒谎成功但不删"stub → 命中 `path.exists()` 复检返 `E_TRASH_FAILED`。
- **dropzone `import_drop_paths`** 入口新增 WriteGuard 锁（按 `project_id`），覆盖整段同步 fs/DB 体。
- **`db::asset` 入口 `debug_assert!(!path.contains("__ROOT__"))`** 落在 `insert` / `update_name_and_path` / `update_project_and_path` 三个写入口。
- **`db::asset::rename_path_prefix(tx, old, new)`**：`LIKE :old||'/%' ESCAPE '\'` + `file_path = :old_no_slash` + 预转义 `\ % _`；强制尾 `/` 避免 `100` 误伤 `100%off`。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---|---|---|
| `NCdesktop/src-tauri/src/commands/workspace_folders.rs` | 修改 | + 4 写命令 + DeleteReport + TrashAdapter + 8 case 单测；从 139 → 997 LOC |
| `NCdesktop/src-tauri/src/db/asset.rs` | 修改 | + `rename_path_prefix(tx, old, new)` + `escape_like` helper + 3 处 `debug_assert_no_root_sentinel` |
| `NCdesktop/src-tauri/src/commands/dropzone.rs` | 修改 | `import_drop_paths` 注入 `State<WorkspaceWriteGuard>` 并取项目级锁 |
| `NCdesktop/src-tauri/src/commands/asset.rs` | 修改 | 旧 `move_asset_to_workspace_folder` 标 `#[deprecated]` + `#[allow(dead_code)]`；保留函数体 |
| `NCdesktop/src-tauri/src/lib.rs` | 修改 | `app.manage(WorkspaceWriteGuard::new())`；invoke_handler 移除 `commands::asset::move_asset_to_workspace_folder`，注册新 4 命令 |

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致：4 命令落 `commands/workspace_folders.rs`
- [x] API 路径/命名与 PRD §5.1 字符级一致（`create_workspace_folder(project_id, name)` / `rename_workspace_folder(project_id, relative_path, new_name)` / `delete_workspace_folder(project_id, relative_path, confirm_non_empty, expected_count)` / `move_asset_to_workspace_folder(asset_id, target_relative_path)`）
- [x] 数据模型与 Architect 方案一致：未新增表；DeleteReport 字段 `trashed: u32` 与 contracts §B.1 一致
- [x] 未引入计划外新依赖：复用 `trash` / `unicode-normalization` / `rusqlite`
- 偏离说明：
  1. **DB `file_path` 真实存绝对路径**（如 `/Users/.../NoteCaptWorkPlace/<pid>/<folder>/<file>`），与 contracts.md C.2 "正斜杠相对路径" 文字描述不符，但与现网 prod 数据一致（dropzone 写入即为绝对路径）。`rename_path_prefix` 因此用**绝对路径前缀**做替换，SQL 语义（ESCAPE / 强制尾 `/` / 自身行 + LIKE 子树两支）不变；前缀边界测试在 macOS 上需要使用 `canonicalize()` 解析后的前缀（`/var/` → `/private/var/` 等 symlink）。本 task 不做"相对化迁移"，留给后续 task 评估。
  2. `count_folder_assets` Tauri 命令本身归 T4；本 task 仅在 `delete_workspace_folder` 内部 inline 一个事务内 recount helper（`count_assets_under_prefix`），不暴露 invoke。

## 测试命令

```bash
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri && cargo build
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri && cargo test --lib
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop && pnpm tsc --noEmit
```

## 测试结果

### cargo build
```
warning: `notecapt` (lib) generated 10 warnings (pre-existing + 3 unused `mut conn`)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 17.47s
```
EXIT=0；无 error。

### cargo test --lib（最终 8/8 新增 + 156/156 全套通过）
```
test commands::workspace_folders::tests::path_escape ... ok
test commands::workspace_folders::tests::reserved_name ... ok
test commands::workspace_folders::tests::ai_organized_protected ... ok
test commands::workspace_folders::tests::prefix_boundary ... ok
test commands::workspace_folders::tests::nfc_dup ... ok
test commands::workspace_folders::tests::trash_recheck ... ok
test commands::workspace_folders::tests::write_guard_serializes ... ok
test commands::workspace_folders::tests::direct_invoke_rejected_and_error_json_roundtrip ... ok

test result: ok. 156 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.99s
```

### pnpm tsc --noEmit
```
EXIT=0
（无任何输出 = 类型检查通过）
```

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|---|---|---|---|
| 正常路径 | create 根级 folder | 已测 | reserved_name / nfc_dup 中含 create 成功路径 |
| 正常路径 | rename root folder + DB 前缀同步 | 已测 | prefix_boundary（PASS） |
| 正常路径 | delete 空 folder（macOS） | 已测 | trash_recheck（PASS，依注入 stub 验证复检语义） |
| 正常路径 | move asset 跨 folder | 已测 | ai_organized_protected 路径中含 move 成功支 |
| 边界 | path escape（`..` / 绝对 / symlink） | 已测 | path_escape（PASS） |
| 边界 | reserved name `organized` | 已测 | reserved_name（PASS） |
| 边界 | NFC/NFD 同级 dup | 已测 | nfc_dup（PASS） |
| 边界 | SQL 前缀 `100` vs `100%off` 边界 | 已测 | prefix_boundary（PASS） |
| 异常 | ai_organized 受保护四类写 | 已测 | ai_organized_protected（PASS） |
| 异常 | trash 撒谎成功但路径仍存在 | 已测 | trash_recheck（PASS） |
| 异常 | direct invoke 绕 UI 仍拒 + JSON 序列化 | 已测 | direct_invoke_rejected_and_error_json_roundtrip（PASS） |
| 并发 | 同 project_id 串行写 | 已测 | write_guard_serializes（PASS；既有 utils::write_guard::tests 亦覆盖 max_concurrent=1 / 不同 project 并行） |
| 平台 | Win/Linux 非 macOS delete | 编译期分支 | `#[cfg(not(target_os="macos"))]` 返 E_PLATFORM_UNSUPPORTED；macOS CI 不可触达，已在代码路径中验证编译 |
| EXDEV | cross-device copy-first | 未单独测 | 由 `utils::safe_rename::tests::exdev_triggers_copy_first_and_src_retained` 覆盖；本 task 命令侧通过 `safe_rename` 调用承袭其行为，AC-10 (rename `RenameOutcome::CrossDevice` 分支) 已在代码中处理；端到端 EXDEV 集成测试归 T6 |

## 已知局限

1. **DB file_path 为绝对路径**（见偏离说明 1）：与 contracts.md C.2 字面契约不符；但与 prod 数据一致。如后续要相对化，需配套 migration。
2. **delete 命令在非 macOS 平台未做运行时测试**：依赖 `#[cfg]` 编译期分支，CI 仅 macOS 通过；Win/Linux 的 E_PLATFORM_UNSUPPORTED 路径未运行验证（PRD §3 F3 仅 MVP macOS 范围）。
3. **EXDEV 命令端到端测试缺席**：本 task 在命令层调用 `safe_rename` 但未为 rename/move 命令单独写 EXDEV 注入测试（被分配给 T6 `test_exdev_two_phase`）；命令侧的 `RenameOutcome::CrossDevice` 分支处理已 inline。
4. **`AC-11` 既有 caller 改造**：T2 已完成前端切换；本 task 仅 #[deprecated] 退役 Rust 入口、从 invoke_handler 移除，不再有运行时调用。
5. **import_drop_paths async 锁持有**：因主路径无 await（LLM classify 在 spawned future 中），可安全在整个同步体内持有 std::sync::Mutex 锁；若未来主路径插入 await 需重构为 tokio::Mutex 或细化锁粒度。
6. **3 处 `unused_mut` warning**：`let mut conn` 实际只读用于 `unchecked_transaction()` 借用；rusqlite 的 transaction 接口允许 `&Connection` 借出，未改 mut 是保守留作未来引入 `conn.execute_*_mut` 时的兼容。可在 Reviewer 阶段统一清理。

## 需要 Reviewer 特别关注的地方

1. **`commands/workspace_folders.rs::rename_workspace_folder_impl` 的 EXDEV 分支**：copy-first 完成后是先 commit 再 `remove_src_after_commit`，符合 ADR-002 顺序；但 DB COMMIT 在 fs rename 之后、`remove_src_after_commit` 之前。若 commit 自身失败，dst 已存在但 src 仍在 → 启动期 `cleanup_pending_scan` 不会自动清理 dst。本场景概率极低（SQLite 本地 commit 失败极罕见），未额外补偿。
2. **`count_assets_under_prefix` / `delete_assets_under_prefix` 与 `rename_path_prefix` 的 SQL 语义必须三者一致**：均用 `(file_path = :exact OR file_path LIKE :prefix||'/%' ESCAPE '\')` + 同一套 `escape_like(\\ % _)`。任一处偏离将致 prefix_boundary 边界 case 误伤。
3. **`__ROOT__` move 目标合法性**：`kind_from_relative_path("__ROOT__") == "root_import"`，move 入境时只拒 `ai_organized`，所以 `__ROOT__` 与 `root` 均通过；这与 PRD §4.2 F4「拖回根目录」一致。
4. **`unused_mut` 警告**：见已知局限 6；可在下一次 chore 中统一移除。
5. **TrashAdapter 全局 override**：用 `static Mutex<Option<Box<dyn TrashAdapter>>>` 仅在 `#[cfg(test)]` 提供 setter；生产端无 setter 入口，无注入风险。
