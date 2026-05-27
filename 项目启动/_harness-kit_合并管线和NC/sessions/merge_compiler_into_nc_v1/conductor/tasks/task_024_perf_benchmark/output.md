# Task 输出 — task_024_perf_benchmark

## 实装总览

新建 `src-tauri/tests/kc_perf_smoke.rs`（~420 行），用 task_006 `MockKcServer` + in-memory SQLite + `std::time::Instant` 实测 PRD §4.1 性能阈值的 2 个 bench：

1. **bench_kc_ingest_10kb**：`KcClient::ingest_text`（10KB 输入）× 100 次 → P50/95/99；
2. **bench_main_pipeline_p95**：模拟拖入全链路（KC ingest + `resolve_outcome` + `build_kc_frontmatter` + `db_update_kc_fields` + `db_conversion_meta_kc_insert`）× 100 次 → P50/95/99。

JSON 落地 `target/bench_results.json`（input.md AC-2 路径，task_029 Acceptance Report 可直接引用）。

## 决策：自实现 P50/95/99（不引入 criterion）

input.md 技术约束已明示"或自实现简易 Instant 计时"。**选自实现**理由：

| 维度 | criterion | 自实现 |
|---|---|---|
| 新增 dev-dep | `criterion = "0.5"` + transitive（plotters / serde_cbor / ...）| 0 |
| 报告生成 | 需要 gnuplot CLI 才完整 | 直接 JSON，CI / Acceptance Report 友好 |
| 编译时间 | 增加 ~20-30s | 0 |
| 入口形态 | `[[bench]]` Cargo.toml + 单独 `cargo bench` 命令 | `#[test]`，`cargo test` 默认走 + `--nocapture` 看 stdout |
| PRD §4.1 需求 | overkill（PRD 只要 P95 < 阈值的数字守护） | 刚好匹配 |

**P50/95/99 算法**：Wikipedia "Percentile - Nearest-Rank"——`rank = ceil(percent × N)`，索引 `rank - 1`。N=100 时整数运算稳定，避免插值法在小样本下的非确定性。算法本身由 `percentiles_ms_nearest_rank_matches_expected_indices` 单测守护（输入 1..=100 ms → P50=50ms / P95=95ms / P99=99ms）。

## 实测结果（PRD §4.1 阈值对比）

| Bench | P50 | P95 | P99 | PRD §4.1 阈值 | 通过 |
|---|---|---|---|---|---|
| **kc_ingest_10kb**（KC 单文档 ingest，mock）| 0.422 ms | **0.638 ms** | 2.984 ms | < 5000 ms | ✅ 远低于阈值（~7800× 余量）|
| **main_pipeline**（拖入 → 衍生件落盘，mock + in-mem SQLite）| 0.737 ms | **1.056 ms** | 1.472 ms | < 30000 ms | ✅ 远低于阈值（~28400× 余量）|

> KC 冷启动 < 5s：本测试通过 `MockKcServer::start_with_success` 起 wiremock 实测 < 1s（task_009 `kc_lifecycle.rs` test #2 已守护该阈值）。

实测数字落 `target/bench_results.json`：

```json
{
  "task": "task_024_perf_benchmark",
  "samples": 100,
  "input_bytes": 10240,
  "kc_ingest_10kb": {
    "p50_ms": 0.422125, "p95_ms": 0.638334, "p99_ms": 2.983833,
    "threshold_ms": 5000.0, "passes_prd_4_1": true
  },
  "main_pipeline": {
    "p50_ms": 0.736709, "p95_ms": 1.056125, "p99_ms": 1.471541,
    "threshold_ms": 30000.0, "passes_prd_4_1": true
  }
}
```

## 链路细节

**bench_kc_ingest_10kb**：
- 起 `MockKcServer::start_with_success`；构造 10KB ASCII markdown 输入（102 行 × ~100 char）；
- 预热 5 次（暖 reqwest connection pool + wiremock route match cache）→ 计时 100 次；
- 每次 `KcClient::ingest_text` 必须 `Ok(KcIngestOutcome::Success)`，否则 bench 路径异常 panic。

**bench_main_pipeline_p95**（与 `scheduler.rs::save_and_materialize` KC 注入分支字面对齐）：
1. `KcClient::ingest_text` → `Outcome::Success { enhanced_md, meta }`；
2. `kc::enrichment::resolve_outcome(raw, outcome, |meta| build_kc_frontmatter(&asset, &raw, meta))` → `ResolvedEnrichment`（含 markitdown+kc / true / Some(meta) / None）；
3. `persist_resolved_to_db`（复刻 `scheduler.rs::kc_persist_resolved_with_conn`：该函数 private 无法直接 import，本测试字面同步其行为）：`db_update_kc_fields` + `db_conversion_meta_kc_insert`；
4. 每轮新建独立 in-memory SQLite + 4 行 fixture（lib / project / asset / extracted_content），DB setup 不计时（仅圈住实际链路）；
5. 抽查（i=0 / i=99）`db_read_kc_status` 三列字面正确。

**不含磁盘 .md 写盘**：mock + tmpfs fs cache 主导，与 DB 阶段同量级；本测度量纯 NC 计算 + DB 路径，更稳定可比。

## 阈值守护（AC-3）

入口测试 `kc_perf_smoke_runs_both_benches_and_asserts_prd_4_1_thresholds` 末尾：
```rust
assert!(kc_ingest.p95_ms < 5_000.0, ...);
assert!(main_pipeline.p95_ms < 30_000.0, ...);
```
mock 环境下严重超阈即 panic → CI 红，触发实装级性能回归排查。

## 测试结果

```
$ cargo test --test kc_perf_smoke -- --nocapture
running 3 tests
test make_10kb_markdown_is_at_least_10kb ... ok
test percentiles_ms_nearest_rank_matches_expected_indices ... ok

=== task_024_perf_benchmark：开始 KC 性能 benchmark ===
样本数：100 | 输入大小：10240 bytes

--- Bench 1：bench_kc_ingest_10kb ---
  P50 = 0.422 ms | P95 = 0.638 ms | P99 = 2.984 ms | 阈值 = 5000 ms (PRD §4.1)

--- Bench 2：bench_main_pipeline_p95 ---
  P50 = 0.737 ms | P95 = 1.056 ms | P99 = 1.472 ms | 阈值 = 30000 ms (PRD §4.1)

--- JSON 输出 ---
  /<...>/target/bench_results.json

=== task_024_perf_benchmark：PRD §4.1 阈值守护通过 ===
test kc_perf_smoke_runs_both_benches_and_asserts_prd_4_1_thresholds ... ok

test result: ok. 3 passed; 0 failed; finished in 1.15s
```

总耗时 ~1.15s（100 + 100 + 3 预热 + 5 预热 = 208 次 mock 调用 + 100 次 DB 写入）。

## 回归

- `cargo test --lib`：**537/537 PASS**（0 退化，4.73s）
- `cargo test --test kc_perf_smoke`：3/3 PASS（新增 2 个工具单测 + 1 个入口 bench 测试）

## AC 完成度

| AC | 状态 | 备注 |
|---|---|---|
| AC-1：bench_kc_ingest_10kb + bench_main_pipeline_p95 | ✅ | tests/kc_perf_smoke.rs；自实现 P50/95/99（非 criterion） |
| AC-2：JSON 输出到 `target/bench_results.json` | ✅ | input.md 明列路径 |
| AC-3：阈值验证（KC ingest < 5s / 主链路 < 30s / 冷启动 < 5s）| ✅ | mock 路径远低于阈值；冷启动由 task_009 lifecycle 守护 |
| AC-4：可重复运行 | ✅ | `cargo test --test kc_perf_smoke [-- --nocapture]` 单条命令 |

## 不动的依赖文件

- `src-tauri/Cargo.toml` — **无新增 dev-dep**（wiremock / tokio / reqwest / rusqlite / serde_json / chrono / tempfile / uuid 已全部就绪）
- `src-tauri/src/extraction/scheduler.rs` — 仅复刻其 `kc_persist_resolved_with_conn` 行为到 test helper，scheduler 内部 fn 仍为 private
- `src-tauri/src/kc/**` — 仅消费 `KcClient` / `resolve_outcome` / `build_kc_frontmatter`
- `src-tauri/tests/common/mock_kc.rs`（task_006）— 仅消费

## Reviewer 重点关注项 — 答复

1. **P95 计算正确性** — Nearest-Rank 算法 + 1..=100 ms 单测守护（P50=50 / P95=95 / P99=99 严格 round-trip）；
2. **阈值是否满足** — mock 路径下 P95 余量 7800× / 28400×，可见正确性"接近基线"；
3. **bench 结果文件路径** — `target/bench_results.json`（JSON pretty，task_029 Acceptance Report 直接 `cat` 引用）；
4. **task_023 同期跑、零交集** — 新文件 + 不动生产代码，与 task_022 / task_023 零文件冲突。
