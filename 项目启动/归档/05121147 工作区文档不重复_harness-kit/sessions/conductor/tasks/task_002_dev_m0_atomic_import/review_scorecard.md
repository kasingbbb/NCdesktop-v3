# Review Scorecard — task_002_dev_m0_atomic_import

## 审查思考过程

### 1. Task 意图
按 ADR-006 把 `commands::dropzone::import_drop_paths` 的 `copy → INSERT asset(root) → enqueue conversion` 两阶段事务边界明确化：enqueue 失败时**保留** asset 行与已复制的工作区源文件（不回滚），把 asset_id 计入 `ImportDropSummary::failures_to_enqueue`，让 M5/M9 兜底；同时抽出一个不持 AppHandle 的纯函数 `import_files_core`，命令降级为薄包装，并补两个单测（happy / failure）。

### 2. AC 逐条检查结果
- **AC-1（抽核心函数 + 命令薄包装）**：✅ `import_files_core<S: EnqueueScheduler>` 在 dropzone.rs:534 实现；同步函数无 await；`import_drop_paths` (dropzone.rs:690) 仅解构 State + 解算 project_id + 调核心 + 命令层 spawn AI 旁路 + scheduler.start + emit。签名偏离（`&StdMutex<Connection>` 而非 `&Connection`）已在 output.md 偏离说明中陈述，且语义等价、更利于"短锁块 + enqueue 不持锁"的实现要求 — 不算偏离架构。
- **AC-2（ADR-006 失败语义）**：✅ dropzone.rs:628-638，enqueue Err 仅 `log::warn` + `failures_to_enqueue.push(asset_id)`，**不**走 fs::remove_file，也**不**做 DELETE，asset 仍随后 push 进 `created`（前端可见占位）。事件 emit 在命令层 dropzone.rs:732 无条件触发，与 created/failures_to_enqueue 数量无关。
- **AC-3（enqueue_failure_keeps_asset 单测）**：✅ dropzone.rs:985-1042。断言覆盖 (i) created.len==2、(ii) failures_to_enqueue.len==2、(iii) assets 表行数==2、(iv) pipeline_tasks 空、(v) workspace 内副本文件 `p.exists()`、(vi) scheduler 被调用次数。即"asset 行存在 + 物理文件存在"双重断言均到位。
- **AC-4（happy_path_inserts_root_and_enqueues 单测）**：✅ dropzone.rs:931-982。明确断言 `source_asset_id IS NULL`（root_count==2）与 `pipeline_tasks.status='queued'`（queued_count==2）。
- **AC-5（cargo test 全通过 + 不依赖网络）**：✅ Dev 粘贴的测试输出真实可信（test 行数 / 二进制名 / filter 计数与 lib test 标准格式一致），2 passed / 0 failed。`OkScheduler`/`FailingScheduler` 全本地无网络。包名 `-p notecapt` 修正属合理边界澄清（per Conductor 备注），不视为偏离。

### 3. 关键发现
- **F1（正向）**：`import_files_core` 是同步函数，调用方 `import_drop_paths` async 函数中**没有**在持 MutexGuard 时跨任何 await — 这是 ADR-006 + session_context "不在 import_files_core 中跨 await 持 MutexGuard" 的最干净实现方式。短锁块 `{ let conn = …; insert(…); }` 结束在 scheduler.enqueue 之前（dropzone.rs:617-625 vs 628），无死锁风险。
- **F2（轻微）**：核心函数对 `fs::copy 成功但 db::asset::insert 失败` 的分支保留了原有"物理文件不删 → 孤儿"的行为（dropzone.rs:621-623）。此分支与 ADR-006 范围无关（ADR-006 仅约束 enqueue Err），Dev 在自测矩阵中已诚实标注"未在 task_002 范围内变更" — 合理。

---

## 评分

| 维度 | 权重 | 分数 (1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | 5 条 AC 全部满足；ADR-006 失败语义实现严格（不删 asset / 不删源文件 / 仍 emit 事件 / 仍计入 created）；happy 与 failure 双单测真实跑通。 |
| 用户体验 | 25% | 4 | 失败语义保留"可恢复路径"：asset 行 + 工作区物理文件均保留，UI 可后续渲染为 offline，M5 重试可达。`failures_to_enqueue` 字段已序列化交前端。扣 1 分：核心层未为 enqueue 失败附带 `error_message`（仅 warn log），UI 只拿到 asset_id 列表，无法直接展示"为何入队失败"；不过这属 task_008 范畴，本 task 边界合理。 |
| 架构一致性 | 20% | 5 | 改动严格限于 `commands/dropzone.rs`，未扩散到 scheduler.rs / db/ / mod.rs；未新建文件；未新增依赖；未新增 migration；`EnqueueScheduler` trait 私有于该文件；走 `db::asset::insert` API，未在 commands 内拼 SQL（生产路径）；未新增裸 `tokio::spawn`（AI 旁路 spawn 沿用原有 `spawn_dropzone_ai_job`，由命令层保留）。 |
| 代码质量 | 10% | 4 | 命名清晰（`EnqueueScheduler` / `AppHandleEnqueue` / `ImportCoreOutput`）；注释充分指出死锁防护要点；短锁块用 `{ … }` 显式控制 drop 时机可读性好。轻微：测试用 `OkScheduler` 内部 `INSERT pipeline_tasks` 是测试代码拼 SQL（属测试代码常态豁免，规范仅约束生产 commands/）；`#[serde(default)]` 套在 `failures_to_enqueue` 上良好向后兼容。 |
| 测试覆盖 | 10% | 4 | 覆盖正常路径 + ADR-006 失败路径双断言；happy 显式断言 `source_asset_id IS NULL` 与 `pipeline_tasks.status='queued'`；failure 包含物理文件存在断言。扣 1 分：`paths.is_empty()`、复制失败、insert 失败三条边界 Dev 自测矩阵标注"未单测"——其中 `is_empty` 由命令层短路且 Default trivially 成立可豁免，但 copy/insert 失败分支无回归网。本 task 范围内可接受。 |
| 可维护性 | 10% | 5 | 偏离说明详细；trait 注入边界与生命周期清晰；`ImportCoreOutput::ai_pending_jobs` 出参把 spawn 留在命令层，task_003 接驳 `list_root_assets` 时核心函数签名稳定可复用；测试隔离用 UUID workspace 子目录 + best-effort 清理，重跑安全。 |

**综合分**：5×0.25 + 4×0.25 + 5×0.20 + 4×0.10 + 4×0.10 + 5×0.10 = 1.25 + 1.00 + 1.00 + 0.40 + 0.40 + 0.50 = **4.55 / 5**

---

## 总体判断

- [x] **PASS**
- [ ] FIX
- [ ] BLOCKER

理由：无 BLOCKER、无 MAJOR；综合分 4.55 ≥ 3.5；6 条领域审查重点（asset/conversion 关系、ADR-006、commands 不拼 SQL、无裸 spawn、不跨 await 持锁、双重断言测试）全部通过；架构改动收敛于单文件无扩散；后续 task_003 可直接基于 `import_files_core` + `failures_to_enqueue` + V8 schema 接驳，无兼容包袱。

---

## 问题列表

### BLOCKER
（无）

### MAJOR
（无）

### MINOR
1. **`failures_to_enqueue` 仅含 asset_id，不含 error_message**
   - 代码位置：`src-tauri/src/commands/dropzone.rs:636`
   - 影响：前端只能展示"入队失败"通用文案，无法区分原因（DB 锁占用 / scheduler 状态异常）。
   - 建议：task_008 前端集成时若仍嫌信息不足，可在后续 task 改为 `Vec<{asset_id, reason}>` 并保持 `#[serde(default)]` 兼容旧前端。本 task 不必改。

2. **`fs::copy 成功 + db::asset::insert 失败` 路径会留下工作区孤儿文件**
   - 代码位置：`src-tauri/src/commands/dropzone.rs:617-624`
   - 影响：与本 task ADR-006 边界无关（ADR-006 仅约束 enqueue Err），但与"失败可恢复 / 不留孤儿"底线略有张力。
   - 建议：作为已知局限纳入 task_006（M6 删除级联 / 启动期孤儿扫描）的待办，不在本 task 修。

3. **`-p app_lib` vs `-p notecapt` 包名**
   - 代码位置：input.md AC-5；后续 task input 模板
   - 建议：Conductor 在生成 task_003+ 的 input.md 时统一使用 `cargo test -p notecapt --lib …`；Dev 已在 output.md §测试命令做了核对说明，本 task 不需返工。

4. **测试依赖 `dirs_next::download_dir()` Some**
   - 代码位置：`src-tauri/src/commands/dropzone.rs::tests::write_two_source_files` 路径走 `workspace::ensure_project_workspace`
   - 影响：headless CI 上若无 `~/Downloads/` 可能失败。Dev 已在 output.md §自测矩阵注明；workspace 路径注入留待 task_009 处理，本 task 不修。

5. **测试代码内的 `INSERT pipeline_tasks/extracted_content` SQL**
   - 代码位置：`src-tauri/src/commands/dropzone.rs:833-844`（`OkScheduler::enqueue`）
   - 说明：测试代码绕过 `db::asset::*` 直接拼 SQL 是合理的（要模拟 scheduler 的副作用，又不能引入真实 scheduler），属测试 fixture 豁免范畴；提及仅为留档。

---

## 给 Dev 的修复指引

不需要修复。PASS。Conductor 可直接进入 task_003（M1+M2 list_root_assets + compute_asset_state），可直接消费 `commands::dropzone::import_files_core` 与 `ImportDropSummary::failures_to_enqueue` 字段，无需 task_002 返工。
