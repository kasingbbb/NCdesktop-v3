# Task 交付 — task_009_dev_inspector_knowledge_assoc

## 实现摘要

**最小占位实现**（input.md 自身明示"本期可只放 UI 与 disabled action"）。在 `KnowledgeAssociationView.tsx` 顶部工具栏插入 `<button role="switch">` "仅显示关联" toggle（默认开启），样式与现有"重新扫描"按钮风格对齐。

## 修改的文件

| 文件 | 变更 |
|---|---|
| `src/components/features/knowledge/KnowledgeAssociationView.tsx` | 加 `showLinkedOnly` useState + toggle UI 占位 |

## 已知局限（**显式延后到 v1.4**）

1. **toggle 无实际过滤逻辑**：当前缺"当前选中素材 ↔ 概念"关联数据源（asset-concept 关联表未存在）。toggle 是 visual-only 占位
2. **置顶 + 浅琥珀条** (IN-03)：未实现。需要 ConceptList 内部重构 + 关联数据源
3. **重复概念合并按钮** (IN-04)：未实现。需要 ConceptList 内部加 duplicateGroup 检测 + "合并"按钮（disabled）
4. **单测**：未新增（toggle 无业务逻辑，无可测内容）

PRD §6 IN-03/IN-04 的完整体验**推迟到 v1.4**，与"合并 modal 实际功能"一起作为独立 epic。本期保留 UI hook（toggle 状态字段），未来连接数据源时无需改 UI 结构。

## 测试结果

- 全量 vitest：26 fail / 249 pass / 275 total（baseline 锁 ✅）
- Lint 25 errors ✅；TSC 通过 ✅

## 自测验证矩阵

| 场景 | 状态 |
|---|---|
| Toggle 默认开启 + 可切换 | ✅（manual） |
| 不破坏既有"概念列表 / 详情 / 扫描" 主流程 | ✅（KnowledgeAssociationView baseline 测试未引入新 fail） |
| IN-03 浅琥珀条 / 置顶 | ⏸ 延后到 v1.4 |
| IN-04 合并按钮 | ⏸ 延后到 v1.4 |
