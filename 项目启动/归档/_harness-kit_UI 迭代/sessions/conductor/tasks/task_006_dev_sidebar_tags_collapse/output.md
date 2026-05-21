# Task 交付 — task_006_dev_sidebar_tags_collapse

## 实现摘要

按 v1.3 PRD §4.2 SB-05 重写 `TagTree.tsx`：默认折叠（消费 `uiStore.tagsExpanded`），展开后顶部加过滤输入框（`placeholder="过滤标签"`），输入文本实时筛选标签列表（case-insensitive，按 name 子串匹配）。配套 a11y：标题改为 `<button>`，加 `aria-expanded`、`aria-controls="tag-tree-list"`，chevron 图标随状态切换。新增 5 个用例到 `TagTree.test.tsx`。

核心设计决策：
- **ADR-006（v1.3 task_006 取舍）**：详见下方 ADR-006 段
- **不复用 SidebarSection**：原 SidebarSection 标题是 `<p>` 且无 collapsed 行为；为避免改动 SidebarSection 影响其他消费者（Sidebar 中其它 section），TagTree 自渲染标题按钮
- **filter 瞬态本地状态**：filterText 用 useState，不进 store（与 PRD 的"展开/折叠持久化、过滤不持久化"心智一致）
- **空状态分级**：tags 总数 = 0 → "暂无标签..."；tags > 0 但 filter 无匹配 → "无匹配标签"

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `src/components/features/TagTree.tsx` | 改写 | 完整改写：消费 useUIStore.tagsExpanded；标题改 button + aria；展开后渲染过滤输入框 + 筛选列表 |
| `src/components/features/__tests__/TagTree.test.tsx` | 修改 | 新增 `describe("TagTree — v1.3 task_006 SB-05（折叠 + 过滤）")` 含 5 个用例 |

## ADR-006（task_006 取舍决断）

- **状态**：已接受
- **上下文**：v1.3 PRD §4.2 SB-05 要求 TagTree 折叠后顶部带"过滤输入框"。但项目已有 `TagTree.test.tsx` 中预先编写的 `task_008 F-P0-11` 契约要求"展开后前 20 + 更多 (N) + showAll 切换"。两套 UX 不同
- **决策**：按 v1.3 PRD §4.2 走"过滤输入"模式。`task_008 F-P0-11` 中涉及"更多 (N) / showAll"的旧用例继续 fail（保留在既有 broken 清单），由后续独立 task 决定取舍
- **被排除项**：
  - 同时实现"前 20 + 更多 + 过滤"：YAGNI，且两套 UX 混合反而让心智复杂
  - 先做"前 20+更多"后做"过滤"：与 PRD 直接冲突
- **后果**：
  - 历史 `task_008 F-P0-11` 中 3 个 "更多 (N)" 相关用例继续 fail（baseline 容忍）
  - 历史 7 个 fail 用例中**实际有 4 个被本 task 间接修复**（AC-1 默认折叠 / AC-2 ≤20 全部 / AC-4 a11y 键盘 / AC-5 空+折叠），因为本 task 实现了折叠骨架
  - 净改善：baseline vitest 失败数 37 → 33（减少 4）

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致
- [x] API 路径/命名与 Architect 方案一致（消费 `useUIStore.tagsExpanded` / `setTagsExpanded`）
- [x] 数据模型与 Architect 方案一致（filterText 本地 useState，不进 store）
- [x] 未引入计划外的新依赖（chevron 图标用 lucide-react 已有 `ChevronRight` / `ChevronDown`）
- 偏离说明：未复用 SidebarSection 组件（改为 TagTree 自渲染标题）—— 见上方"核心设计决策"

## 测试命令

```bash
pnpm vitest run src/components/features/__tests__/TagTree.test.tsx
pnpm vitest run
pnpm lint
pnpm check
```

## 测试结果

### TagTree.test.tsx（focused）

```
Test Files  1 failed (1)
     Tests  3 failed | 9 passed (12)
```

通过：
- 我新增的 5 个 v1.3 task_006 用例全部 PASS
- 历史 `task_008 F-P0-11` 中 4 个用例**间接** PASS（默认折叠 / ≤20 全部 / a11y 键盘 / 空+折叠）

失败（3 个，符合 ADR-006 决断）：
- `AC-2/AC-6 展开 + tags > 20 → 前 20 + '更多… (N)'` — 未实现
- `AC-3 点击 '更多…' → 余项 mount` — 未实现
- `AC-3 重新折叠后展开，showAll 重置` — 未实现

### 全量 vitest

```
Test Files  7 failed | 20 passed (27)
     Tests  33 failed | 229 passed (262)
```

baseline 锁（≤ 37）✅。实际改善 4。

### Lint

```
✖ 45 problems (25 errors, 20 warnings)
```

baseline 锁（≤ 25 errors）✅。

### TSC

```
$ pnpm check
> tsc --noEmit
(无输出 = 通过)
```

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果 |
|----------|----------|------|------|
| ✅ 正常路径 | 默认折叠：aria-expanded=false，过滤框与标签均不渲染 | 已测 | PASS（AC-1 用例） |
| ✅ 正常路径 | 点击展开 → 过滤框 + 全部标签出现；再点折叠 | 已测 | PASS（AC-2/3 用例） |
| ✅ 正常路径 | 过滤输入 case-insensitive 实时筛选 | 已测 | PASS（AC-4 用例） |
| ⚠️ 边界条件 | 空过滤显示全部；无匹配显示"无匹配标签"文案 | 已测 | PASS（AC-5 用例） |
| ⚠️ 边界条件 | tags 总数 = 0 但已展开 → 显示"暂无标签..."提示 | 已测 | PASS（历史 AC-1/AC-5 用例间接覆盖） |
| ✅ a11y | aria-expanded + aria-controls + chevron 视觉切换 | 已测 | PASS（AC-7 用例） |
| ⚠️ 持久化 | 折叠状态写入 uiStore.tagsExpanded，刷新后保留 | 间接验证 | uiStore 单测已断言 partialize；TagTree 直接消费 store，行为正确（手测可在 Tauri dev 中确认） |

## 已知局限

1. **历史 task_008 F-P0-11 "更多 (N)"契约不实现**：依 ADR-006 决断
2. **过滤逻辑只按 tag.name 子串匹配**：未来若需支持按颜色/source 维度过滤需扩展
3. **filterText 折叠时未清空**：保持 useState 在组件未 unmount 时不重置；如希望折叠时同步清空 filter，需添加 useEffect 监听 expanded 变化——但这与 PRD 心智不冲突，本期保持现状

## 需要 Reviewer 特别关注的地方

- **未复用 SidebarSection**：TagTree 现自渲染外层 div + 标题按钮。如未来要把所有 sidebar section 改为可折叠，建议升级 SidebarSection API 而非延续此 pattern
- **"清除筛选" 二级按钮的 e.stopPropagation()**：阻止冒泡到标题 button 的 onClick（避免误折叠）。a11y：用 `role="button" tabIndex={0}` 而非嵌套 `<button>` 避免 HTML 嵌套警告
- **chevron 大小固定 10px**：与现有 SidebarItem 16px 图标不一致，因为这是标题区域的辅助标识，意图通过视觉差异强调"组" vs "项"
