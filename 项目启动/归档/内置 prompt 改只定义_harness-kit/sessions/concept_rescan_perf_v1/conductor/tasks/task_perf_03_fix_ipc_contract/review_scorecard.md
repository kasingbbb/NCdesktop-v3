# Review Scorecard — task_perf_03_fix_ipc_contract

## 审查思考过程

1. **Task 意图**：修复 task_perf_02 review_scorecard 的 BLOCKER #1（"前端调旧 wrapper `extract_concepts_for_library` 但传新参数名 `forceFull`，与后端 `force: bool` 不匹配"）+ MAJOR #1（"output.md 与实际代码事实不符"）。采用 review_scorecard "首选修复方向"：切前端 invoke 到 task_perf_01 已注册的新 IPC `start_concept_extraction`（参数 `force_full: bool`），保持 JS payload 形态 `{ libraryId, forceFull }` 不变，依赖 Tauri 自动 camelCase ↔ snake_case 转换。

2. **AC 检查结果**：见下方矩阵。

3. **关键发现**：
   - **BLOCKER 已修复**：tauri-commands.ts:620 invoke command name 从 `"extract_concepts_for_library"` 切到 `"start_concept_extraction"`，payload `{ libraryId, forceFull }` 与后端 `start_concept_extraction(library_id: String, force_full: bool)` 经 Tauri camelCase 转换后完全对齐。
   - **MAJOR 已修复**：tauri-commands.ts:601-614 新增 JSDoc 明确说明"调用新入口，因旧 wrapper 参数名不兼容 forceFull"，文档与代码事实一致。
   - **范围严守**：实际改动仅 2 个文件（前端 IPC wrapper + 测试 mock 断言），Rust 零触碰，store / view / types 零触碰，progress.md 零触碰，PR-4 / R6 零触碰。

## AC 检查结果

| AC | 项 | 结果 | 证据 |
|----|----|----|----|
| Fix-1 | invoke 第一个参数为 `"start_concept_extraction"` | ✅ | tauri-commands.ts:620（grep 在 `src/lib/`、`src/components/` 全仓库扫描，旧 command name 字面量已清零） |
| Fix-2 | payload 保持 `{ libraryId, forceFull }` | ✅ | tauri-commands.ts:621-622 |
| Fix-3 | 测试 mock 断言同步到新 command name | ✅ | KnowledgeAssociationView.test.tsx:325 `expect(cmdName).toBe("start_concept_extraction")`；line 294 it 描述更新为"...调用 start_concept_extraction" |
| Fix-4 | 后端 `start_concept_extraction` 已在 invoke_handler 注册 | ✅ | src-tauri/src/lib.rs:231（task_perf_01 已落地，本 Fix 未改） |
| Fix-5 | tsc 0 error + vitest 全绿 | ✅ | tsc 0 输出 0 error；vitest 6 file / 71 test all passed |
| Fix-6 | 范围合规：仅前端 1-3 行核心 + 测试同步，未触碰 Rust / 已 PASS 产物 / progress.md | ✅ | git diff 显示本次 Fix 仅触 `tauri-commands.ts`（+15/-3）+ `KnowledgeAssociationView.test.tsx` mock 断言节；其它文件改动属并行 task_perf_01/02 已交付物，与本 Fix 无关 |

## 评分

session_context.md 未单独指定权重，沿用 task_perf_02 review_scorecard 已使用的权重映射（功能正确性 25% / 性能 25% / 错误隔离 15% / 进度反馈 15% / 代码质量 10% / 测试覆盖 10%）。本 Fix 是 narrow 修复，性能 / 进度反馈 / 错误隔离维度本身不在变更范围内，按"是否破坏既有维度"评估。

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | BLOCKER #1 根除——前端 invoke 切到正确入口，payload key `forceFull` 经 Tauri camelCase→snake_case 转换后精确匹配后端 `force_full: bool`。同文件反例对照 `synthesizeKnowledgeUnits({ libraryId, force })` ↔ Rust `force: bool`（tauri-commands.ts:649-657）证实该转换约定全仓库稳定 |
| 性能 | 25% | 5 | 仅切 IPC 入口，后端 4 路并发 + buffer_unordered + 8KiB 截断逻辑全部完整继承（在 start_concept_extraction 主函数中，非旧 wrapper），用户首次看到的 7-10min 性能收益现已能真正触达 |
| 错误隔离 | 15% | 5 | 后端 emit `status / error` 字段、前端 `extractionProgress.error` 写入路径、UI "扫描出错…"分支全部未改；error 字段透传链路完整 |
| 进度反馈 | 15% | 5 | 5 状态推导（preboot/starting/running/completed/error）+ 脉冲 + 文案 + ETA 一概未改，task_perf_02 已 PASS 维度零回归 |
| 代码质量 | 10% | 5 | JSDoc 注释明确说明"为何走新入口"，避免未来读者重蹈覆辙；改动范围最小化（diff 实质 4 行代码 + 14 行注释）；JS 端导出签名 `(libraryId, forceFull)` 保持 task_perf_02 既定形态，store/view 零感知 |
| 测试覆盖 | 10% | 4 | mock 断言 `toBe("start_concept_extraction")` 已锁定 command name；payload `toEqual({ libraryId, forceFull: true })` 已锁定 key 形态。**未加** review_scorecard BLOCKER#1 验证标准 #4 推荐的"严格契约 mock"（mock 仅当 payload 含 forceFull 时 resolve）—— Dev 在 output.md § 需要 Reviewer 关注 #3 给出理由（避免与 Tauri runtime 序列化耦合，且当前 mock 已足够），Reviewer 接受此判断为 MINOR 而非 MAJOR |

**综合分：4.9/5**（加权：5×0.25 + 5×0.25 + 5×0.15 + 5×0.15 + 5×0.10 + 4×0.10 = 1.25 + 1.25 + 0.75 + 0.75 + 0.5 + 0.4 = **4.9**）

## 总体判断

- [x] **PASS**
- [ ] FIX
- [ ] BLOCKER

**理由**：BLOCKER #1（IPC 契约错位）+ MAJOR #1（文档失真）双双根除；tsc 0 error / vitest 71/71 全绿 / grep 证据链完整；范围严守，零回归。本 Fix 是 review_scorecard "首选修复方向"的精确执行。

## 跨契约一致性矩阵（5 项）

| 项 | 后端（task_perf_01 已交付） | 前端（task_perf_02 + 本 Fix） | 一致？ | 状态 |
|----|------|------|--------|------|
| **IPC command name** | `start_concept_extraction` 已注册（src-tauri/src/lib.rs:231） | `invoke("start_concept_extraction", ...)`（tauri-commands.ts:620） | ✅ | Fix 后唯一一处 invoke 已切到新 command，旧 wrapper 仅作向后兼容保留 |
| **payload force_full / forceFull** | `force_full: bool`（src-tauri/src/commands/knowledge.rs:123） | `forceFull: boolean` payload key（tauri-commands.ts:622） | ✅ | Tauri 默认 ArgumentCase::Camel 把 `force_full` 转 JS key `forceFull`，匹配 |
| **payload library_id / libraryId** | `library_id: String`（src-tauri/src/commands/knowledge.rs:122） | `libraryId: string` payload key（tauri-commands.ts:621） | ✅ | 同上 Tauri 转换约定；反例对照 `synthesize_knowledge_units` 印证 |
| **status 字段** | emit `status` 字面 JSON（knowledge.rs:601） | `progress.status` listen + types/knowledge.ts:74 | ✅ | task_perf_02 已对齐，本 Fix 未改 |
| **事件名** | `notecapt/concept-extraction-progress`（knowledge.rs:595） | `listen("notecapt/concept-extraction-progress", ...)`（KnowledgeAssociationView.tsx:77） | ✅ | task_perf_02 已对齐，本 Fix 未改 |

**结论：5/5 ✅** 跨端契约完整一致。

## Tauri camelCase 转换约定佐证

| 前端 invoke payload key | 后端 Rust 参数名 | Tauri 自动转换 | 仓库内状态 |
|---|---|---|---|
| `{ libraryId }`（tauri-commands.ts:55, 多处） | `library_id: String` | camelCase ↔ snake_case | ✅ 仓库内多 command 长期工作 |
| `{ libraryId, force }`（tauri-commands.ts:653-656，synthesize_knowledge_units） | `library_id: String, force: bool` | camelCase ↔ snake_case（force 单词无下划线，转换前后字面相同） | ✅ 工作正常 |
| **`{ libraryId, forceFull }`**（本 Fix 后） | **`library_id: String, force_full: bool`**（start_concept_extraction） | camelCase ↔ snake_case | ✅ 与上述模式同构 |

## 问题列表

### BLOCKER

无。

### MAJOR

无。

### MINOR

1. **未加严格契约锁定测试**（review_scorecard task_perf_02 BLOCKER#1 验证标准 #4 推荐）
   - **代码位置**：`src/components/features/knowledge/__tests__/KnowledgeAssociationView.test.tsx`
   - **症状**：当前 mock 仅 `expect(cmdName).toBe("start_concept_extraction")` + `expect(payload).toEqual(...)`；review_scorecard 推荐改 mock 为"只有 payload key 是 `forceFull` 时才 resolve，否则 reject"以增强契约抵抗未来误改。Dev 在 output.md § 需要 Reviewer 关注 #3 给出合理理由（Tauri runtime 序列化耦合 / E2E 才是终极锁定）。
   - **修复方向**：（可选）后续可另开 task 加 E2E 烟测覆盖真机 IPC 跨端往返。
   - **验证标准**：本期可不动，不阻塞 PASS。

2. **未做真机 Tauri runtime 端到端冒烟**（Dev output.md § 已知局限 #1）
   - **症状**：本 Fix 限定为代码层 + 单测层，未启动 `tauri dev` 真机点按钮验证。
   - **修复方向**：Reviewer 阶段或 Conductor PASS 后人工 QA 执行。
   - **验证标准**：本期不阻塞 PASS（mock + grep + 反例对照 + 后端 wrapper 注册四重锁定已足够把发布风险压到极低）。

3. **task_perf_02 review_scorecard 的 4 项遗留 MINOR**（completed 态永久挂屏 / mock 双 microtask 时序 / ETA 公式硬编码 60 秒 / 完成态无 timer 自动收起）—— 本 Fix 范围之外，按 task_perf_02 自承延后到 P2。

## 实跑验证记录（Reviewer 现场）

### 1. tsc

```bash
$ cd "/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop"
$ pnpm tsc --noEmit
（0 输出，0 error）
```

✅ 0 type error。

### 2. vitest

```bash
$ pnpm test src/lib/__tests__/ src/components/features/knowledge/ --run

 RUN  v4.1.1 /Users/.../NCdesktop

 Test Files  6 passed (6)
      Tests  71 passed (71)
   Start at  10:17:16
   Duration  1.26s
```

✅ 全部 71 个测试通过（含 task_perf_02 14 条 + v1.3 task_009 占位 + 本 Fix 更新过的 IPC mock 断言）。

### 3. grep 契约证据

**后端注册（未改，本 Fix 不动）**：
```
src-tauri/src/lib.rs:228:            commands::knowledge::extract_concepts_for_library,
src-tauri/src/lib.rs:231:            commands::knowledge::start_concept_extraction,
src-tauri/src/commands/knowledge.rs:119: pub async fn start_concept_extraction(... library_id: String, force_full: bool, ...)
src-tauri/src/commands/knowledge.rs:461: pub async fn extract_concepts_for_library(... library_id: String, force: bool, ...)
```

✅ `start_concept_extraction` 已在 invoke_handler 中注册（line 231），前端不会撞 "command not found"。

**前端 invoke（Fix 后）**：
```
src/lib/tauri-commands.ts:620:  return invoke<ConceptExtractionProgress>("start_concept_extraction", {
src/components/features/knowledge/__tests__/KnowledgeAssociationView.test.tsx:325:    expect(cmdName).toBe("start_concept_extraction");
```

✅ 全仓库 `src/lib/` + `src/components/` 范围内，旧 command name 字面量已清零；新 command name 出现两次（实际 invoke + 测试断言），形成闭环。

### 4. git diff 范围合规

```
src/lib/tauri-commands.ts                                       | 21 +-
src/components/features/knowledge/__tests__/KnowledgeAssociationView.test.tsx  | （含 task_perf_02 既有改动 + 本 Fix mock 断言更新）
```

✅ 本 Fix 实际改动仅 2 个前端文件，Rust 零触碰，store/view/types 零触碰。

## R6 / 已 PASS 产物零触碰核验

| 文件/模块 | 状态 |
|---|---|
| `src-tauri/**`（所有 Rust） | ✅ 本 Fix 未触碰（task_perf_01 改动属于其交付范围） |
| `src/components/features/knowledge/KnowledgeAssociationView.tsx` | ✅ 本 Fix 未触碰 |
| `src/stores/knowledgeStore.ts` | ✅ 本 Fix 未触碰 |
| `src/types/knowledge.ts` | ✅ 本 Fix 未触碰 |
| R6 / User Prompt 相关：`PromptCustomizationPanel.tsx` / `SettingsPanel.tsx` / `userPromptStore.ts` / `types/user-prompt.ts` / `tauri-commands.ts` 内 User Prompt 段（line 817-854） | ✅ 全部未触碰 |
| PR-4 半成品 / `promptStore.ts` | ✅ 未触碰 |
| `progress.md` | ✅ 未触碰 |

## 给 Conductor 的建议

- **本 Fix PASS**，可立即 commit + 进入 ACCEPTANCE 阶段。
- task_perf_02 review_scorecard 的 BLOCKER #1 + MAJOR #1 双双根除，task_perf_02 评分实际效果上从 3.075/5 提升至预期 ≥ 4.5/5（功能正确性 2→5、代码质量 4→5、测试覆盖 3→4 等）。
- task_perf_01 review_scorecard 的 BLOCKER（同源问题，描述为后端侧）现已通过前端切换入口规避——后端旧 wrapper 保留作向后兼容、新 wrapper 主路径正常工作，task_perf_01 也可视为 BLOCKER 解决（无需改 Rust）。
- 真机 E2E 冒烟建议在 ACCEPTANCE 阶段执行（启动 `tauri dev`，导航至"知识库 → 知识关联 → 重新扫描"，验证进度条不再瞬间跳"扫描出错"）。
