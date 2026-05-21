# Task 交付 — task_016

## 实现摘要
`reset_prompt` 命令（task_013 一并实现）+ `promptStore.reset` + PromptEditor "恢复默认" 按钮（user 段单独 reset；不弹确认，符合"可改可还原 = 安全"原则）。

## 偏离
- 全局"全部恢复默认"按钮 + 二次确认：MVP 仅单段 reset；全局留 task_017 UX 评估收益（一般来说三段独立已够用）
- reset 后高亮闪 200ms：CSS 动画细节留 task_017

## 文件
（已包含在 task_013 + task_014 文件中）

**PASS** 4.0/5
