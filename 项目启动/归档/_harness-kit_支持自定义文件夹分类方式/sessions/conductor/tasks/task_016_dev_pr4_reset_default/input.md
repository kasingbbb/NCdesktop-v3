# Task 输入 — task_016_dev_pr4_reset_default

## 目标
F16 三段独立"恢复默认"图标 + 全局"全部恢复默认"二次确认；与 settings KV 删除联动。

## 前置条件
- 依赖 task：task_013 + task_014
- 必须先存在的文件/接口：`reset_prompt` 命令、`PromptEditor`

## 验收标准（AC）
1. 三段每段右上角 reset 图标按钮；点击后立即恢复该 field 默认（无确认，便于快速试错）
2. 全局"全部恢复默认"按钮在 PromptEditor 顶部，二次确认弹窗
3. reset 后 `validated_offline` / `user_skipped_validation` 标志同步清除
4. 视觉：reset 后 textarea 高亮闪一下（200ms）
5. 单测：(a) 单段 reset (b) 全局 reset (c) 标志清除

## 技术约束
- 复用 `reset_prompt` 命令（task_013 已有）
- 不引入新 UI 库

## 参考文件
- task_001 output.md §F16 + ADR-008
- task_013 / task_014 output.md

## 预估影响范围
- 修改：`PromptEditor.tsx`（reset 按钮组 + confirm dialog ~100）、`promptStore.ts`（reset action ~30）

## Reviewer 重点关注
- 单段 reset 不弹确认是否符合用户预期（可改可还原 = 安全）
- 全局 reset 是否触发 dry-run 重做（应不触发，仅清状态）
