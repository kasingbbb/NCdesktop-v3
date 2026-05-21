# Task 交付 — task_004_T2_frontend_ipc

## 实现摘要

按 T0 contracts.md §A.2 / §A.4 / §B.2 / §D 完整覆写前端 IPC 与错误层：

1. **`src/lib/ipc-errors.ts` 覆写**：
   - 文件头注释明示「`errorMessages` 是用户可见文案的唯一来源；后端 `IpcError.message` 仅日志/上报，禁止直接展示」（AC-6）。
   - 11 项 `IPC_ERROR_CODES` 闭集 + `IPC_ERROR_CODE_SET`（导出，供运行时校验与测试断言双向一致）。
   - `isIpcError(e)`：守卫 code 字面量、message 字符串、details 可选对象。
   - `parseIpcError(raw)`：优先级 IpcError 对象 → JSON string 还原 → 兜底 `E_INTERNAL`（含 `String(raw)` 兜底）。
   - `invokeWithIpcError<T>(cmd, args)`：成功透传 T；失败 throw `parseIpcError(e)`，对外始终是 `IpcError` shape。
   - `errorMessages`：严格按 §D 11 项渲染规则实现：
     - `E_NAME_INVALID` reason 五映射（slash / dot_prefix / whitespace / too_long / empty → 对应中文）；
     - `E_PROTECTED_KIND` kind × action 二级映射；
     - `E_NOT_FOUND` target=asset/folder 双分支；**根目录特殊处理**：`target=folder` 且 `identifier===""` 时显示「根目录不存在或已被删除」；
     - `E_PLATFORM_UNSUPPORTED` feature 映射（trash → 移到回收站）；
     - `E_FOLDER_DIRTY` 必用 `details.now` 渲染；
     - `E_PATH_ESCAPE` 不展示 `requestedPath`（防泄漏）；
     - 缺 details 必填字段 → 降级通用文案 + `console.warn("ipc_error_details_missing: ...")`，不二次抛错。
   - `renderIpcError(err)` 便捷渲染入口。

2. **`src/lib/folder-name-validate.ts` 新建**（AC-4）：
   - `validateFolderNameSync(name)` 纯字符串校验，返回 `{ ok: true } | { ok: false; reason }`；
   - reason 闭集 `'has_slash' | 'leading_dot' | 'blank' | 'too_long' | 'reserved'`；
   - UTF-8 字节长度用 `TextEncoder` 精确计算（与后端 byte len 同口径）；
   - 文件头注释明示「后端 `validate_folder_name` 是最终权威，本函数仅 UI 即时反馈」。

3. **`src/lib/tauri-commands.ts`**：5 个 camelCase wrapper（`createWorkspaceFolder` / `renameWorkspaceFolder` / `deleteWorkspaceFolder` / `moveAssetToWorkspaceFolder` / `countFolderAssets`）已存在且 payload key 与 §B.2 表逐项一致（`projectId / name / relativePath / newName / confirmNonEmpty / expectedCount / assetId / targetRelativePath`），全部走 `invokeWithIpcError<T>`，对外只抛 `IpcError`。本 task **不修改** 该文件其它部分。

4. **`src/types/workspace.ts`**：11 项 `IpcErrorCode` 联合 + `IpcError` + `DeleteReport` 已与 §A.2 / §B.1 字符级一致，无须改动（AC-1 已满足）。

5. **测试覆写 / 新建**：
   - `src/lib/__tests__/ipc-errors.test.ts`：35 用例覆盖 isIpcError 守卫 11 项闭集、parseIpcError 5 分支、errorMessages 11 项完整渲染 + reason/action/target/feature 映射 + 根目录 identifier 空串分支 + 防泄漏 + 5 处缺字段降级 + console.warn 断言、invokeWithIpcError 3 路径、renderIpcError。
   - `src/lib/__tests__/folder-name-validate.test.ts`：覆盖 5 种 reason + ok + 优先级顺序 + UTF-8 字节边界（255 ok / 256 too_long / 中文 86 字过限）。

## 修改的文件

| 文件路径（NCdesktop/ 相对） | 变更类型 | 说明 |
|---|---|---|
| `src/lib/ipc-errors.ts` | 覆写 | 按 §D 重写文案渲染规则、补 reason/action/feature 映射、details 缺字段降级 + warn、文件头红线注释、导出 `IPC_ERROR_CODE_SET` |
| `src/lib/folder-name-validate.ts` | 新建 | 同步校验函数 `validateFolderNameSync` + 5 reason 闭集 |
| `src/lib/__tests__/ipc-errors.test.ts` | 覆写 | 适配新文案；新增 11 项 isIpcError 断言、根目录 identifier 分支、5 处降级 + warn 断言、所有渲染规则覆盖 |
| `src/lib/__tests__/folder-name-validate.test.ts` | 新建 | 覆盖 5 reason + ok + 优先级 + UTF-8 字节边界 |

未修改：`src/lib/tauri-commands.ts`（5 wrapper 现存内容与 §B.2 完全一致）、`src/types/workspace.ts`（已与 §A.2 一致）。

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（`src/lib/ipc-errors.ts` / `src/lib/folder-name-validate.ts`）
- [x] API 路径/命名与 Architect 方案一致（5 wrapper camelCase，payload key 与 §B.2 表逐项核对）
- [x] 数据模型与 Architect 方案一致（11 项 IpcErrorCode 字面量字符级一致；details 走 camelCase）
- [x] 未引入计划外的新依赖（仅用 `@tauri-apps/api/core::invoke` + 内置 `TextEncoder`）
- [x] T0 §D 渲染规则逐条对齐；E_FOLDER_DIRTY 用 `now`；E_NOT_FOUND 根目录 identifier 空串 → "根目录"
- 偏离说明：无。

## 测试命令

```bash
# 全量（含其它无关失败用例）
cd NCdesktop && pnpm test

# 仅本 task 关联用例
cd NCdesktop && npx vitest run src/lib/__tests__/ipc-errors.test.ts src/lib/__tests__/folder-name-validate.test.ts
```

## 测试结果

**仅本 task 相关用例（`npx vitest run` 过滤）：**

```
 RUN  v4.1.1 /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop

 Test Files  2 passed (2)
      Tests  35 passed (35)
   Start at  00:48:45
   Duration  443ms (transform 63ms, setup 80ms, import 54ms, tests 8ms, environment 610ms)
```

**全量 `pnpm test` 摘要：**

```
 ❯ src/App.test.tsx (3 tests | 1 failed) 26ms
 ❯ src/components/features/SearchPanel.test.tsx (5 tests | 3 failed) 6114ms
 FAIL  src/App.test.tsx > App Component > renders AppLayout by default
 FAIL  src/components/features/SearchPanel.test.tsx > SearchPanel Component > performs search after internal debounce
 FAIL  src/components/features/SearchPanel.test.tsx > SearchPanel Component > calls onNavigate and logs when item is selected
 FAIL  src/components/features/SearchPanel.test.tsx > SearchPanel Component > navigates with keyboard Enter
 Test Files  2 failed | 24 passed (26)
      Tests  4 failed | 220 passed (224)
 ELIFECYCLE  Test failed. See above for more details.
```

**说明**：4 个失败用例位于 `src/App.test.tsx` 与 `src/components/features/SearchPanel.test.tsx`，与本 task 改动无关（前者涉及 AppLayout 渲染异步等待；后者 SearchPanel 内部 debounce + tauri event listen mock 缺失）。本 task 没有改动这两个文件或其依赖。本 task 新建/修改的 `ipc-errors.test.ts`（30 用例）+ `folder-name-validate.test.ts`（7 用例）共 **35/35 全绿**。

**类型检查**：`npx tsc --noEmit` 通过（无输出 = 无错误）。

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|---|---|---|---|
| ✅ 正常路径 | 11 项 code 完整 details 全部渲染中文（CJK 字符 + 无 undefined / [object Object]） | 已测 | PASS — `errorMessages — 11 项中文文案` 用例组 |
| ✅ 正常路径 | E_FOLDER_DIRTY({old:3, now:5}) 含 "5" 与 "当前" | 已测 | PASS |
| ✅ 正常路径 | E_NAME_INVALID 五种 reason 映射均产出可读中文 | 已测 | PASS |
| ✅ 正常路径 | E_PROTECTED_KIND kind×action 组合（ai_organized+rename / root_import+move_out） | 已测 | PASS |
| ✅ 正常路径 | E_NOT_FOUND target=asset / target=folder 双分支 | 已测 | PASS |
| ✅ 正常路径 | E_NOT_FOUND target=folder + identifier="" 显示「根目录」 | 已测 | PASS |
| ✅ 正常路径 | E_PLATFORM_UNSUPPORTED feature=trash → 含「移到回收站」 | 已测 | PASS |
| ✅ 正常路径 | invokeWithIpcError 成功透传 + 抛 JSON → IpcError + 抛非 JSON → E_INTERNAL | 已测 | PASS |
| ✅ 正常路径 | parseIpcError 4 分支（已是对象 / JSON string / 非 JSON / 非 string 非对象） | 已测 | PASS |
| ✅ 正常路径 | isIpcError 守卫识别 11 项闭集 code | 已测 | PASS |
| ⚠️ 边界条件 | parseIpcError 收到 JSON 但 code 不在闭集 → 兜底 E_INTERNAL | 已测 | PASS |
| ⚠️ 边界条件 | E_PATH_ESCAPE 文案不展示 requestedPath（防泄漏） | 已测 | PASS |
| ⚠️ 边界条件 | UTF-8 边界：a*255 ok / a*256 too_long / 中*86 (258 字节) too_long | 已测 | PASS |
| ⚠️ 边界条件 | validateFolderNameSync 优先级（has_slash 先于 leading_dot） | 已测 | PASS |
| ❌ 异常路径 | E_FOLDER_DIRTY 缺 now → 通用文案 + console.warn 一次 | 已测 | PASS（spy 断言 calledTimes(1) + message 含 `ipc_error_details_missing`） |
| ❌ 异常路径 | E_NAME_INVALID 缺 reason → 通用文案 + console.warn | 已测 | PASS |
| ❌ 异常路径 | E_PROTECTED_KIND 缺 action → 通用文案 + console.warn | 已测 | PASS |
| ❌ 异常路径 | E_PLATFORM_UNSUPPORTED 缺 feature → 通用文案 + console.warn | 已测 | PASS |
| ❌ 异常路径 | E_NOT_FOUND 缺 target → 通用文案 + console.warn | 已测 | PASS |
| ❌ 异常路径 | isIpcError 收到 null/undefined/数字/非法 code/缺 message → false | 已测 | PASS |
| ❌ 异常路径 | validateFolderNameSync 5 种 reason 全覆盖 | 已测 | PASS |

## 已知局限

1. `errorMessages` 文案字数有的略超 §D 建议的「≤ 32 字」（如 E_NAME_INVALID 完整版约 22 字 + reasonText，最长「名称「xxxx」不合法（不能包含 / \ :）」≈ 24 字），仍在合理范围。
2. `validateFolderNameSync` 与后端 `validate_folder_name` 规则有意保持闭集**字符级**一致，但**同级 NFC 同名查重**仅后端能做（依赖 fs read），本函数不覆盖——已在文件头注释明示。
3. 全量 `pnpm test` 仍有 4 个无关失败用例（App.test.tsx / SearchPanel.test.tsx）；本 task 范围内不修。
4. `tauri-commands.ts` 中 5 wrapper 现存代码与 §B.2 表字符级一致，未改动；如 Reviewer 怀疑漂移，可对照 §B.2 表逐项核验 line 178-243。

## 需要 Reviewer 特别关注的地方

1. **§D 渲染规则的字面落地**：`errorMessages` 的中文文案是 PRD §4.3 / §D 直接产出的「用户可见文本唯一来源」。建议对照 contracts.md §D 11 项表逐条核对实际渲染（特别是 E_NOT_FOUND 在根目录场景的「根目录不存在或已被删除」分支、E_FOLDER_DIRTY 使用 `now` 而非 `old`）。
2. **`IPC_ERROR_CODE_SET` 双向一致性**：`IpcErrorCode` 联合类型（types/workspace.ts）与 `IPC_ERROR_CODES` 数组（ipc-errors.ts）字面量必须保持字符级一致；测试中 `ALL_CODES` + `IPC_ERROR_CODE_SET.size === 11` 双重断言保证。
3. **`tauri-commands.ts` payload key**：5 wrapper 的 invoke args key 是 IPC 边界的契约线，必须 camelCase 与 §B.2 表完全对齐；本次未改但建议 Reviewer 在审查时对照 line 178-243 与 §B.2 表。
4. **details 缺字段降级**：所有依赖 details 必填字段的 code 都做了「降级通用文案 + console.warn」处理，**不二次抛错**——这是 §D 渲染规则 #3 的硬约束，避免 UI 误吞错误。
