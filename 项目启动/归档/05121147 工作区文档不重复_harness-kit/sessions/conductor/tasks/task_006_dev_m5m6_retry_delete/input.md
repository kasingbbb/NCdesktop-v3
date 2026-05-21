# Task 输入 — task_006_dev_m5m6_retry_delete

## 目标
M5 失败重试：包装现有 `retrigger_extraction` 为对外 `retry_asset_conversion`；M6 删除级联：实现 `db::asset::delete_with_cascade`，确保 root + derivative + 两文件 + conversion_meta + pipeline_tasks + extracted_content + outbound cache 目录无孤儿。

## 前置条件
- 依赖 task：task_004（resolve_asset_pair）、task_005（outbound cache 路径解析）
- 必须先存在的文件/接口：
  - `commands::extraction::retrigger_extraction`（已存在，task_011 实现）
  - `idx_pipeline_tasks_active_unique`（V7 已落地，幂等护栏）
  - FK CASCADE 已启用：`assets`/`conversion_meta`/`extracted_content`/`asset_tags`

## 验收标准（AC）
1. **AC-1（M5）**：在 `commands::asset` 或 `commands::extraction` 中新增对外命令 `retry_asset_conversion(asset_id) -> ()`，内部直接调用 `commands::extraction::retrigger_extraction`（避免重复逻辑）。注册到 `lib.rs`。前端 wrapper 新增 `retryAssetConversion`。
2. **AC-2（M5 幂等）**：连续 5 次调用 `retry_asset_conversion` 单测断言：
   - `pipeline_tasks` 在该 asset 下"活动态（queued+running）"行数 ≤ 1（V7 索引保证）
   - `conversion_meta` 历史行数随真实重试次数累加（≥ 5，如果 scheduler 实际跑过；纯命令侧测试只断 ≤ 1 的活动态）
3. **AC-3（M6）**：新增 `db::asset::delete_with_cascade(conn, asset_id) -> Result<DeleteCascadeReport, String>`：
   - 解算 root + derivative
   - 删 root 与 derivative 的物理文件（`fs::remove_file`，文件不存在视为成功）
   - `DELETE FROM pipeline_tasks WHERE asset_id IN (root.id, derivative.id)` （手工，因无 FK）
   - `DELETE FROM assets WHERE id = root.id` —— FK CASCADE 自动清 derivative / conversion_meta / extracted_content / asset_tags
   - 返回 `DeleteCascadeReport { removed_root_file: bool, removed_derivative_file: bool, removed_pipeline_tasks: usize, ... }`，便于日志/测试断言。
4. **AC-4（M6 outbound cache）**：删除时同时 `fs::remove_dir_all(cache_dir().join("NCdesktop/outbound").join(asset_id))`，失败仅 warn。
5. **AC-5（M6 命令）**：`commands::asset::delete_asset` 改为调用 `delete_with_cascade`（替换现有简单 `db::asset::delete`）。命令签名不变（兼容前端）。
6. **AC-6（M6 单测）**：`db::asset::tests::delete_with_cascade_no_orphans`：
   - 构造 root + derivative + 2 个 `conversion_meta` + 1 个 `pipeline_tasks` + 1 个 `extracted_content` + 写 2 个临时文件作为 source/derivative
   - 调用 `delete_with_cascade(root.id)`
   - 断言：`assets`/`conversion_meta`/`extracted_content`/`pipeline_tasks` 中相关行全部为 0；2 个临时文件均不存在
7. **AC-7（M6 derivative.id 入参）**：传入 derivative.id 也应级联到 root（与 ADR-007 解算一致）。
8. **AC-8**：`cargo test -p app_lib --lib db::asset` 与 `commands::asset` 全部通过。

## 技术约束
- 不在 commands/ 中拼 SQL：delete_with_cascade 在 db/。
- `pipeline_tasks` 无 FK，必须手工 DELETE（task_001_architect §六 已明示）。
- FK CASCADE 已启用（`db::mod::open` 中 `PRAGMA foreign_keys=ON`），单测内存库需复用 `run_migrations` 保留 PRAGMA。
- 失败的物理文件清理仅 warn，不阻断 DB 删除（避免文件被锁导致整个删除回滚）。
- 中文文案。

## 参考文件
- `src-tauri/src/commands/extraction.rs::retrigger_extraction`（M5 复用）
- `src-tauri/src/db/asset.rs::delete`（M6 改造起点）
- `src-tauri/src/db/migration.rs`（核对 FK CASCADE 列表）
- `task_001_architect/output.md` §ADR-007 / §十 风险登记（pipeline_tasks 无 FK）

## 预估影响范围
- 新建文件：无
- 修改文件：
  - `src-tauri/src/db/asset.rs`（+ delete_with_cascade）
  - `src-tauri/src/commands/asset.rs`（delete_asset 改造）
  - `src-tauri/src/commands/extraction.rs` 或 `commands/asset.rs`（+ retry_asset_conversion wrapper）
  - `src-tauri/src/lib.rs`（注册 retry_asset_conversion）
  - `src/lib/tauri-commands.ts`（+ retryAssetConversion）
- 估算变更：~400 行（含 ~200 行测试）
