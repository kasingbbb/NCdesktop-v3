# Task 输入 — task_013_ux_review

## 目标

按 PRD §9.1 用户视角验收清单逐条做 UX 体验审查，输出 `ux_scorecard.md`。审查方式：手测 + 自动化测试结果交叉验证。任何一条用户视角 AC 不通过即整体 FAIL。

## 前置条件

- 依赖 task：**task_002 ~ 012 全部完成且 PASS**
- 必须先存在的文件/接口：所有前述 task 的 output.md + review_scorecard.md 都已产出

## 验收标准（Acceptance Criteria）

UX 审查需逐条对应 PRD §9.1 的 9 条用户视角验收：

1. **AC-UX-1**：首次启动（`showLearningFeatures=false`），Sidebar 主导航 ≤ 6 项，无任何"复习/课程/技能"字样
2. **AC-UX-2**：开启学习模式后，"学习中心"分组以 200ms 淡入出现，且只包含"今日 + 课程表"两项；不再出现"今天没有课程"占位行
3. **AC-UX-3**：点击"知识"入口，默认进入 KnowledgeHub 的 `concepts` step；URL 显示 `#/knowledge-hub/concepts`
4. **AC-UX-4**：StepNav 显示链条 "素材 N › 概念 N › 知识库 N › 技能 N"；任一项计数为 0 时该 step 不显示计数
5. **AC-UX-5**：TAGS 默认折叠；点击展开后显示过滤输入框；展开状态在重启后保留
6. **AC-UX-6**：Inspector tab 顺序为 详情 / 知识关联 / 时间流；切换瞬间无闪烁
7. **AC-UX-7**：TodayView 在无任务时只显示一句"今日无待处理 + 引导文案"；顶部 0/0/0 计数栏不渲染
8. **AC-UX-8**：悬浮窗在主窗口聚焦时半透明并退到右下；主窗口失焦立刻恢复
9. **AC-UX-9**：暗色模式下所有上述变化的对比度仍 ≥ WCAG AA

## 输出

`sessions/conductor/tasks/task_013_ux_review/ux_scorecard.md`：

```markdown
# UX 体验审查报告 — NoteCapt v1.3 主界面收敛

## 总体评级
- 评级：[PASS | FAIL]
- 综合体验分：[1-5]/5
- 北极星达成度：[非学生首启 60s 是否只看到"工作区 × 知识链条"] — [是/否] + 证据

## 9 条用户验收逐条结果

### AC-UX-1: 首次启动 Sidebar ≤ 6 项
- 状态：[PASS/FAIL]
- 证据：[截图描述 / 文字描述]
- 备注：[如有]

### AC-UX-2 ~ AC-UX-9（同上格式）

## 引擎健康指标
- 北极星视觉验证：[是否符合 §02 设计原则]
- P-04 零数据零信号：[是否所有空状态遵守]
- P-05 沿用令牌不发明：[grep 行内 hex 数量 = 0]

## 发现的体验问题（如有）
| 严重程度 | 问题描述 | 涉及 task | 建议 |
|----------|----------|-----------|------|
| [BLOCKER/MAJOR/MINOR] | ... | ... | ... |

## 回归测试矩阵
| 场景 | 通过 |
|------|------|
| `pnpm test` 全绿 | ✅/❌ |
| `pnpm lint` 0 warning | ✅/❌ |
| `pnpm check` 0 error | ✅/❌ |
| macOS 手测 9 条 UX AC | ✅/❌ |
| dark mode 视觉验证 | ✅/❌ |

## 给 PM 的建议
[1-3 句话总结：能否进入 v1.3 首发？是否需要回填？]
```

## 技术约束

- **必须实测**：每条 AC 至少手测一次（启动 `pnpm tauri:dev` 实际操作 Sidebar / KnowledgeHub / Inspector / Dropzone）
- **暗色模式**：macOS 系统切换到 dark mode 验证对比度
- **不允许仅根据代码推断**：报告必须基于实际运行结果
- **截图保存**：如有问题，截图存到 `sessions/conductor/tasks/task_013_ux_review/screenshots/`

## 参考文件

- `product/prd/notecapt-v1.3-ui_prd_v1.md` §9.1 §9.2
- 所有前序 task 的 output.md（task_002 ~ task_012）

## 预估影响范围

- **新建文件**：
  - `sessions/conductor/tasks/task_013_ux_review/ux_scorecard.md`
  - 可能：截图若干

- **修改文件**：无（纯审查）

---

## Reviewer 重点关注项

本 task 本身就是审查；本 task 的 reviewer = PM。审查者关注：
- 每条 AC 是否有真实证据（不要"看代码推断 PASS"）
- 体验问题严重程度判定是否合理
- 北极星指标是否真的达成（如不达成，PR 必须 FAIL 且回填）
