# Task 输出 — task_027b_extract_kc_persist_helper

## 实装路径决策：路径 A（最小侵入，修正版可见性）

**最终选择**：scheduler.rs 内 `kc_persist_resolved_with_conn` 升级为 `pub fn`（**非** `pub(crate)`，**非** 抽新模块）+ `#[doc(hidden)]` + doc-comment 说明 "task_027b 测试基础设施，生产代码请走 `save_and_materialize`"。

### Crate 可见性核查（推翻 input.md 的"路径 A 推荐 pub(crate)"）

input.md AC-1 路径 A 建议 `pub(crate) fn`，但**实际不可行**：

- Rust integration test（`src-tauri/tests/*.rs`）每个文件是**独立的 external crate**——它们以 `use app_lib::xxx` 引用 lib，对 lib 而言**不是同一 crate**。
- `pub(crate)` 只允许 **lib 内部** 可见（即 `src/` 下的代码 + `src/` 内 `#[cfg(test)] mod`）。
- integration test crate 要 import 必须用 **完全 `pub`**。

**因此采纳"修正推荐"**：`pub fn` + `#[doc(hidden)]` 标注。`#[doc(hidden)]` 让 rustdoc 不导出（公开 API 文档站点看不到），但 import 可见——符合"测试基础设施" 语义。生产代码入口仍是 `save_and_materialize`（task_012 内部调 `kc_persist_resolved` → 内部再调 `kc_persist_resolved_with_conn`），可见性零回退。

### 为什么不选路径 B（抽新模块）

input.md 路径 B 建议抽 `kc/persist.rs` 新模块。否决理由：
1. scheduler.rs 内 `kc_persist_resolved` 与 `kc_persist_resolved_with_conn` 的双 API 形态已经稳定（task_012），抽模块需迁移 2 个 fn + 改 1 处调用 → 新模块独立性收益小、对原模块语义割裂大；
2. 新模块要给 conv_meta / extraction DB 调用层 ~25 行外加 use re-export，超出"单源化"必要范围（路径 A 共改 4 行可见性 + 注释，路径 B 至少 60 行）；
3. `kc_persist_resolved_with_conn` 本质是 scheduler 的 "DB 落地纯函数" 助手，留在 scheduler.rs 与上下文邻近（save_and_materialize KC 注入 ≈ 12 行 + helper 80 行成对存在）——抽离反而降低可读性。

## 修改清单

### 1. scheduler.rs（+8 行 / -2 行）

- `fn kc_persist_resolved_with_conn(...)` → `pub fn` + `#[doc(hidden)]` + doc-comment 4 行追加（task_027b 决策说明）

### 2. tests/kc_failure_injection.rs（-83 行）

- 删 `simulate_scheduler_kc_persist`（55 行 helper 体）
- 删 `failure_code_from_str`（10 行 helper 体）
- 删 unused imports `db_conversion_meta_kc_insert, update_failure_code`
- 5 处调用点 `simulate_scheduler_kc_persist(...)` → `kc_persist_resolved_with_conn(...)`
- 加 import：`use app_lib::extraction::scheduler::kc_persist_resolved_with_conn;`

### 3. tests/kc_e2e_pipeline.rs（-134 行）

- 删 `persist_resolved_to_db`（63 行 helper 体 + 12 行 doc）
- 删 `parse_kc_failure_code`（11 行）
- 删 guard 测试 `persist_helper_matches_kc_persist_resolved_with_conn_for_success`（38 行）—— canonical 单源化后该 guard 失效（test 端不再有独立 helper 可能漂移）
- 删 unused imports：`FailureCode`, `ResolvedEnrichment`, `KcMeta`, `KcTagsSource`
- 4 处调用 `persist_resolved_to_db(...)` → `kc_persist_resolved_with_conn(...)`
- 加 import：`use app_lib::extraction::scheduler::kc_persist_resolved_with_conn;`

### 4. tests/kc_perf_smoke.rs（-65 行）

- 删 `persist_resolved_to_db`（62 行 helper 体 + 注释）
- 删 unused imports：`db_conv_meta`（perf bench 不再直接用 DB API），`ResolvedEnrichment`
- 2 处调用 `persist_resolved_to_db(...)` → `kc_persist_resolved_with_conn(...)`
- 加 import：`use app_lib::extraction::scheduler::kc_persist_resolved_with_conn;`

## 行数预算

- **修改统计**：+34 行新增 / -296 行删除（git diff --stat）
- **净减 262 行**（消除三处 helper ≈ 200 行复刻 + ≈ 60 行 doc 注释 + 1 处 guard 测试）
- **实际新增逻辑代码** < 10 行（只增 `pub` 标识 + 3 处 import 行）
- 远低于 80 行预算上限

## 测试结果（0 退化）

| 测试 | 期望 | 实际 |
|--|--|--|
| `cargo test --test kc_failure_injection` | 5/5 PASS | **5 passed; 0 failed** |
| `cargo test --test kc_e2e_pipeline` | 4/4 PASS（删 guard 后） | **4 passed; 0 failed** |
| `cargo test --test kc_perf_smoke` | 3/3 PASS | **3 passed; 0 failed** |
| `cargo test --lib` | 537/537 PASS | **537 passed; 0 failed** |

## 单源化验证

```
grep "fn simulate_scheduler_kc_persist\|fn persist_resolved_to_db\b" src-tauri/   → 0 处
grep "fn kc_persist_resolved_with_conn" src-tauri/                                  → 1 处（scheduler.rs:1378）
grep "simulate_scheduler_kc_persist\|persist_resolved_to_db" src-tauri/             → 1 处历史 doc 注释
```

**canonical fn**：`app_lib::extraction::scheduler::kc_persist_resolved_with_conn`（pub fn + #[doc(hidden)]，唯一 DB 写入入口）。

## Reviewer 关注点回应

1. **canonical fn 真单源化**：✓（grep 全仓 0 处复刻）
2. **三处 test 直接 import canonical**：✓（无 wrapper layer，直接 `kc_persist_resolved_with_conn(...)`）
3. **guard 测试**：✓ 删除（不再需要，canonical 单源 → drift 不存在）
4. **路径 A vs B 决策依据**：✓（见上 "Crate 可见性核查" + "为什么不选路径 B"）

## 不变量验证

- ✓ scheduler.rs `kc_persist_resolved`（pub-free，scheduler 内调用）**未动**
- ✓ scheduler.rs `save_and_materialize` 内 task_012 注入逻辑（≈ 12 行）**未动**
- ✓ `db_conv_meta::parse_failure_code` 已是 `pub(crate)`（lib 内可用），test 改用 canonical fn 后**无需触碰**
- ✓ task_012 / 015b 单测 + integration test `kc_enrichment_integration.rs` **未动**
