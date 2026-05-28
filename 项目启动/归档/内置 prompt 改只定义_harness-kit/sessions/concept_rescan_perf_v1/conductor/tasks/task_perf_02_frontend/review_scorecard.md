# Review Scorecard — task_perf_02_frontend

## 审查思考过程

1. **Task 意图**：改造"知识关联 → 重新扫描" UI，消除"0/87 卡死"错觉（脉冲条 + 5 状态分支）、按钮 running 时 disabled、新增 IPC `forceFull` 参数透传，与并行 task_perf_01 后端契约对齐。
2. **AC 检查结果**：见下方矩阵。
3. **关键发现**：
   - **BLOCKER #1**：前端 invoke `extract_concepts_for_library` 时 payload 写 `forceFull`，但后端该 command 的 Rust 参数是 `force: bool`（不是 `force_full`），Tauri 2 IPC 反序列化会失败。该 task 自称"已自检 ✅"（output.md line 124）但事实不符。点击"重新扫描"按钮会在运行时报错并跳到"扫描出错…"分支。
   - 5 状态进度条 / 按钮态 / 文案 / 测试结构性都正确，仅"接哪个后端 command + 用哪个参数名"这一关键决策错了。

## AC 检查结果

| AC | 项 | 结果 | 证据 |
|----|----|----|----|
| AC-1 | 5 状态进度条（preboot/starting/running/completed/error）+ 脉冲 + 错误兜底 | ✅ | KnowledgeAssociationView.tsx:370-496；test 5 状态全覆盖 |
| AC-2 | running 按钮 disabled + aria-disabled + 文案 + title | ✅ | KnowledgeAssociationView.tsx:216-238；test 3 用例覆盖 |
| AC-3 | IPC 参数透传 forceFull | ❌ | **payload key 名与后端 `extract_concepts_for_library(force: bool)` 不一致**（详 BLOCKER #1）。AC-3 input.md 第 42-46 行同时给出了 `start_concept_extraction(libraryId, forceFull)` 的范本，task_perf_02 dev 选择保留旧 command 名却把 payload 改成 `forceFull` —— 两边都没对上 |
| AC-4 | 既有 listen 不破坏 + payload 类型保留 + `error` 字段可选 | ✅ | tsx:74-100；types/knowledge.ts:70-80；后端 emit 字段名匹配 |
| AC-5 | tsc 0 error + vitest 全绿 | ✅ | tsc 实跑 0 输出；vitest 14/14 passed |

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 2 | 5 状态推导/按钮态/listen 全部正确（4 项 AC ✅），但 AC-3 IPC 调用层契约 broken — 点击按钮在真实 runtime 会立即跳错误态。该按钮是本期主交付路径，跑不通即"扫描中…"按钮态根本看不到，整条主链路死锁 |
| 性能 | 25% | 3.5 | 本 task 仅前端 UI，性能维度借后端 task_perf_01 实现；前端代码无性能负担（一次 useEffect + 监听 + 状态推导），但因 IPC 契约错误，后端的 7-10min 并发收益用户实际拿不到 |
| 错误隔离 | 15% | 4 | error 字段透传 + 兜底"未知错误"+ store catch 分支写 extractionProgress.error。前端层面隔离逻辑齐全；但**讽刺的是**这次唯一会触发的错误就是 BLOCKER 引发的 serde 报错本身 |
| 进度反馈 | 15% | 5 | 5 状态推导清晰；脉冲态/preboot/启动中文案分支扎实；ETA 公式正确（87 × 60 / 4 / 60 = 22 分钟）；data-phase 属性便于 e2e；视觉一致性高 |
| 代码质量 | 10% | 4 | tsx 重写后边界清楚；data-testid + data-phase 数据属性提升可测性；JSDoc 充分；唯一硬伤是注释/output.md 反复声明"已对齐后端 force_full"但实际未对齐，文档准确性失守 |
| 测试覆盖 | 10% | 3 | 14 个 it 覆盖 5 状态 + 按钮态 + invoke 调用层；但 AC-3 测试只断言"前端给 mock 传了什么"，**完全不能捕获跨端契约错误**（mock 永远 resolve 成功）。建议至少加一个集成式断言 `invoke` 的 payload key 必须在后端能 deserialize 的合法形式（或锁定后端真实参数名） |

**综合分：3.075/5**（加权计算：2×0.25 + 3.5×0.25 + 4×0.15 + 5×0.15 + 4×0.10 + 3×0.10）

## 总体判断

- [ ] PASS
- [ ] FIX
- [x] **BLOCKER**

**理由**：契约不一致直接让"重新扫描"按钮**在真实 runtime 无法工作**。AC-3 验证失败，主交付路径断链。session_context 权重下"功能正确性"占 25%，无法 PASS。

## 前后端契约一致性矩阵

| 项 | 后端 task_perf_01 | 前端 task_perf_02 | 一致？ | 风险 |
|----|--------------------|---------------------|--------|------|
| **IPC command 名** | 同时注册 `extract_concepts_for_library` (wrapper, `force: bool`) 与 `start_concept_extraction` (新, `force_full: bool`)（src-tauri/src/lib.rs:228,231；commands/knowledge.rs:119, 461） | invoke `"extract_concepts_for_library"`（tauri-commands.ts:620） | ⚠️ 名字匹配，但**选错入口** — 前端调旧 wrapper（参数名 `force`），却传新参数名 `forceFull`；本可调新 command `start_concept_extraction`（参数名 `force_full`），与前端的 `forceFull` camelCase 自动转换匹配 | BLOCKER |
| **payload 字段 `force` vs `forceFull`** | 旧 command 期望 `force: bool`（自 JS 端为 `force`），新 command 期望 `force_full: bool`（自 JS 端为 `forceFull`） | 实际发送 `forceFull: true` | ❌ 后端 `extract_concepts_for_library` 函数签名是 `force: bool`，frontend `forceFull` 不会自动转成 `force` — Tauri 序列化失败 / 缺字段报错 | BLOCKER |
| **payload `status` vs `state`** | 后端 emit `status` 字段（knowledge.rs:601）；ExtractionProgress struct rename_all=camelCase 保留 status；task_perf_01 output.md §6.2 明确指出"用 `status` 不要用 `state`" | 前端读 `progress.status`（5 处），types/knowledge.ts:74 已是 `status` | ✅ 完全对齐 | 无 |
| **事件名 `notecapt/concept-extraction-progress`** | knowledge.rs:595 字面 `"notecapt/concept-extraction-progress"` | KnowledgeAssociationView.tsx:77 字面 `"notecapt/concept-extraction-progress"` | ✅ 完全对齐 | 无 |
| **payload 字段 totalAssets / conceptsFound** | 后端 emit `totalAssets / processed / conceptsFound / status`（camelCase 字面 JSON） | 前端 listen 回调 `event.payload.totalAssets / processed / conceptsFound / status` | ✅ 字面对应 | 无 |
| **error 字段** | 后端当前不在 emit 中放 `error`（task_perf_01 output.md 第 4 节明确：单文档失败仅 log，不 emit） | 前端 `event.payload.error ?? null`，兜底类型 `error?: string \| null` | ✅ 前向兼容（fallback 到 null） | 无 |

## 问题列表

### BLOCKER（必须修复）

1. **IPC 调用契约错位：调用旧 command 名却传新参数名**
   - **代码位置**：`src/lib/tauri-commands.ts:616-624`、`src/components/features/knowledge/KnowledgeAssociationView.tsx:108-111`
   - **症状**：前端 `invoke("extract_concepts_for_library", { libraryId, forceFull })`，后端该 command 签名是 `extract_concepts_for_library(library_id: String, force: bool)`（src-tauri/src/commands/knowledge.rs:461-468）。Tauri 2 把 JS 的 `forceFull` 转 `force_full` 后送进 Rust，但 Rust 期望的字段名是 `force`，**反序列化失败**。点击"重新扫描"按钮 → Tauri reject string → store catch 分支 → 进度条直接跳到"扫描出错…"。
   - **证据**：output.md line 28 / line 103-124 多次声明"task_perf_01 backend 当前实现的参数名已是 `force_full`"——**事实上只有新 command `start_concept_extraction` 是 `force_full`；旧 wrapper `extract_concepts_for_library` 是 `force`**（task_perf_01 output.md line 44 也明确说"旧 IPC 名保留兼容"）。同仓库的 `synthesize_knowledge_units(force: bool)` ↔ frontend `{ libraryId, force }` 是反例对照（tauri-commands.ts:649-657）。
   - **修复方向**（任选其一）：
     - **首选**：把 `tauri-commands.ts::extractConceptsForLibrary` 改为调用新 command `start_concept_extraction`，命令名/参数名都对齐后端推荐入口（task_perf_01 output.md 第 211-221 行的接口建议）；保留函数名/签名以避免改动调用方。这也与 input.md AC-3 第 42-46 行给出的范本一致。
     - **替代**：保留旧 command 名 `extract_concepts_for_library`，把 invoke payload 改回 `force`（不改 JS 函数签名上的 `forceFull` 命名，只在 invoke 调用处重映射）。但失去与新 command 语义靠齐的好处。
   - **验证标准**：
     1. `grep -n "invoke.*extract_concepts_for_library\|invoke.*start_concept_extraction" src/lib/tauri-commands.ts` 中的 command 名与后端 Rust 函数名匹配
     2. invoke payload key 必须是该 Rust 函数参数名 camelCase 形式（`force` ↔ `force` 或 `force_full` ↔ `forceFull`）
     3. 重跑 vitest 14/14 仍绿
     4. **新增一条 e2e 性质的断言**（推荐）：在测试中把 mock invoke 注册成"只有 payload 包含 `force_full` snake_case 或 `force` 时才 resolve，否则 reject"——把契约写进测试，避免下次复发

### MAJOR

1. **自检/output.md 与实际代码事实不符**
   - **代码位置**：output.md 第 28 行、第 103 行、第 114 行、第 124 行
   - **症状**：多处声明"已 grep 确认 backend `force_full`，对齐 OK"，但 grep 显示后端旧 command 仍是 `force`。这是高优先级问题因为它**让 Reviewer 失去对自检结论的信任基线**，未来类似交叉契约场景会重复踩坑。
   - **修复方向**：修复 BLOCKER #1 后同步更新 output.md 表述，明确说明"调用的是新 command `start_concept_extraction`"或"调用旧 command 用旧参数名 `force`"。
   - **验证标准**：output.md 自检表条目能被任意 reviewer 用一条 grep 在 60 秒内复现。

### MINOR

1. **进度条 completed 态永久挂屏**（output.md 第 97 行已自承）。本期接受，P2 处理。
2. **测试的 invoke mock 双 `await Promise.resolve()` 时序兜底**（output.md 第 105 行已自承）。本地稳定，未来如出现 CI flaky 改 `waitFor`。
3. **ETA 公式硬编码 60 秒/篇**：task_perf_01 实测降到 ~15 秒/篇（output.md 第 169-176 行性能预估）。本期文案与"截断前"实测一致，不强求改但建议未来 P2 同步。
4. **完成态进度条无"自动收起"timer**（output.md 第 97 行）。

## 给 Dev 的修复指引

### 修复范围约束

- **只修以上 BLOCKER 与 MAJOR**，不要连带重构 5 状态进度条 / 按钮态 / 测试结构（这些维度已 PASS）。
- 修复涉及前端**单一文件**（`src/lib/tauri-commands.ts`），不应触及 `KnowledgeAssociationView.tsx` / `knowledgeStore.ts` / `types/knowledge.ts` / 测试以外的代码。
- **不修改任何 Rust 文件**（task_perf_01 范围 — 物理隔离）。
- **不修改 progress.md**。
- **不修改 R6 文件**：`PromptCustomizationPanel.tsx / SettingsPanel.tsx / userPromptStore.ts / types/user-prompt.ts`。

### 修复后必跑

```bash
cd "/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop"
pnpm tsc --noEmit
pnpm test src/components/features/knowledge/ --run
grep -n "extract_concepts_for_library\|start_concept_extraction" src/lib/tauri-commands.ts src-tauri/src/commands/knowledge.rs src-tauri/src/lib.rs
```

预期产出：
- tsc 0 error
- vitest 14/14 pass（含新增的 1 条契约锁定断言）
- grep 输出显示前端 invoke 的 command 名与后端 `#[tauri::command]` 函数名一一映射，且 payload key 名对应 Rust 参数 camelCase 形式

## 实跑结果

### tsc

```
$ pnpm tsc --noEmit
（0 行输出，0 error）
```

### vitest

```
Test Files  1 passed (1)
     Tests  14 passed (14)
   Start at  10:03:31
   Duration  955ms
```

注：vitest 14/14 全绿但不能证明 IPC 契约一致 —— mock invoke 永远 resolve，不会暴露 backend serde 失败。

### grep 契约证据

| 文件 | 行号 | 内容 |
|---|---|---|
| `src/lib/tauri-commands.ts:620` | invoke | `invoke<...>("extract_concepts_for_library", { libraryId, forceFull })` |
| `src-tauri/src/commands/knowledge.rs:461-465` | 后端 wrapper | `pub async fn extract_concepts_for_library(... library_id: String, force: bool)` |
| `src-tauri/src/commands/knowledge.rs:119-123` | 后端新 command | `pub async fn start_concept_extraction(... library_id: String, force_full: bool)` |
| `src-tauri/src/lib.rs:228,231` | 注册 | 两个 command 都已注册 |

## R6 / 已 PASS 产物零触碰核验

| 文件/模块 | 状态 |
|---|---|
| `src/components/features/PromptCustomizationPanel.tsx` | ✅ 未触碰（git diff 空） |
| `src/components/features/SettingsPanel.tsx` | ✅ 未触碰 |
| `src/stores/userPromptStore.ts` | ✅ 未触碰 |
| `src/types/user-prompt.ts` | ✅ 未触碰 |
| `tauri-commands.ts` 中 User Prompt 段落（line 817-854） | ✅ 未触碰（diff 仅在 line 598-625） |
| 任何 Rust 文件 | ✅ task_perf_02 dev 未改（worktree 中 Rust 改动全部归 task_perf_01）|
| `progress.md` | ✅ 未触碰 |
| PR-4 半成品 / promptStore.ts | ✅ 未触碰 |
