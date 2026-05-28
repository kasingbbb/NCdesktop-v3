# Task 输出 — task_022_failure_injection_tests

## 实装总览

新建 `src-tauri/tests/kc_failure_injection.rs`（~520 行），用 task_006 `MockKcServer` 注入 KC 5 类失败 HTTP 响应，端到端验证：
- `KcClient::ingest_text` 的 `Result<_, KcCallError>` 分类正确；
- `kc::enrichment::resolve_outcome` 输出的 `ResolvedEnrichment` 4 字段（final_md / extractor_type / kc_enriched / failure_code_for_meta）严格符合 ADR-004 §"5 类失败映射"；
- failure_code 字面值与 `FailureCode::EKc*.as_str()`（task_003）严格一致；
- **PRD 不可妥协底线 #2**：`extracted_content.status` 在 KC 失败兜底后仍是 `'extracted'`。

## 测试覆盖（5/5，2 P0 + 3 建议全实装）

| # | 测试名 | mock scenario | KcCallError | failure_code | DB.kc_enriched |
|--|--|--|--|--|--|
| 1 | **failure_a_unavailable_falls_back_to_markitdown_md**（P0） | unavailable     | Unreachable      | E_KC_UNAVAILABLE       | "false"   |
| 2 | **failure_d_timeout_falls_back_to_markitdown_md**（P0）     | timeout         | Timeout          | E_KC_TIMEOUT           | "false"   |
| 3 | failure_b_internal_error_falls_back_with_failure_code        | internal_error  | Internal{KC_INTERNAL} | E_KC_ENRICH_FAILED | "false"   |
| 4 | failure_c_llm_unavailable_partial_writes_rule_only_md        | llm_unavailable | LlmUnavailable{Some} | E_KC_LLM_UNAVAILABLE | **"partial"** |
| 5 | failure_e_input_too_large_falls_back                         | input_too_large | InputTooLarge    | E_KC_INPUT_TOO_LARGE   | "false"   |

## 测试粒度决策

**轻量路径**：直接调 `KcClient::ingest_text` + 本地复刻 `call_error_to_outcome`（map_call_error_to_outcome 的等价物，task_011 模式）+ 公开 `resolve_outcome` —— 避免构造真实 `AppHandle`（要 mock window / window_event / db state / KcProcessManager / KcClient 等）。

**新增 DB 断言粒度**（task_022 vs task_011 的独特增量）：每个测试新建 in-memory SQLite + `run_migrations`（v18 列已就绪）+ 模拟 scheduler 写入序列：

1. `upsert_extraction_result(asset_id, "...", "...", 3, "markitdown", None)` — 模拟 markitdown 阶段成功，写 `extracted_content.status='extracted'`；
2. `resolve_outcome` 产出 `ResolvedEnrichment`；
3. 共用 helper `simulate_scheduler_kc_persist`（复刻 `scheduler.rs::kc_persist_resolved_with_conn`）：`db_update_kc_fields` + `db_conversion_meta_kc_insert` + `update_failure_code`；
4. 共用 helper `assert_db_invariants_post_kc_failure`：复读 `extracted_content` + `conversion_meta`，断言 status 仍 `'extracted'`、extractor_type 仍 `'markitdown'`、failure_code 字面与 `FailureCode::as_str()` 一致。

## 隔离设计（AC-5）

- 端口：每个 mock 实例独立动态空闲端口（wiremock `MockServer::start()` 行为）；
- DB：每个测试独立 `:memory:` 数据库（`Connection::open_in_memory()`，不共享状态）；
- timeout 测试：用 `reqwest::Client::builder().timeout(100ms)` + mock `set_delay(500ms)` 在 ~100ms 内触发 Timeout（生产 client 默认 60s 不适用，由 `KcClient::new_with_http_client` 注入短超时）；
- 网络：全本地 127.0.0.1（无外部网络依赖）。

## 测试结果

```
$ cd src-tauri && cargo test --test kc_failure_injection
running 5 tests
test failure_b_internal_error_falls_back_with_failure_code ... ok
test failure_c_llm_unavailable_partial_writes_rule_only_md ... ok
test failure_a_unavailable_falls_back_to_markitdown_md ... ok
test failure_e_input_too_large_falls_back ... ok
test failure_d_timeout_falls_back_to_markitdown_md ... ok
test result: ok. 5 passed; 0 failed; finished in 0.12s
```

合计 ~120ms（远低于 input.md AC-3 "10s" 约束）。

## 回归

- `cargo test --lib`：**537/537 PASS**（0 退化，5.03s）
- `cargo test --test kc_enrichment_integration`：先前 7/7 baseline 不变（task_011 不动）

## 不动的依赖文件

- `src-tauri/src/kc/enrichment.rs`（task_011） — 仅消费
- `src-tauri/tests/common/mock_kc.rs`（task_006） — 仅消费
- `src-tauri/src/extraction/scheduler.rs`（task_012） — 仅复刻其 `kc_persist_resolved_with_conn` 行为到测试 helper（scheduler 内部仍为 `fn`）
- `src-tauri/Cargo.toml` — 无新增依赖（wiremock / tokio / reqwest / rusqlite 已是 dev-dep / direct dep）

## Reviewer 重点关注项 — 答复

1. **5 类失败覆盖完整性** — 5/5 全部实装（2 P0 + 3 建议升级到全覆盖）；
2. **DB status='extracted' 断言（不可妥协底线）** — `assert_db_invariants_post_kc_failure` helper 强制守护，每个测试都跑；
3. **测试隔离** — 独立端口（wiremock 动态）+ 独立 in-memory DB；
4. **timeout 测试时长** — 100ms client + 500ms mock delay → 触发 Timeout < 100ms（总测试 < 150ms）。
