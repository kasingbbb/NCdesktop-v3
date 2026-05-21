# Task 003 — Dev E1 Output（工作区完整性 F-1 / F-2）

## 状态
DONE — `cargo check` + `cargo test --lib` 全绿（61 passed）。

## 交付

### V9 迁移（`src-tauri/src/db/migration.rs`）
- `assets.derivative_version INTEGER NOT NULL DEFAULT 0`
- `extracted_content.content_hash TEXT`
- 新表 `concepts_extraction_log(id, library_id, asset_id, content_hash, extracted_at)` + UNIQUE 约束
- 索引：`idx_assets_source_version` / `idx_extracted_content_hash` / `idx_cel_library` / `idx_cel_asset`
- `PRAGMA user_version = 9`

### 新模块
- `src-tauri/src/utils/safe_name.rs` — `sanitize_stem()`（6 单测通过）
- `src-tauri/src/db/concepts_extraction_log.rs` — `fetch_logged_pairs` / `insert`（幂等）
- `src-tauri/src/lib.rs` — 注册 `pub mod utils;`

### 数据层
- `Asset` 新增 `derivative_version: i32`
- `db::asset::set_derivative_version(conn, id, version)`
- `db::extraction::set_content_hash(conn, asset_id, hash)`
- 6 处 Asset 字面量构造点补字段

### scheduler.rs 重构（E1 核心）
- `source_asset_should_materialize`：放宽为 `source_asset_id.is_none()`（所有原件都物化，.md 源也不再跳过）
- 新增 `source_asset_is_markdown` 判别函数
- 新增 `write_derivative_md`（共享写盘逻辑）：
  - 读 `source_asset.derivative_version` → `N+1`
  - 若有旧派生件，先 `_versions/<source_id>/v{N}.md` 归档后覆写
  - 覆写前 prepend YAML frontmatter（source_asset_id / derivative_version / extracted_at / extractor_type / quality_level）
  - 写盘后 `set_derivative_version(source) = set_derivative_version(derivative) = N+1`
  - 源件 + 衍生件均写 `content_hash = sha256(body)`
  - `propagate_tags_to_derivative` 同步标签
- 新增 `materialize_placeholder(failure_code, reason)` — 不支持/空白/失败路径产出占位 .md
- 新增 `materialize_source_markdown` — .md 源读源文件并注入 frontmatter 写工作区
- `materialize_md` 签名变更为 `(app, asset, body, quality_level, extractor_type)`
- 调用点改造：
  - unsupported 分支 → `materialize_source_markdown` 或 `materialize_placeholder("unsupported")`
  - 两处成功但 `structured_md` 为空 → `materialize_placeholder("empty_extract")`
  - 抽取终态失败（`retry + 1 >= max_retries`） → `materialize_placeholder("extract_failed")`
  - panic 终态 → `materialize_placeholder("extract_panic")`
- `write_derivative_md` 内留 TODO 标记供 task_006 E4 接入 `enqueue_concept_extract_if_needed`

## 验收
- ✅ `cargo check` 通过
- ✅ `cargo test --lib`：61 passed；含 `utils::safe_name::*` 6 例 + `db::tests::migration_v*` 5 例
- ✅ V9 迁移幂等（覆盖 V3/V4 相同模式）
- 待验证（依赖运行时）：H-W01..W06 工作区 .md 产出（需 GUI 集成测试，落到 task_008）
