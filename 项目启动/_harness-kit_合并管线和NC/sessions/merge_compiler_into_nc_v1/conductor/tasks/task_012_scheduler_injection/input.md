# Task 输入 — task_012_scheduler_injection

## 目标
在 `scheduler.rs::save_and_materialize` 中注入 KC enrichment step（在 `materialize_md` 前），按 ResolvedEnrichment 落地最终 MD、更新 extracted_content KC 字段、写 conversion_meta 失败码。

## 前置条件
- 依赖 task：task_011（kc::enrichment）、task_002（DB schema v18 已就绪）
- 必须先存在的文件/接口：
  - `src-tauri/src/kc/enrichment.rs::enrich / resolve_outcome / ResolvedEnrichment`
  - `db v18` 已迁移

## 验收标准（Acceptance Criteria）
1. **AC-1**：修改 `scheduler.rs::save_and_materialize`（1272 行附近）注入点：
   - 在 `if source_asset_should_materialize(asset)` 内部、`if source_asset_is_markdown(asset)` 之前
   - 仅对**非 markdown 原件**走 KC enrich（markdown 原件直接走 materialize_source_markdown）
   - 调用顺序：
     ```rust
     let kc_outcome = kc::enrichment::enrich(app, asset, &r.structured_md).await;
     let frontmatter_writer = |meta: &KcMeta| { task_013::build_kc_frontmatter(asset, r, meta) };  // 由 task_013 提供
     let resolved = kc::enrichment::resolve_outcome(r, kc_outcome, frontmatter_writer);
     // 更新 extracted_content KC 字段
     task_015::db_update_kc_fields(app, &task.asset_id, &resolved.kc_enriched, ...);
     // 写 final_md 而不是 r.structured_md
     materialize_md(app, asset, &resolved.final_md, r.quality_level, &resolved.extractor_type);
     // 写 conversion_meta 含 failure_code（如有）
     if let Some(fc) = resolved.failure_code_for_meta { write_kc_conversion_meta(..., fc, ...) }
     ```
2. **AC-2**：`db_save_extraction_result` 调用不变（基础字段写入仍按原逻辑），KC 字段由后续 `db_update_kc_fields` 单独 UPDATE
3. **AC-3**：现有 markitdown 测试用例（task_008 中的 `decide_next_step` 等）保持 PASS
4. **AC-4**：新增 scheduler 单元测试：
   - `save_and_materialize_with_kc_success_writes_enhanced_md`
   - `save_and_materialize_with_kc_disabled_falls_back_to_raw_md`
   - `save_and_materialize_with_kc_partial_writes_partial_md_and_meta`
   - `save_and_materialize_markdown_asset_skips_kc`（.md 原件不走 KC）
5. **AC-5**：emit 事件按 task_011 AC-1 步骤 5 完成（不重复，由 enrich 内部 emit）

## 技术约束
- 不污染 scheduler.rs:110-479 主循环（注入点限定在 save_and_materialize 内）
- 注入逻辑 ≤ 25 行（reviewer 重点）
- 主链路 status 必须保持 `'extracted'`（即便 KC 失败也是 extracted）
- 历史行为（无 KC 的纯 markitdown 路径）必须**完全保留**——通过 `kcEnabled=false` 验证
- `materialize_source_markdown`（.md 原件直接物化）路径不走 KC（PRD §3.1 范围声明）

## 参考文件
- Architect output.md §"ADR-003 KC enrichment 注入点"
- `src-tauri/src/extraction/scheduler.rs:1272-1310` save_and_materialize 现状
- `src-tauri/src/extraction/scheduler.rs:1297` `source_asset_should_materialize` + `source_asset_is_markdown` 分支
- task_011 input.md（enrich 与 resolve_outcome 签名）

## 预估影响范围
- 新建文件：无
- 修改文件：
  - `src-tauri/src/extraction/scheduler.rs`：save_and_materialize 注入 ~25 行 + 新增 `write_kc_conversion_meta` helper + 测试

## Reviewer 重点关注项
- **scheduler.rs 主循环代码量**（110-479 行）是否被污染——必须只动 1272-1310 区段
- **错误传播**：KC 失败时函数仍返回 ()，不上抛 panic
- **async 边界**：save_and_materialize 当前是 sync 还是 async？如果是 sync 需要让它接受 `tokio::runtime::Handle::current().block_on()` 包装；建议改为 async fn
- **markdown 原件不走 KC** 的逻辑分支正确性（防御性测试）
- **conversion_meta** 写两次的语义（task_012 与 task_015 边界）

## 复杂度
M（1d 工作量，~600 行；最大复杂度在与 task_013 / task_015 的边界协调）
