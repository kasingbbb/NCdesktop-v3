# 下一会话恢复 Prompt — custom_prompt_v1

> 重新开机后，把下面 ``` 之间的内容**整段复制粘贴**到新对话框即可，Conductor 会自动接续到 task_010。

---

## 复制以下内容到新对话框

```
继续 NCdesktop "用户自定义 Prompt 功能" Conductor 流水线（custom_prompt_v1 session，L 复杂度）。

【上下文】
- 上一阶段已 commit：ec8ec3c "feat: 用户自定义 Prompt（task 1-9 PASS，仅剩 Architecture Guard）"
- 进度真相来源：/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/内置 prompt 改只定义_harness-kit/sessions/custom_prompt_v1/conductor/progress.md
- 角色定义：/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/内置 prompt 改只定义_harness-kit/roles/conductor/conductor/prompt.md
- 交接契约：/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/内置 prompt 改只定义_harness-kit/core/handoff_contracts.md

【当前状态】
- STATE: UX_REVIEWED（task_009 PASS 阶段性节点）
- 9/10 task 已 PASS（task_001~task_009）
- 仅剩 task_010_architecture_guard 终审待启动

【task_009 UX 评审已识别需修复项（不阻塞但建议修）】
- 3 个 MAJOR：① saving 按钮缺 spinner + 成功 toast；② 错误横条所有子项重复显示同一条错误；③ aria-disabled + aria-label 无障碍属性缺失
- R4 文案落地（方案 B）：para / tagging 折叠头加共享调用副标题（~10 行）
- 详见 sessions/custom_prompt_v1/conductor/tasks/task_009_ux_review/output.md 末尾 "可选 fix list"

【请按以下决策路径推进，并自主执行（遵循 Conductor 自主推进模式，不每步问询）】

第一步：读取 progress.md + task_009 output.md 末尾 fix list，决定路径：
- 路径 A（推荐）：先做 task_007 v2 微改修 3 MAJOR + R4 文案落地（合计 ~50 行），dispatch Dev → Reviewer → 进 task_010
- 路径 B：直接进 task_010（让 Architecture Guard 决定是否要求回头修 MAJOR）

请选择路径 A 推进。若 Dev 估算工作量超出 80 行则降级为路径 B。

第二步：按选定路径，dispatch subagent 执行（用 general-purpose 类型）：
- 路径 A：先 Dev 修复 task_007 v2，Reviewer 验证；再 dispatch task_010_architecture_guard
- 路径 B：直接 dispatch task_010_architecture_guard

task_010 角色定义：roles/conductor/architecture_guard/prompt.md
task_010 input.md 已存在：sessions/custom_prompt_v1/conductor/tasks/task_010_architecture_guard/input.md

第三步：所有 task PASS 后，进入 ACCEPTANCE 状态，按 Conductor prompt § 验收暂停格式向 PM 输出验收摘要。

【关键约束】
- 每次状态变更后立即更新 progress.md
- 不修改 PR-4 半成品（stores/promptStore.ts / components/settings/PromptEditor.tsx / commands/prompts.rs）
- 不重新启动 task_001~009（已 commit 落地）
- 完成所有 task 后再次 commit 并向 PM 报告验收
- subagent 不可看见此对话，brief 必须自包含

开始吧。
```

---

## 当前已完成 task 汇总（供查阅）

| Task | 描述 | 评分 | 状态 |
|------|------|------|------|
| task_001_architect | 技术方案 + 9 task 拆分 + 5 ADR + 9 风险 | — | DONE |
| task_002_dev_backend_data | migration V15 + DB + 4 Tauri commands + AppMode 修复 | 4.9/5 | PASS |
| task_003_dev_backend_validation | prompt_runtime（默认/合并/3 层守卫/占位符/双字节校验）+ classify 拆段 | v1 FIX → v2 5.0/5 | PASS |
| task_004_dev_llm_injection | chat.rs system 合并 + 3 处 LLM assemble + AC-8 字面回归 | 4.80/5 | PASS |
| task_005_dev_frontend_contract | types/user-prompt.ts + tauri-commands 4 函数 + contract test | 5.0/5 | PASS |
| task_006_dev_frontend_store | zustand userPromptStore（loadAll/setDraft/save/reset/resetAll + UTF-8 byteLen） | 5.0/5 | PASS |
| task_007_dev_frontend_ui | PromptCustomizationPanel + SettingsPanel Tab | 4.80/5 | PASS（待 v2 微改 < 50 行） |
| task_008_test_e2e | Rust e2e 20/20 + 33 项手动测试清单 | 4.80/5 | PASS |
| task_009_ux_review | UX 评审（启发式 3.5/5，5 旅程畅通） | PASS | DONE |
| task_010_architecture_guard | Architecture Guard 终审 | — | **待启动** |

## 测试覆盖快照（commit ec8ec3c）

- `cargo test --lib`：342/342 PASS
- `cargo test --test user_prompt_e2e`：20/20 PASS
- `cargo build`：0 error / 5 基线 warning
- 前端 vitest user-prompt 相关：57/57 PASS（contract 14 + store 20 + UI 23）
- `pnpm tsc --noEmit`：0 error
