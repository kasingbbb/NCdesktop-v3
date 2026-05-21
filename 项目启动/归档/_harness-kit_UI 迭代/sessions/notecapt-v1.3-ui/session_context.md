# Session Context — NoteCapt Desktop · 主界面收敛 v1.3

> 项目特定上下文配置。所有角色 Prompt 必须从本文件读取领域信息，不得自行假设。

---

## 1. 项目信息 [必填]

- **项目名称**：NoteCapt Desktop — 主界面收敛 v1.3
- **一句话描述**：在已有 KnowledgeHub 聚合视图与"学习模式开关"基础上，把主界面收敛到"工作区 × 知识链条"双轴，为 NotebookLM 全人群首发可用做最后一刀
- **项目类型**：Desktop App（Tauri 2 + React 19）
- **复杂度等级**：**L**（UI 是产品核心；25+ 改造点；跨 Sidebar / KnowledgeHub / Inspector / Dropzone / Settings / globals.css 多个模块；首发阻塞项）
- **PRD 来源**：用户提供 `NoteCapt Iteration Spec v1.html`（已落地为 `product/prd/notecapt-v1.3-ui_prd_v1.md`），跳过 Debate 阶段

---

## 2. 技术上下文 [必填]

- **主语言**：TypeScript（严格模式）
- **框架/运行时**：
  - 前端：React 19 + Vite + Tauri 2 桌面壳
  - 样式：Tailwind v4 + 设计令牌（`src/styles/globals.css` 中的 CSS variables）
  - 状态：Zustand（`src/stores/*.ts`，带 persist partialize）
  - 测试：Vitest（已有 `KnowledgeHubView.test.tsx`、`AppLayout.test.tsx`、`TagTree.test.tsx`、`DropzoneApp.test.tsx` 等基线）
  - 路由：Hash-based（`useHubHashRoute.ts`），无外部路由库
- **数据库**：SQLite（Tauri 端，本次迭代**不涉及**）
- **关键外部依赖**：`lucide-react`（图标）、`@tanstack/react-virtual`、`@crabnebula/tauri-plugin-drag`
- **现有代码库**：改造现有代码（位于 `../../NCdesktop/src/`，相对 harness-kit 根）
- **目标部署环境**：本地 Tauri 桌面应用（macOS aarch64 已有 DMG 打包流程）

---

## 3. 关键约束 [必填]

- **安全性要求**：**低** — 纯前端 UI 重排，不涉及认证、用户数据传输、序列化反序列化新接口
- **性能要求**：**中** — Sidebar 重渲染要避免订阅整张 store 表；StepNav 的 counts 计算需 useMemo 守护
- **用户体验要求**：**高** — 本次迭代的全部目的就是体验收敛，每条改动都对应 §1 现状盘点中的具体痛点
- **可维护性要求**：**高** — 全部走 CSS token，禁止行内 hex/box-shadow 字面量；删除已废弃 prop 时同步删除测试覆盖
- **不可妥协的底线**：
  1. **不重写已有 store**：本次只新增 `uiStore.tagsExpanded` 字段，其他 store 不动结构
  2. **不动 Rust 命令**：`src-tauri/` 仅允许动到 Tauri 2 已暴露的 window focus/blur 监听
  3. **不引新路由库**：保持 `useHubHashRoute.ts` 体系，hash 迁移矩阵不可破坏
  4. **零数据零信号**：计数为 0 时不渲染 badge；分组为空不渲染 section（PRD 原则 P-04）
  5. **沿用令牌不发明**：颜色/圆角/阴影必须走 `globals.css` 已有 CSS var；新增 token 必须集中声明并加 dark mode 覆盖
  6. **向后兼容 LS 状态**：`migrateLegacySection("search") → "recent"` 等迁移用例不得破坏

---

## 4. 质量偏好（影响 Reviewer 评分权重）

| 维度 | 权重 | 说明 |
|------|------|------|
| 功能正确性 | 20% | 改造点按 PRD 编号逐条对应，AC 可通过 vitest 验证 |
| 安全性 | 5% | 纯 UI 改造，安全敏感度低 |
| 代码质量 | 20% | 命名/结构清晰；不引入计划外依赖 |
| 测试覆盖 | 20% | 每个 P0 改造点必须有 vitest 用例 |
| 架构一致性 | 15% | 改造严格落在指定文件，不顺手重构外圈 |
| 可维护性 | 10% | 令牌沿用、dark mode 同步、a11y 不破坏 |
| **UX 体感** | 10% | 北极星：非学生首启 60s 内只见"工作区 × 知识链条"；空状态不哀号；浮窗不抢焦 |

> 合计 100%。本项目 UX 体感虽然权重 10%，但属于"否决项"——任意条 PRD §9.1 用户视角验收未通过即整体 FAIL。

---

## 5. 领域特定代码规范

```
TypeScript / React 通用：
- 严格模式：no implicit any、exhaustive switch、prop 类型必填
- 组件就近文件命名（PascalCase.tsx），hooks 用 useXxx 前缀
- 不引入 default export 新增项（统一 named export）
- 不使用 emoji 装饰（PRD §7.2 明令禁止 🎉/恭喜/加油 等）

Zustand store：
- 只 select 需要的字段，禁止订阅整张表（如 useStore(s => s.assets) 而非 useStore()）
- 计数类派生用 useMemo + length，避免 deep equality
- persist partialize 必须显式列出持久化字段；新增字段需更新 partialize

样式：
- 颜色 / 圆角 / 阴影 / 动效 duration 一律走 globals.css 中的 CSS var
- 禁止行内 hex（#abc123）和 box-shadow 字面量
- 新增 token：先在 :root 声明亮色，再在 @media (prefers-color-scheme: dark) 或 .dark 选择器补暗色
- 动效 duration 限定三档：--duration-instant 100ms / --duration-fast 200ms / --duration-normal 300ms

测试：
- 每个 P0 改造点配 vitest 用例；用 @testing-library/react 的 user-event 而非 fireEvent
- 测试文件就近放置（features/__tests__/ 或 同目录 *.test.tsx）
- 测试不能 mock 整个 store，应用 zustand 提供的 setState 重置真实 store

可访问性：
- nav 使用 aria-label；tablist 保留 role/aria-selected
- 折叠组件用 aria-expanded
- 仅装饰性图标加 aria-hidden

国际化：
- 文案默认中文（产品默认语言），不引入 i18n 库
- 文案中性陈述，避免感叹号、emoji 装饰
```

---

## 6. 领域特定审查重点

```
Reviewer 在审查本次迭代时必须重点检查：

1. **状态门控**（最易漏）：
   - 任何"学习"相关 UI 是否用 useEffectiveLearningSettings() 派生而非直接读 settings.showLearningFeatures
   - showLearningFeatures=false 时是否真的不渲染 Calendar / 今日复习 / 学习中心 整个分组
   - TODAY 分组为空时是否完全不渲染（不许"今天没有课程"占位行）

2. **零数据零信号**：
   - hub badge "n·n·n" 任一为 0 时整条 badge 不渲染（不是显示 0·0·0）
   - TodayView 顶部计数栏全 0 时整行不渲染
   - StepNav 单个 step 的 count 为 0 时仅显示 step 名

3. **令牌使用**：
   - 是否引入了行内 hex 或行内 box-shadow（应一律走 CSS var）
   - 新增 sidebar-active-bg/fg、hub-count-bg/fg 是否在 dark mode 也声明
   - 旧 amber 导航选中色是否全部替换为 sidebar-active-*

4. **持久化兼容**：
   - migrateLegacySection 仍能把 "search" 迁移到 "recent"
   - useHubHashRoute 的旧 hash 迁移矩阵不变（"#/skills"、"#/knowledge"）
   - 新增 uiStore.tagsExpanded 加入 partialize

5. **副作用洁净度**：
   - Sidebar 不要订阅 store 整张表，只 select 长度
   - Dropzone 的 window focus/blur 监听必须在 unmount 时清理
   - StepNav counts 用 useMemo，不在 render 中直接调用

6. **PRD §9.1 用户验收**：
   - 逐条对应；任一条目失败即整体 FAIL
```

---

## 7. 角色专业背景补充

- **Architect 应具备的专业知识**：
  - React 19 渲染优化（useMemo / 选择器订阅模式）
  - Tailwind v4 + CSS Variables 设计令牌体系
  - Tauri 2 window 事件 API（focus/blur 监听）
  - Zustand persist + migration 模式
- **Dev 应能熟练操作**：
  - vitest + @testing-library/react 写交互测试
  - lucide-react 图标替换
  - Hash-based 路由迁移（不破坏 useHubHashRoute 的 migrateLegacyHash）
- **Reviewer 应重点关注的风险域**：
  - 暗色模式对比度（WCAG AA）
  - LS 老用户升级路径（migrateLegacySection / migrateLegacyHash）
  - 学习模式开关对 UI 的全量覆盖（容易遗漏某个角落仍渲染学习相关内容）

---

## 8. 文件路径约定 [必填]

- **PRD 路径**：`product/prd/`
- **Session 记录路径**：`sessions/`
- **进度文件**：`sessions/conductor/progress.md`
- **架构方案存放**：`sessions/conductor/tasks/task_001_architect/output.md`
- **目标项目源码根**：`../../NCdesktop/src/`（相对 harness-kit 根目录，即 `项目启动/NCdesktop/src/`）
- **测试运行目录**：`../../NCdesktop/`（在该目录执行 `pnpm test` / `pnpm lint` / `pnpm check`）

---

## 9. 辩题概述 — **本次跳过 Debate**

PRD（HTML 文档）已由设计方提供完整的：
- 北极星与设计原则（六条 P-01 ~ P-06）
- 现状盘点与差距诊断
- 三阶段 Roadmap（PHASE 0/1/2 含工时预估）
- 改造点 ID 化（SB-01 ~ SB-07、KH-01 ~ KH-05、IN-01 ~ IN-04、DZ-01 ~ DZ-04、ES-01 ~ ES-04、TK-01 ~ TK-04）
- 验收清单与测试矩阵

→ **Architect 直接基于此 PRD 拆 task，不再开 Debate**。

---

## 10. 关键测试命令

```bash
# 在 ../../NCdesktop/ 目录下执行
pnpm test         # vitest 单元测试
pnpm lint         # eslint
pnpm check        # tsc --noEmit 类型检查
pnpm tauri:dev    # 启动 Tauri 桌面应用做手测
```

---

## 11. PM 偏好（用户隐性反馈）

- 用户已就该项目使用 harness-kit 完成 10+ 次迭代（见 `归档/` 内历史），熟悉流程，期望**最小废话、按节奏推进**
- 偏好"链条优于并列"的信息架构表达
- 偏好"令牌沿用 + 局部增补"而非推倒重做
- 偏好按 PRD 编号一一对应的 task 拆分（SB-01 → task_002，便于回溯）
