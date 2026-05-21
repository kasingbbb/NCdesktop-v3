# Conductor Progress

## 当前状态

- **STATE**: **COMPLETED**（v1.3 主界面收敛 12 个 task 全部交付；UX 审查产出 CONDITIONAL PASS）
- **Session**: notecapt-v1.3-ui
- **当前 Task**: task_013 UX 审查已交付
- **更新时间**: 2026-05-12

## 项目元数据

- **项目名称**: NoteCapt Desktop · 主界面收敛 v1.3
- **复杂度**: L
- **PRD 路径**: `product/prd/notecapt-v1.3-ui_prd_v1.md`
- **Debate**: 跳过

## 已完成 Tasks

- [x] **bootstrap** — Session 目录创建、session_context.md 填写、PRD 落地
- [x] **task_001_architect** — Architect 技术方案 + 5 个 ADR + 12 个 task input.md
- [x] **baseline 摸底** — 发现 baseline broken：58 test failures + 27 lint errors
- [x] **task_001_5_baseline_fix** — uiStore 补 9 成员 + persist.integration.test 加描述。结果：vitest 58→37，lint 27→25
- [x] **task_002_dev_uistore_tags_expanded** — uiStore 加 tagsExpanded + setter + partialize + migrate；新增 5 用例。结果：uiStore.test 41/41，全量 vitest 37 fail（持平）
- [x] **task_006_dev_sidebar_tags_collapse** — TagTree 默认折叠 + 过滤输入；ADR-006 决断按 PRD 走"过滤"模式
- [x] **task_003_dev_sidebar_search_removal** — Sidebar 删 Search 项；SidebarFooter dot/badge 替代 TF SidebarItem
- [x] **task_004_dev_sidebar_knowledge_merge** — Sidebar 合并"知识中心" + hub badge（ADR-007 选项 C "全 0 不渲染"）
- [x] **task_005_dev_sidebar_learning_center** — 学习中心 SidebarSection（今日+课程表，sidebar-learning-fade-in 200ms）
- [x] **task_007_dev_hub_chain** — DEFAULT_HUB_STEP=concepts；StepNav 链条+counts+chevron+data-step；同步更新 useHubHashRoute.test 5 用例
- [x] **task_008_dev_inspector_tabs** — Inspector BOTTOM_TABS 重排 [详情/知识关联/时间流]；新建 Inspector.test 4 用例
- [x] **task_009_dev_inspector_knowledge_assoc** — KnowledgeAssociationView 加 toggle UI 占位（IN-03/04 真实功能延后 v1.4）
- [x] **task_010_dev_empty_state** — TodayView 去 🎉 + stats 全 0 不渲染 + 文案中性
- [x] **task_011_dev_dropzone_focus** — DropzoneApp 监听 onFocusChanged + opacity 0.45（退避位置延后 v1.4）
- [x] **task_012_dev_tokens_consolidate** — Sidebar-active 改冷蓝 token + 新增 hub-count/accent-amber/duration 三档 token
- [x] **task_013_ux_review** — UX 审查报告 CONDITIONAL PASS：7/9 完全通过、1 PARTIAL（Dropzone 位置）、1 ADVISORY（dark WCAG 需手测）

## v1.3 最终交付总结

- **PR 总数**：6 PR（PR-A ~ PR-F），全部完成
- **新增/修改用例**：~38 个新增 vitest 用例（uiStore 5 + TagTree 5 + Sidebar 6 + KnowledgeHubView 3 + useHubHashRoute 5 修复 + Inspector 4 + DropzoneApp 0 mock 修复 + SidebarFooter 5 修复 + 其它）
- **Baseline 健康度演进**：vitest 失败 58 → **26**（净改善 -32）；lint errors 27 → **25**（净改善 -2）；tsc 持续通过
- **关键 ADR**：7 个（ADR-001~005 在 Architect 阶段；ADR-006 task_006 TagTree UX 取舍；ADR-007 task_004 Sidebar PR-B 综合决断）
- **北极星达成**：✅ 非学生首启 60s 内只见"工作区 × 知识链条"
- **明确延后到 v1.4 的子项**：合并按钮真实功能 / Dropzone 退避位置 / EmptyState 通用化 / "今日" badge 数字 / dark WCAG 手测

## Baseline 快照（v1.3 后续 task 的 DoD 锁，**已根据 task_006 收益更新**）

> 后续 task_003~013 验收口径：以下指标**不可恶化**：
> - 全量 vitest 失败数 ≤ **33**（更严格）
> - 全量 lint errors 数 ≤ **25**
> - tsc 必须通过

## 既有 Broken 清单（A1 不修，已与 PM 同步）

**Test failures（51 个，扣掉 task_001.5 修复的 7 个 = baseline 58 − 7 = 51）**：
- `WorkspaceFolderListView.test.tsx` (~26) — T5b 工作区文件夹编辑 UI 实现未做
- `SettingsPanel.test.tsx` (9) — 学习功能 tab + rAF 时序契约未实现
- `TagTree.test.tsx` (7) — task_008 F-P0-11 "前 20 + 更多 (N)" 折叠机制未实现
- `ContentArea.test.tsx` (2) — 学习模式渲染防御未实现
- 其它（~7）— App.test、其他集成

**Lint errors（25 个，扣掉 task_001.5 修复 2 个）**：
- React 19 严格规则错（~10）：setState in effect、ref during render、impure function during render
- TypeScript no-explicit-any（~5，全在 .test 文件中）
- fast-refresh only-export-components (2)
- react/no-danger 规则未定义 (1)
- 其它（~7）

> **影响后续 task 的 AC 口径调整**：v1.3 task_002~013 input.md 的 AC "pnpm test/lint 全绿" 改为 **"新增用例 PASS + 不引入新的 baseline 失败"**（任一 task 完成后跑 `pnpm test/lint`，失败数严格 ≤ task_001.5 完成后的快照）

## 待执行 Task 队列（task_001.5 完成后）

### PHASE 0 — 主界面收敛（P0，首发阻塞）

- [ ] task_002_dev_uistore_tags_expanded (SB-07)
- [ ] task_003_dev_sidebar_search_removal (SB-01, SB-06)
- [ ] task_004_dev_sidebar_knowledge_merge (SB-02, SB-03)
- [ ] task_005_dev_sidebar_learning_center (SB-04)
- [ ] task_006_dev_sidebar_tags_collapse (SB-05) **← 注意：与 TagTree task_008 历史契约冲突**
- [ ] task_007_dev_hub_chain (KH-01 ~ KH-05)

### PHASE 1 — 细节体感（P1）

- [ ] task_008_dev_inspector_tabs (IN-01, IN-02)
- [ ] task_009_dev_inspector_knowledge_assoc (IN-03, IN-04)
- [ ] task_010_dev_empty_state (ES-01 ~ ES-04)
- [ ] task_011_dev_dropzone_focus (DZ-01 ~ DZ-04)

### PHASE 2 — 视觉打磨（P2）

- [ ] task_012_dev_tokens_consolidate (TK-01 ~ TK-04)

### 收尾

- [ ] task_013_ux_review

## 已知问题 / Blockers

- **task_006 与历史 TagTree 契约冲突**：PRD §4.2 SB-05 要求"展开后顶部带过滤输入框"，但现有 TagTree.test.tsx (task_008 F-P0-11) 契约要求"展开后前 20 + 更多 (N) + showAll 切换"。两套 UX 模式不同，需在 task_006 启动前增补 ADR-006 决断（建议：以 v1.3 PRD 为准 = 过滤输入；TagTree.test.tsx 中 task_008 用例归入"既有 broken"清单，本期不修）

## 关键决策记录

- **[2026-05-12 boot]** 跳过 Debate
- **[2026-05-12 boot]** 复杂度 L
- **[2026-05-12 boot]** ADR-001~005（见 task_001_architect/output.md）
- **[2026-05-12 task_001.5]** PM 选定 A1 最小可解锁修复范围；task_002~013 验收口径改为"新增用例 PASS + 不引入新 baseline 失败"
- **[2026-05-12 task_001.5]** ADR-006 待补：task_006 TagTree 折叠 UX 取舍（过滤输入 vs 前 20+更多）

## 状态转移日志

- **[2026-05-12 boot]** STATE: ∅ → INIT
- **[2026-05-12 boot]** STATE: INIT → ARCHITECTURE
- **[2026-05-12 boot]** STATE: ARCHITECTURE → ARCHITECTURE_DELIVERED
- **[2026-05-12 PM 验收]** STATE: ARCHITECTURE_DELIVERED → TASK_START | PM 同意 5 ADR + 12 task 拆分
- **[2026-05-12 摸底]** STATE: TASK_START → ESCALATE | baseline broken（58 test fail + 27 lint error） | 风险: 高
- **[2026-05-12 PM 决策]** STATE: ESCALATE → DEVELOPING | PM 选 A1 最小可解锁；新增 task_001.5
