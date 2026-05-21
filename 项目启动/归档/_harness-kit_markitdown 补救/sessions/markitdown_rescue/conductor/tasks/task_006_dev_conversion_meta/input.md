# Task 输入 — task_006_dev_conversion_meta

## 目标
新建 `conversion_meta` 表（append-only 日志），并实现 `db/conversion_meta.rs` 的 CRUD。

## 前置条件
- 依赖 task：task_002（迁移基线）、task_004（标签函数已落，避免本 task 与之并行触发 migration 顺序竞争）
- 必须先存在：迁移机制（`db/migration.rs`）

## 验收标准（AC）
1. **AC-1**：迁移 V{N+2} 创建 `conversion_meta` 表（字段见 architect 方案 §五.2）+ 3 个索引（source / derived / converted_at）。
2. **AC-2**：迁移幂等：`IF NOT EXISTS`，重跑无副作用。
3. **AC-3**：`db/conversion_meta.rs` 提供：
   - `insert(conn, &ConversionMetaRow) -> Result<(), String>`
   - `list_by_source(conn, source_asset_id) -> Result<Vec<ConversionMetaRow>, String>`（按 `converted_at DESC`）
   - `latest_for_source(conn, source_asset_id) -> Result<Option<ConversionMetaRow>, String>`
4. **AC-4**：`ConversionMetaRow` 与 task_005 的 `ConversionAttempt` 字段一一对应（外加 `id` / `source_asset_id` / `derived_asset_id`）；serde camelCase。
5. **AC-5**：rusqlite 单测：插入 3 条不同 `converter_name` → `list_by_source` 按时间倒序返回 3 条；外键级联：删除 source asset → 相关 conversion_meta 也被删（`ON DELETE CASCADE` 验证）。

## 技术约束
- **不**对 `(source_asset_id, converter_name)` 加唯一约束（ADR-004：append-only 日志）。
- `id` 由调用方生成 UUID（与 `db::asset::insert` 风格一致）。
- 参数化绑定；禁止字符串拼接 SQL。
- 时间戳统一 RFC3339 字符串（`chrono::Utc::now().to_rfc3339()`）。

## 参考文件
- `src-tauri/src/db/migration.rs`
- `src-tauri/src/db/mod.rs`
- 架构方案 §三 ADR-004、§五.2

## 预估影响范围
- 新建文件：
  - `src-tauri/src/db/conversion_meta.rs`
- 修改文件：
  - `src-tauri/src/db/migration.rs`
  - `src-tauri/src/db/mod.rs`（pub mod + re-export 模型）
