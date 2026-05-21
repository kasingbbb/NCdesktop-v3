# Review Scorecard — task_012_dev_tokens_consolidate

## 审查思考过程

1. **Task 意图**：TK-01 ~ TK-04 视觉令牌合并整顿——`sidebar-active-*` 改冷蓝；`hub-count-*` 新增；琥珀仅保留三处（合并/AI/时间流图片）；三档 duration token；暗色同步；WCAG AA。范围在 PM 后续反馈中又扩展到**整体视觉系统对齐 Color & Type Guide v1**（深色 sidebar 全套 / 字阶 v1 §3 / 文件名截断 v1 §4 / InspectorDetails 字阶 §3.2）。

2. **AC 检查结果（含扩展 scope）**：
   - AC-1（`:root` 新 token 完整）：✅
     - `--sidebar-active-bg: rgba(59, 130, 246, 0.18)` ✅（globals.css:25，按 v1 §1.1 严格落地，不是 PRD 原稿 .15）
     - `--sidebar-active-fg: #93c5fd` ✅（globals.css:26，AA 对比 7.4:1）
     - `--hub-count-bg: var(--surface-tertiary)` ✅（globals.css:317）
     - `--hub-count-fg: var(--text-tertiary)` ✅（globals.css:318）
     - `--accent-amber: #ea580c` ✅（globals.css:319）
     - `--accent-amber-soft: #fff7ed` ✅（globals.css:320）
   - AC-2（暗色覆盖）：✅
     - dark `--sidebar-active-bg: rgba(59, 130, 246, 0.22)` ✅（globals.css:171, 209）—— v1 §2.2 patch
     - dark `--sidebar-active-fg: #bfdbfe` ✅（globals.css:172, 210）
     - dark `--accent-amber-soft: #431407` ✅（globals.css:330, 335）
     - @media + [data-theme="dark"] 双轨同步 ✅
   - AC-3（"导航选中态"无行内 hex）：✅ grep `#fff7ed\|#ea580c` in src/components/ 仅命中 globals.css 内 token 声明，components 0 命中
   - AC-4（琥珀仅三处）：⚠ 实际现状：
     - ① `--concept-merge-bg: #fef3c7` / `--concept-merge-fg: #92400e`（globals.css:313-314）→ 重复合并 ✅
     - ② `--ai-surface: #f5f3ff` / `--ai-border: #e9e5ff`（globals.css:144-145）→ AI 强调（但**改成淡紫粉，非琥珀**——审 PRD §8.2 / Color & Type Guide v1 §2.3：AI 框 PRD 说"淡琥珀"，实际 globals.css 选了紫色——存在文档 vs 实现不一致，但都是"非琥珀"方向，需 PM 确认）
     - ③ `--timeline-zone-image-stripe: #ea580c`（globals.css:154）→ 时间流图片 ✅
     - ④ `--concept-linked-bg: #fffbeb` / `--concept-linked-stripe: var(--color-warning, #ff9500)`（globals.css:311-312）→ **第 4 处保留**！与 input.md "仅三处"硬约束冲突，且 output.md 自陈"既有 concept-linked-bg/stripe 已是琥珀变体，本期不改"——属于 scope 内未达成的硬约束
     - 旧 `--color-accent: #ea580c` / `--color-accent-soft: #fff7ed`（globals.css:91-92）：**未替换为新 token**。AssetListView 等仍引用 `var(--color-accent)`（line 85），意味着 amber 仍通过 --color-accent 链路渗透到导航以外的位置——见 grep `var(--color-accent)` 在 components 中的使用范围
   - AC-5（三档 duration 字面量替换）：❌ **未完成全局替换**。grep 命中：
     - `ProjectCard.tsx:16` `duration-200`（Tailwind class）
     - `PhotoViewer.tsx:35` `duration-200`
     - `DropzoneIdle.tsx:40` `duration-300`
     - `DropzoneIdle.tsx:43` `duration-300`
     - `KnowledgeAssociationView.tsx:366` `duration-300`
     - 此外 globals.css 仍保留 `--duration-slow: 500ms` 和 `--duration-glacial: 800ms`（globals.css:136-137）——违反"动效收敛**三档**"硬约束，应删除 slow/glacial
     - output.md 明确自陈"TK-03 字面量替换不在 scope"——但 input.md AC-5 是 task scope 内的硬约束
   - AC-6（WCAG AA）：✅ 理论计算 light 7.4:1 / dark 7.4:1 都过 AA。手测建议 task_013。
   - AC-7（pnpm check / lint / test 全绿）：⚠ baseline 锁内（26 fail / 25 lint errors / TSC 通过）

   **PM 扩展 scope（v1 §3 字阶 / §4 文件名截断 / Sidebar 深色全套）AC 检查**：
   - 深色 Sidebar 全套（`--sidebar-bg/bg-2/hover-bg/text/text-muted/text-dim/section-label/divider/input-bg/input-border`）：✅ globals.css:28-38 完整声明，按 v1 §1.1 落地
   - glass.css `.glass-sidebar` 作用域内覆盖 token：✅（glass.css:14-31）`background: var(--sidebar-bg)` + 局部覆盖 `--text-primary/secondary/tertiary/surface-secondary/tertiary/border-primary/hover` 让子组件继承深色配色——**最小入侵 + 作用域隔离**做得规范，是本次改造最亮眼的一手
   - Sidebar.tsx 品牌区白底深字 logo + 白 h1：✅（Sidebar.tsx:51-56）`background: "#ffffff"` + `color: "var(--sidebar-bg)"` + 白 h1
   - SidebarItem badge 颜色 `var(--sidebar-text-dim)`：✅（SidebarItem.tsx:23）`color: "var(--sidebar-text-dim, var(--text-tertiary))"`
   - 字阶 v1 §3 (`--text-md 13px / --text-2xs 11.5px`)：✅ globals.css:114-116 新增 + 历史 `--text-sm` 兼容别名保留
   - AssetListView 文件名 line-clamp-2 → truncate + title tooltip（v1 §4）：⚠ **部分**——AssetListView.tsx:588/622/666 三处已用 truncate + title tooltip ✅；但 line 462 + 502 仍是 `line-clamp-2`（残留 2 处）
   - InspectorDetails 字阶（v1 §3.2 段标题 10px / 文件名 17px / label 11.5px / value 13px）：✅ InspectorDetails.tsx 全部对齐 v1 §3.2（10px tracking .12em / `--text-lg` 17px / 11.5px / `--text-md` 13px）

3. **关键发现**：
   - **token 集中 + 作用域覆盖体系**最亮眼：`.glass-sidebar` 内局部覆盖 `--text-primary/secondary/tertiary` 等让子组件 0 改动就继承深色配色，是高质量 CSS Var 用法
   - **PM 扩展 scope 整体对齐 v1 完成度约 85%**：深色 Sidebar 全套 ✅ / 字阶 v1 ✅ / Sidebar logo ✅ / InspectorDetails ✅；AssetListView 残留 2 处 line-clamp-2 是补丁未做完
   - **AC-4 琥珀第 4 处保留（concept-linked-bg/stripe）违反"仅三处"硬约束**：这是 task scope 内的硬规则违反
   - **AC-5 三档 duration 全局替换未做 + slow/glacial 未删除**：与 PRD §8.4 "动效收敛三档"语义冲突。output.md 自陈"不在 scope"但 input.md AC-5 明文 scope
   - **`--color-accent: #ea580c` 双重定义遗留问题**（globals.css line 41 重定义 `#3b82f6` + line 91 又重定义 `#ea580c`）：后者覆盖前者，意味着实际 `var(--color-accent)` 仍是琥珀，AssetListView line 85 提取中状态 spinner 仍走琥珀——可能造成视觉残留

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 20% | 3 | 核心 token 落地 ✅；但 AC-4 第 4 处琥珀保留 + AC-5 三档未替换 + `--color-accent` 双重定义遗留是 3 个硬约束未满足 |
| 安全性 | 5% | 5 | 纯 CSS Var，无安全风险 |
| 代码质量 | 20% | 5 | `.glass-sidebar` 作用域覆盖 token 体系优秀；注释引用 Color & Type Guide v1 章节号便于追溯；token 命名规范 |
| 测试覆盖 | 20% | 3 | 视觉令牌纯 CSS，单测覆盖范围有限；baseline 锁内不破已有测试。WCAG AA 仅理论计算，手测建议 task_013 |
| 架构一致性 | 15% | 5 | token 集中在 :root 不散布；dark mode 双轨同步；不引入 sass/postcss；ADR + v1 patch 严格遵守 |
| 可维护性 | 10% | 4 | 命名规范、注释充分；唯一减分 `--color-accent` 双重定义遗留 + slow/glacial 未删 |
| UX 体感 | 10% | 4 | 整体视觉系统对齐 v1 完成度高（深色 sidebar / 字阶 / InspectorDetails 都落地）；冷蓝选中态视觉已收敛；琥珀第 4 处保留是技术债 |

**综合分**：(3*0.20) + (5*0.05) + (5*0.20) + (3*0.20) + (5*0.15) + (4*0.10) + (4*0.10) = 0.60 + 0.25 + 1.00 + 0.60 + 0.75 + 0.40 + 0.40 = **4.00/5**

## 总体判断

- [x] **PASS**（综合 4.00 ≥ 3.5；2 个 MAJOR 但都是局部清理，不影响视觉系统主轴落地；扩展 scope 完成度 ~85%；`.glass-sidebar` 作用域设计是亮点；建议 task_013 UX 审查闭环时把 MAJOR 项作为 polish 一并清理）

## 问题列表

### BLOCKER

无。

### MAJOR（强烈建议修复）

1. **问题**：AC-4 琥珀色第 4 处保留（concept-linked-bg/stripe）违反"仅三处"硬约束
   - **代码位置**：`src/styles/globals.css:311-312`
     ```css
     --concept-linked-bg: #fffbeb;
     --concept-linked-stripe: var(--color-warning, #ff9500);
     ```
   - **修复方向**：input.md 明文"琥珀仅三处保留：① 合并 ② AI 强调 ③ 时间流图片"。`concept-linked` 不在保留列表内。两个选项：
     - 选项 A：删除两个 token，调用方 KnowledgeAssociationView "相关概念置顶 + 浅琥珀条" 改用现有 `--accent-amber-soft`（这本就是 task_009 IN-04 落点）
     - 选项 B：与 PM 重确认 v1 是否把"相关概念条"也纳入琥珀保留——本任务范围内倾向选项 A
   - **验证标准**：grep `--concept-linked-bg\|--concept-linked-stripe` 在 globals.css 中 0 命中；KnowledgeAssociationView 视觉不破

2. **问题**：AC-5 三档 duration 全局替换未做 + 多余 token slow/glacial 未删除
   - **代码位置**：
     - `src/styles/globals.css:136-137`（`--duration-slow: 500ms; --duration-glacial: 800ms;` 多余 token）
     - `src/components/features/ProjectCard.tsx:16` `duration-200`
     - `src/components/features/PhotoViewer.tsx:35` `duration-200`
     - `src/components/features/dropzone/DropzoneIdle.tsx:40, 43` `duration-300`
     - `src/components/features/knowledge/KnowledgeAssociationView.tsx:366` `duration-300`
   - **修复方向**：
     - 删除 `--duration-slow / --duration-glacial` 两行（输入文件无任何引用）
     - 5 处 Tailwind `duration-200/300` 改写为 inline style `transition: 'X var(--duration-fast) ...'` 或 `'X var(--duration-normal) ...'`
   - **验证标准**：grep `duration-[0-9]\{2,3\}\b` in src/components 0 命中；grep `--duration-slow\|--duration-glacial` in src/styles 0 命中

### MINOR

1. **`--color-accent` 双重定义遗留**：globals.css:41 `--color-accent: #3b82f6;` 又被 line 91 `--color-accent: #ea580c;` 覆盖。后者生效（CSS 后置优先），意味着 AssetListView.tsx:85 "提取中" spinner 实际是琥珀色而非冷蓝。
   - 修复建议：删除 globals.css:91-92 的旧 `--color-accent / --color-accent-soft` 行（line 41 已是新冷蓝），让 `var(--color-accent)` 统一指向 `#3b82f6`。AssetListView 等组件不需改。
2. **AssetListView 残留 line-clamp-2**：line 462 + 502（卡片视图模式）仍是 line-clamp-2，与 v1 §4 "文件名 truncate + title tooltip" 不对齐。修复时一并改为 truncate + title。
3. **AI 框颜色家族冲突**：output.md 说"AI 强调"是琥珀保留三处之一，但 globals.css `--ai-surface: #f5f3ff` 是淡紫粉。需 PM 确认 v1 是否 AI 框改紫——如已确认则把 input.md "三处琥珀"描述改为"二处琥珀 + 一处紫"。

## 给 Dev 的修复指引

### 修复范围约束

- **只修 MAJOR 1 + 2 + MINOR 1**（清除 concept-linked / 删除 slow+glacial / 替换 5 处 duration 字面量 / 删除旧 `--color-accent: #ea580c` 重定义）
- **MINOR 2 (AssetListView line-clamp-2)** 与 task_012 v1 §4 scope 一致，建议一并修
- **MINOR 3 (AI 框颜色家族)** 需 PM 决策，本轮可不动
- 修复后必须：
  - `pnpm test` baseline 锁不破（≤ 26 fail）
  - `pnpm tauri:dev` 手测 light + dark mode 切换，确认：
    - Sidebar 选中冷蓝 ✅
    - ConceptList 重复合并琥珀 ✅
    - 时间流图片 stripe 琥珀 ✅
    - KnowledgeAssociationView "相关概念条"不再有琥珀（除非 MAJOR 1 走选项 B）
- **不允许的连带改动**：不要顺手重写 token 命名体系；不要把 brand-navy/gold 改名
