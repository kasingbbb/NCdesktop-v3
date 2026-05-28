# Task 输入 — task_015b_close_td3_parse_failure_code

## 目标
关闭 TD-3 技术债：消除 `parse_failure_code` 的双源（DRY 违反），让 `db/conversion_meta.rs` 中的 canonical parser 成为单一字面来源，scheduler.rs 删除 workaround local 版本并 import canonical。

## 背景
- TD-3 在 task_003 Reviewer 上抛：`src/db/conversion_meta.rs:263 parse_failure_code()` 未扩展 5 个 KC 字面值（`E_KC_UNAVAILABLE` / `E_KC_TIMEOUT` / `E_KC_LLM_UNAVAILABLE` / `E_KC_ENRICH_FAILED` / `E_KC_INPUT_TOO_LARGE`）
- task_012 Dev 用 scheduler 内 `parse_failure_code` mini-parser 作 workaround（scheduler.rs:1448-1463 + 测试 2045 行）
- task_012 Reviewer（4.58/5 PASS）识别为 MAJOR-1：**未关 TD-3，仅规避**

## 前置条件
- task_012（commit `d7f5fac5`）已落地
- task_003（commit `2c3389bd`）的 `FailureCode::EKc*` 5 个枚举变体齐全

## 验收标准（Acceptance Criteria）

1. **AC-1**：在 `src-tauri/src/db/conversion_meta.rs:263` 的 `parse_failure_code(code: &str) -> Option<FailureCode>` 中补全 5 个 KC 字面分支：
   - `"E_KC_UNAVAILABLE"` → `FailureCode::EKcUnavailable`
   - `"E_KC_TIMEOUT"` → `FailureCode::EKcTimeout`
   - `"E_KC_LLM_UNAVAILABLE"` → `FailureCode::EKcLlmUnavailable`
   - `"E_KC_ENRICH_FAILED"` → `FailureCode::EKcEnrichFailed`
   - `"E_KC_INPUT_TOO_LARGE"` → `FailureCode::EKcInputTooLarge`
   - 实现：直接调 `FailureCode::*.as_str()` round-trip（最稳）或显式 match（同步 `as_str()` 字面，单测守护）

2. **AC-2**：将 `db/conversion_meta.rs` 的 `parse_failure_code` 改为 `pub(crate) fn`（或 `pub fn` 视项目约定），使 scheduler.rs 可调用

3. **AC-3**：删除 `src-tauri/src/extraction/scheduler.rs:1448-1463` 的 local `parse_failure_code` 实装；改 scheduler.rs 中调用方为 `crate::db::conversion_meta::parse_failure_code(...)`

4. **AC-4**：保留 task_012 的守护测试 `parse_failure_code_recognises_all_five_kc_variants`（scheduler.rs:2045），但**改为调用 canonical parser**，以测它—— 这就把守护从 scheduler-local 跨到了 canonical 一侧

5. **AC-5**：在 `db/conversion_meta.rs` 内追加同名守护测试 `parse_failure_code_recognises_all_five_kc_variants`（5 KC 字面 round-trip + as_str 一致性）

## 技术约束
- **不动 FailureCode 枚举本身**（task_003 已固化）
- **不动 task_012 注入逻辑**（kc_persist_resolved / kc_persist_resolved_with_conn）；只动 parse_failure_code 调用点
- **0 测试退化**：lib 应仍 ≥ 512 PASS（baseline）+ 你新增的 1-2 个守护测试
- 总修改 ≤ 30 行 + 测试

## 参考文件
- `src-tauri/src/db/conversion_meta.rs:263` canonical parser 现状
- `src-tauri/src/extraction/scheduler.rs:1448-1463` workaround 待删
- `src-tauri/src/extraction/scheduler.rs:2045` 守护测试待迁移
- `src-tauri/src/extraction/failure_code.rs::FailureCode::as_str()`（task_003 commit `2c3389bd`）

## 预估影响范围
- 修改：`src-tauri/src/db/conversion_meta.rs`（+ 5-10 行 + 1 测试）、`src-tauri/src/extraction/scheduler.rs`（- 15 行 local parser + 调用方 import 改 path）

## Reviewer 重点关注
- 是否真删了 scheduler-local 版本（不能留下 dead code）
- canonical parser 5 KC 字面与 FailureCode::EKc*.as_str() 严格 round-trip（最好直接调 as_str 比对而非硬编码字面）
- scheduler.rs 测试迁移到 canonical 后，原 scheduler-local 守护测试是否需保留

## 复杂度
XS（≤ 30 行 + 1-2 测试，半小时内完工）
