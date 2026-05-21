# Task 输入 — task_003_T1_backend_utils

## 目标
实现后端工具层：依赖加引、`IpcError` enum、`validate_and_canonicalize` / `validate_folder_name` / `resolve_relative_path` / `nfc_normalize` / `safe_rename` / `WorkspaceWriteGuard`，启动期挂 `nfc_heal_workspace` + `cleanup_pending_scan`。

## 前置条件
- 依赖 task：task_002_T0_contracts（必须先 DONE，`contracts.md` 已固化错误码闭集与签名）
- 必须先存在的文件/接口：
  - `NCdesktop/src-tauri/src/workspace.rs` 既有 `project_workspace_dir` / `ProjectFolderRoot` / `assert_scope`
  - `NCdesktop/src-tauri/src/startup.rs` 既有 `bootstrap(db_path)`
  - `NCdesktop/src-tauri/Cargo.toml`

## 验收标准（Acceptance Criteria）
1. **AC-1**：`cargo test --manifest-path NCdesktop/src-tauri/Cargo.toml utils::ipc_error::` PASS（覆盖：`IpcError` serde 序列化为合法 JSON 字符串；11 个 code variant 全覆盖；`Into<String>` impl 把 IpcError 转为 invoke 边界 string）。
2. **AC-2**：`cargo test --manifest-path NCdesktop/src-tauri/Cargo.toml utils::nfc::` PASS（覆盖：`nfc_normalize` 把 NFD `"参考"` 归一为 NFC `"参考"`；`nfc_heal_workspace` mock 一个 NFD 目录后 rename 成 NFC）。
3. **AC-3**：`cargo test --manifest-path NCdesktop/src-tauri/Cargo.toml utils::safe_rename::` PASS（覆盖：同卷 rename 成功；mock EXDEV（用 `simulate_exdev` cfg）走 copy-first 顺序：copy → fsync → rename → 仅 remove src 在 commit 后才发生；copy 中断不影响 src）。
4. **AC-4**：`cargo test --manifest-path NCdesktop/src-tauri/Cargo.toml utils::write_guard::` PASS（覆盖：同 project_id 串行；不同 project_id 并行；read 路径不取锁）。
5. **AC-5**：`workspace::validate_and_canonicalize(project_id, rel)` 单测：(a) `../../etc` 返 `E_PATH_ESCAPE`；(b) `/etc/passwd` 返 `E_PATH_ESCAPE`；(c) symlink 指向 `/tmp` 返 `E_PATH_ESCAPE`；(d) `"__ROOT__"` 返 workspace 根 canonical 路径。
6. **AC-6**：`workspace::validate_folder_name(name)` 单测：(a) `"a/b"` → `E_NAME_INVALID`；(b) `".hidden"` → `E_NAME_INVALID`；(c) `""` → `E_NAME_INVALID`；(d) `"organized"` → `E_NAME_RESERVED`；(e) `"a".repeat(300)` → `E_NAME_INVALID`。
7. **AC-7**：`Cargo.toml` 含 `trash = "5"` 与 `unicode-normalization = "0.1"`，`cargo build --manifest-path NCdesktop/src-tauri/Cargo.toml` 成功。
8. **AC-8**：`startup::bootstrap` 末尾调用 `nfc_heal_workspace()`，失败仅 log 不 panic；新增 `cleanup_pending_scan()` hook 在 nfc 之后调用，能识别 `*.tmp` 残留（写一个单测构造 tmp dir 验证清理）。

## 技术约束
- 依赖加引 **仅 2 项**：`trash`、`unicode-normalization`；不顺手加其他（PRD §5）。
- `IpcError` 必须 `#[derive(Serialize)]`；Tauri 边界用 `impl From<IpcError> for String { fn from(e) -> String { serde_json::to_string(&e).unwrap_or(...) } }`，前端 JSON.parse 还原（ADR-001）。
- `safe_rename`：先 `fs::rename`，捕跨设备错误（`io::ErrorKind::CrossesDevices` 或 raw errno `EXDEV=18` macOS）后走两阶段（ADR-002）；copy 阶段失败返 `E_CROSS_DEVICE`；rename(tmp→final) 失败前不动 src。
- `WorkspaceWriteGuard`：`Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>`；提供 `lock_for(&project_id) -> MutexGuard<'_, ()>`（ADR-003）。
- `nfc_heal_workspace`：扫描 `~/Downloads/NoteCaptWorkPlace/*/`；若 readdir 字节 B 的 NFC 形式 N ≠ B，且 `parent/N` 不存在，则 `fs::rename(parent/B, parent/N)`；命中 N 已存在则仅 `log::warn!` 不覆写（ADR-005）。
- 启动期所有扫描均**串行 + 非阻塞**：失败必须只 log，不能让 bootstrap 抛错。
- 路径校验**必须 `canonicalize()` 后 `starts_with(workspace_root_canonical)`**（PRD §4.1.1）。
- `__ROOT__` sentinel 仅在 `resolve_relative_path()` 内消费一次（ADR-004）。
- 命名校验：禁含 `/ \ :`、禁 `.` 开头、禁同级同名、禁保留字 `organized`、禁空、长度 ≤ 255 字节（ADR-008 / 底线 9）。"禁同级同名"是 DB-aware 校验，本 task 仅实现纯字符串校验部分；同名检测在 T3 命令内调用 fs read。
- Commit 用中文 Conventional Commits（如 `feat(workspace): 新增后端工具层`）。
- 不顺手改无关代码。

## 参考文件
- 现有：`NCdesktop/src-tauri/src/workspace.rs`、`NCdesktop/src-tauri/src/startup.rs`、`NCdesktop/src-tauri/src/utils/`（如已有 mod.rs；否则新建）、`NCdesktop/src-tauri/Cargo.toml`
- 契约：`sessions/conductor/tasks/task_002_T0_contracts/contracts.md`
- 方案：output.md ADR-001（IpcError）、ADR-002（EXDEV）、ADR-003（write guard）、ADR-004（`__ROOT__`）、ADR-005（NFC）、ADR-008（命名校验）

## 预估影响范围
- 新建文件：
  - `NCdesktop/src-tauri/src/utils/ipc_error.rs`
  - `NCdesktop/src-tauri/src/utils/nfc.rs`
  - `NCdesktop/src-tauri/src/utils/safe_rename.rs`
  - `NCdesktop/src-tauri/src/utils/write_guard.rs`
- 修改文件：
  - `NCdesktop/src-tauri/Cargo.toml`（+2 deps）
  - `NCdesktop/src-tauri/src/utils/mod.rs`（注册新 mod）
  - `NCdesktop/src-tauri/src/workspace.rs`（追加 `resolve_relative_path` / `validate_and_canonicalize` / `validate_folder_name`）
  - `NCdesktop/src-tauri/src/startup.rs`（挂 `nfc_heal_workspace` + `cleanup_pending_scan`）
