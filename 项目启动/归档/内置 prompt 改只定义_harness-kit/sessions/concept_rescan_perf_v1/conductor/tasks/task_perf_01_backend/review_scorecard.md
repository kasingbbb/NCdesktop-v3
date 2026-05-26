# Review Scorecard — task_perf_01_backend

## 审查思考过程

1. **Task 意图**：把 NCdesktop 概念抽取后端从严格串行改为 4 路并发 + 8 KiB content 截断 + 错误隔离 + P1 增量扫描（V16 `assets.concept_extracted_at` 字段），目标 87 文档全量 84 min → < 10 min。
2. **AC 检查结果**：AC-1（V16）✅；AC-2（并发）✅；AC-3（截断）✅；AC-4（错误隔离）✅；AC-5（增量）✅；AC-6（emit_progress 不破坏）✅；AC-7（cargo 全绿 + 已 PASS 产物零触碰）✅。
3. **关键发现**：**前端 IPC 契约破裂（BLOCKER）** —— 前端调旧名 `extract_concepts_for_library` 传 `forceFull`，但后端 wrapper 函数签名是 `force: bool`（不是 `force_full`），Tauri v2 默认 `rename_all = "camelCase"` 把后端参数 `force` 直接当作 JS key `force`，前端传 `forceFull` 时找不到 key → `Error::InvalidArgs("missing required key force")`，"重新扫描"按钮在生产环境一点必失败。

---

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 2 | 单元测试齐全且全 PASS（355 lib + 20 e2e），但前端 IPC 桥接路径在生产会立即失败（见 BLOCKER-1）；新 IPC `start_concept_extraction` 本身完整、增量/全量逻辑齐全 |
| 性能 | 25% | 5 | `buffer_unordered(4)` 闭包结构正确（DB 锁短作用域、LLM HTTP 锁外、INSERT OR IGNORE 兜底竞争）；content 截断到 8 KiB byte-safe UTF-8；预估 87 doc 全量 5.4 min（< 10 min 目标） |
| 错误隔离 | 15% | 5 | 单文档失败仅 `log::error!` 不抛 `?`、processed 推进、concepts_found 不变、失败者 `concept_extracted_at` 保持 NULL（下次增量自动重试）；`buffer_unordered_with_simulated_failures_isolates_errors` 实测覆盖 |
| 进度反馈 | 15% | 5 | `emit_progress` 函数签名 / 事件名 / payload 字段全部零改动；`AtomicUsize` + `Relaxed` 并发安全计数器；`atomic_counter_concurrent_increments_yield_correct_total` 实测覆盖 100 并发 fetch_add |
| 代码质量 | 10% | 4 | 注释详尽；常量集中；INSERT OR IGNORE + 重查 id 的并发竞争兜底设计合理；扣 1 是因为 wrapper 参数命名（`force` 而非 `force_full`）与前端 camelCase 桥接断裂 |
| 测试覆盖 | 10% | 4 | 12 单元测试 + 4 新 migration 测试齐全；buffer_unordered + truncate + 增量过滤实测覆盖；但**缺一个直接验证 `extract_concepts_for_library(force_full)` 参数桥接的集成测试**——若有，本次 BLOCKER 在 Dev 自测阶段就能被捕获 |

**综合分**：25%×2 + 25%×5 + 15%×5 + 15%×5 + 10%×4 + 10%×4 = **0.5 + 1.25 + 0.75 + 0.75 + 0.4 + 0.4 = 4.05/5**

---

## 总体判断

- [x] **BLOCKER**

理由：前后端 IPC 参数名桥接断裂，"重新扫描"按钮在生产环境必失败。修复极简（改 wrapper 参数名 `force` → `force_full`），但**未修复前必失败**，符合 BLOCKER 定义。

---

## 问题列表

### BLOCKER（必须修复，否则不可能 PASS）

#### BLOCKER-1：旧 IPC wrapper 参数名与前端 payload 不匹配，"重新扫描"按钮在生产环境立即失败

- **问题**：
  - 后端 wrapper 函数签名：`extract_concepts_for_library(db, app, library_id: String, force: bool)`
  - 前端 invoke 调用：`invoke("extract_concepts_for_library", { libraryId, forceFull })`
  - Tauri v2 默认 `ArgumentCase::Camel`（参 `tauri-macros-2.5.5/src/command/wrapper.rs:50` `argument_case: ArgumentCase::Camel`），把 Rust 参数 `force` 转 JS key `force`（lowerCamelCase of `force` = `force`），不是 `force_full`，也不是 `forceFull`。
  - Tauri ipc/command.rs:97-103：`v.get(self.key) → None → Err("command extract_concepts_for_library missing required key force")`，直接抛错回前端。
  - 前端 store catch 分支会把这个 Tauri Error 写入 `extractionProgress.error`，UI 显示"扫描出错：…missing required key force…"
- **代码位置**：
  - `src-tauri/src/commands/knowledge.rs:465`（wrapper 签名 `force: bool`）
  - `src/lib/tauri-commands.ts:620-623`（前端 invoke payload `{ libraryId, forceFull }`）
- **修复方向**（**只动一行**）：
  1. 把 `src-tauri/src/commands/knowledge.rs:465` 的参数名 `force` 改为 `force_full`；
  2. `src-tauri/src/commands/knowledge.rs:467` 内调用 `start_concept_extraction(db, app, library_id, force).await` 同步改为 `force_full`；
  3. 函数注释（line 459 "旧 `force=true` 等价新 `force_full=true`"）改为说明 wrapper 本身就用 `force_full`，仅函数名不同。
- **验证标准**：
  - `cargo test --lib knowledge` 仍全 PASS（不应影响单测）
  - 加一个新单测：用 `serde_json::json!({"libraryId": "x", "forceFull": true})` 模拟前端 payload 字面构造，验证 Tauri 反序列化 `forceFull` → Rust `force_full` 参数 = true（可参考 `tauri::test::mock_invoke`，或者更轻量地用 `serde_json::from_value::<{ library_id: String, force_full: bool }>` 验证 serde 端转换正确）
  - 前端 vitest 测试不变（前端 mock 不验证真实 Tauri 序列化，但生产环境验证靠 Conductor 协调的 e2e 烟测）

---

### MAJOR（强烈建议修复）

#### MAJOR-1：`extract_concepts_for_library` wrapper 持有 DB `State` 调用 async fn `start_concept_extraction(db, ...)` 是 borrow-跨-await 的反模式风险

- **问题**：wrapper line 467 `start_concept_extraction(db, app, library_id, force).await` 把 `State<'_, Database>` 直接转交给 inner async fn。Tauri 的 `State<'_, T>` 携带 lifetime `'_ = 'r`，理论上能跨 await 因为 State 本身是 `Send + Clone-ish ref`，但**模式上**让 wrapper 和 inner 持有同一个 State 引用会让借用检查器对未来的 wrapper 改动（比如想在转发前加 metric 钩子）非常脆弱。
- **代码位置**：`src-tauri/src/commands/knowledge.rs:461-468`
- **修复方向**：保持现状即可（编译通过），但建议在 wrapper 上加一行注释说明 State 转交是有意的、不是手误。或更彻底的：让 wrapper 不持有 State，直接通过 `app.state::<Database>()` 内取一次后转交给 inner。本期不强制。
- **验证标准**：cargo build 0 error / 0 new warning。

---

### MINOR（可选）

1. **`mark_asset_concept_extracted` 用 SQLite `datetime('now')`**（line 699）而其他写入用 `chrono::Utc::now().to_rfc3339()`（line 311）。两种时间戳格式不同（SQLite 写 `YYYY-MM-DD HH:MM:SS` 无时区，chrono 写 RFC3339 含 Z 后缀）。两套时间字段共存在 assets 表上不影响功能（只有 `concept_extracted_at` 走 datetime('now')），但和 V14 `legacy_unverified` 一样的范式不一致小瑕疵。建议统一为 chrono RFC3339；非阻塞。

2. **`fetch_library_assets` 旧函数标 `#[allow(dead_code)]`**（line 610）但调用 `fetch_library_assets_for_extraction(conn, library_id, true)` —— 实质成为别名而非旧实现。若不打算保留回退诊断，建议直接删除以减少代码膨胀；保留也无副作用。

3. **`#[deprecated]` 的两个 build_*_prompt 函数**（line 788 / 818）仍带 `#[allow(dead_code)] + #[deprecated]` 双标记。Dev output.md 第 9 行声称"wrapper 用普通文档注释而非 `#[deprecated]`"，但这两个函数（task_004 既有遗留产物，非本 task 新加）仍有 `#[deprecated]`。属 task_004 遗留，本 task 不应修。不影响本 task review。

---

## 前后端契约一致性矩阵

| 项 | 后端 task_perf_01 | 前端 task_perf_02 | 一致？ | 风险 |
|----|--------------------|---------------------|--------|------|
| **IPC command 名** | 同时注册 `start_concept_extraction`（新）+ `extract_concepts_for_library`（旧 wrapper），见 `src-tauri/src/lib.rs:228, 231` | 调旧名 `extract_concepts_for_library`，见 `src/lib/tauri-commands.ts:620` | ✅ | 旧 wrapper 接住了前端调用，IPC 路由 OK |
| **payload state/status** | emit_progress 用 `status`（`src-tauri/src/commands/knowledge.rs:601`），未改 | listen 读 `event.payload.status`（`src/components/features/knowledge/KnowledgeAssociationView.tsx:84`），types/knowledge.ts:74 也是 `status` | ✅ | 双方一致 — `status` 不是 `state`，input.md AC-6 的 `state` 字段是命名错误，Dev 选择保留 `status` 与既有前端契合，正确决策 |
| **forceFull 参数** | wrapper 函数签名 `force: bool`（`src-tauri/src/commands/knowledge.rs:465`）；Tauri 默认 camelCase 把 `force` 序列化为 JS key `force` | invoke payload `{ libraryId, forceFull }`（`src/lib/tauri-commands.ts:622`） | ❌ | **BLOCKER**：前端 `forceFull` 找不到对应后端 `force` key → Tauri 反序列化 fail → 重新扫描按钮在生产必失败 |
| **事件名** | `notecapt/concept-extraction-progress`（`src-tauri/src/commands/knowledge.rs:595`）+ `notecapt/concept-extraction-done`（line 443）；零改动 | listen `notecapt/concept-extraction-progress`（`src/components/features/knowledge/KnowledgeAssociationView.tsx:77`） | ✅ | 字面一致 |

---

## 给 Dev 的修复指引

### 问题清单（按优先级排序）

#### BLOCKER

1. **修复 `extract_concepts_for_library` wrapper 参数名**
   - **代码位置**：`src-tauri/src/commands/knowledge.rs:465-467`
   - **修复方向**：把 wrapper 的参数 `force: bool` 改为 `force_full: bool`，并同步更新 line 467 转发参数；注释也相应调整（"语义：旧 `force=true` 等价新 `force_full=true`" → 改为"参数名 `force_full` 与新 IPC 一致，仅命令名 `extract_concepts_for_library` 与 `start_concept_extraction` 并存"）。
   - **验证标准**：
     - `cargo test --lib knowledge` 全 PASS（既有 34 单测 + 1 新增 = 35 PASS）
     - 新增单测 `extract_concepts_wrapper_accepts_force_full_camelcase_payload`：构造 `serde_json::json!({"libraryId": "lib_x", "forceFull": true})`，通过 `serde_json::from_value::<MockArgs>` 反序列化到一个临时 struct `{ library_id: String, force_full: bool }`（mock 后端参数解析），断言 `force_full == true`
     - 前端 vitest 不需要新加（前端 mock 不验真 Tauri serde）

#### MAJOR

1. **wrapper 转发 DB State 注释**（可选）
   - **代码位置**：`src-tauri/src/commands/knowledge.rs:461-468`
   - **修复方向**：在 wrapper 函数 doc-comment 中加一行说明 `State<'_, Database>` 是有意转交给 inner async fn，未来加 metric/log 钩子时要先 destructure 出 conn 引用。

### 修复范围约束

- **只修以上列出的 BLOCKER-1**（MAJOR-1 注释建议可选 / MINOR 全部 P2）
- **不要改 progress.md**
- **修复完成后必须重跑**：
  - `cargo test --lib migration` ≥ 12 PASS
  - `cargo test --lib knowledge` ≥ 35 PASS（含新增 1 个）
  - `cargo test --lib` ≥ 355 PASS
  - `cargo test --test user_prompt_e2e` 20/20 PASS
  - `cargo build` 0 error / 5 既有 warning（无新增）

---

## 实跑验证记录（Reviewer 现场）

| 命令 | 结果 |
|------|------|
| `cargo test --lib migration` | **12 passed; 0 failed**（fresh_db_to_v16 / idempotent / v16_existing_column 全 PASS） |
| `cargo test --lib knowledge` | **34 passed; 0 failed**（buffer_unordered / atomic / truncate / 增量 / mark 全 PASS） |
| `cargo test --lib`（全表） | **355 passed; 0 failed**（基线 342 + V15 = 343；本 task +12 = 355；与 output.md 预期吻合） |
| `cargo test --test user_prompt_e2e` | **20 passed; 0 failed**（task_004/008 用户自定义 prompt 链路完全保持） |
| `cargo build` | **0 error / 5 warning**（与改造前 main 分支完全一致，0 new warning） |
| grep 已 PASS 产物 | `prompt_runtime.rs` / `chat.rs` / `commands/llm.rs` / `commands/user_prompt.rs` / `db/user_prompt.rs` 零改动 ✅ |
| grep 事件名 + IPC 名一致性 | `notecapt/concept-extraction-progress` / `notecapt/concept-extraction-done` 前后端字面一致 ✅；`start_concept_extraction` 已 invoke_handler 注册（lib.rs:231）；旧 `extract_concepts_for_library` wrapper 同时注册（lib.rs:228）✅ |
| Tauri serde 行为求证 | `tauri-macros-2.5.5/src/command/wrapper.rs:50` 确认默认 `ArgumentCase::Camel`；`tauri-2.10.3/src/ipc/command.rs:97-103` 确认 key 缺失直接返回 `"missing required key {key}"` Error。综合验证 BLOCKER-1 在生产必复现 |
