# Review Scorecard — task_001_5_baseline_fix

## 审查思考过程

1. **Task 意图**：建立 v1.3 主体迭代的"可执行 baseline"。最小可解锁修复范围 A1：补全 `uiStore.ts` 中 WorkspaceFolderListView.tsx 和 uiStore.test.ts 引用但缺失的 4 个 state 字段 + 5 个 actions，让 task_006 T4 的 7 个新用例从 fail → pass；并修 `uiStore.persist.integration.test.ts` 中 2 处 `@ts-expect-error` 缺描述的 lint error。
2. **AC 检查结果**：
   - AC-1 ✅ 4 新字段（`editingFolderPath`/`pendingNewFolder`/`pendingRenameIds`/`dragOverPath`）默认值正确（`uiStore.ts:156-159`）
   - AC-2 ✅ 5 actions 行为正确：`startCreating`/`cancelCreating` 直接赋值，`startRenaming`/`finishRename` 通过 `new Set(s.pendingRenameIds)` 拷贝保证新实例，`setDragOverPath` 直接赋值（`uiStore.ts:224-247`）
   - AC-3 ✅ UIStore interface 在 `uiStore.ts:91-95, 118-123` 含全部 9 新成员，tsc 通过（output 已显示 `pnpm check` 无输出）
   - AC-4 ✅ 4 新字段不进 partialize（`uiStore.ts:254-258` 只含 `activeSidebarSection`/`todayLastTab`/`tagsExpanded`；`uiStore.test:355-367` 已断言 4 字段均不在 LS）
   - AC-5 ✅ uiStore.test 36/36 全绿
   - AC-6 ✅ `uiStore.persist.integration.test.ts:140, 149, 158` 三处 `@ts-expect-error` 均已加描述（"模拟运行时误传旧值"等）
   - AC-7 ✅ eslint 该文件 0 error（lint errors 总数 27→25，正好减 2）
   - AC-8 ✅ stores/__tests__/ 62/62 全绿
   - AC-9 ✅ tsc 通过
   - AC-10 ✅ 全量 vitest 37 fail（≤51 上限），意外超额改善 +14（WorkspaceFolderListView 连锁 fail 被间接修复）
   - AC-11 ✅ lint 25 errors（=25 上限）
   - AC-12 ✅ output.md 末尾"既有 Broken 快照"清晰列出 37 个 fail 的分布与 25 个 lint errors 的类别

3. **关键发现**：
   - **Set 不变性正确**：`startRenaming`/`finishRename` 均使用 `new Set(s.pendingRenameIds)` 拷贝，`uiStore.test:325` `expect(after).not.toBe(before)` 通过验证
   - **finishRename 幂等正确**：`Set.delete("ghost")` 返回 false 但不抛错，符合 JS spec；`uiStore.test:343-346` 已断言不抛错
   - **partialize 严格保护**：`uiStore.test:355-367` 断言 4 新字段在 LS state 中均 `not.toHaveProperty`，绿色通过
   - **意外超额收益**：补完 selector 链路后，WorkspaceFolderListView 因 `useUIStore` 返回 undefined 的连锁 fail 被间接解决（+14 个），符合 PM 的 A1 思路

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 20% | 5 | 12 条 AC 全部满足；7 个 task_006 T4 新用例 100% PASS；连锁收益 +14 |
| 安全性 | 5% | 5 | 纯前端 store 字段补全，无安全敏感面 |
| 代码质量 | 20% | 5 | 字段命名风格与现有 setter 对齐；setter 实现简洁；Set 拷贝模式正确；无引入计划外依赖 |
| 测试覆盖 | 20% | 5 | 7 个新用例覆盖默认值/单字段 setter/Set 不变性/幂等/partialize 排除，正常+边界+异常均测 |
| 架构一致性 | 15% | 5 | 严格只动既有文件、严格只追加新成员到 interface 末尾、不动现有 partialize/migrate 结构 |
| 可维护性 | 10% | 5 | 字段聚合到 interface 末尾且加注释（`// ── 工作区文件夹列表编辑态（task_006 T4...）`）；幂等/不变性靠 JS 内建语义而非自定义复杂逻辑 |
| UX 体感 | 10% | 5 | 本 task 仅 store 层，UX 影响通过解锁后续 task_006 间接实现，无负面影响 |

**综合分：5.00/5**（加权计算：0.20×5 + 0.05×5 + 0.20×5 + 0.20×5 + 0.15×5 + 0.10×5 + 0.10×5 = 5.00）

## 总体判断

- [x] **PASS**

## 问题列表

### BLOCKER（必须修复）

无。

### MAJOR（强烈建议修复）

无。

### MINOR（可选）

1. **output.md 表述瑕疵**：在 `自测验证矩阵` 中第 7 行写"5 新字段不进 partialize 白名单"，但实际本 task 引入的是 4 新字段（第 5 字段 `tagsExpanded` 由 task_002 引入且**应该**进 partialize）。代码与 test 实际行为正确，仅是文档措辞不严谨。**位置**：`output.md` 自测验证矩阵；**修复**：建议改为"4 新字段（编辑态）不进 partialize 白名单"。**优先级**：低，不影响功能。

## 给 Dev 的修复指引

无需修复。本 task 直接 PASS，进入 task_002 review。

---

**Reviewer 备注**：
- 此 task 是 v1.3 主体迭代的 unblocker，超额完成 +14 测试改善
- baseline 锁更新：**vitest fail ≤ 37 / lint errors ≤ 25 / tsc 通过**（后续 task_002~013 严格不可超过）
- session_context "状态门控/零数据零信号/令牌使用"等领域审查项与本 task 无关（本 task 不涉及 UI 改动）
