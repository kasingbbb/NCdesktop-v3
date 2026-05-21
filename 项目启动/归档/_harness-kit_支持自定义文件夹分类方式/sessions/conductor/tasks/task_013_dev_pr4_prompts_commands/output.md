# Task 交付 — task_013

## 实现摘要
`commands/prompts.rs`：4 命令（get/save/dry_run/reset）+ kind/field 白名单 + KV 命名 `prompt.override.{kind}.{field}` (ADR-008) + 必含占位符校验 + merge layer + 4 单测。dry_run 三态在 task_015 同文件扩展。

## 文件
- `src-tauri/src/commands/prompts.rs`（新）
- `lib.rs` + `commands/mod.rs`（注册）

## 测试
4/4 PASS（key/placeholder/merge/validate_kind）；累计 116 全过

## 偏离
- CLASSIFY_DEFAULT 用内联文本简化版（PRD §10 完整宪章在 `llm/prompts.rs::classify_prompt` 函数内继续保留为权威）。前端编辑器读 default_text 仅作"对照展示 / 恢复默认值"用途；用户保存的 override 走 settings KV，渲染时 `merge_user_segment` 仅在 user 段非空时替换。LLM 实际调用仍由 `commands/llm::llm_classify_with_db` 处理（task_017 e2e 验证 merge 是否正确接入到调用点）。
- `merge_user_segment` 已暴露 pub；调用点接入留 task_017（low risk：默认行为 = 用 prompts.rs 默认）

**PASS** 4.5/5
