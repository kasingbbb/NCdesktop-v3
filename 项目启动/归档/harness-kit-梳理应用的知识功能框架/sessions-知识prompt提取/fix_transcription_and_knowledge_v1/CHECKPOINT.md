# Session Checkpoint — 2026-04-23

Saved mid-`task_003_dev_E1` due to usage-limit pressure.

## 已完成文件改动（已落盘 src-tauri/）

### 新增
- `src-tauri/src/utils/safe_name.rs` — `sanitize_stem()` + 6 单测（替换 `/\:*?"<>|` 与控制字符为 `_`；折叠空格；保留 CJK/emoji；截断 120 字符）
- `src-tauri/src/db/concepts_extraction_log.rs` — `fetch_logged_pairs(conn, library_id) -> HashSet<(asset_id, content_hash)>` + `insert(conn, library_id, asset_id, content_hash)`（`INSERT OR IGNORE` 幂等）

### 修改
- `src-tauri/src/db/mod.rs` — `pub mod concepts_extraction_log;`
- `src-tauri/src/utils/mod.rs` — `pub mod safe_name;`
- `src-tauri/Cargo.toml` — 依赖新增 `sha2 = "0.10"`
- `src-tauri/src/db/migration.rs` — V9 迁移：`assets.derivative_version INTEGER DEFAULT 0`、`extracted_content.content_hash TEXT`、`concepts_extraction_log` 表、索引 `idx_assets_source_version` / `idx_extracted_content_hash` / `idx_cel_library` / `idx_cel_asset`，`PRAGMA user_version = 9`
- `src-tauri/src/models/asset.rs` — `Asset` 新增 `#[serde(default)] pub derivative_version: i32`
- `src-tauri/src/db/asset.rs` — `ASSET_SELECT` 追加 `COALESCE(derivative_version, 0)` 第 15 列；`row_to_asset` 读第 14 索引；新增 `set_derivative_version(conn, id, version)`；测试 `sample_asset` 补 `derivative_version: 0`
- `src-tauri/src/db/extraction.rs` — 新增 `set_content_hash(conn, asset_id, content_hash)`
- `src-tauri/src/commands/sync.rs` / `commands/asset.rs`（2 处）/ `commands/dropzone.rs` / `db/tag.rs`（测试） — 所有 `Asset { ... }` 字面量补 `derivative_version: 0`

## 未完成（task_003 剩余核心重构）

目标文件：`src-tauri/src/extraction/scheduler.rs`（651 行）

待改动：
1. `source_asset_should_materialize`（~line 455）：当 `source_asset.kind == "md"` 时应返回 `true`（即 .md 源也要 materialize，走 frontmatter 注入路径）
2. 新增函数 `materialize_source_markdown(app, source_asset)`：读取源 .md 正文 → 调 frontmatter 注入 → 写工作区 `<safe_stem>.md`
3. 新增函数 `materialize_placeholder(app, source_asset, failure_code, reason)`：对 unsupported/empty/error 分支产出占位 .md（含 YAML frontmatter + 失败原因）
4. 重写 `materialize_md`（~line 479–651）：
   - 入口读 `source_asset.derivative_version` 作为 N
   - 若已有派生 .md 存在：拷贝旧文件到 `_versions/<source_asset_id>/v{N}.md` 后再覆写
   - 覆写前 prepend YAML frontmatter（source_asset_id / derivative_version / extracted_at / extractor_type / quality_level）
   - 成功写盘后：`set_derivative_version(conn, source_id, N+1)` 与 `set_content_hash(conn, asset_id, sha256(structured_md))`
5. 调用点改造：
   - `scheduler.rs:149` unsupported 分支 → `materialize_placeholder(..., "unsupported", ...)`
   - `scheduler.rs:204-208` / `289-293` 成功但 `structured_md` 为空 → `materialize_placeholder(..., "empty_extract", ...)`
   - `db_handle_task_error`（~line 436）终态失败分支 → `materialize_placeholder(..., "extract_failed", err_msg)`
6. 新增 `enqueue_concept_extract_if_needed(app, asset)` 在 save 成功后调用（为 task_006 E4 铺路，可留 TODO）

## 后续 Task 顺序

- task_004（E2 F-3/F-4）— 版本化；主要在 `materialize_md` 内完成，与 task_003 重叠
- task_005（E3 F-6）— YAML frontmatter 注入；同一文件
- task_006（E4 F-7）— 保存 asset 后入队 `concept_extract`
- task_007（E5 F-8/F-9/F-10）— 改 `commands/knowledge.rs::extract_concepts_for_library`：尊重 `force` 参数；按 `concepts_extraction_log` 去重；跳过 `user_edited=1` 概念
- task_008（E6 F-11）— 仅回归测试；不改源

## 验证

- 未运行 `cargo check`（需在恢复后首先执行）
- 已知风险：Asset 字面量可能仍有遗漏构造点，`cargo check` 会暴露
