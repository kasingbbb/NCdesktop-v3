# Architect 技术方案 — NoteCapt Desktop v1.3 主界面收敛

## 项目概述

将主界面从"功能清单铺平"收敛到"工作区 × 知识链条"双轴心智，让非学生用户首启 60s 内只看到必要的事。改造点全部位于 `src/components/layout/*`、`src/components/features/KnowledgeHubView/*`、`src/stores/uiStore.ts`、`src/styles/globals.css` 几处，不动 Rust 业务命令、不重写已有 store 结构、不引新路由库。本期工作量集中在 React UI 层 + CSS token 层。

---

## 需求理解确认（自检）

1. **PRD 核心需求**（用我自己的话复述）：
   - **PHASE 0（P0）**：把 Sidebar 从 13 项收敛到 ≤ 6 项；把 KnowledgeHub 4-step 从平级 chip 改成"带 chevron + counts"的链条；把 Search 从 Sidebar 主体移到 Footer；TODAY 整组在无课时不渲染；新增 `uiStore.tagsExpanded` 让 TAGS 默认折叠
   - **PHASE 1（P1）**：Inspector tab 顺序改为 详情 / 知识关联 / 时间流；EmptyState 共用组件 + TodayView 修整；Dropzone 主窗聚焦时半透明退避
   - **PHASE 2（P2）**：导航选中色统一冷蓝 token，琥珀回收为强调色，动效收敛三档

2. **硬约束**（从 session_context.md §3 提取）：
   - 不重写已有 store
   - 不动 Rust 命令（仅允许 Tauri 2 已暴露的 window focus/blur 监听）
   - 不引新路由库
   - 零数据零信号（计数 0 不渲染 badge；空 section 不渲染）
   - 沿用令牌不发明（颜色 / 圆角 / 阴影一律 CSS var）
   - 向后兼容 LS 状态（`migrateLegacySection("search") → "recent"` 不破）
   - 禁用感性文案

3. **高风险项**（来自 PRD §9.4 + Bridge Summary）：
   - StepNav counts 抖动（store hydration 期间 length=0 → 误显示 0·0·0）
   - Tauri 2 window focus/blur 跨平台差异（DZ-01 备选方案不进 P0）
   - 暗色对比度（必须 WCAG AA）
   - 老用户 LS 残留 → migrateLegacySection 已覆盖（加单测验证）

4. **技术决策点**（需 Architect 决断）：
   - **决策点 A**：Sidebar "知识" 入口的 `navigateHub` 目标（library vs concepts）→ 见 ADR-001
   - **决策点 B**：`uiStore.tagsExpanded` 字段命名与持久化策略 → 见 ADR-002
   - **决策点 C**：Sidebar 学生态"学习中心"分组与原 Calendar / 今日复习 / TODAY 三处的关系 → 见 ADR-003
   - **决策点 D**：StepNav counts 抖动的守卫策略 → 见 ADR-004
   - **决策点 E**：Tauri window focus/blur 监听的封装位置 → 见 ADR-005

---

## 技术选型

本项目沿用现有技术栈，**本次迭代不引入任何新依赖**：

| 维度 | 选型 | 备注 |
|------|------|------|
| UI 框架 | React 19 | 已有 |
| 桌面壳 | Tauri 2 | 已有，仅消费 window 事件 API |
| 样式 | Tailwind v4 + CSS Variables | 全部走 globals.css 中的 token |
| 状态 | Zustand + persist | uiStore 已有 partialize/migrate |
| 路由 | 自研 `useHubHashRoute` (hash-based) | 不引外部路由库 |
| 图标 | lucide-react | 已有 |
| 测试 | Vitest + @testing-library/react | 已有 |

---

## Architecture Decision Records

### ADR-001：Sidebar "知识" 入口 navigateHub 目标 = `concepts`

- **状态**：已接受
- **上下文**：PRD §4.3 SB-02 与 §5.1 之间存在轻度冲突。§4.3 代码草图写 `navigateHub("library")`，§5.1 又说默认落点应为 `concepts`
- **决策**：以 §5.1 为准。"知识"入口点击 → `setSidebarSection("knowledge-hub")` + `navigateHub("concepts")`；同时把 `DEFAULT_HUB_STEP` 从 `assets` 改为 `concepts`
- **被排除项**：
  - `library`：用户从"知识"入口跳转，落到"知识库"在心智上属于"下一步"，会让用户错以为前序素材/概念已无关联
  - `assets`：太早期，违背 PRD 北极星"工作区 × 知识链条"——assets 仍属于"工作区"心智
- **后果**：
  - `concepts` step 必须能 standalone 渲染（已确认：`<ConceptsStep />` 无 prop 依赖）
  - `useHubHashRoute` 在无 hash 或空 hash 时默认落 concepts（修改 `DEFAULT_HUB_STEP` 即可，hook 不动）

### ADR-002：`uiStore.tagsExpanded` 字段命名与持久化

- **状态**：已接受
- **上下文**：PRD SB-07 要求新增 `uiStore.tagsExpanded:boolean`，默认 false，partialize 持久化
- **决策**：
  - 字段名：`tagsExpanded`（与现有 `inspectorOpen`、`todayLastTab` 命名风格对齐——小驼峰，名词在前）
  - setter：`setTagsExpanded(expanded: boolean)`
  - 默认值：`false`
  - 持久化：进 partialize；migrate 函数对缺失字段返回 `false`（不需要 version bump，新字段缺失走默认值即可——但为保险起见我们把 version 维持在 1，让 migrate 函数显式返回 tagsExpanded）
- **被排除项**：
  - `tagsCollapsed`（反向命名）：与 PRD 描述方向不一致，且 setter 名变扭
  - 不持久化（瞬态）：违反 PRD §9.1 "展开状态在重启后保留"
- **后果**：
  - uiStore 的 UIStore interface 新增 `tagsExpanded` + `setTagsExpanded`
  - partialize 增列 `tagsExpanded`
  - migrate 函数补一行 `tagsExpanded: Boolean((persisted as ...)?.tagsExpanded ?? false)`

### ADR-003：学生态"学习中心"分组与原 Calendar / 今日复习 / TODAY 三处的合并

- **状态**：已接受
- **上下文**：PRD §4.2 学生态截图显示，开启学习模式后在"知识"与 PROJECTS 之间插入"**学习中心**"分组，含"今日 + 课程表"两项。当前 Sidebar 在 `showLearningFeatures` 真时分别渲染 `Calendar` SidebarItem、`今日复习` SidebarItem、和单独的 `TODAY` SidebarSection
- **决策**：
  - 删除原 `Calendar` SidebarItem（位置 src/components/layout/Sidebar.tsx:81-88）
  - 删除原 `今日复习` SidebarItem（src/components/layout/Sidebar.tsx:89-96）
  - 删除原独立的 `TODAY` SidebarSection（src/components/layout/Sidebar.tsx:117-126）—— 这是占位"今天没有课程"的所在
  - 在合并后的"知识"入口下方、ProjectTree 之前，新增 `<SidebarSection title="学习中心">`，仅当 `showLearningFeatures === true` 时渲染
  - "学习中心"分组内含：
    - `<SidebarItem icon={<Sun/>} label="今日" badge={todayCount} />`——仅当今日任务数 > 0 渲染（按 PRD §9.1 "无任务时不出现"）。点击 → `setSidebarSection("today")`
    - `<SidebarItem icon={<CalendarDays/>} label="课程表" />`——总是渲染。点击 → `setSidebarSection("calendar")`
  - 分组容器添加 className `sidebar-learning-fade-in`，由 globals.css 提供 200ms fade-in keyframe（已有现成 class 即可，否则新增）
- **被排除项**：
  - "保留旧三项 + 不开学习中心分组"：违背 PRD §4.2 设计意图
  - "今日 badge 显示 0"：违反 P-04 零数据零信号
- **后果**：
  - `useEffectiveLearningSettings` 已存在，直接消费；不引入新派生
  - 今日任务数来源：复用 `TodayView.tsx` 同款数据 hook，或新建薄包装 `useTodayCount()`——本期建议直接读 store
  - Calendar SidebarItem 删除后，`activeSidebarSection === "calendar"` 仍合法（点击"课程表"切换），不需删 union member

### ADR-004：StepNav counts 抖动守卫

- **状态**：已接受
- **上下文**：风险登记表显示，store 初始化期间所有 length=0，会导致 StepNav 闪一下"0 › 0 › 0 › 0"
- **决策**：
  - 父组件 `KnowledgeHubView/index.tsx` 用 `useMemo` 聚合四个 store 的 length：
    ```ts
    const counts = useMemo<Record<HubStep, number>>(() => ({
      assets: assetCount,
      concepts: conceptCount,
      library: libraryCount,
      skills: skillCount,
    }), [assetCount, conceptCount, libraryCount, skillCount]);
    ```
  - **不引入 isLoading 守卫**：理由是 store hydration 在 React mount 之前已完成（zustand persist 是同步 rehydrate），抖动主要来自后端数据异步加载。本期接受"加载中 count=0 不显示数字"——这正好符合 P-04 零数据零信号
  - **counts 为 0 的处理**：StepNav 渲染时，count > 0 才渲染 `<span class="ct">{n}</span>`；count === 0 时仅显示 step label，不显示数字
- **被排除项**：
  - 显式 isLoading prop + spinner：过度工程，违背 P-04
  - 把 counts hydration 卡在 Suspense 边界：成本过高，且 zustand 不天然支持
- **后果**：
  - StepNav 接受 `counts?: Partial<Record<HubStep, number>>`（可选；不传时退化为旧行为）
  - 父组件不传 counts 时，StepNav 不渲染任何 count，向后兼容性好

### ADR-005：Tauri window focus/blur 监听封装

- **状态**：已接受
- **上下文**：DZ-01 要求 Dropzone 在主窗聚焦时半透明退避。Tauri 2 已暴露 `window.onFocusChanged` 事件 API
- **决策**：
  - 在 `features/dropzone/DropzoneApp.tsx` 内消费 `getCurrent().onFocusChanged(({ payload: focused }) => ...)`，**不抽通用 hook**——理由是当前全应用只有 Dropzone 这一处消费该事件
  - 监听器返回 unlisten 函数，必须在 `useEffect` cleanup 中调用
  - 半透明阈值由 CSS opacity 实现，不操作 native window opacity（避免触发 Tauri 平台差异）
  - 退避位置：写入 `settingsStore.dropzonePosition = { x: viewport.width-220, y: viewport.height-200 }`
- **被排除项**：
  - 抽 `useTauriWindowFocus()` hook：本期仅一处消费，YAGNI
  - 用 native window opacity（`getCurrent().setOpacity()`）：跨平台不稳定
  - `setInterval` 轮询 window 状态：留作 fallback，不进 P0
- **后果**：
  - DropzoneApp.tsx 多一段 `useEffect` 监听 + cleanup
  - settingsStore 的 `dropzonePosition` 字段已存在（按 PRD DZ-03 "默认值改为..."暗示），如不存在则新增

---

## 系统架构

### 模块边界与依赖

```
┌──────────────────────────────────────────────┐
│  src/styles/globals.css                      │
│  - 新增 token：sidebar-active-*, hub-count-* │
│  - 暗色 mode 覆盖                            │
└─────────────────────┬────────────────────────┘
                      │ (CSS var 全局可用)
                      ▼
┌──────────────────────────────────────────────┐
│  src/stores/uiStore.ts                       │
│  - 新增 tagsExpanded + setter + partialize   │
│  - migrate 函数补字段                        │
└─────────────────────┬────────────────────────┘
                      │
       ┌──────────────┼──────────────────────┐
       ▼              ▼                      ▼
┌──────────┐  ┌──────────────────┐  ┌────────────────────┐
│ Sidebar  │  │ KnowledgeHubView │  │ Inspector / Today  │
│ (重构)   │  │ - DEFAULT_STEP   │  │ - tab reorder      │
│          │  │ - StepNav 升级   │  │ - EmptyState 替换  │
└──────────┘  └──────────────────┘  └────────────────────┘
       │
       └──→ TagTree (消费 tagsExpanded)
       └──→ SidebarFooter (重构为单行三段)

┌──────────────────────────────────────────────┐
│  features/dropzone/DropzoneApp.tsx           │
│  - Tauri window focus 监听                   │
│  - 退避到右下                                │
└──────────────────────────────────────────────┘
```

### 关键不变式

- **不动 Rust**：所有改造仅在 TypeScript / TSX / CSS 层。`src-tauri/` 唯一被读取的是 Tauri 2 已暴露的 `getCurrent().onFocusChanged` API
- **uiStore.activeSidebarSection union 保持不变**：本次不引入新 section 值（"knowledge-hub"、"today"、"calendar" 全部复用）
- **`useHubHashRoute` 不动**：只改 `DEFAULT_HUB_STEP` 常量
- **migrateLegacy 链路不动**：`migrateLegacySection`、`migrateLegacyHash` 现有逻辑不修改，但要补单测验证 "search" 用例

---

## 数据模型变更

仅 1 处：`uiStore.UIStore` interface 与 store state 新增：
```ts
tagsExpanded: boolean;
setTagsExpanded: (expanded: boolean) => void;
```
partialize 增列 `tagsExpanded`；migrate 函数补 `tagsExpanded: Boolean(persisted?.tagsExpanded ?? false)`。**version 不变**（保持 1）—— 缺失字段走 migrate 默认。

---

## API 设计

本项目为纯前端 UI 迭代，**无后端 API 变化**。前端组件 prop 接口变化集中在 `StepNav`：
```ts
interface StepNavProps {
  steps: readonly HubStep[];
  current: HubStep;
  onSelect: (next: HubStep) => void;
  counts?: Partial<Record<HubStep, number>>; // 新增
}
```

`EmptyState` 组件 API 待 task_010 中根据现有 `features/EmptyState.tsx` 实际签名决定是否扩展 `cta` 槽。

---

## 目录结构（变更范围）

```
src/
├── components/
│   ├── layout/
│   │   ├── Sidebar.tsx                         ← 重写（task_004/005）
│   │   ├── SidebarFooter.tsx                   ← 重构（task_003）
│   │   ├── SidebarItem.tsx                     ← 可能微调（badge 槽）
│   │   ├── Inspector.tsx                       ← tab 重排（task_008）
│   ├── features/
│   │   ├── KnowledgeHubView/
│   │   │   ├── index.tsx                       ← StepNav 升级 + counts 父级聚合（task_007）
│   │   │   ├── types.ts                        ← DEFAULT_HUB_STEP 改 concepts（task_007）
│   │   │   ├── KnowledgeHubView.test.tsx       ← 新增 4 个用例（task_007）
│   │   ├── EmptyState.tsx                      ← 检查 + 可能加 cta 槽（task_010）
│   │   ├── TagTree.tsx                         ← 默认折叠 + 过滤输入（task_006）
│   │   ├── today/TodayView.tsx                 ← 顶部计数栏零渲染 + 去 🎉（task_010）
│   │   ├── knowledge/KnowledgeAssociationView.tsx  ← toggle 默认开 + 浅琥珀条（task_009）
│   │   ├── dropzone/DropzoneApp.tsx            ← focus/blur 监听（task_011）
│   │   ├── __tests__/
│   │   │   ├── Sidebar.test.tsx                ← 新增（task_004/005）
│   │   │   ├── TagTree.test.tsx                ← 扩展（task_006）
│   │   │   ├── EmptyState.test.tsx             ← 新增（task_010）
│   ├── ...
├── stores/
│   ├── uiStore.ts                              ← 加 tagsExpanded（task_002）
│   ├── settingsStore.ts                        ← dropzonePosition 默认（task_011）
│   ├── __tests__/uiStore.test.ts               ← 扩展，加 search→recent 单测（task_002）
├── styles/
│   ├── globals.css                             ← 新增 token + dark 覆盖（task_012）
```

---

## 安全考量

本期纯前端 UI 改造，无认证 / 无新 API / 无序列化反序列化新接口。安全敏感度为 **低**。需要保留的安全实践：

1. **不引入 XSS 风险**：所有用户输入（TAGS 搜索框、合并 modal 名称）使用 React 默认的 escape，不调用 `dangerouslySetInnerHTML`
2. **不暴露 store 内部状态**：通过 selector 订阅，避免泄漏未脱敏字段
3. **CSP 不变**：不引入新的 inline script / inline style 来源

---

## 风险登记表

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| StepNav counts 在 hydration 期间显示 0·0·0 | 中 | 中 | ADR-004：count === 0 时仅显示 step label，本身就符合 P-04 |
| 老用户 LS `activeSidebarSection === "search"` | 高（已有发布版本） | 低 | `migrateLegacySection` 已覆盖；task_002 加单测验证 |
| Tauri window focus 事件 Linux WM 不触发 | 低（macOS 优先） | 中 | DZ-01 ADR-005 仅作主路径；备选方案不进 P0 |
| 暗色模式对比度不达标 | 中 | 中 | task_012 必须实测 WCAG AA |
| Sidebar 重构破坏 AppLayout.test 基线 | 中 | 中 | 每个 P0 task 之后跑 `pnpm test`；Sidebar 重构 task 配套新建 Sidebar.test.tsx |
| Inspector tab 重排导致 rightPanelMode 持久化失效 | 低 | 低 | IN-02 ADR：`rightPanelMode` 字段未持久化（已确认 uiStore 当前 partialize 不含此项），所以重排无需 migrator |
| 学生态"学习中心"分组中"今日"badge 与 TodayView 数据不一致 | 中 | 低 | task_005 中明确"今日"badge 数据源 = TodayView 同款 hook，单测验证 |

---

## Task 清单

按 PHASE 0/1/2 串行交付。Task 编号对应 PRD 改造点 ID 集合。

### PHASE 0 — 主界面收敛（P0，首发阻塞）

| Task ID | 目标 | 涉及 PRD 改造点 |
|---------|------|-----------------|
| **task_002** | uiStore 新增 `tagsExpanded` 字段、setter、partialize、migrate；扩展 uiStore.test 加 search→recent + 字段默认 false 用例 | SB-07 |
| **task_003** | Sidebar 移除 Search 项；SidebarFooter 改为单行三段（⌘K 搜索 · ⚙ 设置 · TF 状态点） | SB-01, SB-06 |
| **task_004** | Sidebar 合并知识库+技能为"知识"入口；右侧 hub badge `素材·概念·库`（任一为 0 整条不渲染）；点击 navigateHub("concepts") | SB-02, SB-03 + ADR-001 |
| **task_005** | Sidebar 学生态：删除 Calendar / 今日复习 SidebarItem 与独立 TODAY 区；新增"学习中心"分组（200ms fade-in），含"今日"+"课程表"两项；今日 badge 仅在 todayCount>0 时渲染 | SB-04 + ADR-003 |
| **task_006** | TagTree 默认折叠（由 `uiStore.tagsExpanded` 控制）；展开后顶部带过滤输入框（placeholder="过滤标签"） | SB-05 |
| **task_007** | DEFAULT_HUB_STEP 改 concepts；StepNav 升级为链条（chevron sep + counts），父组件 useMemo 聚合四个 store 长度透传；padding 调整；data-step 属性；扩展 KnowledgeHubView.test 加 4 个用例 | KH-01 ~ KH-05 + ADR-001/004 |

### PHASE 1 — 细节体感（P1）

| Task ID | 目标 | 涉及 PRD 改造点 |
|---------|------|-----------------|
| **task_008** | Inspector TABS 数组重排为 [inspector, knowledge_association, timeline-flow]；新增 Inspector.test 验证顺序 | IN-01, IN-02 |
| **task_009** | KnowledgeAssociationView 默认 toggle "仅显示与当前素材相关" 开启；相关概念置顶 + 浅琥珀条 `--concept-linked-stripe`；重复条目右侧加"合并"按钮（UI 占位 + disabled action + data-merge-id） | IN-03, IN-04 |
| **task_010** | EmptyState 组件 audit + 加 cta 槽（如需）；TodayView 顶部计数栏全 0 时整行不渲染；去掉 emoji 🎉；所有 step / list 空状态统一调用 EmptyState | ES-01 ~ ES-04 |
| **task_011** | DropzoneApp 监听 Tauri 2 window focus/blur；聚焦时 opacity .45 + 退避到右下；失焦时恢复；hover 提示"拖入文件以快速导入"；settingsStore.dropzonePosition 默认值改为右下 | DZ-01 ~ DZ-04 + ADR-005 |

### PHASE 2 — 视觉打磨（P2）

| Task ID | 目标 | 涉及 PRD 改造点 |
|---------|------|-----------------|
| **task_012** | globals.css 新增 sidebar-active-*、hub-count-* token + 暗色覆盖；删除项目内所有行内 amber 引用；琥珀仅保留三处（重复概念合并 / AI 强调框 / 时间流图片 zone stripe）；动效收敛三档 duration | TK-01 ~ TK-04 |

### 收尾

| Task ID | 目标 |
|---------|------|
| **task_013** | UX 体验审查（对应 PRD §9.1 用户视角验收 9 条），输出 ux_scorecard.md |

---

## Task 依赖拓扑

```
task_002 (uiStore.tagsExpanded)
    │
    ├──→ task_006 (TagTree 折叠 consume tagsExpanded)
    │
task_003 (Sidebar Search 删 + Footer)  ──┐
                                          ├──→ task_004 (Sidebar 知识合并)
                                          │      │
                                          │      └──→ task_005 (Sidebar 学习中心分组)
                                          │
task_007 (HUB 链条化) [独立]              │
                                          │
task_008 (Inspector tabs) [独立]          │
task_009 (Inspector 知识关联) [独立]      │
task_010 (EmptyState + TodayView)         │
task_011 (Dropzone focus)                 │
                                          │
                          ▼               ▼
                       task_012 (令牌微调 + 全局清理)
                                          │
                                          ▼
                       task_013 (UX 体验审查)

可并行执行：
  - task_003 ↔ task_007（不同模块）
  - task_008 ↔ task_009 ↔ task_010 ↔ task_011（不同模块）
  - 串行执行：task_002 → task_006；task_004 → task_005；task_012 必须最后
```

### 推荐合并顺序（PR 节奏）

按 PRD §9.4 "P0 拆三个 PR" 建议：
- **PR-A（task_002 + task_006）**：uiStore + TagTree 折叠（最小，先合）
- **PR-B（task_003 + task_004 + task_005）**：Sidebar 整体重构（中等，对应 PRD 中"Sidebar 重构"）
- **PR-C（task_007）**：KnowledgeHubView 链条化
- **PR-D（task_008 ~ 011）**：PHASE 1 细节体感（可拆 4 个小 PR 或合 1 个）
- **PR-E（task_012）**：视觉令牌
- **PR-F（task_013）**：UX 审查后的回填

---

## Task 粒度自检

对每个 P0 task 做自检：

| Task | 单一目标 | 可独立测试 | 规模适中（<2000 行） | 依赖清晰 | AC 可验证 |
|------|----------|------------|----------------------|----------|-----------|
| task_002 | ✅ uiStore 字段 | ✅ uiStore.test | ✅ <50 行 | ✅ 无 | ✅ getState().tagsExpanded === false |
| task_003 | ✅ 删 Search + Footer | ✅ Sidebar.test | ✅ <100 行 | ✅ 无 | ✅ DOM 不含 Search 文本 |
| task_004 | ✅ 知识入口合并 | ✅ Sidebar.test | ✅ <200 行 | ✅ task_003 完成 | ✅ DOM 只有一个"知识"项 + hubBadge 渲染条件 |
| task_005 | ✅ 学习中心分组 | ✅ Sidebar.test | ✅ <200 行 | ✅ task_004 完成 | ✅ showLearningFeatures 切换前后 DOM diff |
| task_006 | ✅ TAGS 折叠 | ✅ TagTree.test | ✅ <150 行 | ✅ task_002 完成 | ✅ 默认 collapsed；点击 toggle；filter input 出现 |
| task_007 | ✅ Hub 链条化 | ✅ KnowledgeHubView.test | ✅ <250 行 | ✅ 无 | ✅ counts 透传；DEFAULT_HUB_STEP === concepts |

全部通过粒度自检。

---

## 启动协议质量闸门

按 bootstrap.md §"质量闸门"：
- [x] session_context.md 所有必填字段已填写
- [x] 复杂度等级已判定（L）
- [x] PRD 已产出（`product/prd/notecapt-v1.3-ui_prd_v1.md`）
- [ ] **PM 确认可以开始编码** ← 等待此处

**Architect 交付完成。等待 PM 确认后进入 TASK_START 状态（首个 task: task_002）。**
