# Task 输入 — task_015_db_kc_fields_writer

## 目标
实装 DB helper：把 KC enrichment 的结果字段（kc_enriched / kc_version / kc_tags_source / kc_doc_id / kc_response_size / kc_duration_ms）写入 extracted_content + conversion_meta 表。

## 前置条件
- 依赖 task：task_002（v18 schema 已迁移）
- 必须先存在的文件/接口：
  - DB user_version=18（extracted_content 与 conversion_meta 已加列）
  - `src-tauri/src/db/asset.rs`（现有 insert/update 模式参考）

## 验收标准（Acceptance Criteria）
1. **AC-1**：在 `src-tauri/src/db/extraction.rs`（或新文件 `db/kc_meta.rs`）追加：
   ```rust
   pub fn db_update_kc_fields(
       conn: &Connection,
       asset_id: &str,
       kc_enriched: &str,         // "true" / "false" / "partial"
       kc_version: Option<&str>,
       kc_tags_source: Option<&str>,
   ) -> Result<(), String>;
   ```
   - SQL: `UPDATE extracted_content SET kc_enriched=?, kc_version=?, kc_tags_source=? WHERE asset_id=?`
2. **AC-2**：扩展 `db::conversion_meta::insert` 或新增 `db_conversion_meta_kc_insert`：
   - 在现有 ConversionMetaRow 上扩展 3 个可选字段：`kc_doc_id: Option<String>`、`kc_response_size: Option<u64>`、`kc_duration_ms: Option<u64>`
   - SQL INSERT 含上述 3 列
3. **AC-3**：实装 `pub fn db_read_kc_status(conn: &Connection, asset_id: &str) -> Result<Option<KcStatusRow>, String>`：
   - 返回 `Option<KcStatusRow { kc_enriched, kc_version, kc_tags_source }>` 供前端 Tauri command 使用
4. **AC-4**：单元测试：
   - `db_update_kc_fields_sets_three_columns`
   - `db_update_kc_fields_idempotent`（重复调用不报错）
   - `db_conversion_meta_kc_insert_persists_three_optional`
   - `db_read_kc_status_returns_none_when_no_row`
   - `db_read_kc_status_returns_values_after_update`

## 技术约束
- 使用 r2d2 连接池（NC 现状 PR #28），通过 `app.state::<Database>().get_conn()` 取连接
- UPDATE 不存在的 asset_id 不报错（rows_affected=0 视为成功，与现状一致）
- ConversionMetaRow 字段扩展需保持向后兼容（旧调用方不需要传 None）—— 通过 Default 或 builder 模式

## 参考文件
- Architect output.md §"数据模型 extracted_content schema migration"
- `src-tauri/src/db/conversion_meta.rs:1-50` ConversionMetaRow 现状
- task_002 input.md（v18 列定义）

## 预估影响范围
- 新建文件：可选 `src-tauri/src/db/kc_meta.rs`
- 修改文件：
  - `src-tauri/src/db/conversion_meta.rs`（扩展 ConversionMetaRow 字段 + insert）
  - `src-tauri/src/db/extraction.rs` 或新建 kc_meta.rs（含 db_update_kc_fields + db_read_kc_status）
  - `src-tauri/src/db/mod.rs`（如建新文件需在此 pub mod）

## Reviewer 重点关注项
- ConversionMetaRow 扩展是否破坏现有调用方（task_002 之前的所有 scheduler write_conversion_meta 调用）
- SQL 拼接安全（用 prepared statement，不字符串拼接）
- 字段类型与 schema 一致（kc_response_size INTEGER → u64，注意 SQLite 是 i64 上限）

## 复杂度
S（0.5d 工作量，~300 行含测试）
