# Task 输入 — task_008_dev_scheduler_fallback

## 目标
改造 `scheduler.rs` 主循环：MarkItDown 失败/空输出 → 自动调用 `get_fallback_extractor_for(mime)` 重试 → 都失败 → `materialize_placeholder`。每一条路径都写一行 `conversion_meta`。同时拆分 `write_placeholder_md` 让 placeholder 不再推进 `derivative_version`。

## 前置条件
- 依赖 task：task_002 / task_003 / task_004 / task_005 / task_006 / task_007（**全部前置**）

## 验收标准（AC）
1. **AC-1**：scheduler 主循环（`start()` 内的提取分支）按以下伪代码组织：
   ```
   primary = get_extractor_for(mime)
   result_primary = primary.extract(...)
   if result_primary.is_ok && quality > 0:
       materialize_md(...); upsert conversion_meta(fallback_used=false)
   else:
       upsert conversion_meta(primary, fallback_used=false, error_class=...)
       fallback = get_fallback_extractor_for(mime) (排除 primary)
       result_fb = fallback.extract(...)
       if result_fb.is_ok && quality > 0:
           materialize_md(...); upsert conversion_meta(fallback_used=true)
       else:
           materialize_placeholder(...); upsert conversion_meta(error_class=...)
   ```
2. **AC-2**：拆分出 `write_placeholder_md(app, source_asset, body)`，**不调用** `set_derivative_version`、**不归档**、**不写 `extracted_content`**（避免 placeholder 把 status 推到 extracted 后真转换被跳过——见架构方案 §九 R3）。
3. **AC-3**：真成功路径写入 `extracted_content.status = 'extracted'` + 推进 `derivative_version`；placeholder 写入 `extracted_content.status = 'failed'`（含错误码），不推进版本号。
4. **AC-4**：集成测试覆盖：
   - T-1 PDF（文字型）→ markitdown 成功 → conversion_meta 1 行，fallback_used=false
   - T-2 PDF（伪装失败：临时把 markitdown 改名）→ fallback 到 pdf_text → conversion_meta 2 行（1 失败 1 成功）
   - T-3 损坏 PDF → 两者都失败 → 1 个 placeholder + conversion_meta 2 行（皆失败）
   - T-4 在 T-3 之后重新替换为有效 PDF 并 retrigger → placeholder 被覆盖为真成功 .md，derivative_version 从 0 → 1
   - T-5 重复跑 T-1 两次 → 工作区只有一个 .md，无 v1/v2 副本（验证幂等）
5. **AC-5**：手测 fallback 提示：拖入 PDF 在 markitdown 不可用时，前端 `extractor_type` 显示为 `pdf_text`（task_010 会展示 fallback 角标，本 task 验收只要后端字段正确）。
6. **AC-6**：scheduler 内 `compute_sha256` 改用 `conversion::file_sha256` 或保留为 wrapper；不允许两个并存的本地实现。

## 技术约束
- placeholder 路径**禁止**调用 `set_derivative_version`；**禁止**调用 `extracted_content` 的 upsert 把 status 推到 extracted。
- fallback 选择必须排除 primary（否则 markitdown→markitdown 死循环）。
- `conversion_meta` 写入**不允许**因为标签/版本号失败而被跳过；写入失败仅 `log::warn!`。
- 不允许 `unwrap()`/`expect()`。

## 参考文件
- `src-tauri/src/extraction/scheduler.rs:141-340, 588-815`
- `src-tauri/src/extraction/extractors/mod.rs::get_fallback_extractor_for`
- `src-tauri/src/db/conversion_meta.rs`（task_006 产出）
- `src-tauri/src/extraction/conversion.rs`（task_005 产出）
- 架构方案 §三 ADR-003 / ADR-006、§九 R2/R3

## 预估影响范围
- 新建文件：无
- 修改文件：
  - `src-tauri/src/extraction/scheduler.rs`（主循环 + write_placeholder_md 拆分）
  - `src-tauri/src/extraction/extractors/mod.rs`（可选：暴露"排除指定 name 的 fallback"辅助函数）
  - `src-tauri/src/extraction/mod.rs`（**关键**：取消 `// pub mod scheduler;` 注释——见下方 M-1）

## Conductor 追加（M-1 跨 task 待办，本 task 关闭点）
- **背景**：`src/extraction/mod.rs:4` 当前是 `// pub mod scheduler;`（注释状态）。这是 task_002 Reviewer 发现的——它意味着自 task_002 起 scheduler.rs 实际**完全不参与编译**，cargo check 0 error 是因为整个模块缺席。
- **本 task 的硬性额外动作**：
  1. 取消 `src/extraction/mod.rs:4` 注释（必须）。
  2. 同步检查 `src/lib.rs` 是否有对 scheduler 的引用被屏蔽；若有也一并恢复。
  3. 取消注释后第一次 `cargo check`，**预期会暴露大量符号引用错误**——这正是本 task 要修的 fallback 编排链路；你需要在本 task 范围内把这些错误清零（如果有非本 task 范围的错误，明确登记为 ESCALATE）。
  4. 在 output.md 的"实现摘要"开头**显式说明**：本 task 同时关闭了 M-1 跨 task 待办；scheduler 模块已重新参与编译。
- **AC 追加（AC-7）**：cargo check 在取消注释 + 本 task 改完后必须 **0 error**（task_002/003/004/005/006/007 应已铺好所有底层符号；如仍有缺失说明前序 task 漏了什么，需 ESCALATE 不要在本 task 私自补）。
