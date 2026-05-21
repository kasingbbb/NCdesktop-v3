# Task 输入 — task_004_dev_db_tag_funcs

## 目标
在 `db/tag.rs` 新增 `propagate_tags_to_derivative` 和 `sync_tags_to_canonical_derivatives` 两个函数；在 `commands/dropzone.rs` 的 AI 打标完成处接入 `sync_tags_to_canonical_derivatives`，让标签传播只存在一处实现。

## 前置条件
- 依赖 task：task_002（assets 表需有 `source_asset_id` 列）
- 必须先存在：`assets.source_asset_id` 已迁移；`apply_llm_classify_to_asset` 函数仍在 `dropzone.rs`

## 验收标准（AC）
1. **AC-1**：`propagate_tags_to_derivative(conn, root_asset_id, derived_asset_id) -> Result<usize, String>`，使用 `INSERT OR IGNORE INTO asset_tags SELECT ?1, tag_id FROM asset_tags WHERE asset_id = ?2`，返回插入行数。
2. **AC-2**：`sync_tags_to_canonical_derivatives(conn, root_asset_id) -> Result<usize, String>`，使用 `INSERT OR IGNORE INTO asset_tags (asset_id, tag_id) SELECT a.id, at.tag_id FROM assets a JOIN asset_tags at ON at.asset_id = ?1 WHERE a.source_asset_id = ?1 AND a.asset_type = 'markdown'`。
3. **AC-3**：`commands/dropzone.rs::apply_llm_classify_to_asset` 在 `db::tag::link_to_asset` 调用之后**立即**调用 `sync_tags_to_canonical_derivatives(&conn, &asset.id)`，失败仅 `log::warn!`，不阻断主流程。
4. **AC-4**：集成测试覆盖三个场景：
   - 场景 A：原件已有衍生 .md → 调用 `propagate_*` → 衍生 .md 拿到全部标签
   - 场景 B：衍生 .md 已存在，原件后补 AI 标签 → `sync_*` 调用后衍生 .md 拿到新增标签
   - 场景 C：重复调用 `propagate_*` 不会产生重复行（INSERT OR IGNORE 验证）
5. **AC-5**：搜全仓确认 `INSERT INTO asset_tags` 只剩 `link_to_asset` / `propagate_*` / `sync_*` 三处；没有"inline 标签复制"散落。

## 技术约束
- 不允许在 dropzone/scheduler/inspector 内 inline 写 INSERT；只能通过 `db::tag::*` 公共函数。
- 失败模式：`propagate_*` / `sync_*` 失败必须用 `log::warn!`，**不能**让标签 IO 失败把 AI 打标主流程拖挂。
- 不允许 `unwrap()`/`expect()`。

## 参考文件
- `src-tauri/src/db/tag.rs`
- `src-tauri/src/commands/dropzone.rs::apply_llm_classify_to_asset`
- `src-tauri/src/extraction/scheduler.rs:694-703`（已经使用 `propagate_tags_to_derivative` 的调用点，本 task 提供函数实现）
- 架构方案 `task_001_architect/output.md` §十一 R6（防止标签传播多处实现）

## 预估影响范围
- 新建文件：无
- 修改文件：
  - `src-tauri/src/db/tag.rs`（+2 fn + 测试）
  - `src-tauri/src/commands/dropzone.rs`（+1 处调用）

## Conductor 追加（M-1 跨 task 待办）
- `src/extraction/mod.rs:4` 当前注释屏蔽了 scheduler 模块。本 task **不取消该注释**（取消后会暴露 task_008 未实现的链路，污染 cargo check）。
- 本 task 完成后必须验证 `cargo check` 仍是 0 error（与 task_002 末态一致）。如出现新 error 说明 dropzone 的 sync 调用引入了对 scheduler 的依赖，需 ESCALATE。
