# Task 输入 — task_001_5_baseline_fix

## 目标

为 v1.3 主体迭代 task_002~013 建立"可执行的 baseline"。**最小可解锁修复范围（A1）**：
1. 补全 `uiStore.ts` 中 WorkspaceFolderListView.tsx 和 uiStore.test.ts 引用但缺失的 5 字段 + 4 方法 + 1 setter，让 `uiStore.test.ts` 中的 `workspace folder list edit state (task_006 T4)` 7 个用例从 fail → pass，让 WorkspaceFolderListView.tsx 的 selector 链路正确（虽然 tsc 现在假通过，但运行时报错）
2. 修 `uiStore.persist.integration.test.ts` 中的 2 处 `@ts-expect-error` 描述缺失（v1.3 task_002 直接涉及文件）

**不在本 task scope 的**：
- 其它历史测试契约（TagTree task_008 / SettingsPanel 学习功能 tab / WorkspaceFolderListView T5b / ContentArea / App.test）保持现状失败
- 其它 lint errors（React 19 严格规则、any 类型、fast-refresh 等）保持现状失败
- 这些"既有 broken"由 v1.3 主体迭代或后续独立 task 处理

## 前置条件

- 依赖 task：**无**
- 必须先存在的文件/接口：
  - `src/stores/uiStore.ts`
  - `src/stores/__tests__/uiStore.test.ts`（含 task_006 T4 用例）
  - `src/stores/__tests__/uiStore.persist.integration.test.ts`
  - `src/components/features/WorkspaceFolderListView.tsx`（消费者验证）

## 验收标准（Acceptance Criteria）

### 1. uiStore 字段补全

1. **AC-1**：`uiStore.ts` 新增 4 个 state 字段，默认值如下：
   - `editingFolderPath: string | null = null`
   - `pendingNewFolder: boolean = false`
   - `pendingRenameIds: Set<string> = new Set()`
   - `dragOverPath: string | null = null`

2. **AC-2**：`uiStore.ts` 新增 5 个 actions：
   - `startCreating()`：设 `pendingNewFolder = true`，清 `editingFolderPath = null`
   - `cancelCreating()`：设 `pendingNewFolder = false`
   - `startRenaming(path: string)`：把 path 加入 `pendingRenameIds`（**返回新 Set 实例**，确保 zustand 浅比较触发渲染），设 `editingFolderPath = path`
   - `finishRename(path: string)`：从 `pendingRenameIds` 移除 path（**新 Set 实例**）；若当前 `editingFolderPath === path`，则清为 null；**幂等**（path 不存在也不抛错）
   - `setDragOverPath(path: string | null)`：直接设

3. **AC-3**：UIStore interface 包含上述 9 个新成员；TypeScript 严格模式编译通过

4. **AC-4**：5 个新字段**不进**partialize 白名单（与现有约定保持一致；uiStore.test:305-317 已断言）

5. **AC-5**：`pnpm vitest run src/stores/__tests__/uiStore.test.ts` 全绿（36 用例全部 PASS，包含原 29 + 新增 task_006 T4 的 7）

### 2. uiStore.persist.integration.test.ts lint 修复

6. **AC-6**：`uiStore.persist.integration.test.ts` line 149 和 158 的 `@ts-expect-error` 加上 ≥3 字符描述（如 `@ts-expect-error 模拟运行时误传旧值 'search'`）

7. **AC-7**：`pnpm eslint src/stores/__tests__/uiStore.persist.integration.test.ts` 0 error

### 3. 整体回归

8. **AC-8**：`pnpm vitest run src/stores/__tests__/` 全绿（uiStore.test + uiStore.persist.integration.test + settingsStore.test）

9. **AC-9**：`pnpm check`（tsc）通过

10. **AC-10**：`pnpm vitest run`（全量）的失败用例数 ≤ 51（baseline 58 fail − uiStore.test 修好 7 个 = 51）。**严格不可超过**——任何超出意味着引入新 baseline 失败

11. **AC-11**：`pnpm lint` 的 error 数 ≤ 25（baseline 27 − uiStore.persist 修好 2 个 = 25）。**严格不可超过**

### 4. 文档

12. **AC-12**：output.md 中明确列出"仍存在的 baseline 失败"（用作后续 task 的"既有 broken 清单"参照）

## 技术约束

- **不动现有字段顺序**：新字段追加到 UIStore interface 末尾；setter 追加到现有 setter 列表末尾，遵循当前命名风格
- **Set 实例规则**：startRenaming / finishRename 必须返回**新 Set 实例**（用 `new Set(state.pendingRenameIds)` 拷贝再 add/delete），否则 zustand 浅比较失败导致组件不重渲染（uiStore.test:270 已断言 `expect(after).not.toBe(before)`）
- **partialize 白名单**：保持只持久化 `activeSidebarSection` 和 `todayLastTab`，**不要**把新字段加进 partialize（uiStore.test:305-317 已断言）
- **不动 migrate 函数**：本期新字段非持久化，无需 migrate
- **不引入新依赖**

## 参考文件

- `src/stores/__tests__/uiStore.test.ts:235-318`（task_006 T4 测试组，定义了字段与方法的契约）
- `src/components/features/WorkspaceFolderListView.tsx:140-146`（生产代码消费者）
- `src/stores/uiStore.ts:72-110`（UIStore interface 现有定义位置）
- `src/stores/uiStore.ts:113-203`（store actions 现有实现位置）
- `src/stores/__tests__/uiStore.persist.integration.test.ts:149, 158`（@ts-expect-error 缺描述位置）

## 预估影响范围

- **修改文件**：
  - `src/stores/uiStore.ts`（加 9 个成员 + 5 个 actions）
  - `src/stores/__tests__/uiStore.persist.integration.test.ts`（加 2 处描述）

- **新建文件**：无

---

## Reviewer 重点关注项

- **Set 不变性**：startRenaming / finishRename 返回新 Set，不要 mutate 原 Set
- **partialize 白名单严格**：新 4 字段绝对不要漏进 partialize（会破坏现有 round-trip 测试）
- **finishRename 幂等**：path 不存在时 `delete` 行为为 no-op，不能抛错
- **WorkspaceFolderListView 端到端**：补完后 `pnpm vitest run src/components/features/__tests__/WorkspaceFolderListView.test.tsx` 是否能跑得通是**advisory 不在本 task 强制**（那是 T5b 历史契约，不在 A1 scope）；只验证 uiStore selector 不再 undefined 即可
- **lint 总 errors 数严格 ≤ 25**：意味着不能"顺手修其他 errors"也不能"引入新 errors"
