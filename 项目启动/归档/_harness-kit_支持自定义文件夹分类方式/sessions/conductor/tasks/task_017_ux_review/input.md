# Task 输入 — task_017_ux_review

## 目标
UX Evaluator 审查 Finder 视图（PR-3）+ 设置面板（PR-4 CategoryManager + PromptEditor）+ Bug 修复后的导入流程（PR-2），产出 scorecard。

## 前置条件
- 依赖 task：task_007、task_011、task_012、task_016（PR-2/3/4 全部 DONE）
- 必须先存在的文件/接口：所有 4 PR 已合入主干

## 验收标准（AC）
1. 按 `roles/conductor/ux_evaluator/prompt.md` 协议产出 scorecard
2. 覆盖场景：A 子目录定向导入 / B 自定义课程分类 / C Prompt 个性化 / D 视图切换
3. 每场景至少 3 条观察 + 1 条改进建议
4. 性能 AC 实测（1k 文件首屏 < 300ms / 切类 < 50ms / dry-run 5s 超时）
5. 一致性审查：紫色毛玻璃基线 + 现有动效语言
6. 输出 scorecard 至 `sessions/conductor/tasks/task_017_ux_review/output.md`

## 技术约束
- 不修改代码；仅观察 + 评分

## 参考文件
- `roles/conductor/ux_evaluator/prompt.md`
- PRD §2 核心场景
- task_001 output.md §安全考量 + §风险登记表

## 预估影响范围
- 新建：`task_017_ux_review/output.md`（scorecard）
