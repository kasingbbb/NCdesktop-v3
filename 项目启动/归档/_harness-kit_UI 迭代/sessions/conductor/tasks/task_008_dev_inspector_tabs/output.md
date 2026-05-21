# Task 交付 — task_008_dev_inspector_tabs

## 实现摘要

`BOTTOM_TABS` 数组重排：`[inspector, knowledge_association, timeline-flow]`。静态信息 tab（详情 + 知识关联）相邻，动态 tab（时间流）沉到末位。同时新建 `Inspector.test.tsx` 含 4 个用例覆盖 AC。

## 修改的文件

| 文件 | 变更 | 说明 |
|---|---|---|
| `src/components/layout/Inspector.tsx` | 修改 | BOTTOM_TABS 顺序 + key union 顺序 |
| `src/components/layout/__tests__/Inspector.test.tsx` | 新建 | 4 用例（tab 顺序 / 默认 mode / 点击切换 2 项） |

## 测试结果

- Inspector.test: 4/4 PASS
- 全量 vitest: 26 fail / 249 pass / 275 total（baseline 锁 ✅）
- Lint 25 errors ✅；TSC 通过 ✅

## 自测验证矩阵

| 场景 | 状态 |
|---|---|
| AC-1 顺序 Inspector / 知识关联 / 时间流 | ✅ |
| AC-3 默认 mode === inspector | ✅ |
| AC-2 rightPanelMode 未持久化 → 重排不需 migrator | ✅（uiStore.partialize 不含此字段） |
| AC-6 role/aria-pressed 保留 | ✅ |
