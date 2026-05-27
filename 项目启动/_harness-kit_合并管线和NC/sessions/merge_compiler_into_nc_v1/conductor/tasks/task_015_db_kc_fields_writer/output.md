# Task 交付 — task_015_db_kc_fields_writer

## 实现摘要

实装 KC enrichment 字段在 NC 端的 DB 读写层。task_002 已落地 v18 schema（extracted_content + conversion_meta 各 3 个 KC 列），本 task 在 Rust 侧实装对应的 CRUD helper，并扩展 `ConversionMetaRow` 结构体承载 3 个新字段。

**核心 API（input.md AC-1 / AC-2 / AC-3）**：
- `db_update_kc_fields(conn, asset_id, kc_enriched, kc_version, kc_tags_source) -> Result<(), String>` —— 在 `extracted_content` 表更新 3 个 KC 列（`kc_enriched` 必填字符串字面 "true" / "false" / "partial"；`kc_version` / `kc_tags_source` 可选）。
- `db_read_kc_status(conn, asset_id) -> Result<Option<KcStatusRow>, String>` —— 读取 KC 三态状态；`KcStatusRow` 是轻量结构，仅含 3 个 Option 字段（不带 raw_text / structured_md 等大字段），专为前端 Tauri command 设计。
- `db_conversion_meta_kc_insert(conn, source_asset_id, converter_name, converter_version, source_mime, source_hash, quality_level, kc_doc_id, kc_response_size, kc_duration_ms) -> Result<String, String>` —— `conversion_meta` 轻量插入封装，接受 `Option<u64>` 入参，内部 saturating-cast 到 i64（防 u64::MAX 溢出 panic），自动生成 UUID + 时间戳。返回新行 id。

**核心设计决策**：

1. **归属选择：写进现有 `src-tauri/src/db/extraction.rs`，不新建 `kc_meta.rs`**。理由：
   - `db_update_kc_fields` / `db_read_kc_status` 操作 `extracted_content` 表，与 extraction.rs 现有的 `update_extraction_status` / `upsert_extraction_result` 等强同源。新建 kc_meta.rs 会让"同一张表的 CRUD 分散在两个文件"反而难维护。
   - `db_conversion_meta_kc_insert` 操作 `conversion_meta` 表，自然落在 conversion_meta.rs。
   - 测试隔离：extraction.rs 新开 `mod kc_tests`（独立于其它将来可能加入的测试 mod），conversion_meta.rs 复用现有 `mod tests`。

2. **ConversionMetaRow 兼容策略：`#[derive(Default)]` + `..Default::default()` spread**。理由：
   - input.md 给的方案是"Default 或 builder"，Default 更轻量，与 NC 代码库风格一致（asset.rs / project.rs 多个 Row 类型已用 `#[derive(Default)]`）。
   - 旧调用方（scheduler.rs:1323）仅需在结构字面量末尾加一行 `..Default::default()`，3 个 KC 字段语义默认 `None`（写入数据库 NULL）。
   - 字段类型选 `Option<i64>` 而非 `Option<u64>`：SQLite INTEGER 列底层是 i64，rusqlite 直接支持 i64 trait；对外接口（`db_conversion_meta_kc_insert`）接受 `u64`，在边界做 saturating-cast，把"u64 安全语义"放在最外层封装。

3. **SQL 全部参数化**：`db_update_kc_fields` 用 `conn.execute` + `params![]`，`db_read_kc_status` 用 `conn.query_row` + `params![]`，无字符串拼接（input.md 技术约束）。

4. **AC-2 选择"扩展 `insert` SQL 列表"**而非"新增 sibling 函数替代旧 insert"：原 `insert` 接受完整 `ConversionMetaRow`，本 task 让它写入全部 15 列（原 12 + KC 3）；3 个 KC 字段若 `None` 则落 NULL。`db_conversion_meta_kc_insert` 是友好封装层（构造 Row 后调 `insert`），不重复 SQL。

5. **`row_to_meta` 不读 KC 列（只填 None 兜底）**：现有 `list_by_source` / `latest_for_source` 的 SELECT 列表没有 KC 列，沿用旧 SELECT 不破坏行解析序号；`row_to_meta` 内显式标记"KC 字段留 None"。未来若有 list-with-KC 需求再扩 SELECT 列表 + row.get(12..14)。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|---------|------|
| `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/db/conversion_meta.rs` | 修改 | (1) `ConversionMetaRow` struct 加 `#[derive(Default)]` 并新增 3 个 `Option<i64>` / `Option<String>` KC 字段；(2) `insert` SQL 扩到 15 列；(3) 新增 `db_conversion_meta_kc_insert` 友好封装函数；(4) `row_to_meta` 显式标注 KC 字段 None 兜底；(5) 测试 helper `sample_row` 与 `serde_derived_asset_id_none_is_json_null` 测试加 `..Default::default()`；(6) 在 `mod tests` 末尾新增 2 个 KC 测试 |
| `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/db/extraction.rs` | 修改 | (1) `use` 加 `Deserialize`；(2) 新增 `pub struct KcStatusRow`（3 个 Option 字段，camelCase 序列化）；(3) 新增 `pub fn db_update_kc_fields`；(4) 新增 `pub fn db_read_kc_status`；(5) 文件末尾新增 `mod kc_tests`（5 个测试用例 + 2 个 setup helper） |
| `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/extraction/scheduler.rs` | 修改 | scheduler.rs:1323 处 `ConversionMetaRow` 字面量构造尾部加一行 `..Default::default()`（兼容性必需的最小改动）|

无新建文件，无删除文件，无新依赖。

## 对 Architect 方案的遵守声明

- [x] **目录结构与 Architect 方案一致**：API 落在 input.md 指定的两个现有文件（`db/extraction.rs` + `db/conversion_meta.rs`），未新建文件。
- [x] **API 路径/命名与 Architect 方案一致**：
  - `db_update_kc_fields` —— 与 input.md AC-1 函数签名字面一致。
  - `db_read_kc_status` / `KcStatusRow` —— 与 input.md AC-3 函数签名字面一致。
  - `db_conversion_meta_kc_insert` —— 与 input.md AC-2 命名候选字面一致。
- [x] **数据模型与 Architect 方案一致**：
  - `extracted_content` 写 3 列 `kc_enriched` / `kc_version` / `kc_tags_source`（task_002 v18 schema）。
  - `conversion_meta` 写 3 列 `kc_doc_id` / `kc_response_size` / `kc_duration_ms`（task_002 v18 schema）。
- [x] **未引入计划外的新依赖**：只用 rusqlite / serde / uuid / chrono 现有 crate。
- **偏离说明**：
  - `ConversionMetaRow` 新增字段时**没有用 `Option<u64>`**而用 `Option<i64>`。理由见"核心设计决策 §2"：rusqlite trait 直接支持 i64，u64 → i64 边界检查放在最外层 `db_conversion_meta_kc_insert` 函数（接受 `Option<u64>`，内部 saturating-cast）。input.md AC-2 字面说"`kc_response_size: Option<u64>`"，本 task 解读为"对外接口暴露 u64 语义"，结构体内部存 i64 是实现细节（与 SQLite INTEGER 上限一致）。
  - 测试数量：input.md AC-4 列举 5 个测试名，本 task 实装时合并 `db_update_kc_fields_idempotent` 测试到同一个函数内（覆盖"重复调用 + 不存在 asset_id + 部分字段清回 None"3 种 idempotent 边界），并额外加 `db_conversion_meta_kc_insert_saturates_u64_to_i64_max`（u64 边界保护）+ `kc_status_row_serializes_as_camel_case`（前端契约保护），共 7 个新测试。

## 测试命令

```bash
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri

# 编译
cargo build --lib

# 本 task 范围测试
cargo test --lib db::extraction::kc_tests
cargo test --lib db::conversion_meta::tests::db_conversion_meta_kc

# 整个 db 模块
cargo test --lib db::

# 全 lib 测试（回归）
cargo test --lib
```

## 测试结果

**`cargo build --lib`**：

```
warning: `notecapt` (lib) generated 5 warnings (run `cargo fix --lib -p notecapt` to apply 4 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 18.97s
```

（5 个 warning 均为预存在的 dead_code，与本 task 无关。）

**`cargo test --lib db::extraction::kc_tests`**：

```
running 5 tests
test db::extraction::kc_tests::kc_status_row_serializes_as_camel_case ... ok
test db::extraction::kc_tests::db_read_kc_status_returns_none_when_no_row ... ok
test db::extraction::kc_tests::db_update_kc_fields_idempotent ... ok
test db::extraction::kc_tests::db_update_kc_fields_sets_three_columns ... ok
test db::extraction::kc_tests::db_read_kc_status_returns_values_after_update ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 491 filtered out; finished in 0.07s
```

**`cargo test --lib db::conversion_meta`**（含 2 个新 KC 测试 + 12 个老回归）：

```
running 14 tests
test db::conversion_meta::tests::latest_for_source_picks_most_recent_and_handles_missing ... ok
test db::conversion_meta::tests::serde_derived_asset_id_none_is_json_null ... ok
test db::conversion_meta::tests::get_conversion_state_returns_failed_when_8code_present ... ok
test db::conversion_meta::tests::get_conversion_state_returns_none_when_no_meta ... ok
test db::conversion_meta::tests::get_conversion_state_null_fc_empty_content_is_legacy_unverified ... ok
test db::conversion_meta::tests::get_conversion_state_returns_success_when_content_present ... ok
test db::conversion_meta::tests::get_conversion_state_returns_legacy_unverified ... ok
test db::conversion_meta::tests::db_conversion_meta_kc_insert_persists_three_optional ... ok
test db::conversion_meta::tests::list_by_source_returns_rows_desc ... ok
test db::conversion_meta::tests::deleting_source_asset_cascades_conversion_meta ... ok
test db::conversion_meta::tests::db_conversion_meta_kc_insert_saturates_u64_to_i64_max ... ok
test db::conversion_meta::tests::update_failure_code_no_row_is_not_error ... ok
test db::conversion_meta::tests::update_failure_code_writes_screaming_snake_and_clears_on_none ... ok
test db::conversion_meta::tests::update_failure_code_targets_latest_row_only ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 482 filtered out; finished in 0.28s
```

**`cargo test --lib db::`**（整 db 模块）：

```
test result: ok. 104 passed; 0 failed; 0 ignored; 0 measured; 392 filtered out; finished in 2.03s
```

**`cargo test --lib`**（整体回归）：

```
test result: ok. 496 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.77s
```

总数：**496 = baseline 489 + 本 task 新增 7**，0 失败，0 跳过。

## 自测验证矩阵

| AC | 场景描述 | 状态 | 测试用例 |
|----|---------|------|---------|
| AC-1 | `db_update_kc_fields` 同时更新 3 列 | 已测 | `db_update_kc_fields_sets_three_columns` PASS |
| AC-1 | UPDATE 不存在的 asset_id 不报错（容忍语义） | 已测 | `db_update_kc_fields_idempotent` 第 4 阶段 PASS |
| AC-1 | 重复调用 UPDATE 幂等（同入参不报错） | 已测 | `db_update_kc_fields_idempotent` 第 3 阶段 PASS |
| AC-1 | UPDATE 后字段可清回 None（kc_tags_source: Some → None） | 已测 | `db_update_kc_fields_idempotent` 第 2 阶段 PASS |
| AC-2 | `db_conversion_meta_kc_insert` 持久化 3 个 KC 字段 | 已测 | `db_conversion_meta_kc_insert_persists_three_optional` PASS |
| AC-2 | u64::MAX 输入不 panic，saturating-cast 到 i64::MAX | 已测 | `db_conversion_meta_kc_insert_saturates_u64_to_i64_max` PASS |
| AC-2 | ConversionMetaRow 向后兼容（旧调用方加 `..Default::default()`） | 已测 | scheduler.rs 编译通过 + 12 个老 conversion_meta 测试 PASS |
| AC-3 | 无行时 `db_read_kc_status` 返回 `Ok(None)` | 已测 | `db_read_kc_status_returns_none_when_no_row` PASS |
| AC-3 | 行存在但 KC 列全 NULL 时返回 `Some(KcStatusRow { 全 None })` | 已测 | `db_read_kc_status_returns_values_after_update` UPDATE 前阶段 PASS |
| AC-3 | UPDATE 后能读到对应 KC 值 | 已测 | `db_read_kc_status_returns_values_after_update` UPDATE 后阶段 PASS |
| AC-3 | KcStatusRow JSON 序列化为 camelCase（前端契约） | 已测 | `kc_status_row_serializes_as_camel_case` PASS |
| AC-4 | 测试数 ≥ 5 | 已测 | extraction.rs 5 + conversion_meta.rs 2 = 7 ✓ |

## 已知局限

1. **`row_to_meta` 不读 KC 列**：现有 `list_by_source` / `latest_for_source` 的 SELECT 列表沿用旧 12 列，所以这两个 API 返回的 `ConversionMetaRow` 中 KC 字段始终是 None（即便 DB 里实际有值）。这是有意为之——避免本 task 范围扩张到读路径变更；如果未来前端需要"按 source 列出含 KC 指标的全部历史行"，应新增 `list_by_source_with_kc()` 函数或扩 SELECT 列表，但本 task 不预先做（YAGNI）。
2. **`ConversionMetaRow.kc_response_size` / `kc_duration_ms` 内部类型是 `Option<i64>`**：对外接口（`db_conversion_meta_kc_insert`）接受 `Option<u64>`，转入结构体时做 saturating-cast。如果调用方直接构造 `ConversionMetaRow` 字面量调 `insert`，需自己处理 i64 类型；这是 input.md AC-2 字面"`Option<u64>`"语义与 rusqlite trait 现实之间的折衷。
3. **idempotent UPDATE 不区分"行存在但未变"与"行不存在"**：两种情况 SQLite 都返回 `rows_affected=0` 且不报错；调用方若需"严格区分"，应先调 `db_read_kc_status` 确认行存在。当前语义与项目内 `update_failure_code` / `update_extraction_status` 一致（input.md 技术约束）。
4. **未走 Tauri command 层端到端集成**：本 task 只交付 DB CRUD 层；`db_read_kc_status` 暴露给前端的 Tauri command（如 `get_kc_status`）由后续 task 落地，本 task 不预先封装。

## 需要 Reviewer 特别关注的地方

1. **`ConversionMetaRow` 兼容性**：核心是 scheduler.rs:1323 字面量构造改动——加了一行 `..Default::default()`。现存调用方只有这一处生产代码 + 两处测试代码（sample_row helper + serde_round_trip）；全部已改妥。**Reviewer 请确认 `#[derive(Default)]` 的字段语义默认值（KC 字段 None / String 字段 ""）不会让"未初始化 ConversionMetaRow"诡异落库**——本 task 防御策略是结构体字段全部必填（即未来若有人忘记初始化某非 KC 字段，仍是字面量缺字段编译错），KC 字段是显式 Optional + 默认 None，语义清晰。

2. **u64 / i64 边界**：`db_conversion_meta_kc_insert` 内的 saturating-cast `n.min(i64::MAX as u64) as i64`。SQLite INTEGER 是 i64，超 i64::MAX 的 u64 值会被截到 i64::MAX。KC 响应体不会突破 i64::MAX（≈ 9.2 EB），但理论边界仍保护住了。**Reviewer 请确认 saturating-cast 的语义（"截断到上限"）是否优于 fail-fast Err，考虑到 NC 现状是"KC 失败时退化为 rule-only"，截断更友好。**

3. **Idempotent UPDATE 语义**：`db_update_kc_fields` 对"asset_id 不存在的行"返回 `Ok(())`（rows_affected=0 视为成功）。这与 task_008 `update_failure_code` 的容忍语义一致（input.md 技术约束第 2 条）。**Reviewer 请确认这是否符合调用方期望——具体场景：enrichment 在 KC 失败后调 `db_update_kc_fields(asset_id, "false", None, None)` 写入"已尝试但失败"，如果此时 extracted_content 行真的不存在（极少见），当前实现会"静默忽略"。如果 Reviewer 偏好显式 Err，可改为 check rows_affected > 0。**

4. **`row_to_meta` KC 字段全 None 兜底（不是 SQL 真值）**：见"已知局限 §1"。Reviewer 若担心"调用方误以为 ConversionMetaRow 中的 KC 字段反映 DB 真值"，可考虑：(a) 加 `#[doc(hidden)]` 标记；(b) 用单独的 `ConversionMetaRowWithKc` 类型分离 read-with-KC 路径；(c) 现状保留——通过文档约定（已加 inline comment）。本 task 采 (c)，YAGNI 原则。

5. **`KcStatusRow` 单独类型 vs 复用 `ExtractedContentRow`**：本 task 引入轻量 `KcStatusRow`（仅 3 字段）而非复用既有 `ExtractedContentRow`。理由：前端 Tauri command 只关心 3 个 KC 字段，复用会拉出 raw_text / structured_md 等大字段，反而浪费 IPC 带宽。**Reviewer 若觉得"两个 Row 类型重复，应统一"，可后续 task 把 `ExtractedContentRow` 扩展为含 KC 字段的"大 Row"，删除 `KcStatusRow`。当前选择是"小 Row 优先"，符合前端契约最小化原则。**
