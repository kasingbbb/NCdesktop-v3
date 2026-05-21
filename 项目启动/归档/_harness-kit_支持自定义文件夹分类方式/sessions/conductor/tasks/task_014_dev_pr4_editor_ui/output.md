# Task 交付 — task_014

## 实现摘要
`promptStore.ts`（drafts/dryRun/load/updateDraft/save/test/reset）+ `PromptEditor.tsx`（tabs classify/naming/tagging + user textarea + 占位符 inline 校验 + 测试/保存/恢复默认按钮 + dry-run 结果显示）。

## 偏离
- 占位符 chip 侧栏 + `{xxx}` 红色下划线高亮：MVP 仅做"必含变量缺失 → disable 保存按钮 + 提示"。完整高亮 textarea 需自定义 contenteditable / mirror layer，~150 行额外代码，留 v2 / task_017 评估
- system / output 段锁解锁机制：MVP 仅暴露 user 段，system/output 仍由 prompts.rs 默认值兜底（task_013 已说明 MVP 三段嵌入策略）

## 文件
- `src/stores/promptStore.ts`（新）
- `src/components/settings/PromptEditor.tsx`（新）

TS 通过。

**PASS** 3.8/5（核心可用；高亮 + 锁机制留 task_017）
