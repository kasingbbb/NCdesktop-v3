# Task 交付 — task_002_dev_uistore_tags_expanded

## 实现摘要

为 v1.3 SB-05（TagTree 折叠）铺路，在 `src/stores/uiStore.ts` 新增 `tagsExpanded:boolean` 字段（默认 false）、`setTagsExpanded` setter、加入 partialize 白名单、扩展 migrate 函数兜底。配套在 `uiStore.test.ts` 新增 5 个用例覆盖默认值、setter、partialize、旧 LS rehydrate、search 老用户兼容。

核心设计决策：
- **命名风格**：`tagsExpanded` + `setTagsExpanded`，与现有 `inspectorOpen`/`setInspectorOpen` 对齐
- **持久化**：进 partialize；migrate 函数对缺失字段返回 `false`（不升级 version）
- **不破坏老用户**：旧 LS 无 `tagsExpanded` 字段 → migrate 走默认 false；同时 search 老用户的 section 迁移路径不变

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `src/stores/uiStore.ts` | 修改 | UIStore interface 加 `tagsExpanded` + `setTagsExpanded`；state 初值加 `tagsExpanded: false`；action 加 `setTagsExpanded`；partialize 出口加 `tagsExpanded`；migrate 函数加 `tagsExpanded` 兜底 |
| `src/stores/__tests__/uiStore.test.ts` | 修改 | partialize round-trip 用例（line 174-192）的 `toEqual` 增加 `tagsExpanded: false`；新增 `describe("tagsExpanded (v1.3 task_002 SB-07)")` 含 5 个用例 |

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（仅修改既有文件）
- [x] API 路径/命名与 Architect 方案一致（ADR-002 约定的 `tagsExpanded` / `setTagsExpanded`）
- [x] 数据模型与 Architect 方案一致（默认 false，进 partialize，version 不升级）
- [x] 未引入计划外的新依赖
- 偏离说明：无

## 测试命令

```bash
pnpm vitest run src/stores/__tests__/uiStore.test.ts   # 验证新增用例 PASS
pnpm vitest run                                         # 验证 baseline 不恶化
pnpm lint                                               # 验证 lint baseline 不恶化
pnpm check                                              # 验证 tsc 通过
```

## 测试结果

### uiStore.test.ts（focused）

```
Test Files  1 passed (1)
     Tests  41 passed (41)
```

新增 5 个用例（行 234-280）：
- AC-1 默认值为 false
- AC-2 setTagsExpanded toggle
- AC-3 partialize 出口包含 tagsExpanded
- AC-4 migrate 旧 LS 无字段 → 默认 false
- AC-4b migrate 'search' 老用户 → recent + tagsExpanded false

### 全量 vitest

```
Tests  37 failed | 220 passed (257)
```

baseline 锁（≤ 37）✅。注：220 = task_001.5 后的 215 + 本 task 新增 5 用例。

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
| ✅ 正常路径 | 首次启动 tagsExpanded === false | 已测 | PASS（AC-1 用例） |
| ✅ 正常路径 | setTagsExpanded(true) → true，setTagsExpanded(false) → false | 已测 | PASS（AC-2 用例） |
| ✅ 正常路径 | partialize 输出包含 tagsExpanded:true | 已测 | PASS（AC-3 用例） |
| ⚠️ 边界条件 | 旧 LS（无 tagsExpanded 字段）rehydrate → tagsExpanded=false | 已测 | PASS（AC-4 用例） |
| ⚠️ 边界条件 | "search" 老用户 LS → activeSidebarSection=recent 且 tagsExpanded=false | 已测 | PASS（AC-4b 用例） |
| ❌ 异常路径 | LS 中 tagsExpanded 类型错误（非 boolean）→ 兜底 false | 未测 | migrate 函数代码已用 `typeof rawTagsExpanded === "boolean"` 守卫，类型错误自动 fallback false。可由未来 task_009 集成测试补充 |

## 已知局限

1. **migrate version 未升级**：保持 version=1。如未来需要更复杂的迁移逻辑（如旧字段重命名），需考虑 version bump
2. **integration test 未扩展**：`uiStore.persist.integration.test.ts` 未加 tagsExpanded round-trip 用例，建议未来 task_009 补充（不在本 task scope）

## 需要 Reviewer 特别关注的地方

- **partialize 现已 3 字段**（activeSidebarSection / todayLastTab / tagsExpanded）—— 任何下游代码若依赖严格的 2 字段断言需更新
- **migrate 函数加 `tagsExpanded`**：reviewer 确认 `typeof rawTagsExpanded === "boolean"` 守卫正确处理 undefined / 类型错误两种边界
