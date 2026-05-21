# Task 输入 — task_009_dev_inspector_knowledge_assoc

## 目标

增强 `KnowledgeAssociationView.tsx`：
- 顶部 toggle "仅显示与当前素材相关" 默认 **开启**
- 当前选中素材关联的概念置顶 + 左侧浅琥珀条 `--concept-linked-stripe`（CSS token 已存在）
- 重复概念条目右侧添加文字按钮 "合并"（`data-merge-id={conceptId}`，**本期 disabled action**，UI 占位即可，点击无功能但有 tooltip "v1.4 合并 modal 待开"）

## 前置条件

- 依赖 task：**无**（与其他 P1 task 并行）
- 必须先存在的文件/接口：
  - `src/components/features/knowledge/KnowledgeAssociationView.tsx`
  - `src/styles/globals.css`（`--concept-linked-stripe` token 已存在）

## 验收标准（Acceptance Criteria）

1. **AC-1**：KnowledgeAssociationView 首次渲染时 toggle 处于开启状态（如 toggle 状态由本地 useState 控制，初始值为 true）
2. **AC-2**：toggle 开启时，列表只显示与当前选中素材关联的概念，且这些概念置顶
3. **AC-3**：置顶的相关概念条目左侧渲染 4px 宽的浅琥珀条（用 `--concept-linked-stripe` token；若 token 不存在，先在 task_012 中补，本 task 内仅写 `background: var(--concept-linked-stripe)`）
4. **AC-4**：toggle 关闭时显示全部概念，无置顶
5. **AC-5**：重复概念条目（duplicateGroup 关联）右侧有"合并"文字按钮，`data-merge-id` 属性正确填充
6. **AC-6**：点击"合并"按钮不抛错（disabled action），有 tooltip "v1.4 合并 modal 待开"
7. **AC-7**：a11y：toggle 是 `<button role="switch" aria-checked={...}>`；合并按钮是 `<button disabled>` 或可见 disabled
8. **AC-8**：单测覆盖：① toggle 默认开启 ② 切换显示行为 ③ 浅琥珀条仅在置顶项 ④ 合并按钮渲染 + data-merge-id
9. **AC-9**：`pnpm check` + `pnpm lint` + `pnpm test` 全绿

## 技术约束

- **不实现合并 modal 实际逻辑**：本期仅 UI 占位；合并按钮 disabled，避免误调
- **toggle 状态**：本期用本地 useState（不进 store），刷新后恢复默认开启
- **重复检测逻辑**：复用现有数据源（如 `useKnowledgeStore(s => s.duplicateGroups)` 之类）；如无，可读现有 selector
- **样式 token**：浅琥珀条用 `--concept-linked-stripe`；如该 token 在 globals.css 不存在，本 task 内**先用 `var(--accent-amber-soft)` fallback**，并在交付中标注，由 task_012 补 token

## 参考文件

- `src/components/features/knowledge/KnowledgeAssociationView.tsx`
- `src/styles/globals.css`（查 --concept-linked-stripe 是否存在）
- `product/prd/notecapt-v1.3-ui_prd_v1.md` §6 IN-03, IN-04

## 预估影响范围

- **修改文件**：
  - `src/components/features/knowledge/KnowledgeAssociationView.tsx`
  - 可能：`src/components/features/knowledge/__tests__/KnowledgeAssociationView.test.tsx`（新建）

- **新建文件**：可能上述测试

---

## Reviewer 重点关注项

- toggle 默认值真的是 true（不要被忘了改回 false）
- 合并按钮 a11y：disabled 状态必须正确，screen reader 不会让用户尝试点击
- 浅琥珀条的 css fallback 处理是否在交付中标注
- 是否引入了任何与 v1.4 modal 相关的实际业务逻辑（应**没有**，UI 占位而已）
