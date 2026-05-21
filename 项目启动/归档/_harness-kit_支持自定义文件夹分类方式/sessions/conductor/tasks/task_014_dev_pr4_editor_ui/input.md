# Task 输入 — task_014_dev_pr4_editor_ui

## 目标
F13/F14 前端 `PromptEditor`：三段编辑器（system 锁/user/output 锁，"我知道风险"解锁）+ 占位符 chip 侧栏 + 静态校验"必含变量集合 ⊆ 已用变量集合"+ 未识别 `{xxx}` 红色下划线。

## 前置条件
- 依赖 task：task_013
- 必须先存在的文件/接口：`get_prompt` / `save_prompt` 命令、`promptStore`

## 验收标准（AC）
1. 新建 `src/stores/promptStore.ts`：state 含 `kind`、`drafts`、`meta`；action `load(kind)`、`updateDraft(kind, field, text)`、`save(kind)`、`reset(kind, field?)`
2. 新建 `components/settings/PromptEditor.tsx`：tab 切 `kind`，每 tab 三段 textarea
3. system / output 段默认锁（readonly）；右上"我知道风险"按钮解锁
4. 占位符 chip 侧栏：可点击插入到光标
5. 静态校验：每 kind 必含变量集合（如 classify 必含 `{content}`）；编辑区检测未识别 `{xxx}` 红色下划线
6. 必含变量缺失 → 保存按钮 disable + 提示"请在 user 段使用 {content}"
7. 不调 LLM；纯前端校验
8. 单测：(a) `{content}` 缺失 disable (b) 红下划线高亮 (c) 锁/解锁 (d) chip 插入

## 技术约束
- 不引入 Monaco（v2）
- TS 严格

## 参考文件
- PRD §10（默认值 + 必含变量集合）
- task_001 output.md §F13-F14
- task_013 output.md（commands 接口）

## 预估影响范围
- 新建：`promptStore.ts`（~150）、`PromptEditor.tsx`（~300）
- 修改：`SettingsPanel` 接入

## Reviewer 重点关注
- 占位符正则 `\{[a-zA-Z_]+\}` 是否覆盖 PRD §10 所有变量
- "我知道风险"解锁是否需要二次确认弹窗
- 长 prompt 滚动 + 下划线渲染性能
