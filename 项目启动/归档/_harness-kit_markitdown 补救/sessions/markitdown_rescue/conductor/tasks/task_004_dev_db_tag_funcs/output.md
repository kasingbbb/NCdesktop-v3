# Task 交付 — task_004_dev_db_tag_funcs

## 实现摘要

在 `db/tag.rs` 紧跟 `link_to_asset` 之后新增两个标签传播函数，并在 `commands/dropzone.rs::apply_llm_classify_to_asset` 的 AI 打标循环结束之后追加 `sync_tags_to_canonical_derivatives` 调用（失败仅 `log::warn!`，不阻断主流程，符合 AC-3）。两个新函数全部使用 `INSERT OR IGNORE INTO asset_tags ... SELECT ...` 参数化语句，保证幂等且 SQL 注入安全。

核心设计决策：

1. **三处唯一性（AC-5）**：全仓 `INSERT (OR IGNORE )?INTO asset_tags` 仅出现在 `link_to_asset` / `propagate_tags_to_derivative` / `sync_tags_to_canonical_derivatives`，grep 已证实（见下方"特别关注"）。
2. **usage_count 维护**：两个新函数在实际产生新行（`inserted > 0`）时刷新被影响的 tag 的 usage_count，保持与 `link_to_asset` 语义一致；零写入时跳过刷新，避免无效查询。
3. **dropzone 集成位置**：放在 `for tag_name in r.tags` 循环结束之后、`Ok(())` 之前，复用同一个 `MutexGuard<Connection>` 作用域。`db::tag::sync_tags_to_canonical_derivatives` 接收 `&Connection`，传 `&conn` 由 `MutexGuard` 自动解引用，无需额外 drop / re-lock。
4. **不动 scheduler / extraction/mod.rs:4**：本次未触碰 `extraction/mod.rs` 任何一行，task_008 的活保留。
5. **不动前端**：本次仅修改 2 个 Rust 文件，未触碰 `src/`。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `src-tauri/src/db/tag.rs` | 修改 | 新增 `propagate_tags_to_derivative`、`sync_tags_to_canonical_derivatives` 两个 pub fn，以及辅助函数 `refresh_usage_count_for_asset_tags`，并追加 `#[cfg(test)] mod tests`（3 个场景） |
| `src-tauri/src/commands/dropzone.rs` | 修改 | 在 `apply_llm_classify_to_asset` 的 `link_to_asset` 循环结束后，追加 `sync_tags_to_canonical_derivatives` 调用，失败仅 `log::warn!` |

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（仍在 `db/tag.rs` 内扩展，未引入新模块）
- [x] API 路径/命名与 Architect 方案一致（函数签名严格按 input.md §AC-1/AC-2 给定的 SQL 模板）
- [x] 数据模型与 Architect 方案一致（依赖 V5 迁移引入的 `assets.source_asset_id` + `asset_type='markdown'` 约定）
- [x] 未引入计划外的新依赖（仅使用已存在的 `rusqlite`、`uuid`、`log`）
- 偏离说明：无

## 测试命令

```bash
cd /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri
cargo check 2>&1 | tail -50
cargo test --lib db::tag 2>&1 | tail -40
```

## 测试结果

`cargo check`（**0 error**，4 个与本任务无关的 warning，全部来自 `src/llm/chat.rs`）：

```
    Checking notecapt v0.1.0 (/Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri)
warning: unused variable: `client` (src/llm/chat.rs:109)
warning: unused variable: `messages` (src/llm/chat.rs:110)
warning: unused variable: `on_chunk` (src/llm/chat.rs:111)
warning: fields `block_type` and `thinking` are never read (src/llm/chat.rs:47)
warning: `notecapt` (lib) generated 4 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.30s
```

`cargo test --lib db::tag`（**3 passed, 0 failed**）：

```
running 3 tests
test db::tag::tests::scenario_c_propagate_is_idempotent ... ok
test db::tag::tests::scenario_b_sync_to_existing_derivatives ... ok
test db::tag::tests::scenario_a_propagate_copies_all_tags ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 37 filtered out; finished in 0.04s
```

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|----------|----------|------|-----------|
| ✅ 正常路径 | A：原件已绑定 2 个标签，对一个 markdown 衍生件调 propagate，衍生件应拿到 2 个标签 | 已测 | PASS（inserted=2，COUNT=2） |
| ✅ 正常路径 | B：原件有 2 个 markdown 衍生件 + 1 个 image 衍生件，原件被 AI 打 2 个标签后调 sync，2 个 .md 各拿 2 标签（共 4 行），image 衍生件不受影响 | 已测 | PASS（inserted=4，markdown 各 COUNT=2，image COUNT=0） |
| ✅ 幂等性 | C：对同一对 (root, deriv) 连续调用 propagate 3 次 | 已测 | PASS（首次 inserted=1，后两次 inserted=0，最终 COUNT=1） |
| ⚠️ 边界条件 | 原件无任何标签时调用 propagate / sync | 隐式覆盖 | SQL `SELECT ... WHERE asset_id=?` 返回 0 行，`execute` 返回 0，函数返回 `Ok(0)`，不刷新 usage_count，安全 |
| ⚠️ 边界条件 | sync 时原件无任何 markdown 衍生件 | 隐式覆盖（场景 B 之外的退化情形） | JOIN 结果为空，`execute` 返回 0，安全 |
| ❌ 异常路径 | dropzone 中 `sync_tags_to_canonical_derivatives` 返回 Err | 已测（代码路径） | 仅 `log::warn!`，不返回 Err，AI 打标主流程不被阻断（符合 AC-3） |
| ❌ 异常路径 | SQL 注入风险 | 已测（人工审计） | 所有 SQL 走 `params![...]` 参数绑定，无字符串拼接 |
| ❌ 异常路径 | unwrap/expect 在非测试代码 | 已测（grep） | 生产代码 0 处 unwrap/expect；仅测试模块内使用 expect/unwrap |

## 已知局限

1. **usage_count 在 sync 大量写入时的开销**：`sync_tags_to_canonical_derivatives` 在 `inserted > 0` 时会查询 `asset_tags WHERE asset_id = root` 取出 tag_id 列表逐个 `refresh_tag_usage_count`。若原件标签很多，会触发多次 UPDATE。当前 AI 标签上限通常 ≤ 10，可接受；若未来批量很大，可优化为单条 UPDATE 全表刷新影响行。
2. **未在 markdown 衍生件 INSERT 处调用 propagate**：input.md §AC-1 仅要求新增函数本身，"在 MarkItDown 转换完成后调用 propagate_tags_to_derivative"应由 task_005/006（MarkItDown 集成）负责接线，本任务范围之外。当前 dropzone 调用的是 sync（解决"先转换、后 AI 打标"时序漏洞），与 propagate 互补。
3. **未引入 transaction 包裹**：sync 内部"INSERT + 多次刷新 usage_count"非原子，若中途崩溃可能造成 usage_count 临时不一致；由于 `refresh_tag_usage_count` 本身是 `UPDATE tags SET usage_count = (SELECT COUNT(*) ...)`，下一次调用会自动纠正，可接受。

## 需要 Reviewer 特别关注的地方

### 1. AC-5 grep 证据（全仓 `INSERT (OR IGNORE )?INTO asset_tags` 仅出现在 3 处）

执行命令：
```bash
grep -rn "INSERT INTO asset_tags\|INSERT OR IGNORE INTO asset_tags" /Users/zhongjiacheng/Documents/project/WorkDesk/NCdesktop/项目启动/NCdesktop/src-tauri/src/
```

输出（**3 行，对应 3 个函数**）：
```
src/db/tag.rs:108:        "INSERT OR IGNORE INTO asset_tags (asset_id, tag_id) VALUES (?1, ?2)",        // link_to_asset
src/db/tag.rs:136:            "INSERT OR IGNORE INTO asset_tags (asset_id, tag_id) \                    // propagate_tags_to_derivative
src/db/tag.rs:162:            "INSERT OR IGNORE INTO asset_tags (asset_id, tag_id) \                    // sync_tags_to_canonical_derivatives
```

- 第 108 行：`link_to_asset`（既有，未改动）
- 第 136 行：`propagate_tags_to_derivative`（本次新增）
- 第 162 行：`sync_tags_to_canonical_derivatives`（本次新增）

满足 R6"防止标签传播多处实现"硬约束。

### 2. M-1 验证（cargo check 错误数）

`cargo check` 仍为 **0 error**，与 task_002/003 末态一致。dropzone 的新调用未引入对 scheduler 的依赖，未取消 `extraction/mod.rs:4` 注释。

### 3. dropzone 调用点位

`src-tauri/src/commands/dropzone.rs::apply_llm_classify_to_asset` 中，`conn` 是 `MutexGuard<Connection>`，新调用使用 `&conn` 自动解引用为 `&Connection`，与同函数内其他 `db::*` 调用风格一致。调用置于 `for tag_name in r.tags { ... }` 循环之后、`Ok(())` 之前，确保所有 AI 标签都已写入原件后再触发同步。

### 4. dropzone.rs 中是否还有其他 `link_to_asset` 路径需要追加 sync？

只有 `apply_llm_classify_to_asset` 一处使用 `link_to_asset`（grep 已确认：`grep -n "link_to_asset" src-tauri/src/commands/dropzone.rs` 仅返回 1 处）。其他 `link_to_asset` 调用方（如 `commands/tag.rs`）属于用户主动打标场景，是否也要同步到衍生件由 task_005+ 决定，本任务不引入。
