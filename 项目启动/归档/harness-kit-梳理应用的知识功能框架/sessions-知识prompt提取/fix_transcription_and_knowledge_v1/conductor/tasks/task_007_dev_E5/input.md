# Task 007 — Dev E5（F-8 增量 / F-9 user_edited 保护 / F-10 viewpoint 稳定性）

## 目标
`commands/knowledge.rs::extract_concepts_for_library` 生效 `force` 参数、基于 `concepts_extraction_log` 去重、跳过 `user_edited=1` 概念的 name/definition 覆写。

F-10：MVP 不改 schema（仍 delete-rebuild viewpoints），仅保证 prompt 稳定。
