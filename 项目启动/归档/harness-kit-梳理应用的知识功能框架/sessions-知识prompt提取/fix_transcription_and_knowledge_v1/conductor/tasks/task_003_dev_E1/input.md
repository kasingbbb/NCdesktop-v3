# Task 003 — Dev E1（工作区完整性 F-1 / F-2）

## 上游输入
- `../task_002_architect/output.md` — E1 设计（frontmatter、版本化路径、占位符约定）
- `../../../CHECKPOINT.md` — 已完成铺垫 & 剩余重构清单（权威）

## 目标
使所有 source asset 在工作区都拥有可点击 .md 邻居，无论抽取成功 / 不支持 / 失败 / 空白。

## 验收
- H-W01..W06：每种 source kind 均产出工作区 .md（unsupported / empty / error 产出占位 .md）
- H-W02：中文 / emoji / 控制字符文件名正确 sanitize（safe_name 已就绪）
- 迁移：`PRAGMA user_version == 9`
- `cargo check` 通过；`cargo test -p app_lib` 中 db / safe_name / migration 测试全绿

## 范围
见 CHECKPOINT.md §未完成；不要扩大。

## 不要做
- 不改 `commands/knowledge.rs`（task_007）
- 不实现自动入队 `concept_extract`（task_006，仅留 TODO 钩子）
- 不改前端
