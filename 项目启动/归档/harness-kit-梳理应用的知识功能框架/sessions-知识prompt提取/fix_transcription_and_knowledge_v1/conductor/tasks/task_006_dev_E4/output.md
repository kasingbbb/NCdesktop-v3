# Task 006 — Dev E4 Output（自动触发链路 F-7）

## 状态
DONE（MVP 事件驱动路径）

## 实现
在 `scheduler.rs::write_derivative_md` 成功尾部：
- `db::project::get_by_id` 解析 `library_id`
- emit `notecapt/concept-extract-requested` 事件，payload：
  - `libraryId`
  - `triggerAssetId`（源件）
  - `triggerDerivedAssetId`（派生 .md）

## 前端契约（待前端承接，不在本 session 范围）
- 监听该事件 → 调用 Tauri command `extract_concepts_for_library(libraryId, force=false)`
- F-8 的 `concepts_extraction_log` 保证去重，不会因多次事件产生重复 LLM 调用

## 安全性
- 事件失败仅 warn，不影响物化主流程
- 前端未接入时，手动按钮仍可通过原有 UI 调 `extract_concepts_for_library`

## 验收
- ✅ 构建通过；事件在成功路径、占位符路径均会发射（占位符也进入 write_derivative_md）
- 🟡 前端监听接入属后续交付；本 session 后端 MVP 完成
