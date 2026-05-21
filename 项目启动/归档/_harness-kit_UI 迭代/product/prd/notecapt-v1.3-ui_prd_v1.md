# NoteCapt Desktop · 主界面收敛 v1.3 — PRD

> **来源**：用户提供的 `NoteCapt Iteration Spec v1.html`（设计 → 工程交付文档）。本 PRD 是该 HTML 的 markdown 等价物 + 末尾 Conductor 桥接摘要。改造点编号（SB/KH/IN/DZ/ES/TK）与原始 HTML 一一对应，便于回溯。

---

## 文档元数据

- **Owner**：设计
- **Target**：v1.3 首发
- **技术栈**：Tauri 2 · React 19 · Tailwind v4
- **Status**：Draft → Architect Ready
- **基线假设**：本次不重写已有 store、不动 Rust 命令、不引新路由库；改造全部落在 `components/layout/*`、`components/features/KnowledgeHubView/*`、`components/features/SettingsPanel.tsx`、`styles/globals.css` 这几处

---

## 01 现状盘点 & 差距诊断

### ✅ 已就位的能力

- **学习模式开关** — `settings.showLearningFeatures` 已存在，`useEffectiveLearningSettings()` 派生 hook 兜底；Sidebar 已通过它控制 Calendar / 今日复习 / TODAY 分组显隐
- **KnowledgeHub 聚合视图** — `features/KnowledgeHubView` 已经把"素材 → 概念 → 知识库 → 技能"做成 4-step 横向 tablist + hash 路由，旧 hash 自动迁移
- **设计令牌完整** — `globals.css` 的颜色/圆角/阴影/动效 token 体系成熟，本次迭代以**沿用 + 局部增补**为主

### ❌ 仍需收敛的问题

| 问题 | 现象 | 影响 |
|------|------|------|
| Sidebar 仍偏密 | 知识库 / 技能两个 hub 入口仍并列平铺；TODAY 区在没课时显示"今天没有课程"占位行；Search 占栏位但已是 ⌘K 浮层动作 | 新用户认知噪音；与"链条而非并列"的心智不一致 |
| HubStep 链条感弱 | 当前 nav 只是 4 个 chip 平排，缺少 *素材 → 概念 → 知识库 → 技能* 的"演化方向"提示，也没有每段的当前计数 | 用户看不出这 4 步的因果关系，仍像 4 个独立菜单 |
| Inspector tab 顺序 | "详情 / 时间流 / 知识关联"——时间流（动态）夹在两个静态 tab 之间 | 切换路径不顺；语义同类没归拢 |
| 悬浮窗与主面板冲突 | Dropzone 浮在内容区中央与空状态文案争抢焦点；多个交互元素挤在小窗 | 主窗口工作时被视觉打断 |
| 空状态噪音 | "今天无待处理"同一信息说 4 遍（顶部 0/0/0 + 大标题 + 副标题 + 提示），庆祝感不合 zero-data | 新用户首屏第一印象差 |
| TAGS 折叠 | 74 个标签全平铺，左栏一打开就是一堵墙 | 滚不动 + 无法定位重点标签 |

---

## 02 北极星 & 设计原则

### ⭐ 北极星

一个**非学生**用户首次启动 NoteCapt 后 60 秒内，应当只看到"工作区 × 知识链条"两件事，没有任何"复习 / 课程 / 技能"字样。

### 六大原则

- **P-01 渐进披露**：默认极简，能力按需出现；学习功能由设置开关解锁；空数据时不渲染入口
- **P-02 链条优于并列**：素材 → 概念 → 知识库 → 技能 表达因果；用 chevron / 计数显示进化方向；不把链条上的环节做成平级菜单
- **P-03 动作浮层 / 位置侧栏**：侧栏=位置（去哪里）；⌘K / 设置 / 导入=动作（怎么做）；动作不占栏位
- **P-04 零数据零信号**：计数为 0 不显示 badge；分组为空不渲染 section；空状态最多一句话
- **P-05 沿用令牌不发明**：颜色 / 圆角 / 阴影走现有 CSS var；新增 token 必须进 globals.css 集中管理；禁止行内 hex 与 box-shadow 字面量
- **P-06 驾驶舱审美**：左深 / 中白 / 右白的三段式不变；大圆角 + 浮起卡片 + 低对比阴影；琥珀仅做强调，导航选中用冷蓝

---

## 03 迭代 Roadmap（3 个 Phase）

| Phase | 名称 | 预估 | 优先级 | 范围 |
|-------|------|------|--------|------|
| **PHASE 0** | 主界面收敛 | 3.5 d | **P0**（首发阻塞） | Sidebar 收敛（§4）、KnowledgeHub 链条化（§5）、TODAY 空分组不渲染、Search 出栏位 |
| **PHASE 1** | 细节体感 | 1.5 d | P1 | Inspector tab 重排（§6）、悬浮窗位置策略（§7）、空状态系统化（§7）、TAGS 默认折叠 + 搜索 |
| **PHASE 2** | 视觉打磨 | 0.5 d | P2 | 令牌微调（§8）、琥珀回收为强调色、暗色模式同步、动效收敛到三档 duration |

**合并节奏建议**：P0 拆三个 PR：①Sidebar 重构 ②KnowledgeHubStepNav 升级 ③Settings UI 增补"学习模式说明"。每个 PR 都需配 vitest 用例。

---

## 04 Sidebar 收敛规范（PHASE 0）

### 4.1 目标信息架构

- **默认态（非学生）**：6 项以内 — Recent / Starred / 知识 / [PROJECTS] / [TAGS（折叠）]
- **学生态**：9 项以内 — 在"知识"与 PROJECTS 之间插入"学习中心"分组（今日 / 课程表）

### 4.2 改造点

| ID | 动作 | 位置 |
|----|------|------|
| **SB-01** | 移除 Sidebar 中的 `<Search>` 项；Search 改为 SidebarFooter 内一个图标按钮（⌘K 浮层入口仍保留全局快捷键） | `src/components/layout/Sidebar.tsx` |
| **SB-02** | 将"知识库"+"技能"两个 SidebarItem 合并为单一 **"知识"** 入口，点击后 `navigateHub('library')`（默认 step 改为 `concepts`，详见 §5） | 同上 |
| **SB-03** | "知识"行右侧 badge 显示链条计数 `素材·概念·库`，用 mono 字号 11px，颜色 `--sidebar-text-dim`；3 个数字用 `·` 分隔。**计数任一为 0 时整条 badge 不渲染** | 同上 |
| **SB-04** | TODAY 分组：当 `showLearningFeatures===false` **或** 课程列表为空时**整组不渲染**（不再渲染"今天没有课程"占位行）。学生态下有课才出现"学习中心 → 今日" | 同上 + `features/calendar/CourseSection.tsx` |
| **SB-05** | TAGS 分组默认折叠；保留 section header + 计数；点击展开后 header 顶部带搜索框（`placeholder="过滤标签"`）。状态用 `uiStore.tagsExpanded` 持久化 | 同上 + `features/TagTree.tsx` + `stores/uiStore.ts` |
| **SB-06** | SidebarFooter 改为单行三段：`⌘K 搜索 · ⚙ 设置 · TF 状态点`；TF 状态点用 6px 圆 + 灰字 | `src/components/layout/SidebarFooter.tsx` |
| **SB-07** | 新增 `useUIStore.tagsExpanded:boolean`（默认 false，partialize 持久化），切换由 TagTree section header 控制 | `src/stores/uiStore.ts` |

### 4.3 代码草图（精简后）

```tsx
<nav>
  <SidebarItem icon={<Clock/>} label="Recent" .../>
  <SidebarItem icon={<Star/>} label="Starred" .../>
  <SidebarItem
    icon={<Network/>}
    label="知识"
    active={activeSidebarSection === "knowledge-hub"}
    badge={hubBadge}                // "120·47·12" 或 undefined
    onClick={() => {
      setSidebarSection("knowledge-hub");
      navigateHub("library");
    }}
  />

  {showLearningFeatures && (
    <SidebarSection title="学习中心" className="sidebar-learning-fade-in">
      {hasTodayItems && <SidebarItem ... label="今日" badge={todayCount} />}
      <SidebarItem icon={<CalendarDays/>} label="课程表" .../>
    </SidebarSection>
  )}

  <ProjectTree />
  <TagTree collapsedDefault />
</nav>
```

### 4.4 DO / DON'T

- ✅ **DO**：用 `useEffectiveLearningSettings()` 读派生值；`hubBadge` 用 `useMemo` 组合 assets / concepts / library 三个 store 的 length；三个数任意为 0 时整体不渲染 badge
- ❌ **DON'T**：不要在 Sidebar 里直接订阅 store 的整张表；只 select 长度。不要新增 `activeSidebarSection` 枚举值（"knowledge-hub" 复用即可）

---

## 05 KnowledgeHub 链条可读化（PHASE 0）

### 5.1 默认落点改为 `concepts`

```ts
// types.ts
export const DEFAULT_HUB_STEP: HubStep = "concepts"; // was "assets"
```

注意 `migrateLegacyHash` 中 `#/skills` / `#/knowledge` 的目标不变；只改默认值不影响迁移矩阵。

### 5.2 StepNav 升级为"链条 + 计数"

- **之前**：4 个平级 chip
- **之后**：带计数与 chevron `›` 的链条 — `素材 120 › 概念 47 › 知识库 12 › 技能 3`

### 5.3 改造点

| ID | 动作 | 位置 |
|----|------|------|
| **KH-01** | StepNav 接受 `counts: Record<HubStep, number>` prop；父组件 useMemo 聚合四个 store 长度后传入 | `KnowledgeHubView/index.tsx` |
| **KH-02** | step 之间插入 `<span aria-hidden>›</span>`，使用 `--text-tertiary` 颜色、12px。键盘可达性靠 button 本身，sep 不参与 tab | 同上 |
| **KH-03** | active step 字重 600，背景 `--surface-tertiary`；inactive 字重 400，hover 时 bg 变 `--surface-secondary`。颜色一律走 token | 同上 |
| **KH-04** | StepNav 容器 padding 由 `py-[--space-2]` 调到 `py-[--space-3]`，给链条一点呼吸；底部 border 保留 | 同上 |
| **KH-05** | 每个 step 增加 `data-step` 属性便于 e2e 测试 selector | 同上 |

### 5.4 Step 内部体验补丁（最小集）

- **AssetsStep** — 顶部加一行说明 chip："120 个素材将提炼出概念 →"
- **ConceptsStep** — 重复/相似概念合并提示用 `--concept-merge-bg / fg` token（已存在）
- **LibraryStep** — 每张知识库卡片右上角显示来源概念数（`n concepts`），点击展开关联
- **SkillsStep** — 空时引导"先在知识库挑选 3 条概念，组合一个 skill →"；带 CTA 跳回上一步

---

## 06 Inspector & Tab 顺序（PHASE 1）

### 6.1 改造点

| ID | 动作 | 位置 |
|----|------|------|
| **IN-01** | `TABS` 数组重排为 `[inspector, knowledge_association, timeline-flow]` | `src/components/layout/Inspector.tsx` |
| **IN-02** | 持久化兼容：若用户上次停在 `timeline-flow`，迁移后仍保留；不强制重置 `rightPanelMode` | `src/stores/uiStore.ts`（本字段未持久化，无需 migrator） |
| **IN-03** | 知识关联面板：默认 toggle"仅显示与当前素材相关"开启；当前选中素材关联的概念置顶 + 浅琥珀条 `--concept-linked-stripe` | `features/knowledge/KnowledgeAssociationView.tsx` |
| **IN-04** | 重复概念合并入口：在每个重复条目右侧加文字按钮"合并"（`data-merge-id`），点击进合并 modal。本期可只放 UI 与 disabled action | 同上 |

---

## 07 悬浮窗 & 空状态系统（PHASE 1）

### 7.1 悬浮窗位置策略

| ID | 动作 | 位置 |
|----|------|------|
| **DZ-01** | 主窗口聚焦时悬浮窗自动半透明 + 退避到屏幕右下角；主窗口失焦后恢复不透明。监听 `window.onfocus/onblur`（Tauri 2 已暴露） | `features/dropzone/DropzoneApp.tsx` + `src-tauri/src/lib.rs` |
| **DZ-02** | 合并冗余 UI：去掉小窗内的"缩放手柄"；拖动改为整条顶部 12px drag region（macOS 习惯），关闭 X 放右上角 | 同上 |
| **DZ-03** | 设置 `dropzonePosition` 默认值改为 `{x: viewport.width-220, y: viewport.height-200}`（首次启动） | `src/stores/settingsStore.ts` |
| **DZ-04** | 悬浮窗 hover 时显示提示 "拖入文件以快速导入"，松开后即时反馈（已有 ToastContainer） | 同上 |

### 7.2 空状态系统

| ID | 动作 | 位置 |
|----|------|------|
| **ES-01** | 抽离 `<EmptyState icon title hint cta? />` 共用组件（已存在 `features/EmptyState.tsx`，需检查 API 是否覆盖 cta 槽） | `features/EmptyState.tsx` |
| **ES-02** | TodayView 顶部计数栏在所有数字为 0 时**整行不渲染**；恢复"有数才出现" | `features/today/TodayView.tsx` |
| **ES-03** | 去掉 emoji 🎉；用 lucide `<Check />` 或不放图标。文案降到 15px 中性陈述 | 同上 |
| **ES-04** | 所有 step / list 空状态统一调用 EmptyState；视觉与文案规范进入 §8 token | 跨文件 |

**禁用文案**：🎉、恭喜、加油、"今天没有 XXX"。空状态不是事件，是状态。

---

## 08 视觉令牌微调（PHASE 2）

### 8.1 新增 / 调整的 token

```css
/* globals.css — 在 :root 中追加（亮色） */
/* 导航选中（替代旧琥珀） */
--sidebar-active-bg: rgba(59, 130, 246, .15);
--sidebar-active-fg: #93c5fd;

/* 链条计数 badge（mono 字号 11px） */
--hub-count-bg: var(--surface-tertiary);
--hub-count-fg: var(--text-tertiary);

/* 琥珀回收为"强调与重复合并" */
--accent-amber: #ea580c;
--accent-amber-soft: #fff7ed;

/* 暗色覆盖 */
@media (prefers-color-scheme: dark) {
  --sidebar-active-bg: rgba(59, 130, 246, .18);
  --accent-amber-soft: #431407;
}
```

### 8.2 收敛规则

| ID | 动作 | 说明 |
|----|------|------|
| **TK-01** | 所有"导航选中态"统一用 `--sidebar-active-*`；删除文件内行内 amber 色 | 影响：Sidebar / StepNav / Inspector seg control |
| **TK-02** | 琥珀仅保留在：① 重复概念合并提示 ② AI 强调框 ③ 时间流图片 zone stripe | 三处共用 `--accent-amber*` |
| **TK-03** | 动效统一收敛到三档：`--duration-instant 100ms` / `--duration-fast 200ms` / `--duration-normal 300ms`。其它 duration 字面量删除 | 影响：SegmentedControl / SidebarItem / Tooltip |
| **TK-04** | 暗色模式同步 `--sidebar-active-bg` 与 `--accent-amber-soft` | 检查对比度 ≥ WCAG AA |

---

## 09 验收清单 & 测试矩阵

### 9.1 用户视角验收

- [ ] 首次启动（`showLearningFeatures=false`），Sidebar 主导航 ≤ 6 项，无任何"复习/课程/技能"字样
- [ ] 开启学习模式后，"学习中心"分组以 200ms 淡入出现，且只包含"今日 + 课程表"两项；不再出现"今天没有课程"占位行
- [ ] 点击"知识"入口，默认进入 KnowledgeHub 的 `concepts` step；URL 显示 `#/knowledge-hub/concepts`
- [ ] StepNav 显示链条"素材 120 › 概念 47 › 知识库 12 › 技能 3"；任一项计数为 0 时该 step 不显示计数
- [ ] TAGS 默认折叠；点击展开后显示过滤输入框；展开状态在重启后保留
- [ ] Inspector tab 顺序为 详情 / 知识关联 / 时间流；切换瞬间无闪烁
- [ ] TodayView 在无任务时只显示一句"今日无待处理 + 引导文案"；顶部 0/0/0 计数栏不渲染
- [ ] 悬浮窗在主窗口聚焦时半透明并退到右下；主窗口失焦立刻恢复
- [ ] 暗色模式下所有上述变化的对比度仍 ≥ WCAG AA

### 9.2 工程验收

- [ ] Sidebar 不再 import `Search` 图标；`activeSidebarSection` 不出现已删除值 `"search"`
- [ ] `migrateLegacySection` 增加用例：传入 `"search"` → 落到 `"recent"`
- [ ] `DEFAULT_HUB_STEP === "concepts"`；`migrateLegacyHash("#/knowledge")` 仍迁移到 `library`
- [ ] StepNav 接受 `counts` prop；`aria-selected`、`role="tablist"` 保留
- [ ] 新 token 全部声明在 `:root` 且暗色模式有对应覆盖
- [ ] 所有受影响组件在 lint + tsc + vitest 下 0 报错
- [ ] `useEffectiveLearningSettings` 在 OFF 状态下派生字段强制 false

### 9.3 测试矩阵

| 文件 | 新增用例 |
|------|----------|
| `Sidebar.test.tsx`（新或扩展） | ① showLearningFeatures=false 时不渲染 Calendar/Today/学习中心；② 开关切换后 200ms 内淡入；③ Search item 不渲染；④ hub badge 在所有计数 > 0 时显示 |
| `KnowledgeHubView.test.tsx`（已存在） | ① 默认 step 为 concepts；② 旧 hash 迁移路径不变；③ counts 透传给 StepNav；④ counts 为 0 不渲染数字 |
| `TodayView.test.tsx`（新或扩展） | ① 全 0 数据下顶部计数栏不渲染；② emoji 🎉 不出现在 DOM |
| `Inspector.test.tsx`（新或扩展） | ① TABS 顺序 inspector/knowledge_association/timeline-flow；② 重排不破坏 rightPanelMode 持久化 |

### 9.4 风险与回滚

| 风险 | 触发条件 | 回滚 |
|------|----------|------|
| 已有用户 `activeSidebarSection` 在 LS 中是 "search" | 升级后 | `migrateLegacySection` 已覆盖（落到 recent） |
| StepNav counts 抖动 | store 数据初始化期间 length=0 | 用 useMemo + isLoading 守卫；首次 hydrate 完成前不渲染 badge |
| 悬浮窗 onfocus 监听跨平台差异 | Tauri 2 在某些 Linux WM 下不触发 | 降级：保留手动拖动 + setInterval 检测 window state（备选，不进 P0） |

---

# Conductor 桥接摘要

## 核心功能清单（带优先级）

| 功能 | 优先级 | 核心用户场景 | 来自 PRD 的关键约束 |
|------|--------|-------------|--------------------|
| Sidebar 收敛（SB-01~07） | **P0** | 首启非学生用户左栏 ≤ 6 项，无"复习/课程/技能"字样 | 用 `useEffectiveLearningSettings()` 派生；不订阅整张 store 表；TAGS 默认折叠且状态持久化 |
| KnowledgeHub 链条化（KH-01~05） | **P0** | 用户进入 KnowledgeHub 默认落 concepts；链条带计数与 chevron | DEFAULT_HUB_STEP 从 assets 改为 concepts；hash 迁移矩阵不变；StepNav 接受 counts prop |
| TODAY 空状态与 Search 出栏位（SB-04, SB-06 同步完成） | **P0** | 无课时整组不渲染；Search 改 Footer 图标 | TODAY 整组消失而非显示"今天没有课程" |
| Inspector tab 重排（IN-01~04） | P1 | tab 顺序：详情 / 知识关联 / 时间流 | rightPanelMode 持久化兼容；合并按钮可放占位 UI |
| 悬浮窗位置策略（DZ-01~04） | P1 | 主窗聚焦时浮窗半透明退避到右下 | 监听 Tauri 2 已暴露的 window focus/blur |
| 空状态系统化（ES-01~04） | P1 | TodayView 全 0 不渲染计数栏；统一 EmptyState 组件；禁用 🎉/恭喜/今天没有 XXX 文案 | 一句话陈述；不用感叹号；统一 EmptyState 调用 |
| 视觉令牌微调（TK-01~04） | P2 | 导航选中色统一冷蓝；琥珀回收为强调色；动效收敛三档 duration | 全部走 CSS var；新增 token 声明 dark mode |

## 不可妥协的技术底线

1. **不重写已有 store**：本次只新增 `uiStore.tagsExpanded` 字段
2. **不动 Rust 命令**：`src-tauri/` 仅允许使用 Tauri 2 已暴露的 window focus/blur 监听
3. **不引新路由库**：保持 `useHubHashRoute.ts` 体系，hash 迁移矩阵不可破坏
4. **零数据零信号**：计数为 0 时不渲染；分组为空不渲染
5. **沿用令牌不发明**：颜色/圆角/阴影必须走 globals.css 已有 CSS var；新增 token 需声明 dark 覆盖
6. **向后兼容 LS 状态**：`migrateLegacySection("search") → "recent"` 等迁移用例不得破坏
7. **禁用感性文案**：禁止 🎉、恭喜、加油、"今天没有 XXX"
8. **暗色对比度 ≥ WCAG AA**

## 已识别的高风险项

| 风险 | 来源 | 当前状态 | 缓解策略 |
|------|------|----------|----------|
| Sidebar 重构波及现有测试 | PRD §9.3 | 待定 | 拆分粒度小，每个改造点单独 task；保留 AppLayout.test 基线 |
| StepNav counts 抖动 | PRD §9.4 | 待定 | useMemo + hydration 守卫，首次未就绪不渲染 badge |
| Tauri 2 window focus/blur 平台差异 | PRD §9.4 | 已搁置（P1） | 主路径走 Tauri API；DZ-01 失败时降级为手动拖动（不进 P0） |
| 老用户 LS 残留 `activeSidebarSection==="search"` | PRD §9.4 | 已解决 | `migrateLegacySection` 已覆盖（落到 recent），加单测确认 |
| dark mode 对比度 | PRD §9.1 末条 | 待定 | TK-04 必须实测 WCAG AA |

## MVP 边界声明

**做什么（v1.3 首发范围）**：
- PHASE 0 全部（SB-01~07、KH-01~05、TODAY 空状态、Search 出栏位）— P0 阻塞
- PHASE 1 全部（IN-01~04、DZ-01~04、ES-01~04）— P1
- PHASE 2 全部（TK-01~04）— P2，时间允许则带入；否则推到 v1.3.1

**不做什么（明确排除）**：
- 不重写 KnowledgeHubView 内部 step 的列表/卡片样式（推到 v1.4）
- 不动 IN-04 的"合并 modal"实际功能（本期仅 UI 占位 + disabled action）
- 不动除"学习中心"以外的 Sidebar 信息架构（PROJECTS 树保持原状）
- 不引入 i18n 库；不动 Rust 业务命令；不重写任何 store
- DZ-01 跨平台兜底（备选 setInterval 方案）不进 P0

## PRD 未达成共识的争议

**无显式争议**。HTML 文档已经过设计方深度思考，唯一一处需要 Architect 决断的是：

> **决策点 A**：`SB-02` 中"知识"入口的 `navigateHub` 目标参数应为 `"library"` 还是 `"concepts"`？  
> HTML 第 4.3 节代码草图写 `navigateHub("library")`，但第 5.1 节又说默认落点应为 `concepts`。两者不一致。  
> **Architect 建议**：以 §5.1 为准，"知识"点击 → `navigateHub("concepts")`，让用户落到链条中段（既能往前看素材、又能往后看库/技能）。代码草图应同步修正。

> **决策点 B**：`uiStore.tagsExpanded` 的 partialize 命名。  
> 现有 uiStore 是否已存在？如不存在需先建立；如已存在需 audit 现有 partialize 字段命名风格。  
> **Architect 行动**：Task 输入前先读取 `src/stores/uiStore.ts` 现状，与 `assetStore` 等的 partialize 风格对齐。

---

**PRD 终稿就绪。Conductor 可启动 ARCHITECTURE 状态。**
