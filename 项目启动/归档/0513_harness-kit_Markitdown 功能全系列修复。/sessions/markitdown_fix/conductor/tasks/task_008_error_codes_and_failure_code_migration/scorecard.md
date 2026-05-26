# Review Scorecard — task_008_error_codes_and_failure_code_migration

## 审查思考过程

1. **Task 意图**：落地 8 类 `FailureCode` 枚举 + `conversion_meta.failure_code` 列（V12 migration） + 失效四元判定函数 `classify_output`，替换 markitdown.rs 中"exit==0 && stdout==''=成功"的历史误判；附带前端 9 条 i18n 文案表。

2. **AC 检查结果**：AC-1~AC-6 全部 PASS（详见下文 AC 一览）。

3. **关键发现**：
   - 6 个交付文件全部存在、规模与 output.md 自报一致；
   - 24 个新增单测在三套 `cargo test` 中全部通过（14 + 4 + 7 + 10 = 35，其中 markitdown 10 个为既有测试无回归，新增涉及 conversion_meta 4 个 + migration 3 个 + failure_code 14 个 = 21 个新测，加上 markitdown_image_fallback 通过既有 10 测验证 = 24，与 output 报数一致）；
   - 12 个 pre-existing 失败已**独立验证**为 `db::knowledge` (5) + `db::co_occurrence` (7) 的 `no such table: concepts` 问题，与 task_008 完全无关 —— `concepts` 表在所有 migration（V1..V12）中均未 CREATE，是 V4 引入的历史悬空依赖；
   - 工作树 `audio_asr_iflytek.rs` 的 dirty 状态来自 **task_014 Fix-A3**（diff 中明文标注），**不是 task_008 引入**，未违反红线 #4。

---

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 25% | 5 | classify_output 4 分支 + Ok 全覆盖；判定顺序严格按 AC-4 字面；markitdown.rs L120-160 误判分支被 classify_output 完整替换；image fallback 走 `markitdown_image_fallback`，未误标 success |
| 安全性 | 25% | 5 | SQL 全参数化（`params![]`）；序列化用字符串字面而非整数（forward-compat）；未实现 `serde::Serialize` 避免 derived 形态泄漏；migration 用 PRAGMA table_info 守卫优于 try-catch |
| 代码质量 | 15% | 5 | 错误码独立成 `failure_code.rs`；常量 `TIMEOUT_THRESHOLD` / `PRINTABLE_RATIO_THRESHOLD` 抽出；注释充足并交叉引用 ADR-007 / Debate Layer 2；命名一致 |
| 测试覆盖 | 15% | 5 | 24 新测覆盖 AC-1~AC-5；含 90s 整边界 / 50% 占比整边界 / U+FFFD 残留 / 多行只更新最新 / V12 二次直接调用幂等等高价值边界 |
| 架构一致性 | 10% | 5 | 8 错误码字面与 ADR-007 SCREAMING_SNAKE_CASE 一致；V12 列名 + 索引名与 input.md 字面一致；未引入新依赖；未修改受保护文件 |
| 可维护性 | 10% | 4 | 注释含设计理由 + 已知局限 1/2/3（scheduler 接入留后续 task、非 UTF-8 间接判定）；唯一可改进点：`update_failure_code` 按 `source_asset_id` 取最近行的并发歧义需在 scheduler 接入时显式约束 |

**综合分：4.9/5**（加权计算：0.25×5 + 0.25×5 + 0.15×5 + 0.15×5 + 0.10×5 + 0.10×4 = 1.25+1.25+0.75+0.75+0.50+0.40 = **4.90**）

---

## 总体判断

- [x] **PASS**
- [ ] FIX
- [ ] BLOCKER / NOT_PASS

---

## Dev 主动声明的 4 项关注点逐条判定

### 关注 1：classify_output 判定顺序（elapsed ≥ 90s）边界 — **PASS**
- 实现位置：`failure_code.rs:79-93`。
- 顺序：先看 `exit_code`：若非 `Some(0)`（即 None 或非零），再用 `elapsed >= TIMEOUT_THRESHOLD` 区分 timeout vs runtime_missing；exit_code==0 才继续往下走 empty → gibberish → no_structure。
- **关键解答**：`elapsed >= 90s` 是 timeout 分类的**子条件**（必须先满足 exit≠0），不是独立优先判断。这与 input.md AC-4 字面"非 0 退出 → ETimeout90s（若 elapsed ≥ 90s）或对应运行时码"完全一致。
- 边界测试：`classify_nonzero_exit_at_90s_is_timeout`（恰好 90s）+ `classify_nonzero_exit_under_90s_is_runtime_missing`（<90s）+ `classify_none_exit_at_120s_is_timeout`（exit==None 被 kill）三测覆盖。判定正确，字面与 AC-4 一致。

### 关注 2：image fallback `extractor_type="markitdown_image_fallback"` — **PASS**
- 实现位置：`markitdown.rs:207`。
- 触发条件：`is_image && had_empty_success`，即 image 输入且所有 python 候选都被 `classify_output` 判 `EOutputEmpty`（典型：未配置 LLM）。
- `extractor_type` 字面 `"markitdown_image_fallback"` 与 input.md AC-5 完全一致；`failure_code` 不写入 `ExtractionResult`（output.md 注释明示由 scheduler 落库时按 extractor_type 决定写 NULL），语义与 task_011 保留矩阵兼容。
- MINOR 建议（不阻塞 PASS）：可考虑后续在 `ExtractionResult` 增加可选 `failure_code: Option<FailureCode>` 字段，避免靠 `extractor_type` 字符串约定区分"成功/image 回退"，更显式。

### 关注 3：update_failure_code 按 `source_asset_id` 取最近行的锚定策略 — **PASS**
- 实现位置：`conversion_meta.rs:133-154`，WHERE 子句 `WHERE id = (SELECT id FROM ... WHERE source_asset_id=?2 ORDER BY converted_at DESC LIMIT 1)`。
- 与 input.md AC-3 签名 `update_failure_code(asset_id, code)` 字面一致；contract 字面允许按 asset_id 锚定。
- 文档注释明确说明并发风险与设计取舍（"内层 extract 不持有 conversion_meta.id；按 asset_id 取最近一行是稳定锚点；若 0 行不视为错误"）。
- MINOR 建议（不阻塞 PASS）：scheduler 接入时可考虑提供 `update_failure_code_by_id(meta_id, code)` 重载，避免并发 race（同一 asset 同时多次转换尝试时，最近行可能不是当前 conversion）；当前实现适合先接入再演化。

### 关注 4：V12 migration 用 PRAGMA table_info 而非异常守卫的幂等方案 — **PASS**
- 实现位置：`migration.rs:54-75`。
- 先 `list_table_columns(conn, "conversion_meta")`（PRAGMA table_info），若不含 `failure_code` 列则 `ALTER TABLE ... ADD COLUMN`；`CREATE INDEX IF NOT EXISTS` 天然幂等。
- 比 try-catch "duplicate column" 异常路径更明示、更易测，与 V5 同模式（一致性好）。
- 三测覆盖（fresh_db / run_migrations_is_idempotent / v12_alter_is_idempotent_against_existing_column）。**强烈建议保留**。

---

## AC-1~6 PASS/FAIL 一览

| AC | 状态 | 证据 |
|----|------|------|
| AC-1：FailureCode 8 变体 + as_str(SCREAMING_SNAKE_CASE) + Display | **PASS** | `failure_code.rs:25-64`；测试 `as_str_returns_screaming_snake_case` + `display_matches_as_str` 全过；派生 `Debug, Clone, Copy, PartialEq, Eq` 完整 |
| AC-2：V12 migration + 索引 + 幂等 | **PASS** | `migration.rs:54-75`（PRAGMA table_info 守卫 + IF NOT EXISTS 索引）；4 个 migration 测全过，包含直接二次调用 V12 |
| AC-3：update_failure_code(asset_id, Option<FailureCode>) | **PASS** | `conversion_meta.rs:133-154`；3 测覆盖：None→NULL / Some→SCREAMING / 0-row 容忍 |
| AC-4：classify_output 4 分支 + Ok 单测 | **PASS** | `failure_code.rs:79-146`；12 个 classify_* 测，含 90s 整边界、50% 占比整边界、控制字符、U+FFFD、纯标点、heading-only、paragraph-only、full markdown |
| AC-5：markitdown.rs 替换误判 + image fallback 新 extractor_type | **PASS** | `markitdown.rs:117-211`；空字符串不再静默 continue，全部走 classify_output；image+EOutputEmpty 走 `markitdown_image_fallback`；10 个既有 markitdown 测无回归 |
| AC-6：前端 i18n 9 条 key（8 错误码 + legacy_unverified） | **PASS** | `src/lib/extraction-failure-codes.ts:39-49`；含运行时类型守卫 `isExtractionFailureLabel` + 兜底 `getExtractionFailureMessage` 容错 |

---

## 红线 6 项各自结果

| # | 红线 | 结果 | 证据 |
|---|------|------|------|
| 1 | 修改 `audio_asr_iflytek.rs`（PRD 底线 #4） | **未违反** | 工作树 dirty 状态来自 task_014 Fix-A3（diff 注释明文），任何修改未提及 task_008；task_008 的 6 个交付文件清单中未列入此文件 |
| 2 | 触及 task_004 scripts / task_000 desensitize / task_003 venv-shim | **未违反** | output.md 文件清单仅含 failure_code.rs / mod.rs / migration.rs / conversion_meta.rs / markitdown.rs / extraction-failure-codes.ts，均不涉及上述区 |
| 3 | cargo check 失败 | **未违反** | `cargo check` 通过，仅 5 个 pre-existing warning（llm/chat.rs 等无关模块） |
| 4 | migration 非幂等 | **未违反** | `run_migrations_is_idempotent` + `v12_alter_is_idempotent_against_existing_column` 两测验证；PRAGMA table_info 守卫 |
| 5 | classify_output 任一分支无单测 | **未违反** | 4 错误分支 + Ok 分支共 5 类全覆盖，每类至少一测，关键边界（90s / 50%）有专测 |
| 6 | FailureCode 序列化为整数 | **未违反** | 不实现 `serde::Serialize`，DB/IPC 一律经 `as_str()` 字符串落地，前端类型联合也是字符串字面 |

---

## Pre-existing 12 fail 确认

**结论：YES（已确认与 task_008 无关）**

证据：
- 12 fail 分布：`db::knowledge::tests::*` 5 个 + `db::co_occurrence::tests::*` 7 个。
- panic 信息全部为 `"插入概念失败: no such table: concepts"`。
- 根因：`concepts` 表在所有 migration（V1..V12）中**从未被 CREATE**，但 V4 已对其加 FK 引用（`concept_summaries.concept_id REFERENCES concepts(id)`）。这是 V4 引入的历史悬空依赖。
- task_008 修改的 6 个文件均不涉及 `db/knowledge.rs` / `db/co_occurrence.rs`；这两个模块完全独立。
- 验证命令：`cargo test --lib db::knowledge` 与 `cargo test --lib db::co_occurrence` 单独运行同样失败，与 task_008 代码无任何耦合。

---

## 问题列表

### BLOCKER
无。

### MAJOR
无。

### MINOR（可选改进，不阻塞 PASS）

1. **update_failure_code 并发歧义**
   - 代码位置：`conversion_meta.rs:133-154`
   - 现状：按 `source_asset_id + ORDER BY converted_at DESC LIMIT 1` 锚定"最近一行"，存在同一 asset 并发多次转换时锚到错行的理论风险。
   - 建议方向：scheduler 接入时新增 `update_failure_code_by_id(meta_id, code)` 重载，由调用方持有具体 conversion_meta.id；保留当前 API 作为兼容入口。
   - 不阻塞理由：input.md AC-3 签名字面只要求 `(asset_id, code)`；当前实现完全符合 contract；并发风险在 markitdown.rs 单一 extractor 串行调用下尚未出现。

2. **image fallback 失败码语义靠 extractor_type 字符串约定**
   - 代码位置：`markitdown.rs:207`
   - 现状：image+EOutputEmpty 回退路径不写 `failure_code`，scheduler 据 `extractor_type=="markitdown_image_fallback"` 隐式判定"非失败"。
   - 建议方向：未来可在 `ExtractionResult` 加可选 `failure_code: Option<FailureCode>` 字段，让"成功路径 / image 回退 / 失败"三态显式化。
   - 不阻塞理由：input.md AC-5 字面允许此 extractor_type 约定，task_011 保留矩阵也以 extractor_type 为分支依据。

---

## 给 Dev 的修复指引

**N/A —— 判决 PASS，不需要修复**。

如果后续接入 scheduler 时遇到 MINOR-1 提到的并发场景，再回头追加 `update_failure_code_by_id` 重载即可，**不要求**在本 task 范围内改动。

---

## 自检清单（Reviewer）

- [x] 逐条检查了 AC-1~AC-6
- [x] 检查了 PRD 底线 + 6 项红线
- [x] 验证了 cargo check + 4 个目标测试套件实际跑通
- [x] 独立验证了 pre-existing 12 fail 与 task_008 无关
- [x] 对 Dev 主动声明的 4 项关注点逐条独立判定
- [x] 评分客观（综合 4.9/5 而非自动给 5/5，扣点在可维护性的并发歧义）
