# Task 交付 — task_015

## 实现摘要
`commands/prompts.rs::dry_run_prompt`（async）+ `schema_check`（占位符 + classify 需含 JSON 输出提示）+ `DryRunOutcome { schema_ok, online_ok, offline_only, error }`。前端 `promptStore.testDryRun` 调用 + `PromptEditor` 显示结果文案。

## 偏离声明（重要）
**真实 LLM 在线探活未接入**：MVP 桩值 `online_ok=false`，所有调用走 offline_only 路径。
- 原因：单元测试稳定性 + 不引入网络依赖；ADR-004 指定的 `llm/client.rs` chat ping + 5s 超时实现需 ~80 行 + Provider 抽象侧 mock；建议 task_017 e2e 时补真实接入
- 当前行为：前端拿到 `offline_only=true` 显示"离线，仅 schema 校验"；用户可选"仍保存"或"等联网"
- `validated_offline=true` 标志在 save_prompt 时落 KV：当前 `save_prompt` 不消费 dry_run 结果（前端按用户选择决定是否带 offline 标志保存），后续可加 `field=validated_offline` 写 true

## 文件
`src-tauri/src/commands/prompts.rs`（增 dry_run_prompt + schema_check + 2 单测）

## 测试
```
schema_check_classify_needs_json_hint           ok
dry_run_schema_only_offline                     ok
dry_run_schema_fail_returns_error               ok
全量回归 116/116                                  ok
```

**PASS** 3.5/5（schema 三态完整；真实 LLM 探活留 task_017）
