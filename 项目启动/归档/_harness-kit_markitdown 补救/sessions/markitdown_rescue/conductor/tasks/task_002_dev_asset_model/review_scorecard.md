# Review Scorecard — task_002_dev_asset_model

## 审查前验证

- [x] 测试结果存在且非空（output.md §测试结果 完整粘贴 `cargo check` 输出）
- [x] 自测验证矩阵存在，正常路径全 PASS（7 行场景，5 个 PASS / 2 个未测但已说明跳过原因，合理）
- [x] 架构遵守声明已填写（5 项全 ✅，偏离=无）

> 交付契约完整，进入实质审查。

---

## 审查思考过程

### 1. Task 意图复述
为 `Asset` 模型与 `assets` 表加上 `source_asset_id: Option<String>` 与 `derivative_version: i32` 两个字段（来自 ADR-001/ADR-002），让 `cargo check` 重新变绿；新增 V5 迁移（PRAGMA 守卫 + 索引），并把全仓 `Asset { ... }` 字面量构造点补齐。

### 2. AC 逐条检查

| AC | 状态 | 证据 |
|---|---|---|
| AC-1 Asset 加 2 字段 + `#[serde(default)]` | ✅ | `models/asset.rs:22-29`，两字段紧跟 `is_starred`，类型 `Option<String>` / `i32`，均带 `#[serde(default)]`，且整体加了 `#[derive(Default)]`；`#[serde(rename_all = "camelCase")]` 保留。 |
| AC-2 V5 迁移 PRAGMA 守卫 + 默认值 | ✅ | `migration.rs:35-63`，`list_table_columns` 取列名集，`source_asset_id TEXT DEFAULT NULL`、`derivative_version INTEGER NOT NULL DEFAULT 0`，完全对齐 Architect §五.1 SQL。 |
| AC-3 幂等可重跑 | ✅ | 两次 ALTER 均在 `if !existing_cols.iter().any(...)` 守卫内；`CREATE INDEX IF NOT EXISTS`；`PRAGMA user_version=5` 重置幂等。 |
| AC-4 索引 `idx_assets_source_asset_id` | ✅ | `migration.rs:54-55`，`IF NOT EXISTS`。 |
| AC-5 `cargo check` 通过 | ✅ | 复核实际运行：`Finished dev profile ... in 0.72s`，0 error / 4 warning（均位于 `src/llm/chat.rs`，pre-existing 与本 task 无关）。 |

### 3. 关键发现

**发现 1（架构级，MAJOR-记录但不阻断本 task）**：`src/extraction/mod.rs:4` 当前确实是 `// pub mod scheduler;`（注释行的紧邻还有一行说明"依赖未恢复的 Asset.source_asset_id / db::extraction / sha2 等"）。这意味着：
- Architect §〇.1 关于"scheduler.rs 引用未定义符号 → cargo check 失败"的前提**部分失真**——scheduler 当前根本不参与编译，cargo check 0 error 不完全是字段补齐的功劳；
- 但对 task_002 本身的 AC 完成判断**没有影响**：AC-1~4 是结构性合同（字段存在、迁移正确、索引存在），都已客观满足；AC-5 要求 model/migration 这块不再有错，也客观达成；
- 真正的影响在**下游 task 的 input.md 必须显式加入"取消 `extraction/mod.rs:4` 注释"这一动作**——这一点 Dev 已在 output §需要 Reviewer 特别关注 #3 明确提请，是高质量的交付级别的"已知风险旗"。
- 我把它登记为 MAJOR，但归属 task_003/004/008 的 input 修订，**不要求 task_002 在本次返工**。

**发现 2（次要，确认本 task 干净）**：3 处构造点（`dropzone.rs:541`、`asset.rs:59`、`sync.rs:155`）全部使用 `..Default::default()` 模式补齐；scheduler.rs:655 已显式填字段（且模块当前未编译，本来就不引发错误）；`row_to_asset` 在 `db/asset.rs:211-229` 列序对齐（is_starred=12, source_asset_id=13, derivative_version=14），与 `ASSET_SELECT` 常量、`get_by_project_and_tag` 内联 SQL 三处完全一致。INSERT 列表也同步加了两列、占位符到 `?15`。

---

## 评分

| 维度 | 权重 | 分数 (1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 30% | 5 | 5 个 AC 全部客观满足；实地 `cargo check` 复跑确认 0 error；3 处 `..Default::default()` 编译通过反向证明 Default impl 完整。 |
| 架构一致性 | 20% | 5 | 字段类型、SQL DDL、索引名、迁移方式（ALTER ADD COLUMN）完全与 §五.1 / ADR-001 一致；未引入新依赖；未触碰 Cargo.toml。 |
| 可维护性 | 15% | 5 | `list_table_columns` helper 抽象合理可复用；doc 注释指向了 ADR-001/002，未来读代码的 Agent 能溯源；字段位置紧跟 `is_starred` 不打乱列序，对 `from_row` 友好。 |
| 安全性 | 10% | 5 | 全部 SQL 参数化（`params![...]`）；新代码无 `.unwrap()` / `.expect()`；错误用 `?` + `map_err`；迁移只 ALTER ADD，无 DROP/RENAME。 |
| 测试覆盖 | 15% | 3.5 | `cargo check` 通过 + 静态推理覆盖幂等性，但未跑端到端"v4 库 → 启动 → V5 迁移 → SELECT 新字段"集成测试。Dev 已明确登记为已知局限 #1，建议挂到 task_003。给 3.5 反映"静态保证充分但运行期未验证"。 |
| 代码质量 | 10% | 5 | 命名清晰；helper 提取得当；doc 注释指向 ADR；与既有迁移函数风格一致；无连带重构。 |

**加权综合分**：
0.30×5 + 0.20×5 + 0.15×5 + 0.10×5 + 0.15×3.5 + 0.10×5
= 1.50 + 1.00 + 0.75 + 0.50 + 0.525 + 0.50
= **4.775 / 5**

---

## 总体判断

- [x] **PASS**
- [ ] FIX
- [ ] BLOCKER

理由：5 个 AC 全部客观满足，`cargo check` 实地复跑 0 error；无 BLOCKER；无 MAJOR 归属本 task。下游影响（scheduler 模块注释）已由 Dev 在交付中显式提请，不构成 task_002 的返工触发。

---

## 问题列表

### BLOCKER
（无）

### MAJOR
（无 — 归属本 task 的 MAJOR 为 0）

### 跨 task 影响登记（不阻断 task_002，但 Conductor 必须在后续 task 的 input.md 体现）

**M-1（跨 task）：`src/extraction/mod.rs:4` 的 `// pub mod scheduler;` 必须在 markitdown 集成全套完工时取消注释**
- **代码位置**：`src-tauri/src/extraction/mod.rs:4`
- **触发时机**：task_003（`db::asset` 三新函数：`find_markdown_derivative` / `update_markdown_derivative` / `set_derivative_version`）与 task_004（`db::tag::propagate_tags_to_derivative` 等）落地后，scheduler 引用的所有符号都齐了，此时必须把 `pub mod scheduler;` 取消注释。
- **修复方向**：在 task_003 或 task_008 的 input.md 末尾"前置/收尾动作"显式追加："验证 `cargo check` 通过后，取消 `src/extraction/mod.rs:4` 的 `pub mod scheduler;` 注释，并再次 `cargo check`，确认 scheduler 加入编译后无残留 error。"
- **验证标准**：task_003/004/008 完工时，`extraction/mod.rs` 中不再存在被注释的 scheduler 行；`cargo check` 在激活 scheduler 后仍 0 error。
- **本次责任归属**：**不属于 task_002**。Reviewer 已在此登记，建议 Conductor 把这一动作并入 task_003 或 task_008 的 input.md。

### MINOR
1. **`v5_asset_derivative_columns` 未显式包事务**：当前每次 ALTER 都独立 `execute_batch`。若第一个 ALTER 成功、第二个 ALTER 失败（理论上极端罕见），DB 会停在"含 source_asset_id 但无 derivative_version 且 user_version 仍是 4"的状态——下次启动 PRAGMA 守卫会跳过已存在的列，继续补另一列，可恢复，不会卡死。Dev 在自测矩阵已经分析过，可接受。后续可考虑统一 `BEGIN; ... COMMIT;` 包裹但不强制。
2. **`v4_knowledge_understanding` 引用 `concepts(id)` 外键但 V3 未存在**（pre-existing 问题，与本 task 无关，仅记录）：Dev 在迁移函数注释里已经标注"V3（concepts 等基表）未在当前源码中存在；V4 仅创建表结构，运行时插入需先建 concepts。"这是历史遗留，不归本 task 修。
3. **Asset 现在可 `Asset::default()` 构造出"空 asset"**：id/asset_type/file_path 均为空串，理论上是无效状态。Dev 已 grep 确认本仓内无 `assert!(!asset.id.is_empty())` 类断言，未来若有新代码以"Asset 总有效"为前提，可能出错。属可接受副作用，不强制收紧。

---

## 给 Dev 的修复指引

**不适用（PASS）**。本 task 无需返工。

唯一需要 Conductor 做的事是：**把"取消 `src/extraction/mod.rs:4` 注释"动作并入 task_003 或 task_008 的 input.md**（详见 M-1 跨 task 影响登记）。这不是 Dev 的修复，而是 Conductor 的调度层动作。

---

## 自检清单

- [x] 逐条 AC 检查
- [x] 实地打开 6 个文件读代码（不是仅看 output.md 描述）
- [x] 复跑 `cargo check` 独立验证（0 error，与 Dev 报告一致）
- [x] session_context §5 代码规范扫描（无 unwrap/expect、SQL 参数化、ALTER 仅 ADD COLUMN）
- [x] 关键发现给出了归属与下一步动作建议
- [x] 评分诚实（测试覆盖只给 3.5 而非 5，因运行期未真实跑迁移）
