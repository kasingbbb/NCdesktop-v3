# Code Review — task_004_dev_rust_co_occurrence

**Reviewer**：Claude Sonnet 4.6  
**Review 日期**：2026-04-11  
**审查范围**：`src-tauri/src/db/co_occurrence.rs`（全文），`src-tauri/src/commands/knowledge.rs`（追加部分：第 194-201 行共现调用、第 534-548 行新 Command）

---

## 总体评价

实现整体扎实，核心算法正确，测试覆盖充分（7 个单元测试覆盖全部 AC）。发现 **1 个 FIX 级问题**（`co_occurrence_count` 语义不一致）、**2 个 FIX 级问题**（回滚后异常处理、事件/返回值时序语义）、**3 个 PASS 级建议**（文档/代码质量层面）。无 BLOCKER。

---

## 问题列表

### 问题 1 — `co_occurrence_count` 首次写入语义与后续更新不一致 【FIX】

**文件**：`src-tauri/src/db/co_occurrence.rs`，第 64–76 行

**问题**：
首次插入时 `co_occurrence_count = shared_count`（共享 asset 数量，可能为 1、2、3……），但 ON CONFLICT 更新时固定执行 `co_occurrence_count + 1`。这导致该字段的语义在两次调用之间发生混淆：
- 第一次：count = "两个概念共享了多少个 asset"
- 第二次：count = "第一次的值 + 1"（不再是共享 asset 数）

Dev 在 output.md 中已意识到这个问题（"Reviewer 关注点 2"），但未修复。

**影响**：
- 前端若用 `co_occurrence_count` 判断关系强度，在重复计算后值会产生误导性增长
- 测试 `repeated_compute_updates_count_not_duplicate` 断言 `occ_count == 2`，是基于"1 个共享 asset 首次设为 1，第二次 +1 = 2"的巧合通过，并未验证多 asset 场景下的语义

**建议修复**：统一语义，二选一：
- **方案 A（推荐）**：始终以"计算次数"为语义，首次插入固定写 `co_occurrence_count = 1`，ON CONFLICT 做 `+1`
- **方案 B**：首次插入写 `shared_count`，ON CONFLICT 更新 `source_asset_ids = excluded.source_asset_ids`（用最新的交集集合替换，不做加法）

```rust
// 方案 A：首次固定 1，重复时 +1
conn.execute(
    "INSERT INTO concept_relations
       (id, concept_a_id, concept_b_id, relation_type, source_asset_ids, co_occurrence_count, created_at)
     VALUES (?1, ?2, ?3, 'co_occurrence', ?4, 1, ?5)
     ON CONFLICT(concept_a_id, concept_b_id, relation_type) DO UPDATE SET
       co_occurrence_count = co_occurrence_count + 1,
       source_asset_ids = excluded.source_asset_ids",
    params![relation_id, concept_a_id, concept_b_id, shared_json, now],
)?;
```

---

### 问题 2 — 事务回滚后继续使用同一连接，错误未完全冒泡 【FIX】

**文件**：`src-tauri/src/db/co_occurrence.rs`，第 86–90 行

**问题**：
```rust
.map_err(|e| {
    let _ = conn.execute("ROLLBACK", []); // 忽略 ROLLBACK 失败
    format!("写入共现关系失败: {e}")
})?;
```
有两个子问题：
1. `ROLLBACK` 失败时错误被静默忽略（`let _ = ...`）。若 ROLLBACK 本身失败，连接可能处于未定义事务状态，后续调用（如 COMMIT）会产生混乱错误，难以诊断。
2. 若中间某对写入失败触发 ROLLBACK，之后对 `conn.execute("COMMIT")` 的调用（第 97 行）将在已回滚的连接上执行，SQLite 会报错（"cannot commit - no transaction is active"），而这个错误会作为"提交事务失败"返回，掩盖了真正的根因。

**建议修复**：使用 rusqlite 的 `conn.transaction()` API 自动管理事务，避免手动 BEGIN/ROLLBACK/COMMIT：

```rust
let tx = conn.unchecked_transaction()
    .map_err(|e| format!("开启事务失败: {e}"))?;

for i in 0..n {
    for j in (i + 1)..n {
        // ... 计算逻辑 ...
        tx.execute("INSERT INTO concept_relations ...", params![...])?;
        relation_count += 1;
    }
}

tx.commit().map_err(|e| format!("提交事务失败: {e}"))?;
```
若任意写入失败，`tx` 的 `Drop` 自动执行 ROLLBACK，无需手动处理。

---

### 问题 3 — `app.emit("concept-extraction-done")` 在共现计算之前触发，但 Command 仍阻塞 【FIX（语义）】

**文件**：`src-tauri/src/commands/knowledge.rs`，第 189–209 行

**问题**：
```rust
// 第 189 行：先发送"完成"事件
let _ = app.emit("notecapt/concept-extraction-done", ...);

// 第 195-201 行：再同步执行共现计算（持锁，可能耗时 1-3s）
{
    let conn = db.conn.lock()?;
    match crate::db::co_occurrence::compute_co_occurrence(&conn, &library_id) { ... }
}

// 第 203 行：函数最终 Ok 返回
Ok(final_progress)
```

前端收到 `concept-extraction-done` 事件后，可能立刻调用 `knowledge_compute_co_occurrence` 或触发读取 `concept_relations` 的 UI 更新。但此时后端的 `extract_concepts_for_library` 仍持有 `db.conn` 互斥锁进行共现计算，导致：
- 前端调用 `knowledge_compute_co_occurrence` → 等待锁 → 超时或卡顿
- 前端读取 `concept_relations` 数据 → 可能读到空结果（事务未提交）

**建议修复**：将共现计算移到 `emit` 之前，或使用 `std::thread::spawn` 完全异步化并通过独立事件通知完成：

```rust
// 方案：先计算，再发送完成事件（简单有效）
{
    let conn = db.conn.lock().map_err(|e| format!("数据库锁获取失败: {e}"))?;
    match crate::db::co_occurrence::compute_co_occurrence(&conn, &library_id) {
        Ok(n) => log::info!("共现关系计算完成，新增/更新 {n} 条关系"),
        Err(e) => log::warn!("共现关系计算失败（不影响提取结果）: {e}"),
    }
} // 锁释放

// 释放锁后再发事件，前端收到时共现数据已就绪
let _ = app.emit("notecapt/concept-extraction-done", ...);
```

---

### 问题 4 — `relation_count` 返回值语义描述与实际行为不符 【PASS / 文档问题】

**文件**：`src-tauri/src/db/co_occurrence.rs`，第 13-16 行注释及 `commands/knowledge.rs` 第 537-539 行注释

**问题**：
注释说"返回新增关系记录数"，但实际上在 ON CONFLICT 路径（已有关系被更新）时，`relation_count` 也会自增。所以返回值是"处理的有交集概念对数"（既包含新插入也包含 ON CONFLICT 更新），而非"新增行数"。

测试 `repeated_compute_updates_count_not_duplicate` 中 `count2 == 1` 实际验证的是"处理了 1 对有交集概念"，不是"新增 1 条记录"。

**建议**：更新函数文档注释，明确为"返回处理的共现关系对数（含新增和更新）"。

---

### 问题 5 — 测试中 `open_db()` 创建临时目录后立即可能被 Drop 【PASS / 潜在不稳定】

**文件**：`src-tauri/src/db/co_occurrence.rs`，第 154-157 行

**问题**：
```rust
fn open_db() -> Database {
    let dir = tempfile::tempdir().expect("tempdir");
    Database::open(&dir.path().join("co_occ_test.db")).expect("open db")
}
```
`dir`（`TempDir`）在函数返回时被 drop，临时目录被删除。但 `Database` 内部持有 `Connection`，而 `Connection` 持有文件路径引用（WAL 模式下还有 `-wal` 和 `-shm` 辅助文件）。在大多数平台（macOS/Linux）上，打开的文件描述符在目录删除后仍有效，测试不会失败；但在 Windows 上，目录无法被删除（因为文件句柄仍持有引用），可能导致 `tempdir()` 报错或清理失败。

**建议**：在测试结构体中保留 `TempDir` 的生命周期：
```rust
struct TestDb {
    _dir: tempfile::TempDir,  // 保持生命周期，函数结束时再 drop
    db: Database,
}
fn open_db() -> TestDb {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = Database::open(&dir.path().join("co_occ_test.db")).expect("open db");
    TestDb { _dir: dir, db }
}
```

---

### 问题 6 — output.md 未提供实际 cargo test 运行结果 【PASS / 交付物完整性】

**文件**：output.md，状态行"DONE（待 cargo check / cargo test 确认）"

**问题**：
output.md 中标注状态为"待确认"，说明 Dev 未实际运行 `cargo test db::co_occurrence`。测试函数本身逻辑正确，但未经过 CI/编译验证即提交，存在潜在编译错误风险（例如 `crate::db::Database` 的字段访问 `db.conn` 是否为 `pub` 需运行时验证）。

**建议**：在 Scorecard 中要求 Dev 补充实际测试输出截图/日志后方可 PASS。

---

## 优点记录

1. **零 LLM 调用**：`co_occurrence.rs` 全文无任何网络调用、无 `LLMClient` 引用，满足技术约束的最高优先级要求。
2. **方向性约束正确**：`if id_a_raw < id_b_raw` 比较和交换逻辑正确，确保 `concept_a_id < concept_b_id`（字典序），无向边唯一存储。
3. **ON CONFLICT 目标列正确**：`ON CONFLICT(concept_a_id, concept_b_id, relation_type)` 与 migration.rs 中创建的 `idx_concept_relations_pair UNIQUE INDEX` 完全吻合。
4. **AC-5 空输入处理**：`if n < 2 { return Ok(0) }` 正确覆盖 0 个和 1 个概念的场景。
5. **serde_json 解析正确**：`serde_json::from_str::<Vec<String>>(s).ok()` 正确处理 NULL 和格式错误的 JSON，降级为空 HashSet，不 panic。
6. **测试覆盖全面**：7 个测试完整覆盖 7 个 AC，包含边界场景（空库、单概念、无交集）和关键场景（方向性、幂等性、多概念部分重叠）。
7. **Command 注册完整**：`knowledge_compute_co_occurrence` 已在 `lib.rs` 的 `invoke_handler` 中注册，`db/mod.rs` 已追加 `pub mod co_occurrence;`。
8. **提取失败不阻断**：共现计算失败时使用 `log::warn!` 记录并继续，不影响提取结果返回。
