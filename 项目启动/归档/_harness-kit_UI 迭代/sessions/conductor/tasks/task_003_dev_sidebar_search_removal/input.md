# Task 输入 — task_003_dev_sidebar_search_removal

## 目标

从 `Sidebar.tsx` 主导航中移除 `<Search>` SidebarItem，将搜索入口移到 `SidebarFooter` 中作为图标按钮，并把 SidebarFooter 改为单行三段格式：`⌘K 搜索 · ⚙ 设置 · TF 状态点`。

## 前置条件

- 依赖 task：**无**（与 task_002 可并行）
- 必须先存在的文件/接口：
  - `src/components/layout/Sidebar.tsx`
  - `src/components/layout/SidebarFooter.tsx`

## 验收标准（Acceptance Criteria）

1. **AC-1**：`Sidebar` 渲染后，DOM 中不再包含 `Search` SidebarItem（无 `label="Search"` 的 nav row）
2. **AC-2**：Sidebar 顶部不再 import `Search` from `lucide-react`
3. **AC-3**：`SidebarFooter` 渲染后，DOM 中有三个段：①带 ⌘K 标识的搜索按钮 ②⚙ 设置按钮 ③TF 状态点（6px 圆 + 灰字 "TF 已插入/未插入"）
4. **AC-4**：点击 Footer 搜索图标按钮触发 `onSearchOpen` 回调（与原 Sidebar Search 项行为等价）
5. **AC-5**：点击 Footer 设置按钮触发 `onSettingsOpen` 回调（保持现有行为）
6. **AC-6**：全局快捷键 ⌘K 仍能打开搜索浮层（**不在本 task 改动**，仅确认未破坏，找到现有快捷键 hook 并验证未被本 task 影响）
7. **AC-7**：`pnpm check` + `pnpm lint` + `pnpm test` 全绿；`AppLayout.test.tsx` 如有 Sidebar Search 相关断言需同步更新

## 技术约束

- **不动 SidebarItem 组件**：仅在 Sidebar.tsx 删除 Search 一行 + import；SidebarItem 接口本身不改
- **SidebarFooter Props**：接受 `onSearchOpen?: () => void`、`onSettingsOpen?: () => void`；不引入新 prop
- **样式走 token**：TF 状态点的颜色用 `var(--text-tertiary)`；6px 圆用 `width: 6px; height: 6px; border-radius: 50%;`
- **TF 状态判断**：当前若 settingsStore 无 `tauriPluginFsAvailable` 之类字段，可暂用 `"TF 未插入"` 固定文案（PRD 未定义 TF 数据源，本期占位即可）；务必在交付中标注"TF 状态点目前用固定 fallback 文案"
- **a11y**：Footer 三个交互元素均为 `<button type="button">`，搜索/设置加 `aria-label`

## 参考文件

- `src/components/layout/Sidebar.tsx:1`（Search import 位置）
- `src/components/layout/Sidebar.tsx:64-68`（Search SidebarItem 现有位置）
- `src/components/layout/SidebarFooter.tsx`（现有结构）
- `src/components/layout/SidebarItem.tsx`（不改）
- `product/prd/notecapt-v1.3-ui_prd_v1.md` §4.2 SB-01, SB-06

## 预估影响范围

- **修改文件**：
  - `src/components/layout/Sidebar.tsx`（删 import + 删 SidebarItem 一段）
  - `src/components/layout/SidebarFooter.tsx`（重构为单行三段）
  - 可能：`src/components/layout/__tests__/AppLayout.test.tsx`（若有 Search 相关断言）

- **新建文件**：可能 `src/components/layout/__tests__/SidebarFooter.test.tsx`（推荐新增，覆盖 footer 三段渲染）

---

## Reviewer 重点关注项

- 确认 `activeSidebarSection === "search"` 不再出现于任何 setter 入参（grep 全局）
- 确认 Sidebar.tsx import 列表中 Search 已删除（防止 lint unused import 警告）
- TF 状态点的 fallback 文案处理是否在交付中明确标注
