# Task 交付 — task_006_dev_m5m6_retry_delete

## 实现摘要

落地 M5 重试 + M6 删除两条命令链：

1. **M5（AC-1 / AC-2）**：在 `commands::extraction` 新增 `retry_asset_conversion`
   薄包装命令，内部直接转发 `retrigger_extraction`，不重复实现 reset + enqueue
   逻辑。注册到 `lib.rs::invoke_handler!`（追加而不重排）。前端 `tauri-commands.ts`
   追加 `retryAssetConversion(assetId)` wrapper。幂等性靠后端三道护栏：
   `retrigger_extraction` 内部 already-running 检查 → `PipelineScheduler::enqueue`
   捕获 UNIQUE 冲突 → V7 部分唯一索引 `idx_pipeline_tasks_active_unique` 兜底。
   AC-2 在 `commands::extraction::tests` 加 `retry_asset_conversion_active_unique_guard_caps_at_one`：
   用 `run_migrations` 构造与生产同款 schema（含 V7 索引），连续 5 次 INSERT
   queued 行模拟连击 → 第 1 次成功、第 2…5 次被索引拦截、终局活动态行数=1。

2. **M6（AC-3 ~ AC-7）**：在 `db::asset` 新增 `DeleteCascadeReport` 结构与
   `delete_with_cascade(conn, asset_id)`：
   - 用 `resolve_asset_pair` 把 root.id / derivative.id 统一解算回 root；
   - 物理文件清理走 `remove_file_lenient`：`NotFound` 视为成功（返回 false），
     其它 IO 错误 `log::warn!` 但不中断 DB 级联（AC-3 / AC-4 原则）；
   - 手工 `DELETE FROM pipeline_tasks WHERE asset_id IN (root.id, derivative.id)`
     —— pipeline_tasks 在 V7 没 FK，必须显式清；
   - `DELETE FROM assets WHERE id = root.id` 触发 FK CASCADE 联动清
     `conversion_meta`（V6 source_asset_id CASCADE）、`extracted_content`（V8
     CASCADE）、`asset_tags`（V1 CASCADE）；
   - **补刀 derivative**：`assets.source_asset_id` V5 加列未带 FK，root 删除
     不会自动连坐 derivative 行，需用 derivative.id 二次 `DELETE FROM assets`。
   - 返回 `DeleteCascadeReport { root_asset_id, derivative_asset_id,
     derivative_existed, removed_root_file, removed_derivative_file,
     removed_pipeline_tasks }` 给命令层用于日志 / 测试断言。

3. **AC-4 outbound 缓存清理**：复用 task_005 的路径口径，把 `outbound_dir_for`
   的对外形式 `pub fn outbound_cache_dir_for(asset_id) -> Option<PathBuf>` 暴露
   在 `commands::outbound`（pure helper，无 IO，None 表示 `dirs_next::cache_dir()`
   无法定位）。`commands::asset::delete_asset` 在释放 DB 锁后调用它 +
   `fs::remove_dir_all`，失败仅 warn。

4. **AC-5 命令切换**：`commands::asset::delete_asset` 改为先 `delete_with_cascade`
   拿 report，再做 outbound 清理。命令签名（`id: String`）不变，前端
   `deleteAsset(id)` 完全兼容。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---|---|---|
| `NCdesktop/src-tauri/src/commands/extraction.rs` | 修改 | + `retry_asset_conversion` Tauri 命令（转发 `retrigger_extraction`）+ 1 单测 `retry_asset_conversion_active_unique_guard_caps_at_one` |
| `NCdesktop/src-tauri/src/db/asset.rs` | 修改 | + `DeleteCascadeReport` 结构 + `delete_with_cascade` + `remove_file_lenient` 私有辅助；`setup_conn` 加 `PRAGMA foreign_keys=ON` 以匹配生产；+ 4 单测（`delete_with_cascade_no_orphans` / `delete_with_cascade_resolves_via_derivative_id` / `delete_with_cascade_missing_file_is_ok` / `delete_with_cascade_returns_err_when_asset_missing`） |
| `NCdesktop/src-tauri/src/commands/asset.rs` | 修改 | `delete_asset` 改调 `delete_with_cascade` + outbound 缓存清理 |
| `NCdesktop/src-tauri/src/commands/outbound.rs` | 修改 | 把私有 `outbound_dir_for` 包装为 `pub fn outbound_cache_dir_for(asset_id) -> Option<PathBuf>`（pure helper） |
| `NCdesktop/src-tauri/src/lib.rs` | 修改 | `invoke_handler!` 在 `retrigger_extraction` 后追加 `retry_asset_conversion`，不重排其它命令 |
| `NCdesktop/src/lib/tauri-commands.ts` | 修改 | 在 `retriggerExtraction` 后追加 `retryAssetConversion(assetId)` wrapper |

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（修改集中在既有 `db::asset` / `commands::asset` / `commands::extraction` / `commands::outbound` / `lib.rs`，**无新建文件**）
- [x] API 路径/命名与 Architect 方案一致（`delete_with_cascade` / `DeleteCascadeReport` / `retry_asset_conversion` / `outbound_cache_dir_for` 全部按 ADR-007 + task_001 §六 + input.md 命名）
- [x] 数据模型与 Architect 方案一致（未新增 / 改字段；FK CASCADE 链条与 V1/V5/V6/V8 既有定义对齐；pipeline_tasks 无 FK → 手工 DELETE，与 §十 风险登记对齐）
- [x] 未引入计划外的新依赖（仅复用 rusqlite / std::fs / log / dirs-next / tempfile(dev)）
- 偏离说明：
  - **`outbound_cache_dir_for` 返回 `Option<PathBuf>`**：input.md AC-4 描述为 "**优先复用 task_005 的路径助手** —— 若 task_005 未公开 `outbound_cache_dir_for(asset_id)`，本 task 在 `commands::outbound` 中追加"。task_005 只暴露了 `outbound_dir_for(cache_root, asset_id)`（私有，含 cache_root 参数），本 task 追加 `outbound_cache_dir_for(asset_id) -> Option<PathBuf>`：内部 `dirs_next::cache_dir()?` 后调 `outbound_dir_for`，对外仅一个 `asset_id` 参数，符合 input.md 期望签名。返回 `Option` 是为了在 `dirs_next::cache_dir()` 极端返回 None 时让命令层 `warn` 兜底而不 panic（与 task_005 内 `prepare_outbound_payload` 第 4 步处理方式一致）。
  - **包名 `-p notecapt`**：input.md AC-8 写 `-p app_lib` 是笔误（`Cargo.toml [package].name = "notecapt"`，`app_lib` 仅是 cdylib 输出名），已沿用 task_002 / 003 / 004 / 005 一致的 `-p notecapt`。

## 测试命令

```bash
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri
cargo test -p notecapt --lib db::asset
cargo test -p notecapt --lib commands::asset
cargo test -p notecapt --lib commands::extraction
cargo test -p notecapt --lib commands::outbound
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop
npm run check
```

## 测试结果

### `cargo test -p notecapt --lib db::asset`

```
running 26 tests
test db::asset::tests::compute_state_failed_when_conversion_meta_has_error_class ... ok
test db::asset::tests::compute_state_completed_without_rendition_falls_through ... ok
test db::asset::tests::compute_state_done_even_when_source_missing ... ok
test db::asset::tests::compute_state_converting_for_queued_or_running ... ok
test db::asset::tests::compute_state_done_when_completed_and_rendition_present ... ok
test db::asset::tests::compute_state_failed_when_pipeline_failed ... ok
test db::asset::tests::compute_state_offline_when_cancelled ... ok
test db::asset::tests::compute_state_offline_when_no_pipeline_no_meta ... ok
test db::asset::tests::delete_with_cascade_returns_err_when_asset_missing ... ok
test db::asset::tests::list_root_assets_empty_project_returns_empty_vec ... ok
test db::asset::tests::find_markdown_derivative_returns_none_when_absent ... ok
test db::asset::tests::find_markdown_derivative_returns_latest_match ... ok
test db::asset::tests::delete_with_cascade_missing_file_is_ok ... ok
test db::asset::tests::list_root_assets_excludes_markdown_derivative ... ok
test db::asset::tests::list_root_assets_isolates_by_project_id ... ok
test db::asset::tests::list_root_assets_joins_latest_pipeline_and_conversion_meta ... ok
test db::asset::tests::delete_with_cascade_no_orphans ... ok
test db::asset::tests::delete_with_cascade_resolves_via_derivative_id ... ok
test db::asset::tests::set_derivative_version_advances_value ... ok
test db::asset::tests::resolve_asset_pair_returns_err_when_asset_missing ... ok
test db::asset::tests::update_markdown_derivative_changes_only_three_columns ... ok
test db::asset::tests::resolve_asset_pair_returns_root_and_derivative_when_input_is_root ... ok
test db::asset::tests::resolve_asset_pair_root_without_derivative_returns_none ... ok
test db::asset::tests::resolve_asset_pair_resolves_via_derivative_id ... ok
test db::asset::tests::list_root_assets_mixed_states_three_roots ... ok
test db::asset::tests::list_root_assets_orders_by_imported_at_desc ... ok

test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 107 filtered out; finished in 0.18s
```

### `cargo test -p notecapt --lib commands::asset`

```
running 13 tests
test commands::asset::tests::build_view_done_when_pipeline_completed_and_rendition_present ... ok
test commands::asset::tests::build_view_failed_uses_error_class_as_reason ... ok
test commands::asset::tests::build_view_failed_falls_back_to_pipeline_error_when_no_error_class ... ok
test commands::asset::tests::build_view_offline_when_no_pipeline_no_meta ... ok
test commands::asset::tests::build_view_source_missing_marks_flag_but_keeps_state ... ok
test commands::asset::tests::build_view_serializes_camel_case ... ok
test commands::asset::tests::rename_rejects_when_asset_missing ... ok
test commands::asset::tests::rename_rejects_empty_after_trim ... ok
test commands::asset::tests::rename_without_derivative_only_writes_root ... ok
test commands::asset::tests::rename_rejects_over_200_bytes ... ok
test commands::asset::tests::rename_via_derivative_id_resolves_to_root ... ok
test commands::asset::tests::rename_derivative_name_uses_sanitize_stem ... ok
test commands::asset::tests::rename_double_writes_root_and_derivative ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 120 filtered out; finished in 0.06s
```

### `cargo test -p notecapt --lib commands::extraction`

```
running 4 tests
test commands::extraction::tests::reset_when_no_row_is_noop ... ok
test commands::extraction::tests::reset_from_failed_clears_error_and_requeues ... ok
test commands::extraction::tests::reset_from_extracted_requeues_for_rerun ... ok
test commands::extraction::tests::retry_asset_conversion_active_unique_guard_caps_at_one ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 129 filtered out; finished in 0.01s
```

### `cargo test -p notecapt --lib commands::outbound`（验证暴露 helper 未破坏既有测试）

```
running 12 tests
test commands::outbound::tests::classify_state_mixed_returns_mixed_states_with_offending ... ok
test commands::outbound::tests::classify_state_single_non_done_returns_state_not_done ... ok
test commands::outbound::tests::outbound_error_serializes_to_camel_case_json ... ok
test commands::outbound::tests::classify_state_all_done_passes ... ok
test commands::outbound::tests::sanitize_replaces_slash_and_backslash ... ok
test commands::outbound::tests::sanitize_preserves_cjk_and_emoji ... ok
test commands::outbound::tests::sanitize_strips_control_chars_and_del ... ok
test commands::outbound::tests::sanitize_trailing_dot_or_space_appends_underscore ... ok
test commands::outbound::tests::sanitize_truncates_long_utf8_and_appends_asset_id_suffix ... ok
test commands::outbound::tests::sanitize_windows_reserved_appends_underscore ... ok
test commands::outbound::tests::reset_outbound_dir_is_idempotent_and_empties_existing_files ... ok
test commands::outbound::tests::link_or_copy_rendition_happy_path_creates_file_with_same_content ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 121 filtered out; finished in 0.00s
```

### `npm run check`（前端 tsc --noEmit）

```
> ncdesktop@0.0.0 check
> tsc --noEmit
```

（无输出 = 0 个 type error）

## AC 实现状态

| AC | 状态 | 验证位置 |
|---|---|---|
| AC-1 `retry_asset_conversion` 包装 + 注册 + 前端 wrapper | ✅ | `commands::extraction::retry_asset_conversion` + `lib.rs` + `tauri-commands.ts::retryAssetConversion` |
| AC-2 连击 5 次活动态 ≤ 1（V7 索引兜底） | ✅ | `retry_asset_conversion_active_unique_guard_caps_at_one` |
| AC-3 `delete_with_cascade` 物理文件 + 手工 pipeline_tasks + FK CASCADE 全链 | ✅ | `db::asset::delete_with_cascade` + `delete_with_cascade_no_orphans` |
| AC-4 outbound 缓存 `remove_dir_all` 失败仅 warn | ✅ | `commands::asset::delete_asset` 内 `if let Err … log::warn!`，复用 `commands::outbound::outbound_cache_dir_for` |
| AC-5 `delete_asset` 改调 `delete_with_cascade`，签名不变 | ✅ | `commands::asset::delete_asset`（`id: String` 入参未变） |
| AC-6 `delete_with_cascade_no_orphans` 全表行数 = 0 + 文件不存在 | ✅ | 测试断言 `assets` / `pipeline_tasks` / `conversion_meta` / `extracted_content` / `asset_tags` 全部为 0，两文件 `!path.exists()` |
| AC-7 传入 derivative.id 也级联到 root | ✅ | `delete_with_cascade_resolves_via_derivative_id` |
| AC-8 测试套件全过 | ✅ | 见上方测试结果（db::asset 26 / commands::asset 13 / commands::extraction 4 / commands::outbound 12，0 failed） |

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|---|---|---|---|
| ✅ 正常路径 | root + derivative + cm + pt + ec + tags 全链级联删除 | 已测 | PASS（`delete_with_cascade_no_orphans`） |
| ✅ 正常路径 | 传入 derivative.id 级联到 root | 已测 | PASS（`delete_with_cascade_resolves_via_derivative_id`） |
| ✅ 正常路径 | retry 命令薄包装转发到 retrigger_extraction | 已测 | 静态：函数体仅 `retrigger_extraction(app, asset_id).await`；命令注册见 lib.rs |
| ⚠️ 边界条件 | 物理文件已被外部删除 → `removed_root_file=false` 但 DB 仍清空 | 已测 | PASS（`delete_with_cascade_missing_file_is_ok`） |
| ⚠️ 边界条件 | root 无 derivative 时只删 root + 关联 | 已测 | PASS（`delete_with_cascade_missing_file_is_ok` 即 root-only 场景） |
| ⚠️ 边界条件 | 连击 5 次 retry 活动态行数被 V7 索引兜底为 1 | 已测 | PASS（`retry_asset_conversion_active_unique_guard_caps_at_one`） |
| ❌ 异常路径 | 不存在的 asset_id → `素材不存在` | 已测 | PASS（`delete_with_cascade_returns_err_when_asset_missing`） |
| ⚠️ 边界条件 | outbound 缓存目录不存在 / 删除失败 | 未自动测 | 命令层 `if cache_dir.exists()` 守卫 + `Err` 仅 `log::warn!`；命令路径依赖真实 `dirs_next::cache_dir()`，留给 reviewer 手测（删除一条素材后观察 `~/Library/Caches/NCdesktop/outbound/<id>/` 已消失，无 panic） |
| ⚠️ 边界条件 | `delete_asset` 命令端到端（real Database + State） | 未自动测 | 同 task_004 / 005 既定策略：核心逻辑下沉到 `db::asset::delete_with_cascade`（已单测），命令外壳仅做锁获取 + outbound 清理，逻辑近乎平凡 |

## 已知局限

1. **`commands::asset::delete_asset` 命令外壳未做端到端自动化测试**：依赖
   `tauri::State<Database>` 与 `dirs_next::cache_dir()`，构造完整 Tauri runtime
   成本高。命令层只剩两段平凡逻辑（调用 `delete_with_cascade` + outbound
   清理），其语义已被 db 层 4 个单测 + outbound 12 个单测覆盖；端到端留给
   reviewer 手测或集成测试 task_009。
2. **`retry_asset_conversion` 命令外壳本身未自动化**：薄包装无业务分支，
   行为完全由 `retrigger_extraction` 决定（既有 5 个单测覆盖）。AC-2 的活动态
   ≤ 1 不变式靠 V7 索引兜底单测保证，比"命令层正向连击"更接近不变式核心。
3. **outbound 缓存目录失败处理**：当前仅 `log::warn!`，未把失败计数透到
   `DeleteCascadeReport`。后续若想在 UI 上展示"DB 删了但缓存清理失败"，需
   再补一个字段，本 task 范围未做（input.md AC-4 明示"失败仅 warn"）。
4. **`delete_with_cascade` 不是单事务**：物理文件清理 + 多条 DELETE 不在
   `BEGIN/COMMIT` 内（rusqlite 单连接默认 autocommit）。极端情况下，pipeline
   DELETE 成功但 root DELETE 失败会留残留 pipeline 行 —— 但 root DELETE 在
   仅 PK + FK 约束下几乎不会失败，且即便失败再次调用会重新解算并清理。
   显式 transaction 可在后续 task 引入。

## 需要 Reviewer 特别关注的地方

1. **derivative 显式 DELETE 的必要性**（`db/asset.rs` 第 ~298 行）：
   `assets.source_asset_id` 在 V5 加列时未带 FK 约束，因此 `DELETE FROM assets
   WHERE id = root.id` 不会自动连坐 derivative 行。本实现在 root DELETE 之后
   显式 `DELETE FROM assets WHERE id = derivative_id`。请确认这一方案符合
   ADR-001 / ADR-007 对 derivative 生命周期的预期；若未来想用 FK 自动联动，
   需要单独一次 schema migration 加 FK，不在本 task 范围。
2. **测试 `setup_conn` 加 `PRAGMA foreign_keys=ON`**（`db/asset.rs` 第 ~345 行）：
   生产路径在 `Database::open` 内开 FK，测试 `setup_conn` 此前未开。新增的
   `delete_with_cascade_no_orphans` 依赖 FK CASCADE，必须打开 PRAGMA。这一改动
   也让之前的测试更贴近生产 schema，所有既有测试仍 PASS。
3. **outbound 缓存清理使用 `outbound_cache_dir_for` 而非直接 `dirs_next::cache_dir`**：
   关键不变式是 task_005 写入路径与 task_006 删除路径必须 byte-for-byte 相等。
   本 task 通过把私有 `outbound_dir_for` 包成 `pub outbound_cache_dir_for` 共享
   同一拼接逻辑（`CACHE_SUBDIR = "NCdesktop/outbound"`），任何路径改动只需改
   一处。请确认这种"helper 暴露"是首选路径，相比"拷贝路径常量"更稳。
4. **`retry_asset_conversion` 与 `retrigger_extraction` 共存**：两者都在
   `invoke_handler!` 中注册。前端约定 task_006 起新 caller 一律用
   `retryAssetConversion`；旧 `retriggerExtraction` 暂留兼容 task_011 已落地的
   调用方。若希望删除旧 wrapper，请明示（本 task 选保留以降低本次 review
   的回归风险）。
