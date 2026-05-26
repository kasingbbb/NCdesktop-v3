# Conductor 状态机（State Machine）

> 本文件定义 Conductor 在整个 session 生命周期中的合法状态、合法转移、以及每次转移的前置/后置条件。Conductor 必须严格遵守，**任何不在本表内的状态跳转都视为非法**。

---

## 状态枚举

| 状态 | 含义 | 谁在工作 |
|------|------|----------|
| `INIT` | session 刚启动，尚未进入架构阶段 | Conductor 自身 |
| `ARCHITECTURE` | Architect 正在设计技术方案 | Architect Worker |
| `TASK_READY` | 架构方案已完成，task 队列就绪，等待 dispatch | Conductor 自身 |
| `DEVELOPING` | 某个 task 的 Dev 正在写代码 | Dev Worker |
| `REVIEWING` | Reviewer 正在审查刚交付的 task | Reviewer Worker（可选先经 Code Reviewer） |
| `FIXING` | Reviewer 判 FIX，Dev 正在修复 | Dev Worker（Fix 模式） |
| `ARCHITECTURE_REVIEW` | Architecture Guard 正在做全局一致性扫描 | Architecture Guard |
| `UX_REVIEW` | 所有功能完成，UX Evaluator 在审查体验 | UX Evaluator |
| `ACCEPTANCE` | 等待 PM 验收 | PM（人类） |
| `ESCALATED` | 触发升级条件，暂停自动推进 | PM（人类） |
| `DONE` | session 正式完成 | — |

---

## 合法转移表

| 当前状态 | 触发条件 | 下一状态 | 前置检查 | 后置动作 |
|----------|----------|----------|----------|----------|
| `INIT` | 项目复杂度为 S，且 PM 确认跳过 Debate | `ARCHITECTURE` | session_context.md 必填项全部完成 | 启动 Architect Worker |
| `INIT` | 项目复杂度为 M/L，Debate 已产出 PRD | `ARCHITECTURE` | PRD 末尾含"Conductor 桥接摘要" | 启动 Architect Worker |
| `ARCHITECTURE` | Architect 已写入 `task_001_architect/output.md` 且 progress.md 含 task 清单 | `TASK_READY` | 每个 task 都有 input.md 且符合"Architect → Dev"契约 | 在 progress.md 标记 task_001 为 DONE |
| `TASK_READY` | 存在前置条件已满足的待执行 task | `DEVELOPING` | 该 task 的 input.md 验收标准（AC）可验证 | 启动新 Dev Worker（不复用旧 Dev） |
| `TASK_READY` | 所有 task 均 DONE | `ARCHITECTURE_REVIEW`（L 级）或 `UX_REVIEW`（M/S 级，且为 UI 项目）或 `ACCEPTANCE`（非 UI 项目） | — | 触发对应 Worker |
| `DEVELOPING` | Dev 已写入完整 `output.md`（含测试结果、自测矩阵、架构遵守声明、浏览器验证段） | `REVIEWING` | 交付契约必含字段全部存在 | 启动 Reviewer Worker |
| `DEVELOPING` | Dev 自行声明无法完成（缺前置、AC 不可达） | `ESCALATED` | — | 输出升级摘要 |
| `REVIEWING` | Reviewer 判 PASS | `TASK_READY` | review_scorecard.md 存在且综合分 ≥ 3.5 | 把该 task 标记为 DONE，继续调度下一个 |
| `REVIEWING` | Reviewer 判 FIX | `FIXING` | scorecard 含"给 Dev 的修复指引"且每项有验证标准 | 启动 Dev Worker（Fix 模式） |
| `REVIEWING` | Reviewer 判 BLOCKER 或 FIX 已达第 3 轮 | `ESCALATED` | — | 输出升级摘要 |
| `FIXING` | Dev 完成修复并重新交付 | `REVIEWING` | 同 DEVELOPING → REVIEWING 的前置检查 | 同上 |
| `ARCHITECTURE_REVIEW` | Guard 评分 ≥ 4 且无 BLOCKER | `UX_REVIEW`（UI 项目）或 `ACCEPTANCE` | guard report 存在 | 推进 |
| `ARCHITECTURE_REVIEW` | Guard 报告含 BLOCKER | `TASK_READY` | — | 在 progress.md 追加"架构修复 task"，回到调度 |
| `UX_REVIEW` | UX Evaluator 判 PASS | `ACCEPTANCE` | report 存在且无 BLOCKER | 推进 |
| `UX_REVIEW` | UX 含 BLOCKER | `TASK_READY` | — | 追加"UX 修复 task" |
| `ACCEPTANCE` | PM 确认验收 | `DONE` | — | 写入 session 完结摘要 |
| `ACCEPTANCE` | PM 提出修改 | `TASK_READY` | PM 反馈已转化为 task input.md | 追加 task |
| `ESCALATED` | PM 决策完成（继续/回溯/终止） | 任一合法状态 | PM 明确指令 | 记录决策日志 |

---

## 全局规则

1. **每次状态变更后，立即更新 `progress.md`**，并在末尾追加一行状态转移日志：
   ```
   [timestamp] STATE: X → Y | Task: task_id | 原因: [一句话] | 风险: [无/低/中/高]
   ```
2. **新 Worker 启动时必读 `progress.md` + `session_context.md`**，不依赖对话上下文。
3. **同一时刻只能有一个 Worker 在 DEVELOPING/FIXING**（防止并发改同一份代码）。但跨 task 的 Dev 可并行 —— Architect 必须在 Task 拓扑中明确标注 `[可并行: ...]`，Conductor 可以一条消息多 Worker 启动。
4. **Dev 永远是新 Agent**：进入 DEVELOPING 或 FIXING 时禁止复用上一个 task 的 Dev 会话。
5. **任何状态都可被 PM 手动打入 `ESCALATED`**。

---

## 异常计数器（隐性维护，每次状态更新后检查）

| 异常 | 触发 | 处理 |
|------|------|------|
| 同 task 进入 FIXING ≥ 3 次 | 计数 fix_round[task_id] | 转 `ESCALATED` |
| 连续 3 个 task 综合评分 ≤ 3 | 计数 low_score_streak | 转 `ARCHITECTURE_REVIEW` |
| 同类 FIX 问题在连续 2 个 task 出现 | 模式匹配 | 输出"模式警告"给 PM，不强制转移 |
| 一个 session 中 `ESCALATED` ≥ 2 次 | 计数 escalate_count | 建议 PM 重审 PRD scope |

---

## 状态图（参考）

```
INIT
 └─→ ARCHITECTURE
      └─→ TASK_READY ←─────────────────────────┐
           ├─→ DEVELOPING ─→ REVIEWING ─PASS─┘
           │                  ├─FIX→ FIXING ─→ REVIEWING
           │                  └─BLOCKER→ ESCALATED
           └─(全部 DONE)→ ARCHITECTURE_REVIEW ─→ UX_REVIEW ─→ ACCEPTANCE ─→ DONE
                                  └─有 BLOCKER─→ TASK_READY (追加修复 task)
```
