# Review Scorecard — task_004_dev_rust_co_occurrence

**Reviewer**：Claude Sonnet 4.6  
**Review 日期**：2026-04-11  
**整体判定**：**FIX**

---

## 各 AC 验收状态

| AC | 描述 | 状态 | 备注 |
|----|------|------|------|
| AC-1 | `compute_co_occurrence` 存在，两两配对，有交集写入 concept_relations | PASS | 实现正确，HashSet 交集逻辑无误 |
| AC-2 | `knowledge_compute_co_occurrence` Command 存在并注册 | PASS | lib.rs 已注册，knowledge.rs 末尾追加正确 |
| AC-3 | 关系方向 `concept_a_id < concept_b_id`（字典序） | PASS | 排序逻辑正确，有专项测试验证 |
| AC-4 | ON CONFLICT DO UPDATE，重复计算更新计数 | PASS（有保留） | ON CONFLICT 目标列与 UNIQUE INDEX 完全匹配；但 co_occurrence_count 语义不一致（见问题 1） |
| AC-5 | 空输入（0 或 1 个概念）返回 0 | PASS | `if n < 2 { return Ok(0) }` 正确实现 |
| AC-6 | 提取完成后调用共现计算，失败不阻断 | PASS（有保留） | 调用存在且失败不阻断；但 emit 顺序有时序隐患（见问题 3） |
| AC-7 | 50 个概念 ≤ 5s | PASS（待运行验证） | 纯内存 HashSet + 事务批量写入，设计上满足要求；output.md 未提供实测数据 |

---

## 审查重点核查

| 审查项 | 结果 | 说明 |
|--------|------|------|
| 零 LLM 调用 | PASS | co_occurrence.rs 全文无任何网络/LLM 调用 |
| 方向性约束（a < b） | PASS | 排序实现正确 |
| 幂等性（ON CONFLICT） | PASS | 目标列与 UNIQUE INDEX 完全匹配，无重复行 |
| 事务（BEGIN/COMMIT） | PASS（有风险） | 手动事务存在回滚后继续操作的隐患（见问题 2） |
| 空输入处理 | PASS | 正确返回 0 |
| 提取后调用 | PASS（有保留） | 存在时序语义问题（见问题 3） |
| 已有测试无回归 | 待验证 | output.md 未提供实际 cargo test 输出 |
| source_asset_ids 解析 | PASS | serde_json 正确解析，容错 NULL 和格式错误 |

---

## 评分

> 本 task 采用 task-level 权重（纯后端 SQLite task）

| 维度 | 权重 | 得分（0-10） | 加权得分 | 说明 |
|------|------|-------------|---------|------|
| 功能正确性 | 40% | 8.5 | 3.40 | 核心逻辑正确；co_occurrence_count 语义问题（-1）；事务回滚隐患（-0.5） |
| 安全性（无 LLM、不破坏数据） | 15% | 9.5 | 1.43 | 零 LLM 调用；只写 concept_relations 新表；外键 CASCADE 保护 |
| 代码质量 | 15% | 7.5 | 1.13 | 结构清晰；事务管理用手动 BEGIN/ROLLBACK 而非 rusqlite transaction API（-1.5）；回滚错误静默忽略（-1） |
| 测试覆盖 | 20% | 8.0 | 1.60 | 7 个测试覆盖全部 AC；未提供实际运行输出（-1）；TempDir 生命周期潜在问题（-0.5）；count2==1 的测试断言语义模糊（-0.5） |
| 架构一致性 | 5% | 9.0 | 0.45 | 模块放置符合扁平结构；Command 注册齐全；入参改为 library_id 的偏离合理且有说明 |
| 可维护性 | 5% | 8.0 | 0.40 | 注释充分；`relation_count` 返回值文档描述不准确（-1）；时序问题增加后期维护理解成本（-1） |
| **合计** | 100% | — | **6.41 / 10** | — |

---

## 整体判定：FIX

**理由**：无 BLOCKER 级问题（零 LLM 调用满足、ON CONFLICT 正确、核心算法正确）。但存在 **3 个 FIX 级问题**需修复后方可合并：

1. **[FIX-1] `co_occurrence_count` 语义统一**（问题 1）：首次插入改为固定 `1`，ON CONFLICT 继续 `+1`，消除语义歧义。
2. **[FIX-2] 事务管理改用 rusqlite `transaction()` API**（问题 2）：避免手动 ROLLBACK 后连接状态不一致的隐患。
3. **[FIX-3] 调整 `emit` 与共现计算的顺序**（问题 3）：先执行共现计算并释放锁，再发送 `concept-extraction-done` 事件，确保前端收到事件时 `concept_relations` 数据已就绪。

---

## Dev 修复后需补充

- 运行 `cargo test db::co_occurrence -- --nocapture` 并将输出贴入 output.md，确认所有 7 个测试通过
- 运行 `cargo test db::tests -- --nocapture` 确认已有测试无回归
- 确认 `cargo check` 零 warning

---

## 不要求修复（可接受）

- `relation_count` 文档注释模糊（问题 4）：修复 FIX-1 后，注释更新为"处理的共现对数（含新增和更新）"即可
- `TempDir` 生命周期（问题 5）：macOS 环境下无实际影响，可列入 TODO
- output.md 未提供实测数据（问题 6）：补充 cargo test 输出后关闭

---

## FIX Round 再审（2026-04-12）

### 三项修复验证结果

| FIX | 验证结果 | 说明 |
|-----|----------|------|
| FIX-1 | ✅ | `co_occurrence.rs` 第 72 行 INSERT VALUES 中 `co_occurrence_count` 已改为字面量 `1`；ON CONFLICT DO UPDATE 保持 `co_occurrence_count + 1`（第 74 行）；`shared_count` 变量已移除。语义完全统一。 |
| FIX-2 | ✅ | 第 31-32 行改用 `conn.unchecked_transaction()`，所有写操作通过 `tx.execute(...)` 执行，提交改为 `tx.commit()`（第 91 行）；手动 BEGIN / ROLLBACK / COMMIT 全部移除。`tx` Drop 时自动回滚，连接状态不一致隐患消除。使用 `unchecked_transaction` 而非 `transaction` 系因 `conn` 为 `&Connection` 不可变引用，技术选型合理。 |
| FIX-3 | ✅ | `knowledge.rs` 第 191-197 行：共现计算在独立块 `{ let conn = ...; compute_co_occurrence(...); }` 内完成并自动释放锁；`app.emit("notecapt/concept-extraction-done", ...)` 在第 200-203 行位于块外，顺序正确。前端收到事件时 `concept_relations` 数据已写入且连接锁已释放。 |

### 修复后各维度重新评分

| 维度 | 权重 | 原得分 | 新得分 | 加权得分 | 变化说明 |
|------|------|--------|--------|---------|---------|
| 功能正确性 | 40% | 8.5 | 9.5 | 3.80 | FIX-1 语义歧义（-1）已消除；FIX-2 事务隐患（-0.5）已消除 |
| 安全性（无 LLM、不破坏数据） | 15% | 9.5 | 9.5 | 1.43 | 无变化 |
| 代码质量 | 15% | 7.5 | 9.0 | 1.35 | FIX-2 改用 rusqlite 原生事务 API（+1.5）；回滚静默问题随 unchecked_transaction Drop 机制已解决（+1）；保留 -0.5 因 `unchecked_transaction` 比 `transaction` 稍弱类型安全（可接受）|
| 测试覆盖 | 20% | 8.0 | 8.5 | 1.70 | output.md 已补充实际 cargo test 输出（7 passed, 0 failed），问题 6 关闭（+0.5） |
| 架构一致性 | 5% | 9.0 | 9.0 | 0.45 | 无变化 |
| 可维护性 | 5% | 8.0 | 9.0 | 0.45 | FIX-3 顺序注释清晰说明业务原因（+0.5）；函数文档注释已更新（+0.5） |
| **合计** | 100% | — | — | **9.18 / 10** | — |

### 更新后综合评分：9.2/10

### 最终判定：PASS

**理由**：三项 FIX 级问题均已正确修复——FIX-1 首次插入语义统一为字面量 `1`；FIX-2 改用 `unchecked_transaction()` 消除手动事务回滚隐患；FIX-3 共现计算先于 `emit` 完成并释放锁，时序语义正确。cargo check 零错误，7 个单元测试全部通过，已有测试无回归。剩余可接受项（`TempDir` 生命周期、`relation_count` 注释措辞）不影响合并。
