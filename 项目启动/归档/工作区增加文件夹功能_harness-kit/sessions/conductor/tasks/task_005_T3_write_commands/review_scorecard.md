# Review Scorecard — task_005_T3_write_commands

> **特例说明**：代码由 T1 Dev 越权落地，本 task 无独立 `output.md`（无 Dev 自报）。本评分基于代码 + `input.md` + T0 `contracts.md` + Architect ADR 直接审视，**跳过"审查前验证（output.md 完整性）"**。Reviewer 自跑测试结果见下文「测试实测结果」节。

---

## 审查思考过程

1. **Task 意图**：在 `commands/workspace_folders.rs` 实现 4 写命令 + `count_folder_assets`，全部走「guard + kind 入口判定 + validate_and_canonicalize」三件套；同事务前缀替换；删除走 trash + path.exists 复检；旧多素材 `move_asset_to_workspace_folder` 退役；invoke_handler! 注册新命令；产出 PRD §6.1 全套 Rust 单测。

2. **AC 检查结果**：
   - AC-1 4 写命令实现：✅
     - `create_workspace_folder_impl`：guard → `validate_folder_name` → `nfc_normalize` → `validate_and_canonicalize` → `assert_no_sibling_nfc_dup` → `fs::create_dir` → 返 `kind: "root"`。✅
     - `rename_workspace_folder_impl`：guard → `kind_from_relative_path` 拒 ai_organized / root_import → 旧路径 canonicalize → 新名 validate → self-equal no-op 排除 → 同级 NFC 查重 → `unchecked_transaction()` → `safe_rename` → `db::asset::rename_path_prefix` → `tx.commit()` → EXDEV 走 `remove_src_after_commit`。✅
     - `delete_workspace_folder_impl`：guard → kind 拒非 root → canonicalize → 平台保护（`#[cfg(not(target_os="macos"))]` 返 `E_PLATFORM_UNSUPPORTED`）→ 事务内 `count_assets_under_prefix` → `confirm_non_empty=false && now>0` → `E_FOLDER_DIRTY{0, now}`；`now != expected` → `E_FOLDER_DIRTY{expected, now}` → `delete_assets_under_prefix` → `trash_delete_path`（含 TrashAdapter 抽象）→ `abs.exists()` 复检 → `E_TRASH_FAILED { reason: "still_exists" }` → `tx.commit()` → 返 `DeleteReport{ trashed: recount }`。✅
     - `move_asset_to_workspace_folder_impl`：先 `get_by_id` 拿 `project_id` → guard → kind 拒 `ai_organized`（允许 `root_import` 即 `__ROOT__`）→ `validate_and_canonicalize(target)` → `create_dir_all` if missing → `unique_path` 避免覆盖 → `safe_rename` → tx UPDATE 失败回滚物理 rename → COMMIT → EXDEV 清理。✅
   - AC-2 `rename_path_prefix`：✅ `src-tauri/src/db/asset.rs:384-415`，SQL 模板 `:new_prefix || substr(file_path, length(:old_prefix)+1) WHERE file_path = :old_no_slash OR file_path LIKE :old_prefix_like ESCAPE '\\'`；`escape_like` 处理 `\ % _`；`:old_prefix` 强制带尾 `/`。完全对齐 ADR-006。✅
   - AC-3 invoke_handler 注册 + 旧命令退役：✅
     - `lib.rs:148-154` 注册 4 新命令 + `count_folder_assets`。
     - `lib.rs:94-95` 注释说明旧 `move_asset_to_workspace_folder`（多素材）已从 invoke_handler 移除。
     - `commands/asset.rs:254-256` 旧函数加 `#[deprecated]` + `#[allow(dead_code)]`，保留函数体未删（避免内部 Rust 调用链断裂，可接受；行为符合 T0 §B.3 "退役"语义）。
     - 前端调用方：`AssetContextMenu.tsx:103` 已切到 `moveAssetToWorkspaceFolder(assetId, targetRelativePath)`（单素材）；`AssetListView.tsx` 无匹配引用（grep 返回空），表明该入口未调用旧多素材 API。
     - `debug_assert!(!path.contains("__ROOT__"))`：`db/asset.rs:8-17,179,213,389` 在 insert / update / rename_path_prefix 入口均防御。✅
   - AC-4 PRD §6.1 单测：✅ 8 个测试全部 PASS（详见下文实测结果），覆盖：路径越界 3 例 / 保留字 create+rename / ai_organized 三类写 / SQL 前缀边界（`100` vs `100%off`）/ NFC 查重 / trash 复检 LyingTrash / 写通道并发 / direct invoke + JSON round-trip。
   - AC-5 既有 read 命令保持 `Result<T, String>` 签名：✅ `get_project_workspace_root` / `list_project_workspace_folders` / `reveal_project_workspace_folder` 签名未变。
   - AC-6 `cargo test --lib commands::workspace_folders` 全绿：✅（8 passed; 0 failed）。

3. **关键发现**：
   - 实现整体严谨度高，10 条领域底线大部分被覆盖；事务原子性、trash 复检、debug_assert 防御链、kind 入口判定均到位。
   - `delete_workspace_folder` 在 trash 失败后早 return（tx 自动 drop 回滚），DB 不残留：✅。
   - 旧多素材入口已从 invoke_handler 注销 + `#[deprecated]` 标注，前端调用方已迁移：✅。
   - 唯一可吐槽点是 `write_guard_serializes` 测试只验证「两线程都成功」未真正断言 `max concurrent = 1`，但 T1 `write_guard.rs` 已自带串行性单测，本处验收较薄但可接受。
   - 3 处 `let mut conn` 多余 mut（cosmetic warning）。

---

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | 4 写命令 + count + 5 命令 invoke_handler 注册全部到位；8/8 单测通过；旧多素材已退役 |
| 安全性 | 25% | 5 | guard 三件套 / kind 入口判定 / validate_and_canonicalize / 同事务原子 / trash 复检 / debug_assert __ROOT__ 全防御链落地；direct invoke 仍拒 |
| 代码质量 | 15% | 4 | 命名清晰、impl/command 分层便于单测；TrashAdapter 注入良好；3 处 `let mut conn` 多余 mut；`assert_no_sibling_nfc_dup` 永远返 `parentRelativePath=""`（root-only 场景下正确但语义未来需扩展） |
| 测试覆盖 | 15% | 4 | PRD §6.1 全套覆盖、核心断言到位；`write_guard_serializes` 仅验证并发不冲突，未硬断言 `max concurrent=1`；NFC 查重依赖 macOS APFS 行为，多平台健壮性未验 |
| 架构一致性 | 10% | 5 | 严格遵守 ADR-001（IpcError）/ADR-003（写通道锁）/ADR-004（__ROOT__）/ADR-006（rename_path_prefix SQL）/ADR-007（safe_rename）/ADR-008（trash + 复检）/ADR-012（kind 入口） |
| 可维护性 | 10% | 4 | impl 分层 + State wrapper 便于 mock；count_assets_under_prefix / delete_assets_under_prefix 与 rename_path_prefix 三处 LIKE 转义逻辑重复（未抽 helper），未来若 schema 变更需同步三处 |

**综合分：4.65/5**（加权计算：0.25·5 + 0.25·5 + 0.15·4 + 0.15·4 + 0.10·5 + 0.10·4 = 1.25 + 1.25 + 0.6 + 0.6 + 0.5 + 0.4 = **4.60**）

---

## 总体判断

- [x] **PASS**
- [ ] FIX
- [ ] BLOCKER

无 BLOCKER；无 MAJOR；MINOR 4 项可在 T4/T5 顺手清理或单独提 issue。综合分 ≥ 3.5，符合 PASS 条件。

---

## 问题列表

### BLOCKER（必须修复）

无。

### MAJOR（强烈建议修复）

无。

### MINOR（可选）

1. **`write_guard_serializes` 断言过弱**
   - **代码位置**：`src-tauri/src/commands/workspace_folders.rs:1032-1059`
   - **现状**：仅 `assert!(t1.join().unwrap().is_ok())` + `t2.join().unwrap().is_ok())`，未硬断言 max concurrent=1。
   - **建议**：在 helper 内部加 `AtomicUsize` busy 计数器并在 critical section 内 `assert!(busy.fetch_add(1) == 0)`；或承认 T1 `write_guard.rs::test_lock_for_serializes_writes_per_project` 已硬断言串行，本处仅作集成保留。
   - **影响**：低；行为正确性已有 T1 单测覆盖。

2. **3 处 `let mut conn` 多余 mut（编译器 warning）**
   - **代码位置**：`workspace_folders.rs:412 / 492 / 618`
   - **建议**：去掉 `mut`，或保留并 `#[allow(unused_mut)]`。

3. **`assert_no_sibling_nfc_dup` 始终回填 `parentRelativePath=""`**
   - **代码位置**：`workspace_folders.rs:213-215`
   - **现状**：当前 create / rename 都只作用于根级文件夹，parent 一定是 workspace root，`""` 正确。
   - **风险**：将来若扩展到子目录创建/重命名，此处需改为从 caller 传入 `relative_path` 的 parent。
   - **建议**：加 TODO 注释或将函数签名扩为 `parent_rel: &str` 参数即可。

4. **三处 LIKE 转义模板重复**
   - **代码位置**：`workspace_folders.rs:267-271, 296-300, 738-742` + `db/asset.rs:escape_like`
   - **建议**：抽 `utils::sql::like_escape_with_trailing_slash(s)` 集中维护，避免未来 schema 变更时遗漏。
   - **影响**：纯重构 nit；当前三处行为一致。

---

## 旧多素材签名退役实测结论

- **后端**：`commands::asset::move_asset_to_workspace_folder` 已加 `#[deprecated]` + `#[allow(dead_code)]`；`lib.rs` invoke_handler 中已删除注册（line 94-95 注释保留为审计 marker）。✅
- **前端**：`AssetContextMenu.tsx:103` 已切到单素材 `moveAssetToWorkspaceFolder(assetId, targetRelativePath)`；`AssetListView.tsx` 无 `moveAssetToWorkspaceFolder` / `move_asset_to_workspace_folder` 引用（grep 空），表明该组件不再调用此 API。✅
- **结论**：**已退役**。残留函数体保留以防内部 Rust 调用，但 Tauri invoke 入口已切断 — 符合 T0 §B.3 退役语义。

---

## AC-3 调用方迁移核验结论

- `src/lib/tauri-commands.ts:224-232` 单素材 wrapper 入参 `assetId` / `targetRelativePath` 与 T0 §B.2 完全一致。
- `AssetContextMenu.tsx:100-108` 单点单素材调用并 catch IpcError 上报。
- `AssetListView.tsx`：无任何对该命令的引用（grep 空）。
- **结论**：**调用方已完成迁移**，无残留多素材调用点。

---

## 测试实测结果

**命令**：
```bash
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop
cargo test --manifest-path src-tauri/Cargo.toml --lib commands::workspace_folders 2>&1 | tail -120
```

**关键输出（warnings 略，仅余 warnings 9 条，全部为 unused import / unused_mut / unused_variable cosmetic）**：

```
warning: variable does not need to be mutable
   --> src/commands/workspace_folders.rs:412:9
warning: variable does not need to be mutable
   --> src/commands/workspace_folders.rs:492:13
warning: variable does not need to be mutable
   --> src/commands/workspace_folders.rs:618:9
warning: `notecapt` (lib test) generated 9 warnings
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.56s
     Running unittests src/lib.rs (src-tauri/target/debug/deps/app_lib-96758bcc4e7fff02)

running 8 tests
test commands::workspace_folders::tests::trash_recheck ... ok
test commands::workspace_folders::tests::path_escape ... ok
test commands::workspace_folders::tests::direct_invoke_rejected_and_error_json_roundtrip ... ok
test commands::workspace_folders::tests::prefix_boundary ... ok
test commands::workspace_folders::tests::ai_organized_protected ... ok
test commands::workspace_folders::tests::write_guard_serializes ... ok
test commands::workspace_folders::tests::reserved_name ... ok
test commands::workspace_folders::tests::nfc_dup ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 148 filtered out; finished in 0.18s
```

**结论**：T3 范围 8 个单测**全绿**；编译通过；9 条 warnings 全部为非阻断 cosmetic（unused import / unused_mut / unused_variable / dead_code），不影响功能与安全。

---

## 给 Dev 的修复指引

**不适用（PASS）**。MINOR 项建议在后续 task（如 T4 集成测试期间）顺手清理，或单独提一个清理 ticket，不阻塞 T3 进入 DONE 状态。
