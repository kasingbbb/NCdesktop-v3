# Task 输入 — task_027b_extract_kc_persist_helper

## 目标
消除 task_022/023/024 三处对 scheduler 私有 `kc_persist_resolved_with_conn` 的字面复刻（DRY 违反，Reviewer 三人独立汇聚共识）：将 scheduler 私有 helper 升级为 `pub(crate)` 或抽到独立模块，三处 test crate 直接调用，消除 drift 隐患。

## 背景
- Reviewer task_022（4.83/5）/ task_023（4.535/5）/ task_024（4.58/5）三人独立识别同一问题：
  - `simulate_scheduler_kc_persist`（task_022 tests/kc_failure_injection.rs）
  - `persist_resolved_to_db`（task_023 tests/kc_e2e_pipeline.rs）
  - `persist_resolved_to_db`（task_024 tests/kc_perf_smoke.rs）
  - 三处字面 ~85% 一致，复刻 scheduler.rs:1372 私有 `kc_persist_resolved_with_conn`
- 根因：scheduler 函数私有 + integration test crate 黑盒，复刻是必要妥协
- 唯一 Success-path drift guard 在 task_023，**不覆盖 Disabled / Failure / Partial 三路径**

## 前置条件
- task_022（commit `f703da4f`）/ task_023（`c0ebde15`）/ task_024（`75045eb2`）已落地
- scheduler.rs 中 `kc_persist_resolved` + `kc_persist_resolved_with_conn` + `parse_failure_code` 都在 task_012 / task_015b 已稳定

## 验收标准

1. **AC-1**：选择实装路径（任选其一，在 output.md 说明决策）：
   - **路径 A（推荐，最小侵入）**：把 `kc_persist_resolved_with_conn` 改为 `pub(crate) fn`；scheduler.rs 内调用方不变；三处 test helper 改为直接调 `crate::extraction::scheduler::kc_persist_resolved_with_conn`
   - **路径 B（彻底解耦）**：抽出新模块 `src-tauri/src/kc/persist.rs`（`pub fn persist_resolved_to_db`），scheduler.rs 内 import 调用；三处 test helper 全删，直接 `use app_lib::kc::persist::persist_resolved_to_db`

2. **AC-2**：三处 test helper 全删，调用点改为 canonical fn

3. **AC-3**：删除 task_023 的 `persist_helper_matches_kc_persist_resolved_with_conn_for_success` guard 测试（不再需要——helper 已统一）；或保留改为对 canonical 的契约测试

4. **AC-4**：所有现有测试不退化：
   - `cargo test --test kc_failure_injection`：5/5 PASS
   - `cargo test --test kc_e2e_pipeline`：5/5 PASS（或 4 个 e2e + 0 guard = 4/4）
   - `cargo test --test kc_perf_smoke`：3/3 PASS
   - `cargo test --lib`：537/537 + 你的调整后 PASS

## 技术约束
- **绝对不动** scheduler.rs 中 `kc_persist_resolved`（pub fn `kc_persist_resolved` 与 `kc_persist_resolved_with_conn` 双 API 都保留 / 或都改 pub(crate)）
- **绝对不动** scheduler.rs `save_and_materialize` 注入逻辑（task_012 12 行注入）
- 总修改 ≤ 80 行（含路径 A：约 5 行可见性 + 3 处 test 改约 30 行；路径 B 约 60 行新模块 + 调用迁移）
- 0 测试退化

## 参考文件
- `src-tauri/src/extraction/scheduler.rs:1372` canonical `kc_persist_resolved_with_conn` 函数体
- `src-tauri/tests/kc_failure_injection.rs::simulate_scheduler_kc_persist`
- `src-tauri/tests/kc_e2e_pipeline.rs::persist_resolved_to_db`
- `src-tauri/tests/kc_perf_smoke.rs::persist_resolved_to_db`

## Reviewer 重点关注
- canonical fn 真单源化（grep 全仓 helper 名应只剩 1 处定义）
- 三处 test 直接 import canonical fn（无 wrapper layer）
- guard 测试是否还需保留（或转换形态）
- 路径 A vs 路径 B 决策依据是否合理

## 复杂度
XS（30-80 行，半小时内完工；不含真机测试）
