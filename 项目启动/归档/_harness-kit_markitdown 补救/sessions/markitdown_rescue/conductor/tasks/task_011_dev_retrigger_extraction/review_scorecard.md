# Review Scorecard — task_011_dev_retrigger_extraction

**裁定**：**FIX**（含 1 个 BLOCKER · 1 个 MAJOR · 1 个 INFO）
**综合分**：72 / 100

---

## 审查前验证（契约 8 字段）

| 字段 | 状态 |
|------|------|
| 完成状态 DONE | ✓ |
| 改动文件清单 | ✓（5 文件） |
| 关键设计要点 | ✓（三道幂等护栏 + 锁释放顺序） |
| 校验结果 | ✓（cargo check 0 error；3 单测 pass；tsc 0 error） |
| AC 自检 | ✓（AC-1/2/3/5 PASS；AC-4 标记待手测） |
| 未触碰文件 guard 声明 | ✓ |
| 设计依据/参考 | ✓ |
| 风险与遗留 | 部分（未识别 scheduler State 未注册） |

---

## 思考协议

### Task 意图
统一 Inspector "重试"入口到后端 `retrigger_extraction` 命令；从 failed/extracted 任一态都能干净重跑；不允许跳过 pipeline 直接置 extracted；幂等防重入队。

### AC 逐条核对

- **AC-1**：`retrigger_extraction(app, asset_id) -> Result<(), String>`，签名匹配（input.md 写的是 `(database, app, asset_id)`，Dev 实现 `(app, asset_id)` —— 因 `Database` 是 Tauri State，从 `app` 取即可，等价且更地道）。校验 asset 存在 ✓；status reset 为 queued ✓；error_message 清空 ✓；pipeline_tasks reset + retry_count=0 ✓；调用 `PipelineScheduler::enqueue` ✓。**PASS**（但见 BLOCKER-1）。
- **AC-2**：`retriggerExtraction(assetId)` 已暴露 + 类型化（void return）。**PASS**。
- **AC-3**：`retryExtraction` 切到 `retriggerExtraction`，前端 statusCache 立即置 `queued`，并 fetch 拉新；无遗留前端模拟逻辑。**PASS**。
- **AC-4**：手测 3 场景脚本清晰；但因 BLOCKER-1，手测大概率会在 happy path 中触发 panic。**待手测且当前实现存在阻塞缺陷**。
- **AC-5**：lib.rs invoke_handler 第 133 行 `commands::extraction::retrigger_extraction`。**PASS**。

### 关键发现

#### Conductor 提请独立判断的两点

1. **mod.rs 修复**：合理修复，非 scope creep。
   - 亲验 diff：仅追加 `pub mod extraction;` 一行；非破坏性。
   - 不修复则 lib.rs 第 133 行无法编译，task_011 的命令注册无效。
   - 与 task_008 发现的 3 处缺口同类问题（pre-existing 文件存在但 mod.rs 未声明）。
   - **判断：合理 call**。

2. **是否激活了旧的 `extract_asset` / `retry_extraction` 命令？**
   - 亲验 lib.rs invoke_handler 全量（第 51-134 行）：**未注册** `extract_asset` 或 `retry_extraction`。
   - 它们存在于 `extraction.rs:8` / `:45`，有 `#[command]` 属性，但未进 invoke_handler。
   - mod.rs 修复仅让模块对外可见、消除 dead_code warning；**不会让前端能调用旧命令**。
   - **判断：未激活旧命令，副作用 = 0**。

#### BLOCKER-1（运行时 panic）：scheduler 从未注册到 Tauri State

- `commands/extraction.rs:111` 调用 `app.state::<PipelineScheduler>()`；
- 亲验 `lib.rs` 中全部 `manage()` 调用 —— 唯一一处为 `app.manage(database)`（第 46 行），**没有 `app.manage(PipelineScheduler::new())`**；
- Dev output.md 第 29 行声称 "scheduler 已注册为 Tauri State" —— 实属未验证假设；
- Tauri 的 `Manager::state::<T>()` 在 T 未 manage 时会 **panic**（"state() called before manage() for given type"），违反 task_011 技术约束"失败仅返回字符串错误；不 panic"；
- 这是 pre-existing 缺陷（旧 `extract_asset` 同样有），但旧命令未注册到 invoke_handler，运行时从未被前端触发；task_011 注册了 `retrigger_extraction`，前端 `retryExtraction` 会调用它，**当用户点 failed 资产的"重试"且通过幂等检查（proceed=true）时，第 111 行会 panic 进程**。
- 修复路径（极小）：lib.rs `setup` 中加 `app.manage(PipelineScheduler::new());`（且 PipelineScheduler::new 已存在，第 24 行）。

#### MAJOR-1：pipeline_tasks 表 schema 缺口（pre-existing，但影响第二道护栏）

- 全仓 `grep "CREATE TABLE.*pipeline"` 仅命中单测内构造（`commands/extraction.rs:176`）；
- 生产 migration.rs / 任何 SQL 文件均**无** `pipeline_tasks` 表定义；
- 即 task_011 主张的"`pipeline_tasks` UNIQUE 防重"在生产 DB 中并不成立（要么表根本未建、要么由别处隐式创建但无 UNIQUE）；
- 第二道护栏失效，但**第一道（前置 status 检查）+ 第三道（scheduler.start 互斥）仍构成可靠护栏**，整体幂等不破。
- 这是 pre-existing 缺口，已超 task_011 scope，但 Dev 在 output 中将其作为护栏依据须修正措辞。

#### INFO-1：output 中新增 `getConversionMeta` / `conversionMetaCache` 超出 task_011 input scope

- input.md 仅要求 `retriggerExtraction`，但 diff 同时追加了 `getConversionMeta` 命令与 store 端 `fetchConversionMeta`；
- 这看起来是 task_010 留下的工作，无害（纯追加、不影响 task_011 AC），但若严格按 scope 应在 task_010 输出中体现。**INFO 不阻塞**。

### 三重幂等护栏验证

| 护栏 | 真的成立？ |
|------|----------|
| 1. 命令前置 status 检查（queued/extracting noop） | ✓ 成立（代码第 87-95 行） |
| 2. pipeline_tasks UNIQUE 防重 | ✗ Schema 中未发现 UNIQUE 约束（pre-existing；见 MAJOR-1） |
| 3. scheduler.start is_running 互斥 | ✓ 成立（scheduler.rs:97-101，TokioMutex 守 boolean） |

→ 实际成立 2/3 道。但第 1 + 第 3 道已足够；MAJOR-1 仅是 Dev 措辞误导。

### scheduler.enqueue API 真实性
- 亲读 scheduler.rs:31：`pub fn enqueue(app: &AppHandle, asset_id: &str) -> Result<String, String>` —— 真实存在。
- `start(&self, app: AppHandle)` —— 真实存在（scheduler.rs:91）。
- Dev 未杜撰 API。

### scheduler 注释状态（task_008 关闭后）
- `lib.rs:11-12` 仍是注释 `// pub mod macos;`（与 task_011 无关）；
- `extraction/mod.rs:9` 是激活态 `pub mod scheduler;`（task_008 关闭后正确状态）；
- 未回退。

### M-1 不变量
- `cargo check`：0 error（4 个 warning 全部来自 `llm/`，与本任务无关）；
- `cargo test --lib commands::extraction`：3 passed / 0 failed；
- 复跑均一致。

### PM 冲突 guard
- `git status` 内 31 个 M 文件中：`tauri-commands.ts` / `extractionStore.ts` 不在 PM 改过的列表（属 task_011 合法范围）；
- 其余 31 个 PM 改过的前端文件：未触碰；
- 后端：`extraction.rs` / `commands/mod.rs` / `lib.rs` 改动均与 task_011 直接相关。

### 代码规范
- 无 `unwrap()` / `expect()` 在生产路径（单测内 `unwrap` 合规）；
- SQL 全部 `params![...]` 参数化；
- 不直接置 `status='extracted'`，仅置 `queued`；
- 错误 `map_err -> String`，无 panic 路径（**除 BLOCKER-1 的 Tauri State 默认 panic**）。

---

## 6 维评分

| 维度 | 权重 | 得分 | 说明 |
|------|------|------|------|
| 功能正确性 | 30% | 18/30 | AC-1~3、5 编码正确；BLOCKER-1 导致 happy path 运行时 panic；AC-4 实际无法跑通 |
| 架构一致性 | 20% | 18/20 | 纯函数 + 命令分层干净；mod.rs 修复延续 task_008 风格 |
| 可维护性 | 15% | 14/15 | `reset_extraction_state` 抽出便于单测；锁释放顺序设计清晰；注释充分 |
| 安全性 | 10% | 9/10 | 参数化 SQL；无 shell 注入面；错误字符串脱敏 |
| 测试覆盖 | 15% | 10/15 | 3 单测覆盖纯函数 3 种态；命令层未做集成测试（受 Tauri State 构造限制，acceptable） |
| 代码质量 | 10% | 8/10 | 无 unwrap；docstring 完整；但 output 中"scheduler 已注册"为未验证主张 |
| **合计** | 100% | **72/100** | |

---

## 裁定：FIX

### 必修（BLOCKER）
1. **lib.rs `setup` 内补 `app.manage(PipelineScheduler::new());`**
   - 位置：第 46 行 `app.manage(database);` 之后；
   - 不补则 `retrigger_extraction` 在通过幂等检查后调用 `app.state::<PipelineScheduler>()` panic；
   - 极小改动，与 task_011 scope 强相关（task_011 注册的命令必须可运行）。

### 应修（MAJOR）
2. **修正 output.md 措辞**：移除"pipeline_tasks UNIQUE 防重"作为护栏依据的表述；或单开 follow-up task 补 `pipeline_tasks` 表迁移与 UNIQUE 约束（pre-existing schema 缺口）。

### 可选（INFO）
3. `getConversionMeta` / `fetchConversionMeta` 若属 task_010 范围，归到该任务输出；不阻塞 task_011 关闭。

---

## 路径
- Scorecard：本文件
- Diff 范围（task_011）：
  - `src-tauri/src/commands/extraction.rs`（+230 行：命令 + 纯函数 + 3 单测）
  - `src-tauri/src/commands/mod.rs`（+1 行 `pub mod extraction;`）
  - `src-tauri/src/lib.rs`（+1 命令注册 + `pub mod utils;` + `get_conversion_meta`）
  - `src/lib/tauri-commands.ts`（+27 行：`retriggerExtraction` + ConversionMetaRow + getConversionMeta）
  - `src/stores/extractionStore.ts`（retryExtraction 切换 + fetchConversionMeta 追加）

---

## Fix 二审（Round 2）

**裁定**：**PASS** · 综合分 **90 / 100**

### 审查前验证
- output.md 顶部"修复说明 / 根因分析"节齐全 ✓
- 含分类（遗漏 / 架构偏离）+ 根本原因 + 影响范围 + "为什么之前没注意到" ✓

### BLOCKER 修复验证 ✅
- `src-tauri/src/lib.rs:51` 实有 `app.manage(extraction::scheduler::PipelineScheduler::new());`
- 位于 `app.manage(database)`（第 46 行）之后，闭包内 ✓
- `PipelineScheduler::new()` 真实签名为 `pub fn new() -> Self`（scheduler.rs:24），无参数无 Result —— Dev 用法正确 ✓
- `cargo check` → **0 error**（4 warning 均来自 llm/，与本任务无关） ✓

### MAJOR 修复验证 ✅
- `db/migration.rs:51` 实有 `fn v7_pipeline_tasks(conn)`
- `run_migrations` 主入口第 29-31 行已注册：`if current_version < 7 { v7_pipeline_tasks(conn)?; }` ✓
- CREATE TABLE 字段对齐 `PipelineTaskRow`（id / asset_id / task_type / status / retry_count / max_retries / error_message / priority / batch_id / created_at / started_at / completed_at）—— 12 字段全对齐，类型/NULL 性正确 ✓
- IF NOT EXISTS 幂等 ✓ · 2 索引 + 1 部分唯一索引 `idx_pipeline_tasks_active_unique ON (asset_id, task_type) WHERE status IN ('queued','running')` ✓
- `PRAGMA user_version = 7;` 已写 ✓

### 已 PASS task 影响评估 ✅
- task_008 AC-7（cargo check 0 error）：仍成立 ✓
- task_006 V6 迁移顺序：V6 在 V7 之前注册（第 25-27 行 vs 29-31 行），顺序正确 ✓
- 两处改动均属"补缺口"非"改已有功能"：
  - lib.rs：纯追加一行 `manage()`；
  - migration.rs：纯追加 V7 函数 + 入口分支，未改 V1~V6。

### 测试回归 ✅
- `cargo test --lib commands::extraction`：**3 passed / 0 failed**（reset_when_no_row_is_noop, reset_from_failed_clears_error_and_requeues, reset_from_extracted_requeues_for_rerun）
- `cargo test --lib db::conversion_meta`：**4 passed / 0 failed**（task_006 不回归）
- `cargo test --lib db::asset`：**4 passed / 0 failed**（task_003 不回归）

### 12 pre-existing failure 复核 ✅
- 全部命中 `db::co_occurrence` (7) + `db::knowledge` (5)
- 失败原因 100% 一致：`no such table: concepts`
- V7 迁移日志正确打印（`数据库迁移 V7 完成：pipeline_tasks 表 + 活动态唯一约束`），未引入新 failure
- 结论：与 V7 添加无关，pre-existing M-3 缺口（concepts 表 DDL 在 V1~V7 均缺）

### 运行时验证选项判定
Dev 选 B（用户手测），合理：
- AppHandle 构造成本远高于 e2e；
- 手测脚本已在 output 第 86-97 行登记三场景（failed/extracted/extracting），可直接交付。

### 6 维评分（Fix 二审）
| 维度 | 权重 | 得分 | 说明 |
|------|------|------|------|
| 功能正确性 | 30% | 28/30 | BLOCKER 修复后 happy path 不再 panic；护栏 3/3 道齐备 |
| 架构一致性 | 20% | 19/20 | 迁移函数沿用 V5/V6 模板；manage 顺序遵循"database 先于 scheduler" |
| 可维护性 | 15% | 14/15 | 注释充分，FIX 标记清晰可追溯 |
| 安全性 | 10% | 10/10 | 仍无 unwrap/panic 路径在生产代码；SQL 全参数化 |
| 测试覆盖 | 15% | 11/15 | 单测无回归；V7 由日志间接验证；手测脚本明确 |
| 代码质量 | 10% | 8/10 | output 中"scheduler 已注册"措辞在第 55 行仍残留旧表述（小瑕，不阻塞） |
| **合计** | 100% | **90/100** | |

### 裁定理由
- BLOCKER 与 MAJOR 全部修复并亲验
- 4 个测试命令全绿（task_011 / task_006 / task_003 测试 0 回归）
- 12 个 db 测试 failure 全部 pre-existing（同 M-3）
- 已 PASS task 未被破坏

→ **PASS**，可关闭 task_011 并进入下一任务。
