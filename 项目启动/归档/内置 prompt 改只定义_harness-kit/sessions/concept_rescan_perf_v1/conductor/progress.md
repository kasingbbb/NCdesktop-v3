# Conductor Progress — concept_rescan_perf_v1

## 当前状态

- **STATE**: `ACCEPTANCE`（IPC Fix PASS，所有 BLOCKER 关闭，等待 PM 真机验收）
- **当前 Task**: 无（流水线已完成）
- **更新时间**: 2026-05-16

---

## 项目摘要

- **Session**: concept_rescan_perf_v1
- **项目名称**: NCdesktop — 知识概念重新扫描性能优化
- **复杂度**: S+（跳过 Debate + 完整 Architect；2 task 并行，文件零交集）
- **诊断真相来源**: `/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/docs/diagnose_concept_rescan_perf.md`
- **Session Context**: `sessions/concept_rescan_perf_v1/session_context.md`

---

## 优化目标量化

| 场景 | 当前 | 目标 |
|------|------|------|
| 首次全量扫描 87 文档 | ~84 min | **< 10 min** |
| 增量扫描（1-2 新文档） | ~84 min（不区分） | **< 30 秒** |
| 首次点击到第一条进度 | 58 秒（用户感知"卡死"） | **< 1 秒**（脉冲条 + 文案） |

---

## 已完成 Tasks

- **task_perf_01_backend** — V16 migration + buffer_unordered(4) 并发 + 8 KiB byte-safe 截断 + 错误隔离 + force_full 增量扫描 + AtomicUsize 并发安全。Reviewer 4.05/5（AC-1~7 全 ✅；初轮 BLOCKER 在 IPC 参数命名 — 已由 task_perf_03 在前端侧解决）。lib 355/355 + e2e 20/20，build 0 error / 0 new warning。生产代码 +814/-146（含 ~430 测试）。DONE 2026-05-16
- **task_perf_02_frontend** — 5 状态进度条（preboot/starting/running/completed/error）+ animate-pulse + 文案预估 + 按钮态 disabled/aria + IPC 调用层 forceFull 参数。Reviewer 3.075/5（AC-1/2/4/5 ✅，AC-3 BLOCKER 在 IPC 入口选错 — 由 task_perf_03 修复）。tsc 0 error + vitest 14/14。+395/-37。DONE 2026-05-16
- **task_perf_03_fix_ipc_contract** — 前端 invoke 字符串从 `"extract_concepts_for_library"` 切到 `"start_concept_extraction"` + 测试 mock 断言同步。Reviewer 4.9/5 PASS（5 项契约一致性全 ✅，0 BLOCKER / 0 MAJOR / 3 MINOR）。实际 2 行核心 + 4 行注释/测试断言。tsc 0 error + vitest 71/71。DONE 2026-05-16

---

## 当前 Task 详情

**并行波次 #1（task_perf_01 + task_perf_02）**

| Task ID | 描述 | 状态 | 文件 |
|--------|------|------|------|
| `task_perf_01_backend` | P0-1 并发 buffer_unordered(4) + P0-2 content 截断 8 KB + P1 增量字段 | dispatched | `src-tauri/src/commands/knowledge.rs` + `src-tauri/src/db/migration.rs`（如需 V16）|
| `task_perf_02_frontend` | P0-3 UI 文案优化 + 脉冲条 + 时长预估提示 | dispatched | `src/components/features/knowledge/KnowledgeAssociationView.tsx` |

> 并行依据：后端改 Rust / 前端改 TS，文件零交集；P1 增量需 backend 新增 DB 字段 → 前端如需触发"全量重扫"按钮，等 backend 完成后 task_perf_02 v2 微改即可。本期前端只做文案 + 脉冲条。

---

## 待执行 Task 队列

- [ ] `task_perf_01_backend`（进行中）
- [ ] `task_perf_02_frontend`（进行中）
- [ ] 后续：双 PASS 后即可 ACCEPTANCE（无 task_010 终审，S+ 跳过）

---

## 已知问题 / Blockers

### IPC 契约 BLOCKER（2026-05-16，双 Reviewer 独立识别）

- **现象**：前端 `tauri-commands.ts:616-624` 调用 `invoke("extract_concepts_for_library", { libraryId, forceFull })`，但后端旧 wrapper 签名是 `extract_concepts_for_library(library_id: String, force: bool)`（参数名 `force` 不是 `force_full`）
- **Tauri 序列化规则**：v2 默认 ArgumentCase::Camel，单字符串参数 `force` → JS key `force`；`force_full` → JS key `forceFull`。前端送 `forceFull` 给期望 `force` 的旧 wrapper → 反序列化失败 `Error::InvalidArgs("missing required key force")`
- **后果**：生产环境用户点击"重新扫描"必失败（vitest mock 因永远 resolve 无法捕获，跨端契约盲区）
- **同时 task_perf_01 已注册新 IPC `start_concept_extraction(library_id, force_full)` 正确等待**：前端只需切换 invoke 字符串即可
- **Fix 路径**：前端 `tauri-commands.ts:616-624` `invoke("extract_concepts_for_library", ...)` → `invoke("start_concept_extraction", ...)`，payload `{libraryId, forceFull}` 不变；Tauri 自动 `forceFull → force_full` 转换

---

## 关键决策记录

- **[2026-05-16]** 复杂度判定为 S+：代码量 ~150 行后端 + 30 行前端，2 文件零交集，无数据迁移（P1 增量字段是新增 column 而非 schema 改变）。跳过完整 Debate + Architect 流程。
- **[2026-05-16]** P1 增量扫描的 DB 字段命名采用 `assets.concept_extracted_at TEXT NULL`（与 task_002 V15 风格一致）。

---

## 状态转移日志

[2026-05-16] STATE: ∅ → DEVELOPING | Task: task_perf_01 + task_perf_02 并行启动 | 原因: S+ 复杂度，跳过 Debate + Architect；诊断报告作为 task input 真相来源；2 task 文件零交集 | 风险: 低
[2026-05-16] STATE: DEVELOPING → REVIEWING | Task: task_perf_01 + task_perf_02 双交付 | 原因: 后端 lib 355/355 + e2e 20/20，前端 tsc 0 error + vitest 14/14 | 风险: 中（已警示前后端 status/state 字段疑似不一致）
[2026-05-16] STATE: REVIEWING → FIX | Task: 双 Reviewer 独立识别 IPC BLOCKER | 原因: 前端调旧 wrapper `extract_concepts_for_library` 期望参数 `force`，但前端传 `forceFull` → 生产环境必失败；vitest mock 无法捕获跨端契约错误。Fix 方案：前端切到新 IPC `start_concept_extraction`（< 5 行） | 风险: 低（修复极简）
[2026-05-16] STATE: FIX → ACCEPTANCE | Task: task_perf_03 Fix PASS（4.9/5）| 原因: 5 项契约一致性全 ✅；vitest 71/71；tsc 0 error；BLOCKER 0；范围合规仅前端 2 文件 6 行。流水线终结，等待 PM 真机验收（重新扫描 87 文档实测耗时） | 风险: 低
