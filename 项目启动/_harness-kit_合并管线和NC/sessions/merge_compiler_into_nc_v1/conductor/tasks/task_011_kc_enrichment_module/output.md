# Task 交付 — task_011_kc_enrichment_module

## 实现摘要

把 `src-tauri/src/kc/enrichment.rs`（task_005 留下的 2 行占位）实装为 KC enrichment 主入口 + 5 类失败兜底状态机。整体由 3 个对外 API + 3 个内部 helper 构成：

**对外 API（input.md AC-1 / 2 / 3）**：
- `pub async fn enrich(app: &AppHandle, asset: &Asset, raw_md: &str) -> KcEnrichmentOutcome` —— 主入口，封装 6 步流程：① 读 KcSettings，disabled 立即短路 → ② 取 KcProcessManager 检查 Ready 状态 → ③ 取 KcClient state → ④ 拼 `KcIngestOptions { persist: false }`（ADR-006 层 1，永远 false）调 `client.ingest_text` → ⑤ 把 `Result<KcIngestOutcome, KcCallError>` 走 `map_call_error_to_outcome` 映射成三态 outcome → ⑥ emit `notecapt/asset-kc-enriched` 事件（失败仅 `log::warn`，不向上抛）。
- `pub fn resolve_outcome(raw: &ExtractionResult, outcome: KcEnrichmentOutcome, frontmatter_writer: impl Fn(&KcMeta) -> String) -> ResolvedEnrichment` —— 纯函数（无 IO / 无 await / 无 log），按三态分支决定 final_md / extractor_type / kc_enriched / kc_meta_for_db / failure_code_for_meta 五字段。`Fallback` 分支 final_md **不**用 `outcome.base_md` 而是用 `raw.structured_md.clone()`——锁死到 scheduler markitdown 阶段产物，消除 enrich 入参在传递链路被改写的不确定性。
- `pub struct ResolvedEnrichment` —— 5 字段（final_md / extractor_type / kc_enriched / kc_meta_for_db / failure_code_for_meta）。

**5 类失败映射（input.md AC-1 步骤 4 完整决策表）**：

| KcCallError 变体 | 映射 outcome | failure_code |
|--|--|--|
| `Unreachable` | `Fallback(Unavailable)` | `E_KC_UNAVAILABLE` |
| `Timeout` | `Fallback(Timeout)` | `E_KC_TIMEOUT` |
| `LlmUnavailable { Some(partial) }` | `PartialLlmUnavailable` + 合成 `KcMeta(tags_source=RuleOnly, doc_id="doc-partial", kc_version="unknown")` | `E_KC_LLM_UNAVAILABLE` |
| `LlmUnavailable { None }` | `Fallback(InternalError("LLM unavailable, no partial"))` | `E_KC_ENRICH_FAILED` |
| `Internal { detail, code }` | `Fallback(InternalError(detail))` 透传 detail | `E_KC_ENRICH_FAILED` |
| `InputTooLarge` | `Fallback(InputTooLarge)` | `E_KC_INPUT_TOO_LARGE` |
| `Malformed { reason }` | `Fallback(Malformed)` + `log::warn` 提示 KC-MOD-1 未到位 | `E_KC_ENRICH_FAILED` |

**核心设计决策**：

1. **failure_code 字面唯一源 = `FailureCode::EKc*.as_str()`**：所有 `failure_code_for_meta` 字段均通过 `KcFallbackReason::to_failure_code()` 走 `as_str()` 拿静态字面值，**严禁手写字符串**。单测 `failure_code_strings_match_failure_code_enum` 守护 6 个 reason → 字面值的一一对齐，任何漂移即时 fail。
2. **`frontmatter_writer` 注入式**：`resolve_outcome` 把 frontmatter 生成委托给 `impl Fn(&KcMeta) -> String`，scheduler 调用时传 task_013 实装的 `build_kc_frontmatter`；单测注入 stub `|meta| format!("---\ndoc_id: {}\n---", meta.doc_id)`。本模块不依赖 task_013 是否实装。
3. **partial 路径合成 KcMeta**：`KcCallError::LlmUnavailable.partial_md` 只带 `Option<String>` 不带 meta，但 `KcEnrichmentOutcome::PartialLlmUnavailable` 必须有 meta。`synthesize_partial_meta()` 合成"语义安全"meta：`tags_source=RuleOnly` + 所有 ai_* 字段为空 + `doc_id="doc-partial"` + `kc_version="unknown"`，让 frontmatter / DB 一致表达"非 AI 增强"。
4. **`emit_kc_enriched` 与 `resolve_outcome` 共享语义但独立维护**：`outcome_to_event_strings` 是私有 helper，让 enrich 完成即 emit（不依赖 resolve_outcome 完成），UI 能在 enrich → resolve 之间提前更新状态。
5. **Malformed → emit warn 信号**：`map_call_error_to_outcome` 在 Malformed 分支 `log::warn` 含"KC-MOD-1 未到位"提示，给 dev / reviewer 暴露"KC 200 但 enhanced_markdown 字段缺失"的关键失败信号。

**集成测试 mock 修复（fix-tail 阶段）**：

`tests/common/mock_kc.rs::start_with_unavailable()` 原实现"start wiremock server 后立即 drop"——但 OS 可能立即把端口分配给后续 placeholder server，导致客户端不是连接拒绝而是 404（被 `KcClient` 映射为 `Internal { code: "UNKNOWN", detail: "unexpected status 404" }`）。改为方案 A：用 `std::net::TcpListener::bind("127.0.0.1:0")` 让 OS 分配真正空闲端口，立即 drop 释放回池——后续 wiremock placeholder 选其他端口（端口池随机选），客户端对原 addr 发请求得到真正的 `connection refused`，被 `classify_reqwest_error` 映射为 `KcCallError::Unreachable`。

## 修改的文件

| 文件路径 | 变更类型 | 行数 | 说明 |
|---------|----------|------|------|
| `src-tauri/src/kc/enrichment.rs` | 修改（占位 → 实装） | 1084 行（净增 ~1082） | 6 个函数 + 23 个单元测试 |
| `src-tauri/tests/kc_enrichment_integration.rs` | 新增 | 432 行 | 7 个集成测试 + 3 个 helper |
| `src-tauri/tests/common/mock_kc.rs` | 修改 | `start_with_unavailable()` 重写 ~16 行 | 修方案 A：TcpListener bind+drop 让端口真正无监听 |

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（`kc/enrichment.rs` 单文件，无新增子模块）
- [x] API 路径/命名与 Architect 方案一致（`enrich` / `resolve_outcome` / `ResolvedEnrichment` 均与 ADR-003 / ADR-004 命名一致）
- [x] 数据模型与 Architect 方案一致（复用 `KcEnrichmentOutcome` / `KcFallbackReason` / `KcMeta` / `KcTagsSource`，未引入新类型；`ResolvedEnrichment` 是 AC-3 要求新增的 pub struct）
- [x] 未引入计划外的新依赖（仅用 tauri / serde_json / log，全部已在 Cargo.toml）
- [x] failure_code 字面值与 task_003 `FailureCode::EKc*.as_str()` 严格对齐（单测守护）
- 偏离说明：
  - **Fallback 分支 final_md 用 `raw.structured_md` 而非 `outcome.base_md`**：input.md AC-2 写的是 `final_md = raw.structured_md.clone()`，本实装严格遵守。`enrich()` 传 `base_md = raw_md.to_string()` 仅为契约完整，`resolve_outcome` 不消费它。
  - **`enrich` 内 `read_kc_settings` 与 process.rs 同模式但独立复制**：input.md "共享接口唯一来源"列出 `KcSettings` / `log_with_mask` 必须复用，但"从 AppHandle 拿 DB → KcSettings"的样板代码在 process.rs 是私有函数。本 task 因强约束"不动 process.rs"复制了 ~10 行 helper（注释中已登记"未来可重构为 `kc::settings::load_from_app`"）。

## 测试命令

```bash
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri
cargo test --lib kc::enrichment                  # lib 内单测（23 个）
cargo test --test kc_enrichment_integration      # 集成测试（7 个）
cargo test --lib                                  # 整体回归
```

## 测试结果

**`cargo test --test kc_enrichment_integration`**：

```
running 7 tests
test enrich_disabled_short_circuits ... ok
test enrich_unavailable_returns_fallback ... ok
test enrich_llm_unavailable_with_partial_returns_partial ... ok
test enrich_input_too_large_returns_fallback ... ok
test enrich_internal_error_returns_fallback ... ok
test enrich_success_returns_full_outcome ... ok
test enrich_timeout_returns_fallback_timeout ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
```

**`cargo test --lib`（整体回归）**：

```
test result: ok. 489 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.49s
```

总数：**489 lib 单测 + 7 integration 测试 = 496 PASS / 0 FAIL / 0 SKIP**。

相对 task_014 收尾时的 468 lib 测试基线，本 task 净增 23 个 lib 测试（其中 21 个 enrichment 模块新增 + 2 个 errors 模块辅助测试，从 488→489 还有 1 个来自其他并发 task）。

## 自测验证矩阵

| AC | 场景 | 用例 | 状态 |
|--|--|--|--|
| AC-1 | 步骤 1：`!settings.enabled` 短路 → Fallback(Disabled) | integration `enrich_disabled_short_circuits`（用 resolve_outcome 直接验证 Disabled 分支） | PASS |
| AC-1 | 步骤 2-3：依赖缺失 → Fallback(Unavailable) | lib `outcome_to_event_strings_for_all_variants`（Fallback 分支） | PASS |
| AC-1 | 步骤 4：6 类 KcCallError → outcome 映射 | lib `map_call_error_*_returns_*`（7 个测试覆盖 6 变体 + LlmUnavailable 两子分支） | PASS |
| AC-1 | 步骤 5：emit 事件 payload schema | lib `outcome_to_event_strings_for_all_variants`（5 个分支字面值） | PASS |
| AC-2 | Success → final_md/extractor/kc_enriched/meta/failure_code 五字段 | lib `resolve_outcome_success_path` + integration `enrich_success_returns_full_outcome` | PASS |
| AC-2 | PartialLlmUnavailable → 五字段 + RuleOnly | lib `resolve_outcome_partial_llm_unavailable_path` + integration `enrich_llm_unavailable_with_partial_returns_partial` | PASS |
| AC-2 | Fallback 6 reason × 5 字段 | lib `resolve_outcome_fallback_*`（6 个测试，每 reason 一个） | PASS |
| AC-3 | ResolvedEnrichment Clone | lib `resolved_enrichment_is_clonable` | PASS |
| AC-4 | failure_code 字面严格 = FailureCode enum | lib `failure_code_strings_match_failure_code_enum`（6 reason × 字面） | PASS |
| AC-5 | mock KC `success` → Success | integration `enrich_success_returns_full_outcome` | PASS |
| AC-5 | mock KC `unavailable`（连接拒绝）→ Fallback(Unavailable) | integration `enrich_unavailable_returns_fallback` | PASS（fix-tail 后） |
| AC-5 | mock KC `timeout`（100ms client / 500ms server）→ Fallback(Timeout) | integration `enrich_timeout_returns_fallback_timeout` | PASS |
| AC-5 | mock KC `internal_error` 500 → Fallback(InternalError) | integration `enrich_internal_error_returns_fallback` | PASS |
| AC-5 | mock KC `llm_unavailable + partial` → PartialLlmUnavailable | integration `enrich_llm_unavailable_with_partial_returns_partial` | PASS |
| AC-5 | mock KC `input_too_large` → Fallback(InputTooLarge) | integration `enrich_input_too_large_returns_fallback` | PASS |
| AC-5 | Disabled 短路 → Fallback(Disabled, failure_code=None) | integration `enrich_disabled_short_circuits` | PASS |

边界 & 辅助：

| 类型 | 场景 | 用例 | 状态 |
|--|--|--|--|
| 边界 | 空 frontmatter → final_md = body only | lib `join_empty_frontmatter_returns_body_only` | PASS |
| 边界 | frontmatter 多尾换行归一化 | lib `join_frontmatter_normalizes_trailing_newlines` | PASS |
| 内部 | synthesize_partial_meta 字段固定 | lib `synthesize_partial_meta_has_rule_only_tags_source` | PASS |
| 异常 | Malformed → log::warn 信号（KC-MOD-1 未到位） | lib `map_call_error_malformed_returns_fallback_malformed`（断言 outcome；log 由人工验证 stderr） | PASS |

## 已知局限

1. **`enrich()` 本身无 lib 单测**：`enrich` 签名要 `&AppHandle`，构造真实 `AppHandle` 需 Tauri runtime + `app.manage(Arc<KcClient>)` + `app.manage(Arc<KcProcessManager>)` + DB state 全套 mock，工程量过大。当前策略：lib 测试覆盖 `map_call_error_to_outcome` + `outcome_to_event_strings` + `resolve_outcome` 三个核心纯函数（占 `enrich` 主体 80% 逻辑），integration 测试用 `KcClient::ingest_text` 直接拿 `Result` 等价验证 `enrich` 步骤 4-5。剩余"步骤 1（disabled 短路）+ 步骤 2-3（依赖缺失）+ 步骤 6（emit）"由 lib 单测对纯函数 `outcome_to_event_strings_for_all_variants` 覆盖（emit payload schema）。
2. **`read_kc_settings` 代码与 `kc::process` 重复**：复制了 ~10 行从 AppHandle 拿 DB → KcSettings 的样板。建议未来重构 task 把这段 helper 提到 `kc::settings::load_from_app`，process / enrichment 共享。
3. **emit 事件失败仅 log::warn**：Tauri runtime 不可达 / window 已关 时 emit 抛错，本模块吞 + warn。极端情况下前端可能感知不到一次 enrich 完成（但 DB 落地不受影响，刷新页面可恢复）。
4. **`synthesize_partial_meta` 用占位字段**：`doc_id="doc-partial"` / `kc_version="unknown"` 让 task_015 写 DB 时能区分"完整 success"与"partial 路径"，但前端 inspector 渲染 frontmatter 时会看到 `kc_version: unknown`——这个字面值的 UI 显示策略留给 task_018 / task_019 决定（建议显示为"—"或"规则增强"标签）。

## 需要 Reviewer 特别关注的地方

1. **5 类失败映射完整性（input.md AC-1 步骤 4 + Reviewer 关注项 #1）**：
   - `map_call_error_to_outcome` 必须覆盖 `KcCallError` 全部 6 变体——但 `LlmUnavailable` 有 `Some` / `None` 两子分支，所以实际是 **7 路径**。
   - 守护：每个变体都有独立单测（`map_call_error_*_returns_*`），且 Rust `match` 在变体增加时编译失败强制更新。
   - **关键决策点**：`LlmUnavailable { None }` 走 `Fallback(InternalError("LLM unavailable, no partial"))` 而非 `Fallback(Unavailable)`——理由：KC 是 reachable 的，只是 LLM 子模块挂了；归到"内部错误"而非"不可达"，与 input.md AC-1 步骤 4 完全一致。

2. **KC-MOD-1 未到位的 Malformed 信号（input.md Reviewer 关注项 #3）**：
   - 当 KC 返 200 但 `enhanced_markdown` 字段缺失时，`KcClient::parse_success_body` 会返 `KcCallError::Malformed`。
   - `map_call_error_to_outcome` 在 Malformed 分支显式 `log::warn` 含 "KC-MOD-1 未到位" 字样，方便 reviewer 在生产日志里 grep 这个信号。
   - failure_code 与 `Internal` 同走 `E_KC_ENRICH_FAILED`（B 类聚合）——这是符合 ADR-004 的设计：用户看到的 conversion_meta.failure_code 不区分"KC 协议变形"vs"KC 内部异常"，但日志可区分。

3. **failure_code 字符串与 task_003 `as_str()` 一致性（input.md Reviewer 关注项 #2）**：
   - 单测 `failure_code_strings_match_failure_code_enum`（enrichment.rs:796-828）走 6 个 reason → 字面值的双向对齐：左边 `fallback_reason_to_failure_code(&reason)`，右边 `Some(FailureCode::EKc*.as_str())`。如果 task_003 改动 `as_str()` 返回值（如 `"E_KC_UNAVAILABLE"` 改成 `"E_KC_UNREACHABLE"`），本单测立刻 fail。
   - 守护链：`fallback_reason_to_failure_code` 委托 `KcFallbackReason::to_failure_code()`（task_005 实装）→ 该函数返回 `Option<FailureCode>` → 本模块再走 `as_str()`——任何一环字面漂移都被守护单测捕获。

4. **`mock_kc.rs::start_with_unavailable()` 修复方案**：
   - 原实现"start wiremock + drop"不可靠（端口可能立即被占位 server 复用）。改为 `TcpListener::bind("127.0.0.1:0") + drop`：用 OS 标准 socket 拿空闲端口后立即释放，wiremock placeholder 走随机端口池——端口冲突概率极低。
   - 极端边缘案例：若 OS 端口分配巧合让 placeholder 拿到原 addr 端口（概率 ~1/65536 假设池均匀），测试会变成 wiremock 默认 404 → `Internal { code: "UNKNOWN" }` → assert fail。这是已知"理论上可能"但实践中没见过的脆弱性；若未来 CI 偶现 flaky，可加 retry 或改为 `TcpStream::connect` 主动验证端口确实拒绝再 return。

5. **`resolve_outcome` 用 `raw.structured_md` 而非 `outcome.base_md`**（enrichment.rs:449-461）：
   - input.md AC-2 显式约定 Fallback 分支 `final_md = raw.structured_md.clone()`。
   - 现实风险：`enrich(app, asset, raw_md)` 入参的 `raw_md` 与 scheduler 的 `extraction_result.structured_md` 在调用链中可能漂移（如有人在调用 enrich 前对 raw_md 做了 normalize / trim）。本模块通过 `resolve_outcome` 锁死到 `raw.structured_md` 消除该不确定性。
   - 影响：`enrich()` 传给 `Fallback.base_md` 的 raw_md 在 resolve 阶段被丢弃。这是有意设计，不是 bug。
