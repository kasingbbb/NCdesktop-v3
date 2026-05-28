# Review Scorecard — task_027b_extract_kc_persist_helper

- Commit: `1df063f0`
- Branch: `feat/windows-unit-13-cloud-ai`
- Reviewer: Claude Opus 4.7（1M ctx）
- Date: 2026-05-28
- 复杂度: XS（净 -262 行：+34 / -296）

## 实装路径回顾

input.md AC-1 给出 A（pub(crate)）/B（抽新模块）二选一；实装走"修正后路径 A"：`pub fn` + `#[doc(hidden)]`。
理由：integration test crate 是 **external crate**，`pub(crate)` 不可见 → 必须 `pub`；`#[doc(hidden)]`
让 rustdoc 不导出，语义上标"测试基础设施"。否决路径 B 的依据（模块语义割裂 / 行数远超预算 / scheduler
内 helper 与 save_and_materialize 上下文邻近）合理。

## 6 维评分

| 维度 | 评分 | 证据 |
|---|---|---|
| 1. DRY 闭环彻底度 | **5.0** | grep `fn simulate_scheduler_kc_persist\|fn persist_resolved_to_db\b`：**0 处**；grep `fn kc_persist_resolved_with_conn`：仅 1 处（scheduler.rs:1378）；三处 test crate 一律 `use app_lib::extraction::scheduler::kc_persist_resolved_with_conn;` 直调 canonical（无 wrapper） |
| 2. 可见性决策合理性 | **4.8** | "pub fn + #[doc(hidden)]" 是 Rust 生态对"测试基础设施"的标准模式；input.md 推荐的 `pub(crate)` 实际不可行（integration test 是 external crate），output.md 显式纠偏并写入 4 行 doc-comment 说明决策；唯一可挑剔点：`#[doc(hidden)]` 不阻止下游 crate import（NCdesktop 暂无下游 crate，可接受） |
| 3. 测试 0 退化 | **5.0** | kc_failure_injection 5/5 / kc_e2e_pipeline 4/4（删 guard 后预期）/ kc_perf_smoke 3/3 / lib 537/537——全部本地复跑验证通过 |
| 4. API 不变性 | **5.0** | git diff 范围严格仅在 `kc_persist_resolved_with_conn` 签名前 7 行（doc + `#[doc(hidden)]` + `pub`）；`save_and_materialize`（task_012 12 行注入）、`kc_persist_resolved`（pub-free wrapper）字面 0 变化 |
| 5. 删除行数合理性 | **4.8** | -262 行净减；三处 helper（~85% 字面一致）+ 1 个 guard 测试（task_023 仅 Success 单路径，canonical 单源后 drift 不可能）+ ~60 行 doc 注释，删除范围与 DRY 收益严格对齐 |
| 6. 文档与决策追溯 | **4.6** | scheduler.rs 内 4 行 doc-comment 标注 task_027b 决策依据 + "生产代码请走 save_and_materialize" 警示；e2e/perf test 删除点都留有"task_027b：原 helper 已删除，bench/test 现直调 canonical"注释；guard 删除的接力守护（`save_and_materialize_with_kc_success_writes_enhanced_md`）已被明文 inline 引用 |

**综合分**：(5.0 + 4.8 + 5.0 + 5.0 + 4.8 + 4.6) / 6 = **4.87 / 5**

## 判定：**PASS**（≥ 4.3）

DRY 闭环：**彻底**（grep 全仓 0 处复刻，canonical 唯一定义在 scheduler.rs:1378）。

## Reviewer 重点回应

### 1. `#[doc(hidden)]` 是否清楚标注"测试基础设施"？

清楚。doc-comment 显式写："`#[doc(hidden)]` 标注表明它是测试基础设施，**生产代码请走 `save_and_materialize`**"。
未来若有人误把它当公开 API 调用，doc-comment + 调用栈追溯（`save_and_materialize` → `kc_persist_resolved` →
`kc_persist_resolved_with_conn`）都会反向引导。

### 2. 删除 guard 测试是否真不再需要？

**真不再需要**。该 guard（`persist_helper_matches_kc_persist_resolved_with_conn_for_success`）原意是守护
"test 端复刻 helper 与 lib 内 canonical 行为等价 / 不漂移"——但 canonical 单源化后，test 端无独立 helper
可漂移，guard 命题在结构上失效。同时 lib 内的 `save_and_materialize_with_kc_*` 单测（Success/Disabled/Partial
3 路径，scheduler.rs:1866-1978）直接守护 canonical 行为，覆盖等价或更强。

### 3. 是否建议补 lib 内 4-outcome 路径单测？

**建议补 1 个**（不阻断 PASS）：

- 现有 lib `#[cfg(test)] mod` 已覆盖 **Success**（Some meta + None fc）/ **Disabled**（None + None）/
  **Partial**（Some meta + Some fc）3 路径。
- **缺失"纯 Failure"路径**（None meta + Some failure_code）——该路径只在
  `kc_failure_injection.rs::failure_b_internal_error_falls_back_with_failure_code` 等 integration test
  里覆盖（integration test 是 external crate，跑得慢且不在 `cargo test --lib` 闭环里）。
- 补一个 `save_and_materialize_with_kc_failure_only_writes_failure_code_no_meta` lib 单测（参考现有 3 个
  test 风格 ~30 行），可在 lib 闭环内独立守护"无 meta 也要 append conv_meta + UPDATE failure_code"语义，
  替代 task_023 删除的 guard 测试更精确（按 4 outcome 全覆盖而非仅 Success）。
- 优先级：**P2 follow-up**（不阻断本 task PASS——integration test 已经覆盖了 5 个 KC 失败子类型）。

## 3 个最关键观察

1. **可见性决策的纠偏比按 input.md 执行更可贵**——output.md 明确推翻 input.md 的 `pub(crate)` 建议、
   补正 `pub fn + #[doc(hidden)]`，附 Rust crate 边界的根因分析，体现了 reviewer 视角而非纯执行视角。

2. **删除 guard 的接力守护链路明晰**——task_023 的 guard 删除后，由 lib 内 3 个
   `save_and_materialize_with_kc_*` 单测接力（甚至覆盖面更广：3 路径 vs 原 guard 仅 Success），
   配合三处 integration test 直调 canonical，drift 风险结构性消除。

3. **修改半径极小但闭环完整**——scheduler.rs 仅改 7 行（含 4 行 doc + #[doc(hidden)] + pub + 1 处空行），
   test crate 净删 ~280 行字面复刻；`save_and_materialize` 与 `kc_persist_resolved` 公开 wrapper
   字面 0 变化；这种"小内核暴露 + 大复刻删除"的形态是 DRY 收敛的教科书示例。

## 后续建议（非阻塞）

- **P2**：补 `save_and_materialize_with_kc_failure_only` lib 单测，4 outcome 路径全覆盖在 lib 闭环里。
- **P3（可选）**：若未来发布 NCdesktop 给下游 crate（plugin / SDK），考虑给 `kc_persist_resolved_with_conn`
  外加 `#[cfg(any(test, feature = "test-helpers"))]` cfg-gate，进一步把测试基础设施隔离出 release。
  当前 NCdesktop 是 binary crate 无下游，无需立即处理。
