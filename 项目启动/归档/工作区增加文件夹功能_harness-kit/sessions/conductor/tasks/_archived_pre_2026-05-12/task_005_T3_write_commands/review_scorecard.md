# Review Scorecard — task_005_T3_write_commands

## 审查思考过程

1. **Task 意图**：实现 4 写命令（create / rename / delete / move_asset_to_workspace_folder），全部走 `WorkspaceWriteGuard` → `kind` 推断 → `validate_and_canonicalize` → 同事务 fs+DB；补齐 PRD §6.1 Rust 单测 8 条。
2. **AC 检查结果**：
   - AC-1 路径越界 ✅（`path_escape` 覆盖 `..` / 绝对 / symlink 三例）
   - AC-2 保留字 ✅（`reserved_name` 含 create+rename 两侧）
   - AC-3 ai_organized 四类写 ✅（`ai_organized_protected` 含 rename/delete/move；create 子目录由 reserved_name 兜底）
   - AC-4 SQL 前缀边界 ✅（`prefix_boundary` 100 vs 100%off 通过；ESCAPE 转义、强制尾 `/` 已落地）
   - AC-5 NFC ✅（`nfc_dup` 通过）
   - AC-6 trash + 复检 ✅（`trash_recheck` 通过；Win/Linux `#[cfg]` 分支返 `E_PLATFORM_UNSUPPORTED`；**全文件 grep 无 `fs::remove_dir_all`**）
   - AC-7 写通道锁 ✅（`write_guard_serializes` 通过；`import_drop_paths` 已加同锁）
   - AC-8 handler 入口判定 ✅（4 命令首行均 `lock_for` + kind 推断；direct invoke 测试覆盖）
   - AC-9 IpcError 序列化 ✅（`direct_invoke_rejected_and_error_json_roundtrip` 反序列化验证 code/details）
   - AC-10 lib.rs 注册 ✅（4 命令 + `manage(WorkspaceWriteGuard::new())` 见 lib.rs:66 / 149-152；cargo build EXIT=0）
   - AC-11 既有 caller 调整 ✅（旧 move 标 `#[deprecated]` + 从 invoke_handler 移除；T2 已切前端；`pnpm tsc --noEmit` EXIT=0）
3. **关键发现**：
   - **核心写路径全部合规**：4 命令首行三件套（WriteGuard → kind → validate_and_canonicalize）字符级到位；EXDEV 走 `safe_rename` copy-first + COMMIT 后 `remove_src_after_commit`，顺序与 ADR-002 一致。
   - **DB 绝对路径偏离**（contracts §C.2）：现网 schema/code 均存绝对路径（dropzone 写入即如此），与 contracts §C.2 字面描述"正斜杠相对路径"不符。但 `rename_path_prefix` 在绝对路径前缀上的 SQL 语义（ESCAPE `\ % _`、强制尾 `/`、自身行 + LIKE 子树双支）完全正确，`prefix_boundary` 测试已通过。**判定：契约文档错误，非实现偏离，列 MINOR**——建议 contracts §C.2 改为"workspace 内的绝对路径"或在后续 task 安排相对化 migration。
   - **3 个 `__ROOT__` debug_assert** 已落 db/asset.rs `insert` / `update_name_and_path` / `update_project_and_path`（17/179/213 行）+ `rename_path_prefix` new_prefix（389 行），覆盖完整。
   - **import_drop_paths 锁覆盖范围**：pid 解析（仅读）后立即取 WriteGuard 持有至函数 return，覆盖整段 fs/DB 写入；pid 解析阶段无写，race 窗口无害。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | 4 命令字符级满足 PRD §5.1 / §6.1；EXDEV / trash / 事务 / 锁顺序全部正确；8/8 + 156/156 全套 PASS |
| 安全性 | 25% | 5 | 路径越界、ai_organized 双拦、保留字、NFC dup、__ROOT__ 防泄漏、SQL ESCAPE、写通道串行——10 条不可妥协底线全部命中；direct-invoke 绕 UI 测试覆盖 |
| 代码质量 | 15% | 4 | `*_impl` + thin `#[tauri::command]` 包装分层清晰；TrashAdapter 注入设计良好；3 处 `unused_mut` 与 `count_assets_under_prefix` / `delete_assets_under_prefix` 与 `rename_path_prefix` 三处 ESCAPE 逻辑可抽公共 helper |
| 测试覆盖 | 20% | 5 | 8 条 AC 单测全部 PASS；HOME 沙箱串行化得当；trash adapter / canonicalize symlink / NFD 磁盘命名 等关键 setup 真实可信 |
| 架构一致性 | 10% | 4 | 4 命令落 `commands/workspace_folders.rs` 与 PRD §5.1 一致；DB file_path 字面与 contracts §C.2 不一致（属契约文档错误，见 MINOR-1） |
| 可维护性 | 5% | 4 | 注释充分（中文/ADR 引用齐全）；TrashAdapter 全局 override 用 `static Mutex<Option<Box<dyn ...>>>` 仅在 `#[cfg(test)]` 设 setter，生产无注入入口；偏离声明清晰 |

**综合分：4.80/5**（加权：5×0.25 + 5×0.25 + 4×0.15 + 5×0.20 + 4×0.10 + 4×0.05 = 4.80）

## 总体判断

- [x] **PASS**

## 问题列表

### BLOCKER
（无）

### MAJOR
（无）

### MINOR（可选）

1. **contracts §C.2 字面契约与 prod 实现不一致**：DB `assets.file_path` 实际存绝对路径（dropzone 写入路径即为绝对），契约文档写"正斜杠相对路径"。
   - **代码位置**：`harness-kit/sessions/conductor/tasks/task_002_T0_contracts/contracts.md:177-179` vs `NCdesktop/src-tauri/src/db/asset.rs:17/179/213`
   - **建议**：契约文档修订为"workspace 内绝对路径，不含 `__ROOT__` sentinel"，或单开 task 评估相对化 migration（涉及全量 UPDATE，需评估对 dropzone、缩略图缓存等 caller 的影响）；本 task 实现侧无需改动。

2. **`count_assets_under_prefix` / `delete_assets_under_prefix` 与 `rename_path_prefix` 三处 ESCAPE+尾斜杠逻辑重复**：均做相同的 `\ % _` 预转义与强制尾 `/` 处理。
   - **代码位置**：`commands/workspace_folders.rs:258-285 / 288-313`、`db/asset.rs:371-373/384-415`
   - **建议**：抽 `pub(crate) fn build_prefix_like(prefix: &str) -> (String /*old_no_slash*/, String /*like_pat*/)` 落 `db::asset`，三处复用，降低边界 case 漂移风险。

3. **3 处 `let mut conn` 引发的 `unused_mut` 警告**：rusqlite `unchecked_transaction()` 可借 `&Connection`，无需 `mut`。
   - **代码位置**：`workspace_folders.rs::rename_workspace_folder_impl` / `delete_workspace_folder_impl` / `move_asset_to_workspace_folder_impl`
   - **建议**：去掉 `mut`。

4. **rename EXDEV 路径在 COMMIT 失败时存在残留 dst**：Dev 已自述（output §需要 Reviewer 特别关注 1）。SQLite 本地 commit 失败极罕见，可接受；建议在 `cleanup_pending_scan` 启动期补一条"dst 存在但 DB 无对应行 → 反向回滚"扫描，留给后续 task。
   - **代码位置**：`workspace_folders.rs:428-434`

5. **`delete_workspace_folder` 中 trash 失败前已执行 `delete_assets_under_prefix`**：trash 失败时事务未 commit → 自动回滚，DB 一致；但若未来该路径插入任何"已成功且不可回滚"的操作，需调整顺序。当前实现正确，留作注释建议（"DELETE 必须在 trash 之前以保证回滚安全"显式注明）。
   - **代码位置**：`workspace_folders.rs:511-521`

## 给 Dev 的修复指引

无（PASS，全部 MINOR 可推迟）。
