# Task 输入 — task_013_dev_pr4_prompts_commands

## 目标
F12 后端：`commands/prompts.rs` 四命令（get / save / dry_run / reset）+ `llm/prompts.rs` 引入 default-override merge 层；持久化复用 `settings` KV，命名规范见 ADR-008。

## 前置条件
- 依赖 task：无（独立可与 PR-1 并行；仅依赖 V1 settings KV）
- 必须先存在的文件/接口：`db/migration.rs::V1` 的 settings 表、`llm/prompts.rs::classify_prompt` 等

## 验收标准（AC）
1. 新建 `src-tauri/src/commands/prompts.rs`：`get_prompt(kind)`、`save_prompt(kind, field, text)`、`dry_run_prompt(...)`（占位实现，task_015 完整）、`reset_prompt(kind, field?)`
2. `kind ∈ {classify, naming, tagging}`；`field ∈ {system, user, output}`
3. `llm/prompts.rs` 增 `merge_prompt(default: &str, override_text: Option<&str>) -> String`；渲染时调用
4. 三段拆分实现：MVP 仍以"嵌入 classify_prompt"形式存在，但 prompts.rs 暴露 `get_segment(kind, field)` 与 `set_segment_override(kind, field, text)`，渲染时按 segment 合并回完整 prompt（PRD §10.205 实现说明）
5. settings KV 命名：`prompt.override.{kind}.{field}` 等（ADR-008）
6. `save_prompt` 调用基础静态校验（占位符存在）；完整校验在 task_014 / 015
7. `reset_prompt` 删除对应 keys；前端读取后渲染 default
8. 单测：(a) get 返回 default+override (b) save 落盘正确 (c) reset 清干净 (d) merge 优先用 override

## 技术约束
- 用 `str::replace` 注入变量（禁 format!）
- KV value 都是 JSON 字符串

## 参考文件
- task_001 output.md §API 设计 + ADR-008
- `src-tauri/src/llm/prompts.rs`（默认值 + classify_prompt）
- `db/migration.rs:646`（settings 表）
- PRD §10.205 实现说明

## 预估影响范围
- 新建：`commands/prompts.rs`（~250）
- 修改：`llm/prompts.rs`（+merge layer ~80）、`lib.rs`、`tauri-commands.ts`
- 测试：`src-tauri/tests/prompts.rs`（~100）

## Reviewer 重点关注
- merge 层是否真的在所有调用点生效（grep `classify_prompt(`）
- KV key 命名是否易于后续 version 字段扩展
- segment 拆分映射是否覆盖 PRD §10 三段
