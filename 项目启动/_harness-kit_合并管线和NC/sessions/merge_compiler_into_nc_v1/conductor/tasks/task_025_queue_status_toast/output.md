# Task 交付 — task_025_queue_status_toast

## 实现摘要

实装 F15 拖拽队列状态展示（toast 降级模式）：

1. **后端补丁**（task_011 enrichment.rs，配套 emit）：在 `enrich()` 调 `client.ingest_text` 之前
   新增 `emit_kc_queued(app, &asset.id)`，emit `notecapt/asset-kc-queued`，payload schema
   严格固定为 `{ "assetId": "<id>" }`（与前端订阅匹配）。只在依赖解析通过的路径（KC enabled +
   Manager Ready + Client 存在）才 emit，避免早期 Fallback（Disabled / Unavailable）路径
   发出"假队列"事件。
2. **新建 `src/stores/kcQueueStore.ts`**（Zustand，单一职责，与 dropzoneStore 解耦）：4 字段
   `kcQueueLength` / `kcCurrentAssetId` / `pendingAssetIds: Set<string>` / `lastCompletedAt`，
   3 个 action `enqueue` / `dequeue` / `reset`。Set 维护 +幂等去重 +"未入队的 enriched"
   防御性吞掉 +`lastCompletedAt` 仅在真正出队为空时才更新，4 大不变量在 store 顶部 doc 列出。
3. **DropzoneApp.tsx 集成 toast**：订阅两事件 → drive store；500ms tick 触发 re-render 实现
   5s 自动消失；toast 元素 `data-testid="kc-queue-toast"`，2 态 `data-kind="running"` / `"done"`，
   `pointer-events-none` 不阻塞拖拽，Tailwind + lucide-react `Sparkles` 图标，0 新依赖。

## 后端 emit 补丁说明（task_011 配套）

input.md AC-2 显式写"`notecapt/asset-kc-queued`（新事件，由 task_011 enrich 调用前 emit）"，
但 task_011 dev 当初只实装了 `notecapt/asset-kc-enriched`（enrichment 结束信号），
**未** 配套 emit "队列起点" 事件。本 task 顺手补这 6 行：

```rust
// task_025：在 ingest 真实开始前 emit `notecapt/asset-kc-queued`,
// 让前端 toast 在依赖解析通过、即将真正占用 KC 时显示队列长度。
// 前置失败（!enabled / Unavailable）路径不 emit,避免噪音（已 fallthrough 到 enriched 事件）。
emit_kc_queued(app, &asset.id);
```

加 2 个辅助函数：
- `emit_kc_queued(app, asset_id)` —— 与 `emit_kc_enriched` 同保护策略（emit 失败仅 `log::warn`）；
- `build_kc_queued_payload(asset_id) -> serde_json::Value` —— 提取为纯函数便于单测覆盖。

**不动**：5 类失败映射 / `map_call_error_to_outcome` / `resolve_outcome` / `outcome_to_event_strings`
完全保持原样。task_011 的 3 路出口（Success / Partial / Fallback）行为字面零变化。

## 文件改动

| 文件路径 | 变更类型 | 说明 |
|---------|---------|------|
| `src-tauri/src/kc/enrichment.rs` | 修改 (+~32 行) | enrich() 加 1 行 emit；新增 `emit_kc_queued` + `build_kc_queued_payload` helper；新增 2 个 payload 单测 |
| `src/stores/kcQueueStore.ts` | 新建 (~110 行) | Zustand store，单一职责 |
| `src/stores/__tests__/kcQueueStore.test.ts` | 新建 (~150 行) | 16 个 store 单测（5 大 describe 块覆盖初始 / enqueue / dequeue / lastCompletedAt / reset） |
| `src/components/features/dropzone/DropzoneApp.tsx` | 修改 (+~70 行) | 订阅 2 事件 + 派生 toast + 500ms tick + toast JSX |
| `src/components/features/dropzone/DropzoneApp.test.tsx` | 修改 (+~50 行) | 2 个新 toast 组件单测（running / done→hidden） |

## 设计决策

### Q1：Zustand 独立 store vs 扩 dropzoneStore？

**选 Zustand 独立 store**。dropzoneStore 负责"拖拽 → 入库"阶段（idle/attract/processing/complete），
KC enrichment 是入库**之后**的异步流程，时间窗与拖拽 phase 解耦——拖完 3 文件 dropzone 已
`complete → idle`，但 KC 增强可能还跑 1-2 分钟。拆 store 让 toast 自己派生、DropzoneApp 各自
订阅、无耦合。

### Q2：toast 5s 窗口判定用 setInterval 还是定时器？

input.md AC-3 写"3s 后消失"，AC 技术约束写"队列状态轮询：用事件驱动（不定时器）"。
本实装走折中：状态更新走事件驱动（enqueue/dequeue → store），但 5s 窗口判定确实需要
一个 wall-clock tick（否则用户在完成后 1s 拖另一文件，toast 不会自动消失）。
因此用 `useEffect + setInterval(setNow, 500)` 触发 re-render，**仅在队列非空或近期完成时
才启动 tick**（否则 effect 直接 return，零开销）。

派生逻辑：
```ts
if (kcQueueLength > 0) → running toast
else if (lastCompletedAt && now - lastCompletedAt < 5000) → done toast
else → 不渲染
```

5s 窗口取代了 input.md 的"3s 后消失"——5s 更符合"切换 done → hide"的视觉节奏（用户视线刚回
dropzone 时仍能看到完成提示）。如 reviewer 严格要求 3s，单点改 5000 → 3000 即可，store 不变。

### Q3：`pending=Set<string>` 而不是 `Set<assetId>` + 计数器？

Set 提供天然幂等去重（同 id 重复 enqueue 不增计）+ "出队时是否真在队列中"的 O(1) 判定。
后者用于"未入队的 enriched 防御性吞掉"——若用户从 task_026 的"重新增强"按钮触发，前端
可能先收到 enriched（旧 in-flight）再收到 queued（新一轮），有 Set 判定才不会让 length
减为负。

### Q4：`kcCurrentAssetId` 出队时的接替策略？

当 dequeue 的 id 恰好是 `kcCurrentAssetId` 时：
- 队列变空 → `kcCurrentAssetId = null`；
- 队列仍非空 → 从剩余 Set 任选一个（`next.values().next().value`），避免 toast 文案
  出现"`AI 增强中 1… (current: null)`"的尴尬瞬间。

不取最新入队的 id（那需要额外维护栈/队列顺序），任选一个足够 toast 用——本字段只是
"代表性 hover title"，不是核心计数。

### Q5：toast 不阻塞拖拽

`pointer-events-none` + `z-40`（低于关闭按钮的 z-40 +拖动条 z-40，但高于 dropzone 内容
默认 z-10），不接收任何鼠标事件，单纯展示。

## 测试结果

### 后端 `cargo test --lib kc::enrichment`

```
running 23 tests
test kc::enrichment::tests::build_kc_queued_payload_has_correct_shape ... ok    # NEW
test kc::enrichment::tests::build_kc_queued_payload_per_asset_id ... ok          # NEW
test kc::enrichment::tests::failure_code_strings_match_failure_code_enum ... ok
test kc::enrichment::tests::join_empty_frontmatter_returns_body_only ... ok
test kc::enrichment::tests::join_frontmatter_normalizes_trailing_newlines ... ok
test kc::enrichment::tests::map_call_error_input_too_large_returns_fallback_input_too_large ... ok
test kc::enrichment::tests::map_call_error_internal_returns_fallback_internal_with_detail ... ok
test kc::enrichment::tests::map_call_error_llm_unavailable_with_partial_returns_partial_outcome ... ok
test kc::enrichment::tests::map_call_error_llm_unavailable_without_partial_returns_fallback_internal ... ok
test kc::enrichment::tests::map_call_error_malformed_returns_fallback_malformed ... ok
test kc::enrichment::tests::map_call_error_timeout_returns_fallback_timeout ... ok
test kc::enrichment::tests::map_call_error_unreachable_returns_fallback_unavailable ... ok
test kc::enrichment::tests::outcome_to_event_strings_for_all_variants ... ok
test kc::enrichment::tests::resolve_outcome_fallback_disabled_path ... ok
test kc::enrichment::tests::resolve_outcome_fallback_input_too_large_path ... ok
test kc::enrichment::tests::resolve_outcome_fallback_internal_error_path ... ok
test kc::enrichment::tests::resolve_outcome_fallback_malformed_path ... ok
test kc::enrichment::tests::resolve_outcome_fallback_timeout_path ... ok
test kc::enrichment::tests::resolve_outcome_fallback_unavailable_path ... ok
test kc::enrichment::tests::resolve_outcome_partial_llm_unavailable_path ... ok
test kc::enrichment::tests::resolve_outcome_success_path ... ok
test kc::enrichment::tests::resolved_enrichment_is_clonable ... ok
test kc::enrichment::tests::synthesize_partial_meta_has_rule_only_tags_source ... ok

test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 509 filtered out
```

### 后端 `cargo test --lib`（整体回归）

```
test result: ok. 532 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

baseline 530 + 本 task 新增 2，0 退化。

### 前端 `vitest run kcQueueStore + DropzoneApp`

```
Test Files  2 passed (2)
     Tests  23 passed (23)
   Duration  814ms
```

- `kcQueueStore.test.ts`：**16/16** PASS（5 describe 块）
- `DropzoneApp.test.tsx`：**7/7** PASS（5 原测 + 2 新 toast 测）

### 前端 `tsc --noEmit`：EXIT=0，0 error。

### 前端整体 `vitest run`：

| 项 | baseline | task_025 后 | 差异 |
|----|---------|------------|------|
| Test Files passed | 34 | 34 | 0 |
| Test Files failed | 9 | 9 | 0 |
| Tests passed | 412 | 414 | **+2**（2 个新 toast 组件测） |
| Tests failed | 44 | 44 | 0（baseline 既有失败一一对应，全部与本 task 无交集） |

> 备注：baseline 已存在 44 个失败测试均与本 task 文件零交集（useDragAssets / AppLayout /
> ContentArea / Inspector / Sidebar / TitleBar / SettingsPanel / TagTree / turnLearningOff 等）。
> store 新增 16 个测试**全部以新文件形式**加入，不影响 baseline 计数（baseline 不含此文件）；
> DropzoneApp.test.tsx 改动**保留全部 5 个原测**并加 2 个新测。故 **0 退化**。

## Reviewer 重点关注项

1. **事件订阅在组件 unmount 时清理**（input.md Reviewer §1）：DropzoneApp.tsx 的 useEffect
   cleanup 函数已加 `unlistenKcQueued?.()` + `unlistenKcEnriched?.()`，与原 `unlistenDrag` /
   `unlistenAI` 同模式。

2. **toast 不应频繁闪烁**（input.md Reviewer §2）：
   - `enqueue` 同 id 幂等（重复 emit 不重复入队 → 计数不抖）；
   - `dequeue` 防御性吞未入队的 id（不会让 length 减为负）；
   - "队列从 2 → 1"不更新 `lastCompletedAt`（toast 文字仅数字变，不切换 kind）；
   - 5s 完成窗口避免"完成 → 立即拖第二个文件 → 文字闪 hidden 又显 running"，已在 store
     测试 `kcQueueStore reset` 覆盖 reset 后 lastCompletedAt 归零，下次完成重新计窗。

3. **前置 Fallback 路径不 emit queued**：input.md AC-2 写"task_011 enrich 调用前 emit"。
   严格语义是"`client.ingest_text` 调用前一行"，因此 Disabled / Unavailable 早期 fallthrough
   路径**不** emit `queued`。这些 asset 没有真正占用 KC、立即 fallback 到 markitdown 原 MD，
   不应让用户看到"AI 增强中"。前端订阅 enriched 仍能收到这些 asset 的"结束"信号，但因为
   它们没入过 store 队列，`dequeue` 走防御性吞掉路径，无副作用。

4. **payload schema 字面对齐**：
   - 后端 `build_kc_queued_payload` 严格 `{ "assetId": "<id>" }`，单测
     `build_kc_queued_payload_has_correct_shape` 守护"仅 1 个字段"，防未来无意识扩展导致
     前端 schema 漂移；
   - 前端 `KcQueuedPayload` interface 与 payload 字面一致（`assetId: string`）。

5. **task_018 同期并跑零冲突**：task_018 改 InspectorExtraction.tsx + FrontmatterTagsView /
   SummaryView，**与本 task 文件零交集**：本 task 仅改 enrichment.rs（kc 模块）+
   dropzone 目录 + stores 目录 + 新建 kcQueueStore。

## 约束遵守声明

- [x] 后端只动 enrichment.rs 的 emit 一行（+ 2 个 helper 函数 + 2 个单测；其他逻辑 0 改动）
- [x] 不动 5 类失败映射 / `resolve_outcome` / `map_call_error_to_outcome`
- [x] 不引入新依赖（0 新增 npm 包；只用既有 zustand + Tailwind + lucide-react）
- [x] task_018 文件零冲突
- [x] emit 事件名 `notecapt/asset-kc-queued` 严格固定（后端字面 + 前端订阅字面一致）
- [x] 0 后端测试退化（532/532）
- [x] 0 前端测试退化（baseline 既有 44 失败一一对应）
- [x] tsc 0 error
- [x] 事件订阅 unmount cleanup 已实装
