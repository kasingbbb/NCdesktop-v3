# Task 007 — Dev E5 Output（F-8 / F-9 / F-10）

## 状态
DONE — `cargo check` + `cargo test --lib` 全绿。

## 实现

### F-8 增量抽取
- `fetch_library_assets` 新增返回列 `content_hash`（COALESCE md_ec.content_hash → ec.content_hash）
- 循环前预加载 `logged_pairs = fetch_logged_pairs(conn, library_id)`
- 每个 asset：若 `!force && logged_pairs.contains((asset_id, hash))` → 跳过，`skipped_incremental++`
- 成功抽取后 `concepts_extraction_log::insert(library_id, asset_id, hash)`（UNIQUE 冲突忽略）
- `force=true` 完全绕过日志，重跑所有资产

### F-9 user_edited 保护
- `existing_concepts` 现包含 `(id, user_edited)` 元组
- 命中 existing 分支仅 `append_source_asset`，不改 name/definition（行为保持）
- 注释显式标注"绝不覆写"约束

### F-10 viewpoint 稳定性
- MVP：保留现有 delete-rebuild，不引入 schema UNIQUE 约束
- 依赖 prompt 层稳定（system message 已明确"Return only valid JSON array"）
- 后续 P1：按 `(concept_id, source_asset_id)` 精细化 upsert

## 底线复诵
- ✅ 底线 1（user_edited 概念绝不被自动覆盖）：existing 分支只 append
- ✅ 底线 2（重抽取不删除旧派生）：由 task_003/004 归档机制保证
- N/A 底线 3：该任务不触发物化

## 验收
- ✅ cargo check / cargo test --lib 绿
- 🟡 运行时验证（第二次导入是否跳过；user_edited 概念 name 保持；文案生成无误）→ task_008 回归矩阵
