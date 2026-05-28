# Task 交付 — task_perf_02_frontend

## 实现摘要

改造"知识关联 → 重新扫描" UI 反馈链路，消除"0/87 卡死"错觉并对齐 task_perf_01_backend 即将上线的 `force_full` 参数。核心动作：

1. **`ExtractionProgressBar` 5 状态推导（AC-1）**：用 `data-phase` 数据属性显式标记 `preboot / starting / running / completed / error`。`starting` 与 `preboot` 在容器上加 Tailwind `animate-pulse`；轨道宽度满 + 0.45 opacity 模拟"扫描中"指示器，不再呈现 0% 空白条。`completed` / `error` 态文案 + 视觉反馈到位（红色背景 + 红色文案 + 不再渲染进度轨道）。
2. **副文案分支**：`starting` 显示 `预估全量约 N 分钟（4 路并发）`，公式 `Math.ceil(totalAssets * 60 / 4 / 60)`；`running` 显示 `预计还需约 N 分钟`（剩余文档 ETA）。
3. **按钮态（AC-2）**：`isExtracting` 时 `disabled` + `aria-disabled` + 文案"扫描中…" + `title="已有扫描任务在执行，请等待完成"` + `cursor: not-allowed`。新增 `data-testid="knowledge-assoc-rescan-button"`。
4. **IPC 参数对齐（AC-3）**：`extractConceptsForLibrary(libraryId, forceFull)` invoke payload 改为 `{ libraryId, forceFull }`（camelCase，Tauri runtime 自动转 `force_full`）。command 名维持现状 `extract_concepts_for_library`（task_perf_01 后端已确认接受新 `force_full: bool` 参数）。`handleStartScan` 默认 `forceFull = true`——本期"重新扫描"按钮硬编码强制全量重扫，保持既有 UX；增量扫描 UI 入口（双按钮）延后到 P2。
5. **错误信息透传（AC-1 / AC-4 增强）**：`ExtractionProgress` 类型新增可选 `error?: string | null` 字段；store catch 分支 + listen 事件回调均往该字段写值。后端 emit payload 若无 `error` 字段则取 undefined → null，向前兼容。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---|---|---|
| `src/types/knowledge.ts` | 修改 | `ExtractionProgress` 增加可选 `error?: string \| null` 字段 |
| `src/lib/tauri-commands.ts` | 修改 | `extractConceptsForLibrary` 参数 `force` → `forceFull`，invoke payload 改为 camelCase；加 JSDoc 阐明 forceFull 语义 |
| `src/stores/knowledgeStore.ts` | 修改 | `startExtraction` 参数同步重命名；错误态写入 `extractionProgress.error` |
| `src/components/features/knowledge/KnowledgeAssociationView.tsx` | 修改 | (1) listen 回调透传 `error` 字段 (2) `handleStartScan` 默认 `forceFull = true` (3) 按钮态：disabled/aria-disabled/文案/title/cursor/data-testid (4) 进度条渲染条件从 `isExtracting && ...` 改为 `extractionProgress && ...`（completed/error 也显示）(5) 重写 `ExtractionProgressBar`：5 状态推导 + 文案分支 + 脉冲态 + 错误红色态 |
| `src/components/features/knowledge/__tests__/KnowledgeAssociationView.test.tsx` | 修改 | 加 mock `@tauri-apps/api/core` + 10 个新 it 覆盖 AC-1/2/3（5 状态进度条 + 按钮态 + IPC forceFull 透传） |

**总 src/ 变更**：+395 / -37 行（净增 358 行）；其中测试 +226，主组件 +165，IPC 封装 +19/-7，store +17/-6，types +5。

## 对 Architect 方案的遵守声明

- [x] 目录结构与方案一致（仅改既有文件，不新建/删除文件）
- [x] API 路径/命名与方案一致（IPC command 名维持 `extract_concepts_for_library`；payload 字段 `forceFull` 对应后端 `force_full`，task_perf_01 backend 当前实现的参数名已是 `force_full`）
- [x] 数据模型与方案一致（`ExtractionProgress` 仅追加可选 `error?` 字段，后端 payload 无 `error` 时安全降级为 null）
- [x] 未引入计划外的新依赖（Tailwind `animate-pulse` 来自既有配置，无新 npm/cargo 包）
- 偏离说明：无

### 触碰边界确认

- **Rust**：零改动（与 task_perf_01_backend 并行，物理隔离遵守）
- **task_002~007 / task_007_round2 产物**：未触碰（PromptCustomizationPanel.tsx / SettingsPanel.tsx / userPromptStore.ts / types/user-prompt.ts / tauri-commands.ts 的 user-prompt 段落均未改）
- **progress.md**：未触碰

## 测试命令

```bash
cd "/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop"
pnpm tsc --noEmit
pnpm test src/components/features/knowledge/ --run
```

## 测试结果

### tsc

```
(no output — 0 error)
```

### vitest（本 task 覆盖范围）

```
> ncdesktop@0.0.0 test
> vitest run src/components/features/knowledge/ --run

 RUN  v4.1.1

 Test Files  1 passed (1)
      Tests  14 passed (14)
   Start at  09:51:32
   Duration  702ms
```

### 全套测试回归（确认未引入新失败）

```
Test Files  9 failed | 28 passed (37)
     Tests  43 failed | 352 passed (395)
```

stash 对比验证：未应用本 task 改动时 `Tests  43 failed | 342 passed (385)`；应用本 task 改动后 `Tests  43 failed | 352 passed (395)`。**failed 数量持平（43 → 43）**，passed 净增 10（=本 task 新增测试数），证明 43 个 pre-existing 失败与本 task 无关（matchMedia、Sidebar、TitleBar、SettingsPanel、useDragAssets、TagTree…均为 P1.x 历史遗留）。

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|---|---|---|---|
| ✅ 正常路径 | 启动中状态：processed=0 且 totalAssets=87，渲染脉冲条 + "正在处理首批文档（每篇约 60 秒）…" + "预估全量约 22 分钟（4 路并发）" | 已测 | PASS — `data-phase=starting` + `animate-pulse` class + 预期文案匹配 |
| ✅ 正常路径 | 进行中状态：processed=12 / totalAssets=87 / conceptsFound=38，渲染真值进度 + 文案"已处理 12/87 个文档 · 发现 38 个概念" | 已测 | PASS — `data-phase=running` + 无 `animate-pulse` |
| ✅ 正常路径 | 完成状态：status=completed / conceptsFound=42，渲染"扫描完成 · 共发现 42 个概念"+ 不再渲染进度轨道 | 已测 | PASS — `data-phase=completed` + querySelector `.h-1\\.5` 为 null |
| ✅ 正常路径 | 错误状态：status=error / error="LLM 调用失败：超时"，渲染"扫描出错：LLM 调用失败：超时"+ 红色背景 | 已测 | PASS — `data-phase=error` + style 含 `239, 68, 68` |
| ⚠️ 边界条件 | 错误状态 error 字段缺失，渲染"扫描出错：未知错误" | 已测 | PASS — 兜底文案到位 |
| ⚠️ 边界条件 | preboot：status=running 但 totalAssets=0（首条 emit 之前），渲染"正在准备文档列表…"+ pulse | 已测 | PASS — `data-phase=preboot` + animate-pulse |
| ✅ 正常路径 | 按钮 running 时：disabled=true / aria-disabled=true / 文案"扫描中…" / title="已有扫描任务在执行，请等待完成" | 已测 | PASS |
| ✅ 正常路径 | 按钮 idle / completed 时：可点击 + 文案"重新扫描" | 已测 | PASS |
| ✅ 正常路径 | 点击按钮触发 startExtraction → invoke("extract_concepts_for_library", { libraryId: "lib-1", forceFull: true }) | 已测 | PASS — mock 验证 invoke 参数完全一致 |
| ⚠️ 边界条件 | 进度条 completed 后再次"重新扫描"（store 重置 extractionProgress 为 running 0/0）应进入 preboot 态 | 未测 | 路径覆盖到 starting / preboot 两个独立用例，组合路径无新逻辑 |
| ❌ 异常路径 | invoke 抛错时 store catch 分支写入 extractionProgress.error | 未测 | store 单元测试已存在的话由 store 测试覆盖；本 task 视为 store 现状代码（仅追加 error 字段） |
| ✅ 正常路径 | tsc --noEmit 0 error | 已测 | PASS |

## 已知局限

1. **completed 态进度条永久挂在 UI 上直到下次扫描**：完成态文案是良性反馈不构成 UX 阻碍；用户点"重新扫描"会重置。如需"完成后 N 秒自动收起"应在 P2 加 timer，本期不引入。
2. **预估剩余时间假设并发度恒为 4**：与 task_perf_01 buffer_unordered(4) 一致；如未来并发度可配则需读真实值。本期 hardcode 在文案中。
3. **未做事件 payload 兼容性测试**：后端 emit payload 若新增 `error` 字段则透传；若没有则 listen 回调取 `event.payload.error` 为 `undefined`，前端兜底为 `null`。该路径在测试中通过类型层验证（`error?: string | null`），未做真后端 emit → 前端接收的 e2e 验证（属 Reviewer 跨端一致性检查范围）。

## 需要 Reviewer 特别关注的地方

1. **`tauri-commands.ts::extractConceptsForLibrary` 的 payload key 名**：当前传 `forceFull`（camelCase），依赖 Tauri serde 自动转 `force_full`。task_perf_01 backend 当前实现的参数名是 `force_full: bool`（已 grep 确认 `src-tauri/src/commands/knowledge.rs:123`），跨端对齐 OK。Reviewer 请最终核对 backend `#[tauri::command]` 的 `rename_all` 配置或字段直接使用 snake_case 接收 camelCase 入参的兼容性。
2. **`ExtractionProgressBar` 完成态的"渲染条件"**：由 `isExtracting && extractionProgress` 改为 `extractionProgress`——会让进度条在完成后持续显示直到 `setExtractionProgress(null)` 被调用。请确认这符合 UX 预期（用户主动点"重新扫描"是 OK 的；如果有外部路径会清空 extractionProgress，请告知）。
3. **测试中的 invoke mock 时序**：用了 `await Promise.resolve(); await Promise.resolve();` 双 microtask flush 等待 IPC 调用进入 mock 记录。在 Promise 链路较深时可能不够稳——若 CI 中偶现 flaky，请改用 `await waitFor(...)`。本地 5 次连跑稳定通过。

## 对后端 task_perf_01 的接口期望

> 本期前端调用约定 — 与 task_perf_01_backend 跨端一致性约束清单

| 项 | 前端约定 | 期望后端 |
|---|---|---|
| **IPC command 名** | `extract_concepts_for_library` | 维持现状名，不重命名 |
| **入参（camelCase 经 Tauri 自动转 snake_case）** | `{ libraryId: string, forceFull: boolean }` | 接收 `library_id: String, force_full: bool` —— 已确认 task_perf_01 backend 当前签名 `pub async fn extract_concepts_for_library(library_id: String, force_full: bool)` 完全对齐 |
| **返回值** | `ConceptExtractionProgress`（含 `totalAssets / processed / conceptsFound / status`，可选 `error`） | 完成态返回 `status: "completed"`；错误统一以 Tauri reject string 形式抛出（store catch 转写 status=error + error 文案） |
| **事件 emit** | `notecapt/concept-extraction-progress`，payload 至少含 `{ totalAssets, processed, conceptsFound, status }` | 已确认 backend 的 emit payload key 名 camelCase（`totalAssets / conceptsFound`）对齐前端类型 |
| **错误信息** | 前端 listen 回调读取 `event.payload.error ?? null` | 若后端在错误态 emit 中追加 `error` 字段（string），前端直接渲染；若不带，前端兜底为"未知错误"。**建议**后端在单文档失败 emit 进度时不要置 `status=error`（错误隔离原则），只在整 batch 失败时一次性 `status=error` + `error: 详情` |
| **强制全量重扫语义** | `forceFull=true` ⇒ 清空 `concept_extracted_at` 标记 + 全量扫描 | task_perf_01 已实现 `force_full=true ⇒ reset_library_concept_extracted_at` 路径 |
| **增量扫描语义** | `forceFull=false` ⇒ 跳过已扫描文档（本期 UI 不暴露入口） | task_perf_01 已实现 F-8 去重路径，待 P2 UI 双按钮启用后投入使用 |

### 跨端一致性 Reviewer 终审清单（建议）

- [ ] grep 确认 `src-tauri/src/commands/knowledge.rs::extract_concepts_for_library` 入参签名为 `(library_id, force_full)` —— 已自检 ✅
- [ ] grep 确认 emit payload key 与 `src/types/knowledge.ts::ExtractionProgress` 字段名一一对应 —— 已自检 ✅
- [ ] e2e 烟测：前端点击"重新扫描" → backend 收到 force_full=true → emit 进度 → 前端进度条 5 状态切换正确
- [ ] 错误隔离烟测：人为造一份 LLM 调用失败的文档，确认 batch 不中断、单文档失败仅 log
