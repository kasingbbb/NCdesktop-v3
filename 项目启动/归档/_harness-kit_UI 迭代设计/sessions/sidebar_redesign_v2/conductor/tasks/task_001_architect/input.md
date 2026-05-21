# Task 输入 — task_001_architect

## 目标

把 PRD v1.0 的 **P0 12 项功能（F-P0-1 ~ F-P0-12）** 拆解为可被 Dev 单 Agent 单会话内独立完成的子 task 清单，并为每个子 task 产出符合 `core/handoff_contracts.md §2` 的独立 `input.md`，写入技术方案 `output.md`（ADR + 目录结构 + 数据/接口模型 + 风险登记表 + Task 依赖拓扑）。

> 你**不要做** P1/P2 的设计。P1/P2 在 v2.0 首发后单独排期。
> 你**不要写业务代码**。本 task 唯一交付物是 `output.md` + 子 task 的 `input.md` 集合。

## 前置条件

- 依赖 task：无（这是 ARCHITECTURE 状态的首个 task）
- 必须先存在的文件/接口：
  - `sessions/sidebar_redesign_v2/prd/notecapt_sidebar_v2_prd_v1.md`（PRD 唯一真相源）
  - `sessions/sidebar_redesign_v2/session_context.md`（项目上下文 + 代码规范）
  - `sessions/sidebar_redesign_v2/debate/session_001/debate_conclusions.md`（如对 PRD 中某项约束动机有疑问时回查）
  - target codebase：`项目启动/NCdesktop/src/` 现状（你需要先实际读一遍现有 store/视图层代码再设计迁移方案）

## 验收标准（Acceptance Criteria）

### AC-A1 · 输出物完整性
1. 文件 `sessions/sidebar_redesign_v2/conductor/tasks/task_001_architect/output.md` 已产出，结构遵守 `roles/conductor/architect/prompt.md` 中定义的章节模板（项目概述 / 技术选型 / ADR / 系统架构 / 数据模型 / API 设计 / 目录结构 / 安全考量 / 风险登记表 / Task 清单 / Task 依赖拓扑）。
2. 每个子 task（task_002 ~ task_NNN）都有独立目录 `tasks/task_00N_xxx/input.md`，且每份 input.md 包含 handoff_contracts §2 的 6 个必填字段：目标 / 前置条件 / 验收标准 / 技术约束 / 参考文件 / 预估影响范围。

### AC-A2 · P0 覆盖率
3. P0 12 项功能 F-P0-1 ~ F-P0-12 在子 task 清单中**逐项可追溯**（output.md 内提供「PRD 功能 ID → 子 task ID」映射表，无遗漏、无越界到 P1/P2）。

### AC-A3 · 依赖时序遵守
4. 子 task 的依赖拓扑严格遵守 PRD §7 的 A→B→C→D→E→F→G 顺序（uiStore → settingsStore → Sidebar/Footer/TitleBar 视图层 → KnowledgeHubView+路由 → AppLayout 状态机 → SettingsPanel UI → 测试），任何标注「可并行」的 task 必须在 output.md 中给出明确论证（不会引入 R8 churn）。
5. F-P0-11（TagTree 最简 cap）若与 C 段并行，必须证明它不读写 uiStore/settingsStore 的新字段，且不依赖新 KnowledgeHubView。

### AC-A4 · 不可妥协底线写入设计
6. 以下 9 条技术底线在对应子 task 的「技术约束」字段中**显式重述**（不许只写"见 PRD"），让 Dev Agent 不需要回查 PRD 即可看到约束：
   - (1) 关学习模式不删任何用户数据（复习记录 / 课程关联 / 概念关联保留）
   - (2) 旧 LocalStorage 启动不报错；任何未知 section 值降级为 `recent`
   - (3) 默认态可见项 ≤ 7（按 PRD §10 「分组标题 + 顶层 SidebarItem」口径）
   - (4) 搜索不进 SidebarItem
   - (5) DEV 调试输出仅在 `import.meta.env.DEV` 下；prod 必须静默
   - (6) 不引入新基础颜色，仅新增语义令牌挂 `globals.css :root`
   - (7) 不引入新状态库 / 新路由库 / 新动画库
   - (8) 关学习模式状态机时序固定：**先路由跳转 → 下一帧 toggle 字段**，不可颠倒
   - (9) KnowledgeHubView 浏览器前进后退必须可用（pushState + popstate 双向同步）

### AC-A5 · 关键 ADR 存在
7. 至少为以下决策点产出独立 ADR（每个 ADR 含「上下文 / 决策 / 被排除项 / 后果」四段）：
   - ADR · `SidebarSection` 的 union type 定义与 `migrateLegacySection()` 的返回值矩阵（含 `"knowledge"|"skills"|"search"` 及未知值的迁移目标）
   - ADR · settingsStore 三字段（`showLearningFeatures` / `bindSchoolCalendar` / `enableDailyReviewReminder`）的依赖强约束实现位置（写入端 setter 拦截 vs. 读取端派生 vs. middleware）—— 必须保证主开关 OFF 时依赖字段「值不丢、读时强制 OFF」
   - ADR · 升级智能开启检测的判定信号（`reviewState` / `courseAssociation` 的具体路径与触发条件）以及只触发一次的标记位
   - ADR · KnowledgeHub 的 hash route 形态（`#/knowledge-hub/:step`）与旧 hash（`#/skills` / `#/knowledge`）的重定向实现层（路由层拦截 vs. 组件层 useEffect）
   - ADR · AppLayout 状态机回退的具体实现机制（Zustand `subscribe` vs. React `useEffect` 监听）以及「下一帧」的实现（`requestAnimationFrame` vs. `setTimeout(0)` vs. `flushSync`）

### AC-A6 · 测试 task 验收标准前置
8. F-P0-12（自动化测试）对应的子 task 在其 input.md 中显式列出至少 10 个旧值兼容迁移用例矩阵（含 `"knowledge"`/`"skills"`/`"search"`/未知字符串/`null`/`undefined`/类型错误等）+ 状态机回退用例矩阵（today→OFF、calendar→OFF、recent→OFF、knowledge-hub→OFF 等），不要把"补测试"留作 Dev 自由发挥。

### AC-A7 · 修改文件白名单收敛
9. 每个子 task 的「预估影响范围」列出的修改文件，必须是 session_context §8 + PM 启动指令中允许的范围内：
   - `src/components/layout/{Sidebar,SidebarFooter,TitleBar,AppLayout,Inspector}.tsx`
   - `src/components/features/{SettingsPanel,TagTree,today/*,knowledge/*,skills/*}`
   - `src/store/uiStore.ts` / `src/store/settingsStore.ts`（推断路径，Architect 须实际读现有代码确认）
   - 新建：`src/components/features/KnowledgeHubView/`（具体子结构由 ADR 决定）
   - `src/styles/globals.css`（仅新增语义令牌）
   - 测试文件路径（按现有 vitest 约定）
   超出白名单的修改必须在 output.md 中显式申请并说明必要性。

## 技术约束

### 来自 session_context.md（不可妥协）
- **技术栈**：React 18 + Vite + Zustand + Tauri 2；**不引入新状态库 / 新路由库 / 新动画库**
- **路由**：hash route，浏览器前后退必须可用
- **类型**：TypeScript 严格模式；store 字段写 union type 而非 string
- **持久化**：Zustand `persist` middleware；新增持久化字段必须显式加入 persist 白名单
- **a11y**：学习模式相关 UI 用**条件渲染**而非 `display:none`（避免 a11y 树污染）
- **样式**：不引入新基础颜色，新令牌挂 `globals.css :root`，语义命名

### 来自 PRD §8（拒绝清单，违反任一直接 PR Reject）
- ❌ 再加一个 SidebarItem 入口
- ❌ 把知识库和技能再变回两个 Tab
- ❌ 学习模式默认开启（除非升级检测到学习数据）
- ❌ 为新功能再加新顶层 section
- ❌ TagTree 全部展开
- ❌ 把搜索放回 Sidebar

### 本 task 自身的过程约束
- 在产出方案前，必须**实际读一遍** `项目启动/NCdesktop/src/` 下现有的 `uiStore` / `settingsStore` / `Sidebar.tsx` / `AppLayout.tsx` / `KnowledgeLibraryView` / `SkillsView` 现状（不要凭 PRD 描述脑补）。
- ADR 的「被排除项」字段不可空 —— 必须写出至少一个被否决的备选方案及理由。
- Task 粒度自检（来自 architect/prompt.md）：单一目标 / 可独立测试 / 单 Agent 单会话内 / 依赖清晰 / AC 可验证 —— 五项任一不满足必须继续拆。
- **绝不**读 `归档/` 文件夹下的任何文件。

## 参考文件

### 必读（按顺序）
- `sessions/sidebar_redesign_v2/prd/notecapt_sidebar_v2_prd_v1.md` — 唯一真相源；重点读「Conductor 桥接摘要」、§3 P0 表、§6 验收清单 AC-1~AC-13、§7 task 依赖时序、§10 Glossary、§11 视觉令牌
- `sessions/sidebar_redesign_v2/session_context.md` — §3 关键约束、§5 代码规范、§6 审查重点
- `sessions/sidebar_redesign_v2/debate/session_001/debate_conclusions.md` — 当对 PRD 某约束动机存疑时回查（不要自行裁定）

### 必读（target codebase 现状，路径以白名单为准）
- `项目启动/NCdesktop/src/store/uiStore.ts`（确认现有 section 字段名/枚举/persist 白名单）
- `项目启动/NCdesktop/src/store/settingsStore.ts`（确认现有 settings 字段结构与持久化方式）
- `项目启动/NCdesktop/src/components/layout/Sidebar.tsx` 现状（统计当前 15 项的具体来源）
- `项目启动/NCdesktop/src/components/layout/AppLayout.tsx` 现状（确认现有路由/section 监听机制）
- `项目启动/NCdesktop/src/components/features/{KnowledgeLibraryView,SkillsView}` 现状（合并前先弄清两者的输入/输出/数据源）
- `项目启动/NCdesktop/src/styles/globals.css`（确认现有 token 命名风格）

### 不要读
- `归档/` 文件夹下的任何文件（PM 明令）

## 预估影响范围

- **新建文件**：
  - `sessions/sidebar_redesign_v2/conductor/tasks/task_001_architect/output.md`（技术方案主文档）
  - `sessions/sidebar_redesign_v2/conductor/tasks/task_00N_*/input.md`（每个子 task 一份，预估 8-10 个子 task）
- **修改文件**：
  - `sessions/sidebar_redesign_v2/conductor/progress.md`（追加 Architect 拆解出的待执行 task 清单 + 依赖拓扑）—— **由 Conductor 在收到 Architect 交付后更新，不由 Architect 直接写**
- **不修改任何 NCdesktop 业务代码**（业务代码改动是 Dev 阶段的事）

---

## 交付完成信号

当 `output.md` 完成且每个子 task 的 `input.md` 已写入时，输出一条简短的完成声明，包含：
1. 子 task 总数与依赖拓扑摘要（A→B→C→D→E→F→G 各阶段对应哪些 task_id）
2. 「PRD 功能 ID → 子 task ID」映射表（用以让 Conductor 一眼核对 P0 覆盖率）
3. 任何在拆解过程中发现的 PRD 歧义 —— 不要自行裁定，列出来交回 Conductor / PM。
