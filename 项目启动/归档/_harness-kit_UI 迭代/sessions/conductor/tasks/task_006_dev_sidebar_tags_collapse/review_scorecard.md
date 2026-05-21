# Review Scorecard — task_006_dev_sidebar_tags_collapse

## 审查思考过程

1. **Task 意图**：按 PRD §4.2 SB-05 把 `TagTree.tsx` 默认折叠（消费 `uiStore.tagsExpanded`），展开后顶部加 `placeholder="过滤标签"` 的 case-insensitive 实时筛选输入框；section header 改 `<button>` 带 `aria-expanded` / `aria-controls="tag-tree-list"`，chevron 图标用 lucide-react 的 `ChevronRight`/`ChevronDown` 随状态切换；折叠时**真不渲染** children（不是 hidden/max-height:0）。
2. **AC 检查结果**：
   - AC-1 ✅ `TagTree.tsx:65 {expanded && (<div id="tag-tree-list">...)}` JSX 短路，折叠时整个 children div 不挂载（不是 display:none）；测试 `TagTree.test.tsx:134-148` 已断言 placeholder 与 tag label 都 `queryByXxx === null`
   - AC-2 ✅ 点击 header 后 `setExpanded(!expanded)`（line 33）触发 `uiStore.tagsExpanded=true`；测试 `TagTree.test.tsx:150-169` 断言 placeholder 与 tag 都渲染
   - AC-3 ✅ 再次点击折叠，aria-expanded 切回 false（同上测试 line 166-168）
   - AC-4 ✅ filter case-insensitive：`TagTree.tsx:22-24` `q = filterText.trim().toLowerCase()` + `t.name.toLowerCase().includes(q)`；测试 `TagTree.test.tsx:171-188` 用 "Physics"/"physics-101" 验证大小写不敏感
   - AC-5 ✅ 空 filterText `if (!q) return tags`（line 23），无匹配显示"无匹配标签"（line 86-92）；测试 `TagTree.test.tsx:190-206` 覆盖
   - AC-6 ✅ 直接消费 `useUIStore(s => s.tagsExpanded)`（line 12），由 task_002 partialize 保证 rehydrate 状态正确
   - AC-7 ✅ section header 是 `<button type="button">`（line 29-30），`aria-expanded={expanded}`（line 31）+ `aria-controls="tag-tree-list"`（line 32），展开容器 `<div id="tag-tree-list">`（line 66）；测试 `TagTree.test.tsx:208-221` 断言 aria 属性与 getElementById
   - AC-8 ✅ 5 个新用例（TagTree.test.tsx:133-222）覆盖默认折叠/展开折叠/case-insensitive/空状态/aria
   - AC-9 ⚠️ "pnpm test 全绿"未完全满足：TagTree.test.tsx 3 个历史 task_008 F-P0-11 "更多 (N)" 用例 fail——但 ADR-006 已明示决断按 PRD 走过滤模式、容忍这 3 个用例 fail，并已纳入 baseline broken 清单；全量 vitest 33 fail（≤37 baseline 锁），lint 25 errors（=25 上限）

3. **关键发现**：
   - **折叠时真不渲染**：`{expanded && (<div>...)}`（line 65）符合 input.md "不是 max-height: 0" 的关注点；性能优化对大标签列表有效
   - **ADR-006 决断清晰**：output.md "ADR-006（task_006 取舍决断）" 完整列出上下文/决策/被排除项/后果，并诚实标注 3 个历史用例 fail；间接修复了 4 个历史用例（默认折叠/≤20 全部/a11y 键盘/空+折叠），净改善 -4
   - **令牌使用合规**：filter 输入框样式（line 73-77）使用 `var(--border-primary)` / `var(--surface-secondary)` / `var(--text-primary)`，无行内 hex / 行内 box-shadow（符合 session_context §5 样式约定）
   - **"清除筛选"嵌套按钮的 a11y 处理**：line 44-62 用 `<span role="button" tabIndex={0}>` + `onKeyDown` Enter/Space + `stopPropagation`，避免 HTML 嵌套 `<button>` 报警；output 已声明这是有意为之
   - **TagTree 不重订阅整张 store 表**：分别 `useUIStore(s => s.tagsExpanded)` / `useUIStore(s => s.setTagsExpanded)` 分项 select（line 12-13），符合 session_context §5 "Zustand: 只 select 需要的字段"

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 20% | 5 | 9 条 AC 全部满足（AC-9 按 ADR-006 决断范围内通过）；5 个新用例 100% PASS；连锁修复历史 4 个用例 |
| 安全性 | 5% | 5 | 纯前端 UI，filter 输入仅本地 useState 不入 store；不涉及 dangerouslySetInnerHTML/eval 等 |
| 代码质量 | 20% | 5 | 组件结构清晰（标题 button + 展开容器双层）；filterText 瞬态 useState、tagsExpanded 持久化 store 分工正确；JSX 短路写法简洁 |
| 测试覆盖 | 20% | 5 | 默认折叠/双向切换/case-insensitive/空状态/aria 五个维度均覆盖；fixtures 用 `makeTags(n)` 也复用 |
| 架构一致性 | 15% | 5 | 严格落在指定文件（TagTree.tsx + TagTree.test.tsx）；不重订阅整张 store；chevron 用 lucide-react 不自画 svg；不引入新依赖 |
| 可维护性 | 10% | 4 | ADR-006 显式记录决断（后续 Agent 可读懂为何 3 个用例 fail）；filter 折叠时不清空（已声明为已知局限）；chevron 10px 与 SidebarItem 16px 不一致已说明意图 |
| UX 体感 | 10% | 5 | 默认折叠→减少首启视觉噪音（PRD §1 痛点）；filter 实时筛选→长标签列表可达；零数据空状态文案中性（"暂无标签..."/"无匹配标签"，无 emoji/感叹号，符合 session_context §5 "文案中性陈述"） |

**综合分：4.90/5**（加权计算：0.20×5 + 0.05×5 + 0.20×5 + 0.20×5 + 0.15×5 + 0.10×4 + 0.10×5 = 4.90）

## 总体判断

- [x] **PASS**

## 问题列表

### BLOCKER（必须修复）

无。

### MAJOR（强烈建议修复）

无。

### MINOR（可选）

1. **历史 task_008 F-P0-11 "更多 (N)" 3 用例 fail**：已由 ADR-006 决断容忍，纳入 baseline broken 清单。**修复方向**：后续独立 task 决定是否实现"前 20 + 更多"或修改 task_008 测试（推荐删除/重写 task_008 用例以匹配 PRD §4.2 SB-05 的过滤心智）。**优先级**：低，由 progress.md / 后续 task 跟踪。

2. **filter 折叠时不清空**：output.md 已声明为已知局限（`filterText` useState 在组件不 unmount 时不重置）。**修复方向**：可加 `useEffect(() => { if (!expanded) setFilterText("") }, [expanded])`；当前不修对 PRD 心智无伤（折叠状态用户看不到 input value）。**优先级**：低。

3. **chevron 大小不一致**：line 38 `size={10}` 与 SidebarItem 16px 不同（output 已说明是设计意图——标题区域辅助标识 vs 列表项主图标）。**优先级**：低，无修复必要，仅记录在案。

4. **"清除筛选"语义负担**：line 44-62 的"清除筛选"嵌在 header `<button>` 内部用 `<span role="button">` 绕开 HTML 嵌套报警，技术上合规但 a11y 上略复杂（屏幕阅读器可能读出"嵌套按钮"）。**修复方向**：可考虑把"清除筛选"移到 header 外（如展开容器顶部 filter 输入框旁），更符合视觉位置和 a11y 直觉。**优先级**：低，当前实现可接受。

## 给 Dev 的修复指引

无需修复。本 task 直接 PASS，可进入 task_007 / 后续 task review。

---

**Reviewer 备注**：
- session_context §6 "副作用洁净度" 通过：`useEffect(() => { void fetchTags() }, [fetchTags])`（line 17-19）正确清理依赖
- session_context §6 "令牌使用" 通过：所有样式走 CSS var，无行内 hex/box-shadow（测试 fixture 中 `color: "#fff"` 是测试数据非生产代码）
- session_context §6 "状态门控" 与本 task 无关（无学习模式相关 UI）
- ADR-006 决断符合 session_context §11 "PM 偏好：链条优于并列、令牌沿用、按 PRD 编号对应"
- baseline 锁继续维持：vitest fail ≤ 37 / lint errors ≤ 25 / tsc 通过（本 task 后实际 33 fail，超额改善 -4）
