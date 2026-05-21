# Task 输入 — task_015_dev_pr4_dry_run_three_state

## 目标
F15 dry-run 三态容灾：在线必过 / 离线 schema-only + `validated_offline=true` / 用户主动跳过 + 二次确认 + `user_skipped_validation=true`。

## 前置条件
- 依赖 task：task_013 + task_014
- 必须先存在的文件/接口：`dry_run_prompt` 命令骨架、`llm/client.rs`、`PromptEditor`

## 验收标准（AC）
1. `dry_run_prompt(kind, draft_text, sample?)` 完整实现（ADR-004）：
   - 在线：spawn LLM 小请求（10 token 输出），5s 超时
   - 离线：仅做 schema 校验（占位符 + 输出格式段）
   - 返回 `DryRunOutcome { online_ok: bool, schema_ok: bool, offline_only: bool, error: Option<String> }`
2. 前端 `PromptEditor` 增"测试 / 保存"按钮：
   - 状态机 idle → testing → online_pass / offline_pass / fail
   - online_pass → 保存按钮 enable
   - offline_pass → 弹"离线，仅 schema 校验"提示，可选"仍保存"或"等联网"；保存时落 `validated_offline=true`
   - fail → 显示错误，可选"修复" 或 "我知道风险，跳过"；后者二次确认弹窗 + 落 `user_skipped_validation=true`
3. `llmStore` 增 online 状态推断（基于 client 可达 + ping 缓存 30s）
4. 离线判定：`!llmStore.online OR dry_run 5s 超时`
5. 单测后端：(a) 在线快路径 (b) 5s 超时降级 (c) schema 失败 (d) sample 注入

## 技术约束
- dry-run 真实消耗 token（极少）；UI 显示"测试中…"
- 二次确认弹窗复用现有 ConfirmDialog

## 参考文件
- task_001 output.md ADR-004
- task_013 / task_014 output.md
- `llm/client.rs`

## 预估影响范围
- 修改：`commands/prompts.rs`（dry_run 完整实现 ~150）、`PromptEditor.tsx`（状态机 ~150）
- 新建：`llmStore.ts` 增 online 字段或新建（~50）
- 测试：`src-tauri/tests/dry_run.rs`（~80）

## Reviewer 重点关注
- 5s 超时是否正确取消 spawn 任务（避免 zombie request）
- `validated_offline=true` 标志在下次联网时的"补做 dry-run"路径（可留 v2 hook）
- 用户跳过路径的可发现性（不能太隐蔽也不能太鼓励）
