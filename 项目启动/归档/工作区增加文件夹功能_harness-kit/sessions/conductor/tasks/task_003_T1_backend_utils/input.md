# Task 输入 — task_003_T1_backend_utils

## 目标
搭建本期后端工具层：引入 `trash` + `unicode-normalization` 依赖；实现 `IpcError` enum、`validate_and_canonicalize` / `resolve_relative_path` / `validate_folder_name`、`nfc_normalize` + 启动期 NFC 自愈、EXDEV-safe `safe_rename` + `cleanup_pending_scan`、`WorkspaceWriteGuard`；并把启动 hook 与 `app.manage` 接入到 `lib.rs` / `startup.rs`。

## 前置条件
- 依赖 task：task_002_T0_contracts（IpcError shape、错误码闭集、`__ROOT__` 编解码契约）
- 必须先存在的文件/接口：
  - `sessions/conductor/tasks/task_002_T0_contracts/contracts.md`
  - `NCdesktop/src-tauri/src/workspace.rs`（既有 `project_workspace_dir` / `workspace_root`）
  - `NCdesktop/src-tauri/src/utils/mod.rs`（既有 mod 声明）
  - `NCdesktop/src-tauri/src/startup.rs`（既有 bootstrap 入口）

## 验收标准（Acceptance Criteria）

1. **AC-1 依赖落地**：`NCdesktop/src-tauri/Cargo.toml` 新增 `trash = "5"`（target-cfg macOS-only 或全平台均可，但 Win/Linux 调用点必须用 `cfg!(target_os = "macos")` 守护）+ `unicode-normalization = "0.1"`。`cargo build` 全平台通过。
2. **AC-2 IpcError 模块**：`src-tauri/src/utils/ipc_error.rs` 实现 `IpcErrorCode`（11 项 `#[serde(rename = "...")]` 闭集）+ `IpcError { code, message, details }` + `impl From<IpcError> for String`（`serde_json::to_string` + 兜底静态 JSON）。提供 11 个工厂便捷构造（`name_invalid` / `name_dup` / ...）。单测覆盖：(a) 11 项 code 序列化字面量与 contracts.md §A.2 TS 字面量字符级一致；(b) `From<IpcError> for String` 输出可被 `serde_json::from_str` 还原；(c) `details: None` 时 `skip_serializing`。
3. **AC-3 workspace 工具扩展**：在 `src-tauri/src/workspace.rs` 追加：
   - `resolve_relative_path(project_id, rel) -> PathBuf`：`"__ROOT__"` → 空相对，拼接 workspace root；其余原样拼接。
   - `validate_and_canonicalize(project_id, rel) -> Result<PathBuf, IpcError>`：字符串层拒 `..` / 绝对 / `\` 开头 / 非法 Component；对**最长存在前缀**做 `canonicalize` 再拼剩余段（叶子可能未创建，便于 create_workspace_folder 复用）；`starts_with(workspace_root_canonical)` 必须为真；symlink 越界 → `E_PATH_ESCAPE`。
   - `validate_folder_name(name) -> Result<(), IpcError>`：禁 `/ \ :`、禁 `.` 开头、禁空白、禁 > 255 字节、禁 `organized`（NFC 后比较，大小写敏感）。同级 NFC 同名留给 T3 fs read 后判定。
   - 单测覆盖：`../../etc` / `/etc/passwd` / symlink → `E_PATH_ESCAPE`；保留字 `organized` → `E_NAME_RESERVED`；5 类非法名各 1 例 → `E_NAME_INVALID`。
4. **AC-4 NFC 模块**：`src-tauri/src/utils/nfc.rs` 实现 `nfc_normalize(s)` / `nfc_eq(a, b)` / `nfc_self_heal(project_root)`（递归）/ `nfc_heal_workspace()`（枚举 `~/Downloads/NoteCaptWorkPlace/*/`）。`nfc_heal_workspace` 失败仅 `log::warn`，绝不 panic / 抛错；NFC 目标已存在时跳过 + warn。单测：NFD `"e\u{0301}"` 自愈为 NFC `"é"`；NFC 目标已存在时跳过；CJK idempotent。
5. **AC-5 safe_rename 模块**：`src-tauri/src/utils/safe_rename.rs` 实现 `safe_rename(src, dst) -> Result<RenameOutcome, IpcError>`：先试 `fs::rename`，捕 `raw_os_error() == Some(18)`（EXDEV）后走两阶段：`copy_dir_all → fsync 递归 → rename(tmp → dst)`；返 `RenameOutcome::CrossDevice { pending_remove_src }`。`remove_src_after_commit(src)` 失败留 `.cleanup_pending` 标记 + log。`cleanup_pending_scan(root)` 启动期清理 `.cleanup_pending` 标记 + 孤立 `.cross_device.tmp`。提供 `#[cfg(test)] mod test_inject` 注入 EXDEV 模拟。单测：(a) 同卷 rename 成功返 `SameVolume`；(b) EXDEV 注入触发 copy-first，src 保留直到 caller commit；(c) `remove_src_after_commit` 清 src；(d) `cleanup_pending_scan` 清标记 + 孤立 tmp。
6. **AC-6 写通道锁**：`src-tauri/src/utils/write_guard.rs` 实现 `WorkspaceWriteGuard { locks: Mutex<HashMap<String, Arc<Mutex<()>>>> }` + `lock_for(project_id) -> Arc<Mutex<()>>`。单测：(a) 同 project 串行（并发上限 = 1）；(b) 不同 project 并行（并发上限 = 2）；(c) 同 project 多次 `lock_for` 返同一 `Arc`。
7. **AC-7 启动 hook 与 manage**：`src-tauri/src/lib.rs` 在 `setup` 中 `app.manage(WorkspaceWriteGuard::new())`；`src-tauri/src/startup.rs` 在 bootstrap 末尾、`PipelineScheduler::recover` 之前调 `nfc::nfc_heal_workspace()` 与 `safe_rename::cleanup_pending_scan(workspace_root)`。失败仅 log，不阻塞启动。
8. **AC-8 `cargo test --manifest-path NCdesktop/src-tauri/Cargo.toml` 全绿**（含所有新增单测）。

## 技术约束
- session_context §5：后端 snake_case；所有新命令注册到 `invoke_handler!`（本 task 仅工具层，不注册命令；命令注册留 T3）。
- ADR-001：`From<IpcError> for String` 必须保证 serde 失败时也能返回合法 JSON 兜底。
- ADR-002：copy-first 两阶段顺序 = copy → fsync → rename → COMMIT → remove src；fsync 失败必须不删 src。
- ADR-003：read & 缩略图不取写锁；本 task 不修改既有 read 命令。
- ADR-004：`resolve_relative_path` 是 sentinel 单点；所有 fs 拼接必须经它。
- ADR-005：NFC 自愈非阻塞，目标已存在时跳过。
- 不顺手改无关代码；不在本 task 注册新 Tauri 命令（留 T3）。

## 参考文件
- `sessions/conductor/tasks/task_002_T0_contracts/contracts.md` §A / §C
- `sessions/conductor/tasks/task_001_architect/output.md` ADR-001 / 002 / 003 / 004 / 005 / 012
- 既有代码：
  - `NCdesktop/src-tauri/src/workspace.rs`（在末尾追加）
  - `NCdesktop/src-tauri/src/utils/mod.rs`
  - `NCdesktop/src-tauri/src/startup.rs`
  - `NCdesktop/src-tauri/src/lib.rs`（`app.manage` 挂点）
  - `NCdesktop/src-tauri/Cargo.toml`

## 预估影响范围
- 新建文件：
  - `NCdesktop/src-tauri/src/utils/ipc_error.rs`
  - `NCdesktop/src-tauri/src/utils/nfc.rs`
  - `NCdesktop/src-tauri/src/utils/safe_rename.rs`
  - `NCdesktop/src-tauri/src/utils/write_guard.rs`
- 修改文件：
  - `NCdesktop/src-tauri/Cargo.toml`（+ trash / unicode-normalization）
  - `NCdesktop/src-tauri/src/utils/mod.rs`（+ 4 mod 声明）
  - `NCdesktop/src-tauri/src/workspace.rs`（+ resolve_relative_path / validate_and_canonicalize / validate_folder_name）
  - `NCdesktop/src-tauri/src/startup.rs`（挂 nfc_heal_workspace + cleanup_pending_scan）
  - `NCdesktop/src-tauri/src/lib.rs`（`app.manage(WorkspaceWriteGuard::new())`；本 task 不注册新命令）
