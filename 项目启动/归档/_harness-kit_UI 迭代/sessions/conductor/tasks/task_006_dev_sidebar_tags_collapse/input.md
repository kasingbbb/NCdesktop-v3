# Task 输入 — task_006_dev_sidebar_tags_collapse

## 目标

将 `TagTree` 默认改为折叠状态（由 `uiStore.tagsExpanded` 控制）。保留 section header + 标签计数。点击 section header 切换展开/折叠。展开后 header 下方出现过滤输入框（placeholder="过滤标签"），实时筛选标签。

## 前置条件

- 依赖 task：**task_002**（需要 `useUIStore.tagsExpanded` 字段）
- 必须先存在的文件/接口：
  - `src/components/features/TagTree.tsx`
  - `src/stores/uiStore.ts` （已含 `tagsExpanded` 字段）
  - 标签数据源（既有 store / hook，从 TagTree 现有代码中找）

## 验收标准（Acceptance Criteria）

1. **AC-1**：TagTree 首次渲染（`tagsExpanded === false`）时，只显示 section header（含"TAGS" 标签 + 总计数 + 右侧 chevron `›`），下方**不渲染**标签列表与过滤框
2. **AC-2**：点击 section header 后，`useUIStore.getState().tagsExpanded === true`，下方渲染过滤输入框 + 标签列表（chevron 变 `⌄` 或类似展开标识）
3. **AC-3**：再次点击 header 折叠回去，`tagsExpanded === false`
4. **AC-4**：展开状态下，过滤输入框 `placeholder="过滤标签"`，输入文本后实时筛选标签列表（按 label includes 匹配，case-insensitive）
5. **AC-5**：filter 输入框为空时，显示全部标签
6. **AC-6**：刷新页面（rehydrate），`tagsExpanded` 持久化状态被恢复（依赖 task_002 partialize）
7. **AC-7**：a11y：section header 是 `<button>`，带 `aria-expanded={tagsExpanded}` 和 `aria-controls="tag-tree-list"`；展开后的容器有对应 id
8. **AC-8**：单测覆盖：① 默认折叠 ② 点击展开 / 折叠 ③ 过滤功能 ④ aria 属性正确 ⑤ rehydrate 后保留展开状态
9. **AC-9**：`pnpm check` + `pnpm lint` + `pnpm test` 全绿

## 技术约束

- **状态来源**：必须读 `useUIStore(s => s.tagsExpanded)` 和 `useUIStore(s => s.setTagsExpanded)`，**不在 TagTree 内部用 useState**（违背持久化要求）
- **过滤逻辑**：在 TagTree 内 useState 维护 `filterText`（瞬态，不持久化）；用 useMemo 派生 filteredTags
- **样式**：filter 输入框 `padding: var(--space-2)`、`border: 1px solid var(--border-primary)`、`border-radius: var(--radius-sm)`；不引入新 token
- **chevron 图标**：用 `lucide-react` 的 `ChevronRight` / `ChevronDown`，不自己画 svg
- **不改 TagTree 数据源订阅**：本 task 只改交互层；标签数据如何获取保持原样

## 参考文件

- `src/components/features/TagTree.tsx`（当前实现）
- `src/components/features/__tests__/TagTree.test.tsx`（已有用例，扩展）
- `src/stores/uiStore.ts`（task_002 修改后状态）
- `product/prd/notecapt-v1.3-ui_prd_v1.md` §4.2 SB-05

## 预估影响范围

- **修改文件**：
  - `src/components/features/TagTree.tsx`
  - `src/components/features/__tests__/TagTree.test.tsx`

- **新建文件**：无

---

## Reviewer 重点关注项

- 折叠时是否真的不渲染列表 DOM（不是 max-height: 0）；性能优化对长标签列表更明显
- 过滤逻辑是否 case-insensitive
- aria-expanded 切换是否正确（防止屏幕阅读器误读）
- 持久化 rehydrate 用 zustand persist 的 `useUIStore.persist.rehydrate()` 或测试中模拟 LS
