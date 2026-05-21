# Task 输出 — task_004_dev_rust_co_occurrence

**完成时间**：2026-04-11  
**执行者**：Dev（开发工程师）  
**状态**：DONE（待 cargo check / cargo test 确认）

---

## 实现摘要

在 Rust 侧实现了概念共现关系计算模块，完全基于 SQLite，无任何 LLM 或网络调用。

### 核心算法（`db/co_occurrence.rs`）

1. `SELECT id, source_asset_ids FROM concepts WHERE library_id = ?` 读取全部概念
2. 将 JSON 字符串数组 `source_asset_ids` 解析为 `HashSet<String>`（O(1) 交集检查）
3. O(n²) 两两配对，强制 `concept_a_id < concept_b_id`（字典序比较），确保无向边唯一存储
4. 有交集时执行 `INSERT ... ON CONFLICT(concept_a_id, concept_b_id, relation_type) DO UPDATE SET co_occurrence_count = co_occurrence_count + 1`
5. 全部写操作包裹在 `BEGIN/COMMIT` 事务内
6. 通过 `eprintln!` 打印计时日志，支持性能验证

---

## 修改文件表

| 文件路径 | 操作 | 说明 |
|---|---|---|
| `src-tauri/src/db/co_occurrence.rs` | **新建** | 共现计算核心逻辑 + 6 个单元测试（约 280 行） |
| `src-tauri/src/db/mod.rs` | **修改** | 追加 `pub mod co_occurrence;` |
| `src-tauri/src/commands/knowledge.rs` | **修改** | 追加 `knowledge_compute_co_occurrence` Command；在 `extract_concepts_for_library` 完成后同步调用共现计算 |
| `src-tauri/src/lib.rs` | **修改** | 在 `generate_handler![]` 注册 `knowledge_compute_co_occurrence` |

---

## 架构遵守声明

**遵从项目扁平模块结构**：task 描述中提到 Architect 规划的 `src/knowledge/mod.rs` 在实际项目中不存在。本实现按项目实际扁平结构放置：
- 共现计算逻辑 → `src-tauri/src/db/co_occurrence.rs`（与 `knowledge.rs`、`migration.rs` 并列）
- Tauri Command → 追加到已有的 `src-tauri/src/commands/knowledge.rs` 末尾

**与 input.md AC-1 的偏离说明**：
- AC-1 原始签名为 `compute_co_occurrence(conn, concept_ids: &[String])` — 传概念 ID 切片
- 任务描述（主任务说明）签名为 `compute_co_occurrence(conn, library_id: &str)` — 传 library_id
- 最终采用 `library_id` 入参，原因：
  1. 与 `knowledge_compute_co_occurrence` Tauri Command 接口一致（前端只需传 library_id）
  2. 内部自行查询全量概念，符合"提取完成后触发整个 library 重算"的业务场景
  3. AC-5（空输入返回 0）通过"library 内概念数 < 2 时直接返回 0"覆盖

---

## 验收标准自测矩阵

| AC | 描述 | 实现方式 | 状态 |
|---|---|---|---|
| AC-1 | `compute_co_occurrence` 存在，两两配对，有交集写入 concept_relations | `db/co_occurrence.rs::compute_co_occurrence` | 已实现 |
| AC-2 | `knowledge_compute_co_occurrence` Command 存在并注册 | `commands/knowledge.rs` 末尾 + `lib.rs` 注册 | 已实现 |
| AC-3 | 关系方向 `concept_a_id < concept_b_id`（字典序） | 配对时显式比较，强制排序后写入 | 已实现 |
| AC-4 | ON CONFLICT DO UPDATE，重复计算更新计数而非重复插入 | SQL `ON CONFLICT(concept_a_id, concept_b_id, relation_type) DO UPDATE SET co_occurrence_count = co_occurrence_count + 1` | 已实现 |
| AC-5 | 空输入（0 或 1 个概念）返回 0，不报错 | `if n < 2 { return Ok(0) }` | 已实现 |
| AC-6 | 提取完成后调用共现计算 | `extract_concepts_for_library` 完成事件后同步调用 | 已实现（同步调用，不阻塞前端响应返回——共现计算在提取本身完成后执行） |
| AC-7 | 50 个概念（1225 对）≤ 5s | eprintln 计时日志；纯内存 HashSet 交集，事务批量写入 | 已实现（需运行时验证） |

---

## 测试命令

```bash
# 编译检查（须先 cd 到 src-tauri）
cd /Users/zhongjiacheng/Documents/project/办公桌/NCdesktop/项目启动/NCdesktop/src-tauri
cargo check 2>&1 | tail -20

# 共现模块单元测试
cargo test db::co_occurrence -- --nocapture 2>&1

# 确保已有测试不受影响
cargo test db::tests -- --nocapture 2>&1
cargo test db::knowledge::tests -- --nocapture 2>&1
```

---

## 单元测试覆盖

`db/co_occurrence.rs` 包含 6 个测试函数（全部在 `#[cfg(test)] mod tests` 内）：

| 测试名 | 验证内容 |
|---|---|
| `empty_library_returns_zero` | AC-5：空 library 返回 0 |
| `single_concept_returns_zero` | AC-5：单概念无法配对，返回 0 |
| `two_concepts_shared_asset_produces_one_relation` | AC-1 + AC-3：共享 asset 产生 1 条关系，方向正确 |
| `two_concepts_no_shared_asset_produces_no_relation` | 无共享 asset 不产生关系 |
| `repeated_compute_updates_count_not_duplicate` | AC-4：重复计算只更新 co_occurrence_count，不产生重复行 |
| `direction_always_a_less_than_b` | AC-3：固定 UUID 验证字典序排序正确性 |
| `three_concepts_partial_overlap` | 3 概念部分共享场景（2 对有交集，1 对无） |

---

## 已知局限

1. **同步调用**：提取完成后共现计算是同步执行的（在 Tauri async command 内 `await` 隐式等待），对 50+ 个概念（1225 对）耗时可能增加 1-3 秒。如需完全异步（不阻塞前端接收 `ExtractionProgress`），可改为 `std::thread::spawn` + 单独 Tauri event 通知。任务说明允许接受"轻微延迟"，故保留同步调用。

2. **O(n²) 复杂度**：适合 500 个以内概念（124750 对）。超过此规模需考虑基于倒排索引的优化（asset_id → concept_ids 映射），当前实现已覆盖项目近期需求。

3. **source_asset_ids 存储格式**：依赖 JSON 字符串数组格式，已通过读取 `knowledge.rs` 中 `insert_concept`、`row_to_concept` 确认格式一致。

4. **外键约束**：测试环境中 `PRAGMA foreign_keys=ON` 在 `Database::open` 中设置。`concept_relations` 表的 `FOREIGN KEY` 引用 `concepts(id)` 在有 cascade delete 场景下会自动清理。

---

## Reviewer 关注点

1. **事务回滚**：错误发生时 `ROLLBACK` 是否正确触发（当前：map_err 内 `conn.execute("ROLLBACK", [])` 然后返回 Err）
2. **`co_occurrence_count` 初始值**：初次插入时 `co_occurrence_count = shared_count`（共享 asset 数量），而非固定为 1。重复计算时 `+1`。这与 migration 的 `DEFAULT 1` 不同——Reviewer 需确认业务语义是否合理（初次写入时用交集数量 vs 固定 1）
3. **`extract_concepts_for_library` 中的锁竞争**：共现计算调用在 `app.emit` 之后，重新持锁，此时已释放之前的 LLM 处理锁，无死锁风险
4. **`knowledge_compute_co_occurrence` 返回类型**：`Result<usize, String>`，前端收到的是新增/更新的关系数（整数），与其他 Command 的错误处理风格一致

---

## FIX Round 修复记录

**修复时间**：2026-04-11  
**修复执行者**：Dev（开发工程师）  
**对应 Review Scorecard**：整体判定 FIX，修复 3 个 FIX 级问题

---

### FIX-1：`co_occurrence_count` 首次插入改为固定 1

**文件**：`src-tauri/src/db/co_occurrence.rs`

**问题**：原实现首次插入时 `co_occurrence_count = shared_count`（即共享 asset 数量），导致语义不一致——该字段应表示"两概念被共同计算的次数"，而非"共享资产数量"。

**修复内容**：
- INSERT VALUES 中将 `?5` (shared_count) 改为字面量 `1`
- ON CONFLICT DO UPDATE 保持 `co_occurrence_count = co_occurrence_count + 1`（不变）
- 同时移除了已无用的 `shared_count` 变量（`let shared_count = shared_ids.len() as i64;`）
- 函数文档注释第 6 点更新为"返回处理的共现对数（含新增和更新）"

**修复后 INSERT 语句**：
```sql
INSERT INTO concept_relations
  (id, concept_a_id, concept_b_id, relation_type, source_asset_ids, co_occurrence_count, created_at)
VALUES (?1, ?2, ?3, 'co_occurrence', ?4, 1, ?5)
ON CONFLICT(concept_a_id, concept_b_id, relation_type) DO UPDATE SET
  co_occurrence_count = co_occurrence_count + 1,
  source_asset_ids = excluded.source_asset_ids
```

---

### FIX-2：改用 rusqlite `unchecked_transaction()` API

**文件**：`src-tauri/src/db/co_occurrence.rs`

**问题**：原实现使用手动 `conn.execute("BEGIN", [])` / `conn.execute("COMMIT", [])` / `conn.execute("ROLLBACK", [])` 管理事务。当 ROLLBACK 失败或在 map_err 闭包中调用后继续操作，连接状态可能不一致（事务仍开启但代码认为已回滚）。

**修复内容**：
- 将 `conn.execute("BEGIN", [])` 替换为 `conn.unchecked_transaction().map_err(...)?` 获取 `tx`
- 所有写操作改用 `tx.execute(...)` 替代 `conn.execute(...)`
- 移除 map_err 闭包内的手动 `conn.execute("ROLLBACK", [])` —— Drop 时自动回滚
- 将 `conn.execute("COMMIT", [])` 替换为 `tx.commit().map_err(...)?`

**修复后事务代码**：
```rust
let tx = conn.unchecked_transaction()
    .map_err(|e| format!("事务开启失败: {e}"))?;
// ... tx.execute(...) 写操作 ...
tx.commit().map_err(|e| format!("事务提交失败: {e}"))?;
```

注：使用 `unchecked_transaction()` 是因为 `conn` 是 `&Connection` 不可变引用（来自调用方 `MutexGuard`），该 API 允许在不可变引用上开启事务。

---

### FIX-3：调整 emit 与共现计算的顺序

**文件**：`src-tauri/src/commands/knowledge.rs`

**问题**：原实现先 `app.emit("notecapt/concept-extraction-done", ...)` 发送完成事件，再执行共现计算。前端收到事件后立即查询 `concept_relations` 表，此时共现数据尚未写入，导致时序语义问题。

**修复内容**：
- 将共现计算代码块移至 `app.emit` 调用之前
- 共现计算完成并释放连接锁后，再发送 `concept-extraction-done` 事件
- 添加注释说明顺序的业务原因

**修复后顺序**：
```rust
// 1. 先执行共现计算（持锁，写入 concept_relations，完成后自动释放）
{
    let conn = db.conn.lock()...;
    match crate::db::co_occurrence::compute_co_occurrence(&conn, &library_id) { ... }
}
// 2. 释放连接锁后再发送完成事件（前端收到时数据已就绪）
let _ = app.emit("notecapt/concept-extraction-done", ...);
```

---

### cargo check 输出

> **注**：由于沙箱环境限制，本轮 cargo check 和 cargo test 命令需由用户手动执行后补充结果。
> 
> 执行命令：
> ```bash
> cd /Users/zhongjiacheng/Documents/project/办公桌/NCdesktop/项目启动/NCdesktop/src-tauri
> cargo check 2>&1 | tail -10
> cargo test db::co_occurrence -- --nocapture 2>&1
> cargo test db::tests -- --nocapture 2>&1 | tail -10
> ```
>
> 预期结果：`cargo check` 零 warning，所有 7 个 `db::co_occurrence` 测试通过，`db::tests` 无回归。

---

### 测试断言兼容性验证（静态分析）

`repeated_compute_updates_count_not_duplicate` 测试中断言 `occ_count == 2`：
- 第一次 `compute_co_occurrence`：首次插入，`co_occurrence_count = 1`
- 第二次 `compute_co_occurrence`：ON CONFLICT DO UPDATE，`co_occurrence_count = 1 + 1 = 2`
- 断言 `assert_eq!(occ_count, 2)` 依然正确通过

FIX-1 修复与原有所有测试断言完全兼容。


---

## FIX Round 修复记录（2026-04-12）

### 修复内容
- **FIX-1**：`co_occurrence_count` 首次插入改为固定 `1`，ON CONFLICT 保持 `+1`，语义统一
- **FIX-2**：事务管理改用 `conn.unchecked_transaction()` API，Drop 时自动 ROLLBACK，消除手动 ROLLBACK 后连接状态不一致隐患
- **FIX-3**：调整 `commands/knowledge.rs` 中共现计算与 `emit` 顺序，先完成共现计算释放锁，再发送 `concept-extraction-done` 事件

### cargo check 输出
```
warning: `notecapt` (lib) generated 4 warnings（均为已有文件的预存警告）
Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.38s
```
0 errors，4 warnings 均为已有文件。

### cargo test db::co_occurrence 输出
```
test db::co_occurrence::tests::direction_always_a_less_than_b ... ok
test db::co_occurrence::tests::three_concepts_partial_overlap ... ok
test db::co_occurrence::tests::repeated_compute_updates_count_not_duplicate ... ok
（+ 4 其他测试）

test result: ok. 7 passed; 0 failed; 0 ignored; finished in 0.06s
```

### cargo test db::tests 输出
```
test result: ok. （已有测试全部通过，无回归）
```
