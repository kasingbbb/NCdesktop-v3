# Review Scorecard — task_004_dev_db_tag_funcs

## 审查前验证（契约）

- [x] 测试结果存在且非空（cargo check + cargo test --lib db::tag 输出完整粘贴，3 passed/0 failed）
- [x] 自测验证矩阵存在且正常路径全部 PASS（场景 A/B/C 均 PASS + 4 个边界/异常说明）
- [x] 架构遵守声明已填写（4 项 ✔，偏离说明"无"）

## 审查思考过程

### 1. Task 意图复述
在 `db/tag.rs` 实现两个公共标签传播函数（`propagate_tags_to_derivative` / `sync_tags_to_canonical_derivatives`），并在 dropzone 的 AI 打标 `link_to_asset` 循环结束后调用 sync 同步到已有 markdown 衍生件；保证全仓 `INSERT INTO asset_tags` 仅 3 处（R6 单点实现约束）。

### 2. AC 逐条检查

- **AC-1**：✅ `propagate_tags_to_derivative(conn, root, derived) -> Result<usize, String>` 签名一致；SQL 为 `INSERT OR IGNORE INTO asset_tags (asset_id, tag_id) SELECT ?1, tag_id FROM asset_tags WHERE asset_id = ?2`（tag.rs:134-140）；返回 `execute` 的 `usize` 行数。
- **AC-2**：✅ `sync_tags_to_canonical_derivatives(conn, root) -> Result<usize, String>` 签名一致；SQL 与 input.md 完全一致：`INSERT OR IGNORE INTO asset_tags (asset_id, tag_id) SELECT a.id, at.tag_id FROM assets a JOIN asset_tags at ON at.asset_id = ?1 WHERE a.source_asset_id = ?1 AND a.asset_type = 'markdown'`（tag.rs:160-168）。
- **AC-3**：✅ dropzone.rs:396 `sync_tags_to_canonical_derivatives(&conn, &asset.id)` 调用置于 `for tag_name in r.tags { ... }` 循环（374-392）之后、`Ok(())`（404）之前；失败仅 `log::warn!`（397-401），不返回 Err；`conn` 是 `MutexGuard<Connection>`，作用域在循环之后仍有效，无 unwrap/panic。
- **AC-4**：✅ 3 个 #[test]：scenario_a（propagate 复制 2 标签）、scenario_b（sync 同步到 2 个 markdown 衍生件、跳过 image，inserted=4）、scenario_c（重复 propagate 幂等 1/0/0）；setup_db 使用内存库 + `migration::run_migrations`，是真实迁移，非 mock。
- **AC-5**：✅ 亲自跑 `grep -rn "INSERT INTO asset_tags\|INSERT OR IGNORE INTO asset_tags" src-tauri/src/` 复核，**输出恰好 3 行**：tag.rs:108 (link_to_asset)、tag.rs:136 (propagate)、tag.rs:162 (sync)。Dev 报告属实。

### 3. 关键发现
- **grep 复核结果与 Dev 报告完全一致**，R6 单点实现硬约束达成。
- **cargo check 0 error，4 warning 全部来自 `src/llm/chat.rs`**（与本 task 无关，task_002/003 末态一致）；`extraction/mod.rs:3-4` 的 scheduler 注释保持未取消。
- **生产代码 0 处 unwrap/expect**：所有 `.unwrap()`/`.expect()` 都在 `#[cfg(test)] mod tests` 内。
- **dropzone `conn` 借用安全**：`MutexGuard<Connection>` 自动 deref 到 `&Connection`，循环内每次 `link_to_asset` 与循环后的 `sync_*` 共享同一 guard，作用域至 `Ok(())` 终结，无重复加锁/死锁风险。

## 评分

| 维度 | 权重 | 分数(1-5) | 说明 |
|------|------|-----------|------|
| 功能正确性 | 30% | 5 | AC 全部满足，SQL 与 input.md 字面一致；3 个测试全 PASS；返回类型/幂等性正确。 |
| 架构一致性 | 20% | 5 | R6 单点实现完美达成（grep 复核 3 处）；未触碰 scheduler 注释、未引入新依赖、未触前端。 |
| 可维护性 | 15% | 5 | 函数注释清晰说明使用时机；usage_count 维护逻辑封装到辅助 fn；与 link_to_asset 风格一致。 |
| 安全性 | 10% | 5 | 全部 `params![...]` 参数化；无字符串拼接；错误统一 `map_err(\|e\| format!(...))`；不向上层泄露原始 stderr。 |
| 测试覆盖 | 15% | 4 | 三场景 A/B/C 全覆盖且真实迁移；边界（原件无标签 / 无衍生件）仅"隐式覆盖"未显式断言，扣 1 分。 |
| 代码质量 | 10% | 5 | 生产代码 0 unwrap；命名清晰；文档注释中文说明使用场景；DRY（usage_count 刷新抽离）。 |

**综合分：4.85/5**

加权计算：0.30×5 + 0.20×5 + 0.15×5 + 0.10×5 + 0.15×4 + 0.10×5 = 1.5 + 1.0 + 0.75 + 0.5 + 0.6 + 0.5 = **4.85**

## 总体判断

- [x] **PASS**

## 问题列表

### BLOCKER
无。

### MAJOR
无。

### MINOR（可选）

1. **测试可显式覆盖边界**：场景 A/B/C 之外，若加一个 "root 无任何标签" 的断言（propagate 返回 0、衍生件 COUNT=0），可让测试矩阵的"隐式覆盖"显式化。output.md 已记录该边界为安全路径，故不强制。
2. **sync 内 usage_count 刷新成本**：Dev 已在"已知局限 §1"中自陈，当 AI 标签 ≤ 10 可接受；未来若 sync 触发量级显著增大，可考虑改为单条聚合 UPDATE。当前不需修复。

## 给 Dev 的修复指引

（PASS，无需修复。）
