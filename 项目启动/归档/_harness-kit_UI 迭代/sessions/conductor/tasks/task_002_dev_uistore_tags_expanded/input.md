# Task 输入 — task_002_dev_uistore_tags_expanded

## 目标

为 `src/stores/uiStore.ts` 新增 `tagsExpanded:boolean` 字段及其 setter，纳入 persist partialize，并扩展 `uiStore.test.ts` 验证字段默认值与 `migrateLegacySection("search") → "recent"` 用例。

## 前置条件

- 依赖 task：**无**
- 必须先存在的文件/接口：
  - `src/stores/uiStore.ts`（已存在）
  - `src/stores/__tests__/uiStore.test.ts`（如不存在则新建）
  - `src/types`（SidebarSection 等类型，已存在）

## 验收标准（Acceptance Criteria）

1. **AC-1**：`useUIStore.getState().tagsExpanded === false`（默认值）
2. **AC-2**：调用 `useUIStore.getState().setTagsExpanded(true)` 后 `tagsExpanded === true`，再 setTagsExpanded(false) 回到 false
3. **AC-3**：partialize 输出对象包含 `tagsExpanded` 字段
4. **AC-4**：模拟 LS 中持久化为 `{ activeSidebarSection: "search" }` 的旧用户，rehydrate 后 `activeSidebarSection === "recent"` 且 `tagsExpanded === false`
5. **AC-5**：单测覆盖：① 默认值 ② setter toggle ③ partialize 输出 ④ migrate "search" 用例
6. **AC-6**：`pnpm check` 0 错误；`pnpm lint` 0 错误；`pnpm test src/stores/__tests__/uiStore.test.ts` 全绿

## 技术约束

- **不改动现有字段**：仅追加 `tagsExpanded` + `setTagsExpanded`；不动 partialize 中现有字段
- **migrate 函数同步更新**：在 `migrate` 函数返回对象中追加 `tagsExpanded: Boolean(persisted?.tagsExpanded ?? false)`
- **版本号不升级**：保持 `version: 1`（新字段缺失会走 migrate 默认值）
- **命名风格**：字段名 `tagsExpanded`（小驼峰），setter `setTagsExpanded`（与现有 `setInspectorOpen` 等对齐）
- **不允许引入 new dependency**

## 参考文件

- `src/stores/uiStore.ts:72-110`（UIStore interface 现有定义）
- `src/stores/uiStore.ts:204-221`（persist config 现有 partialize / migrate）
- `sessions/conductor/tasks/task_001_architect/output.md` ADR-002（命名与持久化决策）
- `product/prd/notecapt-v1.3-ui_prd_v1.md` §4.2 SB-07

## 预估影响范围

- **修改文件**：
  - `src/stores/uiStore.ts`（追加字段 + setter + partialize + migrate）
  - `src/stores/__tests__/uiStore.test.ts`（扩展，如不存在则新建）

- **新建文件**：无（uiStore.test.ts 已存在，仅扩展）

---

## Reviewer 重点关注项

- partialize 是否真的包含了 `tagsExpanded`（DevTools 看 LS key="ui-store" 的 state）
- migrate 函数对 `persisted` 为 undefined 时（首次启动）不抛错
- "search" → "recent" 用例必须有断言（防止后续重构破坏）
