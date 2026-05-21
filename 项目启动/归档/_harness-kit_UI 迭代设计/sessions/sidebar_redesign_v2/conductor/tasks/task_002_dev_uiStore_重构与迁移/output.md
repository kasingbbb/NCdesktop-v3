# Task 交付 — task_002_dev_uiStore_重构与迁移

## 实现摘要

A 段 — uiStore 枚举重构 + Zustand `persist` 引入 + `migrateLegacySection()` 兼容迁移函数 + TodayView 相关字段（`todayLastTab` 持久化、`_learningJustEnabled` 瞬态）+ 升级智能 ON 评估 hook（fail-open，仅 CourseEvent + Concepts 信号）。

核心设计决策（与 ADR-001 / ADR-003 / ADR-006 对齐）：
- `SidebarSection` union type 落入 `types/ui.ts`，新增 `knowledge-hub`，删除 `search`，保留 `today` / `calendar`（仅在学习模式开启时由视图层条件渲染）
- 引入 `VALID_SECTIONS` runtime 列表，配 `_AssertCovers` 编译期 union ↔ 数组双向覆盖断言，防止未来增删枚举忘记同步
- 迁移函数同时挂在两处：persist `migrate` 选项（rehydrate 时第一帧规范化）+ `setSidebarSection` setter 入口（防御未来 Dev 误传）
- DEV 警告路径：`if (import.meta.env.DEV) console.warn('[uiStore] ...')`，prod 构建被 tree-shake
- `partialize` 出口字段严格只含 `[activeSidebarSection, todayLastTab]`，`_learningJustEnabled` 显式排除
- 升级智能 ON：独立 hook `useEvaluateLearningAutoEnableOnce`，一次性标记位 `learningAutoEnableEvaluated` 落入 settingsStore（task_003 接入），ref + 标记位双重护栏防 StrictMode 双调用与跨重启重评估，**只能由 false→true，禁止反向写**
- 学习数据信号 = `calendarStore.events.length > 0` OR `knowledgeStore.concepts.length > 0`，`Promise.allSettled` 容错并行；任何失败按"未检测到"走（fail-open，PM §B 裁定）

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---|---|---|
| `src/types/ui.ts` | 修改 | `SidebarSection` 重定义为 7 项 union（含 knowledge-hub，删 search）；新增 `TodayTab` |
| `src/stores/uiStore.ts` | 修改 | 引入 `persist` middleware；`migrateLegacySection` / `migrateLegacyTodayTab` / `devWarn` 私有；新增 `todayLastTab`/`_learningJustEnabled` 字段 + setters；setter 入口拦截；partialize 严格白名单；`migrate` 选项接管旧值规范化 |
| `src/hooks/useEvaluateLearningAutoEnable.ts` | 新建 | `useEvaluateLearningAutoEnableOnce()` — 升级智能 ON 评估入口（依赖 settingsStore.learningAutoEnableEvaluated 字段，由 task_003 在 AppSettings 类型中正式落入） |
| `src/stores/__tests__/uiStore.test.ts` | 新建 | 28 用例：migrateLegacySection 全矩阵 + DEV warn + setter 拦截 + persist 默认/round-trip/migrate 钩子 + todayTab 字段 |

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（`stores/`、`types/`、`hooks/`、`stores/__tests__/`）
- [x] API 路径/命名与 Architect 方案一致（`migrateLegacySection`、`useEvaluateLearningAutoEnableOnce`、`activeSidebarSection`、`todayLastTab`、`_learningJustEnabled`）
- [x] 数据模型与 Architect 方案一致（SidebarSection 7 项 union；persist 白名单严格 2 项；瞬态字段不进白名单）
- [x] 未引入计划外的新依赖（zustand persist 是 zustand 自带 middleware）
- 偏离说明：
  - `useEvaluateLearningAutoEnableOnce` 中对 `settingsStore.updateSetting`/`settings` 使用临时类型断言，**因为 `learningAutoEnableEvaluated` / `showLearningFeatures` 字段需 task_003 才落入 `AppSettings`**。task_003 完成后必须把断言移除。已在 hook 顶部注释明示。

## 测试命令

```bash
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop && pnpm test src/stores/__tests__/uiStore.test.ts
```

## 测试结果

```
> ncdesktop@1.0.0 test /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop
> vitest run src/stores/__tests__/uiStore.test.ts

 RUN  v4.1.1 /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop

 Test Files  1 passed (1)
      Tests  28 passed (28)
   Start at  21:44:17
   Duration  582ms (transform 32ms, setup 53ms, import 25ms, tests 5ms, environment 429ms)
```

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|---|---|---|---|
| ✅ 正常路径 | migrateLegacySection 7 个合法新值原样返回 | 已测 | PASS（5 个用例） |
| ✅ 正常路径 | persist 默认值（recent / null / false） | 已测 | PASS（3 个用例） |
| ✅ 正常路径 | setter 写入合法值幂等 | 已测 | PASS |
| ✅ 正常路径 | partialize 白名单严格 2 项，瞬态字段排除 | 已测 | PASS |
| ⚠️ 边界条件 | 旧值 knowledge/skills → knowledge-hub | 已测 | PASS（2 个用例） |
| ⚠️ 边界条件 | 已删除值 search → recent | 已测 | PASS（migrate + setter 两路） |
| ⚠️ 边界条件 | null / undefined → recent，不告警 | 已测 | PASS |
| ⚠️ 边界条件 | 空字符串 / 未知字符串 → recent | 已测 | PASS |
| ⚠️ 边界条件 | persist migrate 钩子规范化旧 LS | 已测 | PASS（含 todayLastTab 非法降级 null） |
| ❌ 异常路径 | 非 string 类型（number/object/array/boolean）→ recent | 已测 | PASS（5 类） |
| ❌ 异常路径 | DEV 环境对未知/旧值/非 string 触发 warn | 已测 | PASS |
| ❌ 异常路径 | DEV 对合法新值 + null/undefined 不触发 warn | 已测 | PASS |
| ⚠️ 边界条件 | 无 LS 时跳过 migrate，走默认值（AC-10） | 已测 | PASS |
| ⚠️ 集成 | useEvaluateLearningAutoEnableOnce hook 行为 | 未测 | 跳过原因：依赖 task_003 settingsStore 字段 + task_009 完整集成测试矩阵；本 task 仅交付 hook 实现 |

## 已知局限

1. **hook 类型断言**：`useEvaluateLearningAutoEnable.ts` 内对 `settingsStore.updateSetting` 的类型断言依赖 task_003 收尾移除。task_003 完成后 Reviewer 应在 task_003 review 中验证断言已被清理。
2. **persist round-trip 完整矩阵**：本 task 仅 smoke（4 用例）。完整 ≥5 用例的 LocalStorage round-trip 矩阵 + TodayView 三态 Tab 用例由 task_009 接管（input.md AC-1b / AC-5b 已明确）。
3. **prod 静默验证**：当前测试在 DEV 环境运行；prod 静默仅靠 Vite tree-shake 静态保证，未在测试中显式验证 `import.meta.env.DEV = false` 路径（Vitest 限制）。

## 需要 Reviewer 特别关注的地方

1. **`uiStore.ts:165-294` persist 配置块**：确认 `partialize` 白名单严格只含 `activeSidebarSection` + `todayLastTab`（白名单溢出 = 数据持久化扩面 = 兼容性风险）
2. **`uiStore.ts:42-47` `_typeCheck` 编译期断言**：确认未来增删 `SidebarSection` 成员时这里会编译失败（防 VALID_SECTIONS 与 union 脱节）
3. **`useEvaluateLearningAutoEnable.ts:34-43` ref + 一次性标记的双重护栏顺序**：确认无任何"未检测到 → 写 false"分支（PRD §11 拒绝清单）；确认只能 false→true
4. **`useEvaluateLearningAutoEnable.ts:65-80` Promise.allSettled 容错**：确认任何 fetch 失败都不会阻塞 hook 写入一次性标记（避免每次启动重试导致用户感知卡顿）
5. **测试矩阵充分性**：28 用例是否真的覆盖 ADR-001 矩阵全部分支；DEV warn 检查是否包括 setter 入口路径（当前主要在 migrate 路径上覆盖）
