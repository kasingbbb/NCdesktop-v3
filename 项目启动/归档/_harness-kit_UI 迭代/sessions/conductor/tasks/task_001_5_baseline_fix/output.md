# Task 交付 — task_001_5_baseline_fix

## 实现摘要

按 PM 选定的 A1 最小可解锁范围，补全 `uiStore.ts` 中 WorkspaceFolderListView.tsx 和 uiStore.test.ts 引用但缺失的 9 个成员（4 字段 + 5 actions），并修 `uiStore.persist.integration.test.ts` 2 处 `@ts-expect-error` 描述缺失。这一改动让 uiStore 相关测试从 baseline 7 fail → 0 fail，并间接修复了 WorkspaceFolderListView 相关测试中因 selector 返回 undefined 导致的连锁失败（全量 vitest 失败数 58 → 37，超额完成）。

核心设计决策：
- **Set 不变性**：`startRenaming`/`finishRename` 用 `new Set(state.pendingRenameIds)` 拷贝后再 add/delete，确保 zustand 浅比较触发渲染
- **partialize 严格保护**：4 新字段**绝不进** partialize 白名单（uiStore.test:305-317 已断言）
- **finishRename 幂等**：`Set.delete` 操作天然幂等；`editingFolderPath === path` strict equality 决定是否清空

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `src/stores/uiStore.ts` | 修改 | UIStore interface 加 9 成员（4 字段 + 5 actions），store body 加 4 字段初始值 + 5 actions 实现 |
| `src/stores/__tests__/uiStore.persist.integration.test.ts` | 修改 | 2 处 `@ts-expect-error` 添加描述（line 149, 158） |

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（仅修改既有文件，不新建）
- [x] API 路径/命名与 Architect 方案一致（命名小驼峰，与现有 setter 风格对齐）
- [x] 数据模型与 Architect 方案一致（新字段为瞬态，不进持久化）
- [x] 未引入计划外的新依赖
- 偏离说明：无

## 测试命令

```bash
# 在 项目启动/NCdesktop/ 目录执行
pnpm vitest run src/stores/__tests__/uiStore.test.ts       # AC-5
pnpm vitest run src/stores/__tests__/                       # AC-8
pnpm check                                                  # AC-9
pnpm vitest run                                             # AC-10
pnpm lint                                                   # AC-11
```

## 测试结果

### uiStore.test.ts（AC-5）

```
Test Files  1 passed (1)
     Tests  36 passed (36)
```

### stores/__tests__/（AC-8）

```
Test Files  3 passed (3)
     Tests  62 passed (62)
```

包含：
- uiStore.test.ts (36/36)
- uiStore.persist.integration.test.ts (13/13)
- settingsStore.test.ts (13/13)

### TSC（AC-9）

```
$ pnpm check
> tsc --noEmit
(无输出 = 通过)
```

### 全量 vitest（AC-10）

```
Test Files  7 failed | 20 passed (27)
     Tests  37 failed | 215 passed (252)
```

**baseline 对比**：
| 指标 | baseline | 本 task 后 | 改善 |
|------|----------|------------|------|
| Test Files failed | 9 | **7** | -2 |
| Tests failed | 58 | **37** | -21 |
| Tests passed | 194 | **215** | +21 |

意外收益 14 个（预期只修 7 个 uiStore.test）—— WorkspaceFolderListView 相关测试中因 uiStore selector 返回 undefined 导致的连锁失败被间接解决。

### Lint（AC-11）

```
✖ 45 problems (25 errors, 20 warnings)
```

**baseline 对比**：
| 指标 | baseline | 本 task 后 | 改善 |
|------|----------|------------|------|
| Errors | 27 | **25** | -2 |
| Warnings | 20 | **20** | 0 |

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果 |
|----------|----------|------|------|
| ✅ 正常路径 | uiStore 4 新字段默认值正确 | 已测 | PASS（uiStore.test 用例 249-255） |
| ✅ 正常路径 | startCreating 设 pendingNewFolder=true, 清 editingFolderPath | 已测 | PASS（uiStore.test 用例 257-262） |
| ✅ 正常路径 | cancelCreating 设 pendingNewFolder=false | 已测 | PASS（uiStore.test 用例 264-268） |
| ✅ 正常路径 | startRenaming 加入 Set 且返回新实例 + 设 editingFolderPath | 已测 | PASS（uiStore.test 用例 270-277） |
| ⚠️ 边界条件 | finishRename 不存在的 path 不抛错 | 已测 | PASS（uiStore.test 用例 293-296） |
| ✅ 正常路径 | setDragOverPath 覆盖与清回 null | 已测 | PASS（uiStore.test 用例 298-303） |
| ⚠️ 边界条件 | 5 新字段不进 partialize 白名单 | 已测 | PASS（uiStore.test 用例 305-317） |
| ⚠️ 边界条件 | finishRename 编辑态匹配时清 editingFolderPath；不匹配时保留 | 已测 | PASS（uiStore.test 用例 279-291） |
| ❌ 异常路径 | 老 LS 已含 search → recent 迁移仍正确 | 已测 | PASS（uiStore.persist.integration 用例 56-63） |

## 已知局限

1. **Baseline 仍存在 37 个 test failures 与 25 个 lint errors**（A1 范围外）：
   - 详见下方"既有 broken 快照"，已与 PM 同步
2. **pnpm check 假通过的根因未深查**：zustand 的 selector 类型推断在字段缺失时返回 `unknown`/`undefined`，掩盖了运行时错误。本 task 通过补全字段绕过了这个隐患，但根因（zustand v5 类型系统设计选择）保留
3. **WorkspaceFolderListView.test.tsx 仍有部分 fail**：本 task 解锁了 selector 链路，但 T5b 文件夹编辑 UI 实现本身未做（非本 task scope）

## 需要 Reviewer 特别关注的地方

- **Set 实例新旧不同**：startRenaming/finishRename 必须 `new Set(prev)` 拷贝，断言 `not.toBe(before)` 已覆盖，但 reviewer 可手动确认实现
- **finishRename 幂等**：path 不存在时 `Set.delete` 返回 false 但不抛错，符合 JS spec
- **partialize 严格保护**：本次未动 partialize 配置，但 reviewer 应确认 4 新字段不在白名单内

---

## 既有 Broken 快照（v1.3 task_002~013 的 baseline 锁，不可超过）

**v1.3 后续 task 验收口径调整**（由 PM 选 A1 决定）：

> **原 AC**：`pnpm test / lint / check` 全绿  
> **新 AC**：以下三个指标**不可恶化**——任一超出即视为引入新 baseline 失败：
> - 全量 vitest 失败数 ≤ **37**
> - 全量 lint errors 数 ≤ **25**
> - tsc 必须通过（无变化）
> 加上各 task 自身要求：新增用例 PASS + 相关测试文件 PASS

### 仍存在的 test failures（37 个）— 不在 v1.3 scope，需独立 task 处理

| 测试文件 | 失败用例数 | 性质 |
|---|---|---|
| `WorkspaceFolderListView.test.tsx` | ~17（baseline ~26，已减少） | T5b 工作区文件夹编辑 UI 实现未做 |
| `SettingsPanel.test.tsx` | 9 | 学习功能 tab + rAF 时序契约未实现 |
| `TagTree.test.tsx` | 7 | task_008 F-P0-11 "前 20 + 更多 (N)" 折叠机制未实现 — **注：与 v1.3 task_006 SB-05 互斥契约**，已在 progress.md 标注 ADR-006 待补 |
| `ContentArea.test.tsx` | 2 | section/show 联动渲染未实现 |
| 其它 | ~2 | App.test 等连锁 |

### 仍存在的 lint errors（25 个）— 不在 v1.3 scope

主要类别：
- `react-compiler / react-hooks` 严格规则（~10）：setState in effect、ref during render、impure function during render
- `@typescript-eslint/no-explicit-any`（~5，全在 .test 文件，tsconfig 已 exclude tsc）
- `react-refresh/only-export-components`（2）
- `react/no-danger` 规则未定义（1）
- `noUnusedVars / noUnusedDisable`（~7）
